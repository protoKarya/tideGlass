// SPDX-License-Identifier: AGPL-3.0-or-later

//! NF1-specific reversal scoring with tissue geometry weighting.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tideglass_core::enrichment::{compute_rges, permutation_p_value, weighted_ks_enrichment};
use tideglass_core::error::{Result, TideGlassError};
use tideglass_core::types::{
    CompoundId, DiseaseSignature, EnrichmentConfig, GeneId, PerturbationSignature, RgesResult,
};

use crate::tissue::{GeneCompartmentMap, TissueWeight};

/// NF1 reversal score extending RGES with tissue geometry weighting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NfReversalScore {
    /// Compound evaluated.
    pub compound_id: CompoundId,
    /// Geometry-weighted RGES score.
    pub weighted_rges: f64,
    /// Standard (unweighted) RGES score for comparison.
    pub standard_rges: f64,
    /// Permutation p-value for the weighted score.
    pub p_value: f64,
    /// Absolute weighted reversal strength.
    pub reversal_strength: f64,
    /// Effective geometry scale parameter used.
    pub geometry_scale_d: f64,
}

/// Configuration for NF1 reversal scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NfScoringConfig {
    /// Core enrichment configuration for permutation testing.
    pub enrichment_config: EnrichmentConfig,
    /// Tissue geometry weight configuration.
    pub tissue_weights: TissueWeight,
    /// Gene-to-compartment assignments.
    pub gene_compartments: GeneCompartmentMap,
}

impl Default for NfScoringConfig {
    fn default() -> Self {
        Self {
            enrichment_config: EnrichmentConfig::default(),
            tissue_weights: TissueWeight::default(),
            gene_compartments: GeneCompartmentMap {
                assignments: HashMap::new(),
            },
        }
    }
}

/// Computes geometry-weighted gene weights for a query gene set.
#[must_use]
pub fn compute_gene_weights(genes: &[GeneId], config: &NfScoringConfig) -> Vec<f64> {
    let default_compartment = config.tissue_weights.primary_compartment;
    genes
        .iter()
        .map(|gene| {
            let compartment = config
                .gene_compartments
                .compartment_for(gene.as_str(), default_compartment);
            config.tissue_weights.compartment_weight(compartment)
        })
        .collect()
}

/// Computes a geometry-weighted directional reversal score.
fn compute_weighted_directional_reversal(
    query_genes: &HashSet<&GeneId>,
    ranked_genes: &[GeneId],
    config: &NfScoringConfig,
    rng: &mut impl Rng,
) -> Result<(f64, f64)> {
    if query_genes.is_empty() || ranked_genes.is_empty() {
        return Ok((0.0, 1.0));
    }

    let weights: Vec<f64> = ranked_genes
        .iter()
        .map(|gene| {
            if query_genes.contains(gene) {
                let compartment = config
                    .gene_compartments
                    .compartment_for(gene.as_str(), config.tissue_weights.primary_compartment);
                config.tissue_weights.compartment_weight(compartment)
            } else {
                1.0
            }
        })
        .collect();

    let score = weighted_ks_enrichment(
        query_genes,
        ranked_genes,
        Some(&weights),
        config.enrichment_config.weight_exponent,
    )?;

    let p_value = permutation_p_value(
        score,
        query_genes,
        ranked_genes,
        Some(&weights),
        &config.enrichment_config,
        rng,
    )?;

    Ok((score, p_value))
}

/// Computes NF1 reversal scores for perturbations against a disease signature.
///
/// # Errors
///
/// Propagates errors from core enrichment functions.
pub fn compute_nf_scores(
    disease: &DiseaseSignature,
    perturbations: &[PerturbationSignature],
    config: &NfScoringConfig,
) -> Result<Vec<NfReversalScore>> {
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
    let standard_results = compute_rges(disease, perturbations, &config.enrichment_config)?;
    let mut rng = rand::rng();
    let mut results = Vec::with_capacity(perturbations.len());

    for (perturbation, standard) in perturbations.iter().zip(standard_results) {
        let (up_weighted, p_up) = compute_weighted_directional_reversal(
            &up_query,
            &perturbation.down_genes,
            config,
            &mut rng,
        )?;
        let (down_weighted, p_down) = compute_weighted_directional_reversal(
            &down_query,
            &perturbation.up_genes,
            config,
            &mut rng,
        )?;

        let weighted_rges = f64::midpoint(up_weighted, down_weighted);
        let p_value = f64::midpoint(p_up, p_down);
        let standard_rges = standard.rges_score;

        results.push(NfReversalScore {
            compound_id: perturbation.compound_id.clone(),
            weighted_rges,
            standard_rges,
            p_value,
            reversal_strength: weighted_rges.abs(),
            geometry_scale_d: config.tissue_weights.geometry_scale_d,
        });
    }

    Ok(results)
}

/// Converts NF reversal scores to standard RGES results for pipeline interoperability.
#[must_use]
pub fn to_rges_results(scores: &[NfReversalScore], n_permutations: u32) -> Vec<RgesResult> {
    scores
        .iter()
        .map(|score| RgesResult {
            compound_id: score.compound_id.clone(),
            rges_score: score.weighted_rges,
            p_value: score.p_value,
            reversal_strength: score.reversal_strength,
            n_permutations,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tideglass_core::types::CellLineId;

    use super::*;
    use crate::tissue::TissueCompartment;

    fn gene(name: &str) -> GeneId {
        GeneId::new(name)
    }

    fn genes(names: &[&str]) -> Vec<GeneId> {
        names.iter().map(|name| gene(name)).collect()
    }

    fn sample_disease() -> DiseaseSignature {
        DiseaseSignature {
            name: Arc::from("NF1"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        }
    }

    fn sample_perturbation() -> PerturbationSignature {
        PerturbationSignature {
            compound_id: CompoundId::new("CHEMBL_NF"),
            cell_line: CellLineId::new("HSCH"),
            dose_um: 10.0,
            duration_h: 24.0,
            up_genes: genes(&["D1", "D2", "D3", "X1", "X2"]),
            down_genes: genes(&["U1", "U2", "U3", "Y1", "Y2"]),
        }
    }

    #[test]
    fn compute_nf_scores_returns_weighted_results() {
        let config = NfScoringConfig {
            enrichment_config: EnrichmentConfig {
                n_permutations: 50,
                min_gene_set_size: 5,
                ..EnrichmentConfig::default()
            },
            ..NfScoringConfig::default()
        };

        let scores = compute_nf_scores(&sample_disease(), &[sample_perturbation()], &config)
            .expect("nf scores");

        assert_eq!(scores.len(), 1);
        assert!(scores[0].reversal_strength > 0.0);
        assert!((scores[0].geometry_scale_d - 2.4).abs() < f64::EPSILON);
    }

    #[test]
    fn gene_weights_reflect_compartment_distance() {
        let mut assignments = HashMap::new();
        assignments.insert(Arc::from("U1"), TissueCompartment::NerveSheath);
        assignments.insert(Arc::from("U2"), TissueCompartment::Endoneurium);

        let config = NfScoringConfig {
            gene_compartments: GeneCompartmentMap { assignments },
            ..NfScoringConfig::default()
        };

        let weights = compute_gene_weights(&genes(&["U1", "U2"]), &config);
        assert!(weights[0] > weights[1]);
    }

    #[test]
    fn to_rges_results_preserves_scores() {
        let nf = NfReversalScore {
            compound_id: CompoundId::new("X"),
            weighted_rges: 0.7,
            standard_rges: 0.5,
            p_value: 0.01,
            reversal_strength: 0.7,
            geometry_scale_d: 2.4,
        };
        let rges = to_rges_results(&[nf], 100);
        assert_eq!(rges.len(), 1);
        assert!((rges[0].rges_score - 0.7).abs() < f64::EPSILON);
    }
}
