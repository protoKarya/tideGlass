// SPDX-License-Identifier: AGPL-3.0-or-later

//! Weighted Kolmogorov–Smirnov enrichment and RGES computation.

use std::collections::HashSet;
use std::hash::BuildHasher;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::error::{Result, TideGlassError};
use crate::types::{DiseaseSignature, EnrichmentConfig, GeneId, PerturbationSignature, RgesResult};

/// Computes weighted Kolmogorov–Smirnov enrichment of a query gene set against a ranked list.
///
/// Genes in `ranked_genes` are ordered from most to least significant. For each gene:
/// - if it is in `query_genes`, the running sum increases by `weight / N_hits`;
/// - otherwise, the running sum decreases by `1 / N_miss`.
///
/// The returned score is the signed maximum deviation from zero over the walk.
///
/// # Errors
///
/// Returns [`TideGlassError::EmptySignature`] if either input list is empty,
/// [`TideGlassError::InsufficientData`] if no ranked genes overlap the query set,
/// or [`TideGlassError::Enrichment`] if weights length mismatches `ranked_genes`.
#[must_use = "enrichment score must be checked for significance"]
pub fn weighted_ks_enrichment<S: BuildHasher>(
    query_genes: &HashSet<&GeneId, S>,
    ranked_genes: &[GeneId],
    weights: Option<&[f64]>,
    weight_exponent: f64,
) -> Result<f64> {
    if query_genes.is_empty() {
        return Err(TideGlassError::EmptySignature {
            context: "query gene set".to_owned(),
        });
    }
    if ranked_genes.is_empty() {
        return Err(TideGlassError::EmptySignature {
            context: "ranked gene list".to_owned(),
        });
    }
    if let Some(w) = weights {
        if w.len() != ranked_genes.len() {
            return Err(TideGlassError::Enrichment {
                reason: format!(
                    "weights length {} does not match ranked genes length {}",
                    w.len(),
                    ranked_genes.len()
                ),
            });
        }
    }

    let n_hits = ranked_genes
        .iter()
        .filter(|gene| query_genes.contains(gene))
        .count();
    let n = ranked_genes.len();
    let n_miss = n.saturating_sub(n_hits);

    if n_hits == 0 {
        return Err(TideGlassError::InsufficientData {
            required: 1,
            actual: 0,
        });
    }
    if n_miss == 0 {
        return Err(TideGlassError::Enrichment {
            reason: "all ranked genes are in the query set; KS statistic is undefined".to_owned(),
        });
    }

    let n_hits_f = crate::count_as_f64(n_hits);
    let n_miss_f = crate::count_as_f64(n_miss);

    let mut running = 0.0_f64;
    let mut best_score = 0.0_f64;

    for (index, gene) in ranked_genes.iter().enumerate() {
        if query_genes.contains(gene) {
            let weight = gene_weight(weights, index, weight_exponent);
            running += weight / n_hits_f;
        } else {
            running -= 1.0 / n_miss_f;
        }

        if running.abs() > best_score.abs() {
            best_score = running;
        }
    }

    Ok(best_score)
}

/// Estimates a permutation p-value for an observed weighted KS enrichment score.
///
/// Gene labels are shuffled on the ranked list while preserving hit count; the fraction
/// of permuted scores with absolute value at or above the observed score is returned.
/// Uses `(count + 1) / (n_permutations + 1)` to avoid zero p-values.
///
/// # Errors
///
/// Propagates errors from [`weighted_ks_enrichment`] or returns
/// [`TideGlassError::Permutation`] when permutation count is zero.
#[must_use = "p-value must be compared against significance threshold"]
pub fn permutation_p_value<S: BuildHasher>(
    observed_score: f64,
    query_genes: &HashSet<&GeneId, S>,
    ranked_genes: &[GeneId],
    weights: Option<&[f64]>,
    config: &EnrichmentConfig,
    rng: &mut impl Rng,
) -> Result<f64> {
    if config.n_permutations == 0 {
        return Err(TideGlassError::Permutation {
            reason: "n_permutations must be greater than zero".to_owned(),
        });
    }

    let n_hits = ranked_genes
        .iter()
        .filter(|gene| query_genes.contains(gene))
        .count();

    if n_hits == 0 {
        return Err(TideGlassError::InsufficientData {
            required: 1,
            actual: 0,
        });
    }

    let n = ranked_genes.len();
    let mut indices: Vec<usize> = (0..n).collect();
    let observed_abs = observed_score.abs();
    let mut count_ge = 0_u32;

    for _ in 0..config.n_permutations {
        indices.shuffle(rng);
        let permuted_hits: HashSet<&GeneId> = indices[..n_hits]
            .iter()
            .map(|&index| &ranked_genes[index])
            .collect();

        let perm_score = weighted_ks_enrichment(
            &permuted_hits,
            ranked_genes,
            weights,
            config.weight_exponent,
        )?;

        if perm_score.abs() >= observed_abs {
            count_ge = count_ge.saturating_add(1);
        }
    }

    let numerator = f64::from(count_ge.saturating_add(1));
    let denominator = f64::from(config.n_permutations.saturating_add(1));

    Ok(numerator / denominator)
}

/// Computes RGES for a disease signature against a slice of perturbation profiles.
///
/// For each perturbation, up-reversal enriches disease-up genes in the drug-down signature;
/// down-reversal enriches disease-down genes in the drug-up signature. The RGES score is
/// the midpoint of both reversals; p-values are combined the same way.
///
/// # Errors
///
/// Returns [`TideGlassError::EmptySignature`] if the disease signature has no genes,
/// [`TideGlassError::InsufficientData`] if `perturbations` is empty or gene sets are too small,
/// or enrichment/permutation errors from underlying computations.
#[must_use = "RGES results drive compound ranking"]
pub fn compute_rges(
    disease: &DiseaseSignature,
    perturbations: &[PerturbationSignature],
    config: &EnrichmentConfig,
) -> Result<Vec<RgesResult>> {
    if disease.up_genes.is_empty() && disease.down_genes.is_empty() {
        return Err(TideGlassError::EmptySignature {
            context: "disease signature".to_owned(),
        });
    }
    if perturbations.is_empty() {
        return Err(TideGlassError::InsufficientData {
            required: 1,
            actual: 0,
        });
    }

    let up_query: HashSet<&GeneId> = disease.up_genes.iter().collect();
    let down_query: HashSet<&GeneId> = disease.down_genes.iter().collect();

    validate_gene_set_size(&up_query, config)?;
    validate_gene_set_size(&down_query, config)?;

    let mut results = Vec::with_capacity(perturbations.len());

    for perturbation in perturbations {
        let up_reversal =
            compute_directional_reversal(&up_query, &perturbation.down_genes, config)?;
        let down_reversal =
            compute_directional_reversal(&down_query, &perturbation.up_genes, config)?;

        let rges_score = f64::midpoint(up_reversal.score, down_reversal.score);
        let p_up = up_reversal.p_value;
        let p_down = down_reversal.p_value;
        let p_value = f64::midpoint(p_up, p_down);

        results.push(RgesResult {
            compound_id: perturbation.compound_id.clone(),
            rges_score,
            p_value,
            reversal_strength: rges_score.abs(),
            n_permutations: config.n_permutations,
        });
    }

    Ok(results)
}

struct DirectionalReversal {
    score: f64,
    p_value: f64,
}

fn compute_directional_reversal(
    query_genes: &HashSet<&GeneId>,
    ranked_genes: &[GeneId],
    config: &EnrichmentConfig,
) -> Result<DirectionalReversal> {
    if query_genes.is_empty() || ranked_genes.is_empty() {
        return Ok(DirectionalReversal {
            score: 0.0,
            p_value: 1.0,
        });
    }

    validate_gene_set_size(query_genes, config)?;

    let score = weighted_ks_enrichment(query_genes, ranked_genes, None, config.weight_exponent)?;

    let mut rng = rand::rng();
    let p_value = permutation_p_value(score, query_genes, ranked_genes, None, config, &mut rng)?;

    Ok(DirectionalReversal { score, p_value })
}

fn validate_gene_set_size(genes: &HashSet<&GeneId>, config: &EnrichmentConfig) -> Result<()> {
    if genes.is_empty() {
        return Ok(());
    }
    if genes.len() < config.min_gene_set_size {
        return Err(TideGlassError::InsufficientData {
            required: config.min_gene_set_size,
            actual: genes.len(),
        });
    }
    Ok(())
}

fn gene_weight(weights: Option<&[f64]>, index: usize, weight_exponent: f64) -> f64 {
    weights.map_or_else(
        || 1.0_f64.powf(weight_exponent),
        |w| w[index].powf(weight_exponent),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;
    use crate::types::{CellLineId, CompoundId};

    fn gene(name: &str) -> GeneId {
        GeneId::new(name)
    }

    fn genes(names: &[&str]) -> Vec<GeneId> {
        names.iter().map(|name| gene(name)).collect()
    }

    #[test]
    fn weighted_ks_positive_when_hits_at_top() {
        let ranked = genes(&["A", "B", "C", "D", "E", "F"]);
        let query: HashSet<&GeneId> = ranked[..2].iter().collect();

        let score =
            weighted_ks_enrichment(&query, &ranked, None, 1.0).expect("enrichment succeeds");
        assert!(score > 0.0, "hits at top should yield positive enrichment");
    }

    #[test]
    fn weighted_ks_negative_when_hits_at_bottom() {
        let ranked = genes(&["A", "B", "C", "D", "E", "F"]);
        let query: HashSet<&GeneId> = ranked[4..].iter().collect();

        let score =
            weighted_ks_enrichment(&query, &ranked, None, 1.0).expect("enrichment succeeds");
        assert!(
            score < 0.0,
            "hits at bottom should yield negative enrichment"
        );
    }

    #[test]
    fn weighted_ks_respects_custom_weights() {
        let ranked = genes(&["A", "B", "C", "D"]);
        let query: HashSet<&GeneId> = ranked[..1].iter().collect();
        let weights = [10.0, 1.0, 1.0, 1.0];

        let weighted = weighted_ks_enrichment(&query, &ranked, Some(&weights), 1.0)
            .expect("weighted enrichment");
        let uniform =
            weighted_ks_enrichment(&query, &ranked, None, 1.0).expect("uniform enrichment");

        assert!(
            weighted.abs() >= uniform.abs(),
            "stronger top weight should not reduce deviation"
        );
    }

    #[test]
    fn weighted_ks_errors_on_empty_query() {
        let ranked = genes(&["A", "B"]);
        let query: HashSet<&GeneId> = HashSet::new();
        let err = weighted_ks_enrichment(&query, &ranked, None, 1.0).unwrap_err();
        assert!(matches!(err, TideGlassError::EmptySignature { .. }));
    }

    #[test]
    fn weighted_ks_errors_on_no_overlap() {
        let ranked = genes(&["A", "B", "C", "D"]);
        let query_genes = genes(&["X", "Y"]);
        let query: HashSet<&GeneId> = query_genes.iter().collect();
        let err = weighted_ks_enrichment(&query, &ranked, None, 1.0).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn weighted_ks_errors_on_weight_length_mismatch() {
        let ranked = genes(&["A", "B", "C"]);
        let query: HashSet<&GeneId> = ranked[..1].iter().collect();
        let weights = [1.0, 2.0];
        let err = weighted_ks_enrichment(&query, &ranked, Some(&weights), 1.0).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn permutation_p_value_is_bounded() {
        let ranked = genes(&["G1", "G2", "G3", "G4", "G5", "G6"]);
        let query: HashSet<&GeneId> = ranked[..3].iter().collect();
        let observed = weighted_ks_enrichment(&query, &ranked, None, 1.0).expect("observed score");

        let config = EnrichmentConfig {
            n_permutations: 200,
            ..EnrichmentConfig::default()
        };
        let mut rng = StdRng::seed_from_u64(42);
        let p = permutation_p_value(observed, &query, &ranked, None, &config, &mut rng)
            .expect("p-value");

        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn permutation_p_value_errors_on_zero_permutations() {
        let ranked = genes(&["A", "B", "C", "D"]);
        let query: HashSet<&GeneId> = ranked[..2].iter().collect();
        let config = EnrichmentConfig {
            n_permutations: 0,
            ..EnrichmentConfig::default()
        };
        let mut rng = StdRng::seed_from_u64(1);
        let err = permutation_p_value(0.5, &query, &ranked, None, &config, &mut rng).unwrap_err();
        assert!(matches!(err, TideGlassError::Permutation { .. }));
    }

    #[test]
    fn compute_rges_returns_one_result_per_perturbation() {
        let disease = DiseaseSignature {
            name: Arc::from("test-disease"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };

        let perturbations = vec![
            PerturbationSignature {
                compound_id: CompoundId::new("CHEMBL1"),
                cell_line: CellLineId::new("A549"),
                dose_um: 10.0,
                duration_h: 24.0,
                up_genes: genes(&["D1", "D2", "X1", "X2", "X3"]),
                down_genes: genes(&["U1", "U2", "Y1", "Y2", "Y3"]),
            },
            PerturbationSignature {
                compound_id: CompoundId::new("CHEMBL2"),
                cell_line: CellLineId::new("MCF7"),
                dose_um: 5.0,
                duration_h: 12.0,
                up_genes: genes(&["D3", "D4", "Z1", "Z2", "Z3"]),
                down_genes: genes(&["U3", "U4", "W1", "W2", "W3"]),
            },
        ];

        let config = EnrichmentConfig {
            n_permutations: 100,
            min_gene_set_size: 5,
            ..EnrichmentConfig::default()
        };

        let results = compute_rges(&disease, &perturbations, &config).expect("compute rges");
        assert_eq!(results.len(), 2);
        assert!(results[0].reversal_strength >= 0.0);
        assert_eq!(results[0].n_permutations, 100);
    }

    #[test]
    fn compute_rges_errors_on_empty_disease() {
        let disease = DiseaseSignature {
            name: Arc::from("empty"),
            up_genes: vec![],
            down_genes: vec![],
            source: Arc::from("none"),
        };
        let perturbations = vec![PerturbationSignature {
            compound_id: CompoundId::new("CHEMBL1"),
            cell_line: CellLineId::new("A549"),
            dose_um: 1.0,
            duration_h: 1.0,
            up_genes: genes(&["A", "B", "C", "D", "E"]),
            down_genes: genes(&["F", "G", "H", "I", "J"]),
        }];
        let config = EnrichmentConfig::default();
        let err = compute_rges(&disease, &perturbations, &config).unwrap_err();
        assert!(matches!(err, TideGlassError::EmptySignature { .. }));
    }

    #[test]
    fn compute_rges_errors_when_gene_set_below_minimum() {
        let disease = DiseaseSignature {
            name: Arc::from("small"),
            up_genes: genes(&["U1", "U2"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };
        let perturbations = vec![PerturbationSignature {
            compound_id: CompoundId::new("CHEMBL1"),
            cell_line: CellLineId::new("A549"),
            dose_um: 1.0,
            duration_h: 1.0,
            up_genes: genes(&["A", "B", "C", "D", "E"]),
            down_genes: genes(&["F", "G", "H", "I", "J"]),
        }];
        let config = EnrichmentConfig {
            min_gene_set_size: 5,
            n_permutations: 10,
            ..EnrichmentConfig::default()
        };
        let err = compute_rges(&disease, &perturbations, &config).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn stronger_reversal_yields_higher_rges_magnitude() {
        let disease = DiseaseSignature {
            name: Arc::from("test"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };

        let good = PerturbationSignature {
            compound_id: CompoundId::new("GOOD"),
            cell_line: CellLineId::new("A549"),
            dose_um: 10.0,
            duration_h: 24.0,
            up_genes: genes(&["D1", "D2", "D3", "D4", "X1", "X2", "X3"]),
            down_genes: genes(&["U1", "U2", "U3", "U4", "Y1", "Y2", "Y3"]),
        };

        let weak = PerturbationSignature {
            compound_id: CompoundId::new("WEAK"),
            cell_line: CellLineId::new("A549"),
            dose_um: 10.0,
            duration_h: 24.0,
            up_genes: genes(&["X1", "X2", "X3", "X4", "D5"]),
            down_genes: genes(&["Y1", "Y2", "Y3", "Y4", "U5"]),
        };

        let config = EnrichmentConfig {
            n_permutations: 50,
            min_gene_set_size: 5,
            ..EnrichmentConfig::default()
        };

        let results = compute_rges(&disease, &[good, weak], &config).expect("rges");
        assert!(
            results[0].reversal_strength >= results[1].reversal_strength,
            "well-ranked reversal should dominate weak overlap"
        );
    }
}
