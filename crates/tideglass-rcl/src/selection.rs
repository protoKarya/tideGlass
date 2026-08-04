// SPDX-License-Identifier: AGPL-3.0-or-later

//! Robust cell line selection by signal-to-noise ratio of |RGES| across compounds.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tideglass_core::enrichment::compute_rges;
use tideglass_core::error::{Result, TideGlassError};
use tideglass_core::types::{
    CellLineId, DiseaseSignature, EnrichmentConfig, PerturbationSignature, RgesResult,
};

/// Signal-to-noise ranking for a single cell line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CellLineRanking {
    /// Cell line evaluated in this ranking entry.
    pub cell_line: CellLineId,
    /// Mean absolute RGES across compounds in this cell line.
    pub mean_abs_rges: f64,
    /// Standard deviation of absolute RGES across compounds.
    pub std_abs_rges: f64,
    /// Signal-to-noise ratio: `mean(|RGES|) / std(|RGES|)`.
    pub snr: f64,
    /// Number of compounds contributing to the SNR estimate.
    pub n_compounds: usize,
}

/// Configuration for robust cell line selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RclConfig {
    /// Enrichment configuration used when computing per-compound RGES.
    pub enrichment_config: EnrichmentConfig,
    /// Minimum compounds required per cell line to compute SNR (default: 2).
    pub min_compounds_per_line: usize,
}

impl Default for RclConfig {
    fn default() -> Self {
        Self {
            enrichment_config: EnrichmentConfig::default(),
            min_compounds_per_line: 2,
        }
    }
}

/// Partitions perturbation signatures by cell line identifier.
#[must_use]
pub fn group_by_cell_line(
    perturbations: &[PerturbationSignature],
) -> HashMap<CellLineId, Vec<PerturbationSignature>> {
    let mut groups: HashMap<CellLineId, Vec<PerturbationSignature>> = HashMap::new();
    for perturbation in perturbations {
        groups
            .entry(perturbation.cell_line.clone())
            .or_default()
            .push(perturbation.clone());
    }
    groups
}

/// Computes signal-to-noise ratio for a single cell line's RGES results.
///
/// SNR = mean(|RGES|) / std(|RGES|). Returns an error if fewer than two compounds
/// are available (std is undefined).
///
/// # Errors
///
/// Returns [`TideGlassError::InsufficientData`] when fewer than two compounds are present.
pub fn compute_snr(results: &[RgesResult]) -> Result<(f64, f64, f64)> {
    if results.len() < 2 {
        return Err(TideGlassError::InsufficientData {
            required: 2,
            actual: results.len(),
        });
    }

    let values: Vec<f64> = results.iter().map(|r| r.reversal_strength).collect();
    let mean = values.iter().sum::<f64>() / tideglass_core::count_as_f64(values.len());

    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / tideglass_core::count_as_f64(values.len().saturating_sub(1));

    let std = variance.sqrt();
    let snr = if std > f64::EPSILON {
        mean / std
    } else {
        f64::INFINITY
    };

    Ok((mean, std, snr))
}

/// Ranks cell lines by SNR of |RGES| across compounds.
///
/// # Errors
///
/// Propagates errors from [`compute_rges`] or [`compute_snr`]. Cell lines with
/// fewer than `config.min_compounds_per_line` compounds are skipped.
pub fn rank_cell_lines(
    disease: &DiseaseSignature,
    perturbations: &[PerturbationSignature],
    config: &RclConfig,
) -> Result<Vec<CellLineRanking>> {
    let groups = group_by_cell_line(perturbations);
    let mut rankings = Vec::new();

    for (cell_line, line_perturbations) in groups {
        if line_perturbations.len() < config.min_compounds_per_line {
            continue;
        }

        let results = compute_rges(disease, &line_perturbations, &config.enrichment_config)?;
        let (mean_abs_rges, std_abs_rges, snr) = compute_snr(&results)?;

        rankings.push(CellLineRanking {
            cell_line,
            mean_abs_rges,
            std_abs_rges,
            snr,
            n_compounds: results.len(),
        });
    }

    rankings.sort_by(|a, b| {
        b.snr
            .partial_cmp(&a.snr)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(rankings)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tideglass_core::types::{CompoundId, GeneId};

    use super::*;

    fn gene(name: &str) -> GeneId {
        GeneId::new(name)
    }

    fn genes(names: &[&str]) -> Vec<GeneId> {
        names.iter().map(|name| gene(name)).collect()
    }

    fn make_perturbation(
        compound: &str,
        cell_line: &str,
        extra_noise: usize,
    ) -> PerturbationSignature {
        let mut up_genes = genes(&["D1", "D2", "D3", "X1", "X2"]);
        let mut down_genes = genes(&["U1", "U2", "U3", "Y1", "Y2"]);
        for i in 0..extra_noise {
            up_genes.push(gene(&format!("N{i}")));
            down_genes.push(gene(&format!("M{i}")));
        }

        PerturbationSignature {
            compound_id: CompoundId::new(compound),
            cell_line: CellLineId::new(cell_line),
            dose_um: 10.0,
            duration_h: 24.0,
            up_genes,
            down_genes,
        }
    }

    #[test]
    fn compute_snr_higher_for_consistent_signal() {
        let consistent = vec![
            RgesResult {
                compound_id: CompoundId::new("A"),
                rges_score: 0.6,
                p_value: 0.01,
                reversal_strength: 0.6,
                n_permutations: 100,
            },
            RgesResult {
                compound_id: CompoundId::new("B"),
                rges_score: 0.62,
                p_value: 0.01,
                reversal_strength: 0.62,
                n_permutations: 100,
            },
        ];

        let variable = vec![
            RgesResult {
                compound_id: CompoundId::new("C"),
                rges_score: 0.9,
                p_value: 0.01,
                reversal_strength: 0.9,
                n_permutations: 100,
            },
            RgesResult {
                compound_id: CompoundId::new("D"),
                rges_score: 0.1,
                p_value: 0.5,
                reversal_strength: 0.1,
                n_permutations: 100,
            },
        ];

        let (_, _, snr_consistent) = compute_snr(&consistent).expect("consistent snr");
        let (_, _, snr_variable) = compute_snr(&variable).expect("variable snr");

        assert!(snr_consistent > snr_variable);
    }

    #[test]
    fn rank_cell_lines_orders_by_snr() {
        let disease = DiseaseSignature {
            name: Arc::from("test"),
            up_genes: genes(&["U1", "U2", "U3", "U4", "U5"]),
            down_genes: genes(&["D1", "D2", "D3", "D4", "D5"]),
            source: Arc::from("synthetic"),
        };

        let perturbations = vec![
            make_perturbation("C1", "HIGH_SNR", 0),
            make_perturbation("C2", "HIGH_SNR", 0),
            make_perturbation("C3", "LOW_SNR", 3),
            make_perturbation("C4", "LOW_SNR", 5),
        ];

        let config = RclConfig {
            enrichment_config: EnrichmentConfig {
                n_permutations: 30,
                min_gene_set_size: 5,
                ..EnrichmentConfig::default()
            },
            min_compounds_per_line: 2,
        };

        let rankings = rank_cell_lines(&disease, &perturbations, &config).expect("rank");
        assert_eq!(rankings.len(), 2);
        assert!(rankings[0].snr >= rankings[1].snr);
    }

    #[test]
    fn compute_snr_requires_at_least_two_compounds() {
        let single = vec![RgesResult {
            compound_id: CompoundId::new("ONLY"),
            rges_score: 0.5,
            p_value: 0.1,
            reversal_strength: 0.5,
            n_permutations: 10,
        }];
        let err = compute_snr(&single).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }
}
