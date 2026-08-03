// SPDX-License-Identifier: AGPL-3.0-or-later

//! Benchmark framework comparing GPS-ranked compounds against OCTAD reference lists.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tideglass_core::error::Result;
use tideglass_core::types::CompoundId;

use crate::metrics::{compute_auc, compute_f1, compute_precision_recall, concordance_correlation};

/// Aggregated benchmark metrics for a ranked compound list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkResult {
    /// Area under the ROC curve.
    pub auc: f64,
    /// Precision at the configured score threshold.
    pub precision: f64,
    /// Recall at the configured score threshold.
    pub recall: f64,
    /// F1 score derived from precision and recall.
    pub f1: f64,
    /// Number of compounds evaluated.
    pub n_compounds: usize,
    /// Number of known actives in the evaluation set.
    pub n_actives: usize,
}

/// Configuration for OCTAD benchmark comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Score threshold for precision/recall computation.
    pub score_threshold: f64,
    /// Reference benchmark identifier (e.g. OCTAD HCC panel).
    pub reference: Arc<str>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            score_threshold: 0.5,
            reference: Arc::from("octad-hcc"),
        }
    }
}

/// A ranked compound entry for benchmark evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedCompound {
    /// Compound identifier.
    pub compound_id: CompoundId,
    /// Ranking score (higher = more likely active).
    pub score: f64,
}

/// Compares GPS and OCTAD ranked compound lists for concordance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OctadComparison {
    /// Benchmark configuration.
    pub config: BenchmarkConfig,
}

impl OctadComparison {
    /// Creates a comparison with the given configuration.
    #[must_use]
    pub const fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Evaluates GPS-ranked compounds against known actives.
    ///
    /// # Errors
    ///
    /// Propagates errors from underlying metric computations.
    pub fn evaluate_gps(
        &self,
        ranked: &[RankedCompound],
        known_actives: &HashMap<CompoundId, bool>,
    ) -> Result<BenchmarkResult> {
        let scores: Vec<f64> = ranked.iter().map(|entry| entry.score).collect();
        let labels: Vec<bool> = ranked
            .iter()
            .map(|entry| {
                known_actives
                    .get(&entry.compound_id)
                    .copied()
                    .unwrap_or(false)
            })
            .collect();

        let auc = compute_auc(&scores, &labels)?;
        let pr = compute_precision_recall(&scores, &labels, self.config.score_threshold)?;
        let f1 = compute_f1(pr.precision, pr.recall);
        let n_actives = labels.iter().filter(|&&label| label).count();

        Ok(BenchmarkResult {
            auc,
            precision: pr.precision,
            recall: pr.recall,
            f1,
            n_compounds: ranked.len(),
            n_actives,
        })
    }

    /// Computes concordance between GPS and OCTAD ranked score vectors for shared compounds.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`concordance_correlation`].
    pub fn compare_rankings(
        &self,
        gps_ranked: &[RankedCompound],
        octad_ranked: &[RankedCompound],
    ) -> Result<f64> {
        let octad_map: HashMap<&str, f64> = octad_ranked
            .iter()
            .map(|entry| (entry.compound_id.as_str(), entry.score))
            .collect();

        let mut gps_scores = Vec::new();
        let mut octad_scores = Vec::new();

        for entry in gps_ranked {
            if let Some(&octad_score) = octad_map.get(entry.compound_id.as_str()) {
                gps_scores.push(entry.score);
                octad_scores.push(octad_score);
            }
        }

        if gps_scores.len() < 2 {
            return Ok(0.0);
        }

        concordance_correlation(&gps_scores, &octad_scores)
    }

    /// Returns compounds where GPS ranks higher than OCTAD by score margin.
    #[must_use]
    pub fn gps_advantage(
        &self,
        gps_ranked: &[RankedCompound],
        octad_ranked: &[RankedCompound],
        margin: f64,
    ) -> Vec<CompoundId> {
        let octad_map: HashMap<&str, f64> = octad_ranked
            .iter()
            .map(|entry| (entry.compound_id.as_str(), entry.score))
            .collect();

        gps_ranked
            .iter()
            .filter_map(|entry| {
                octad_map
                    .get(entry.compound_id.as_str())
                    .and_then(|&octad_score| {
                        if entry.score - octad_score >= margin {
                            Some(entry.compound_id.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranked(id: &str, score: f64) -> RankedCompound {
        RankedCompound {
            compound_id: CompoundId::new(id),
            score,
        }
    }

    #[test]
    fn evaluate_gps_perfect_ranking() {
        let comparison = OctadComparison::default();
        let ranked = vec![
            ranked("A", 0.9),
            ranked("B", 0.8),
            ranked("C", 0.2),
            ranked("D", 0.1),
        ];
        let mut actives = HashMap::new();
        actives.insert(CompoundId::new("A"), true);
        actives.insert(CompoundId::new("B"), true);
        actives.insert(CompoundId::new("C"), false);
        actives.insert(CompoundId::new("D"), false);

        let result = comparison
            .evaluate_gps(&ranked, &actives)
            .expect("evaluate");
        assert!((result.auc - 1.0).abs() < 1e-10);
        assert_eq!(result.n_actives, 2);
    }

    #[test]
    fn compare_rankings_perfect_concordance() {
        let comparison = OctadComparison::default();
        let gps = vec![ranked("A", 0.9), ranked("B", 0.5)];
        let octad = vec![ranked("A", 0.9), ranked("B", 0.5)];

        let ccc = comparison.compare_rankings(&gps, &octad).expect("compare");
        assert!((ccc - 1.0).abs() < 1e-10);
    }

    #[test]
    fn gps_advantage_detects_higher_scores() {
        let comparison = OctadComparison::default();
        let gps = vec![ranked("A", 0.9), ranked("B", 0.4)];
        let octad = vec![ranked("A", 0.5), ranked("B", 0.4)];

        let advantage = comparison.gps_advantage(&gps, &octad, 0.2);
        assert_eq!(advantage.len(), 1);
        assert_eq!(advantage[0].as_str(), "A");
    }
}
