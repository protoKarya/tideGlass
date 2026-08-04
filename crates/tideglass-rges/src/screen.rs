// SPDX-License-Identifier: AGPL-3.0-or-later

//! Drug screening with p-value filtering and Benjamini–Hochberg FDR correction.

use serde::{Deserialize, Serialize};
use tideglass_core::error::{Result, TideGlassError};
use tideglass_core::types::{CompoundId, RgesResult};

/// A ranked RGES hit with raw and FDR-adjusted p-values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedRgesHit {
    /// Compound identifier from the perturbation screen.
    pub compound_id: CompoundId,
    /// Combined reversal gene expression score.
    pub rges_score: f64,
    /// Raw permutation p-value before multiple-testing correction.
    pub p_value: f64,
    /// Benjamini–Hochberg adjusted p-value (q-value).
    pub adjusted_p_value: f64,
    /// Absolute reversal strength used for ranking.
    pub reversal_strength: f64,
    /// Permutation count used when computing the raw p-value.
    pub n_permutations: u32,
}

/// Configuration for RGES compound screening filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    /// Raw p-value ceiling before FDR correction (default: 1.0 — no pre-filter).
    pub p_value_threshold: f64,
    /// Maximum adjusted p-value (q-value) to retain a hit (default: 0.05).
    pub fdr_threshold: f64,
    /// Minimum reversal strength magnitude to retain a hit (default: 0.0).
    pub min_reversal_strength: f64,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            p_value_threshold: 1.0,
            fdr_threshold: 0.05,
            min_reversal_strength: 0.0,
        }
    }
}

/// Applies Benjamini–Hochberg FDR correction to a slice of p-values.
///
/// Returns adjusted p-values in the same order as the input slice.
///
/// # Errors
///
/// Returns [`TideGlassError::Enrichment`] if any p-value is outside `[0, 1]`.
pub fn benjamini_hochberg(p_values: &[f64]) -> Result<Vec<f64>> {
    if p_values.is_empty() {
        return Ok(Vec::new());
    }

    for (index, &p) in p_values.iter().enumerate() {
        if !(0.0..=1.0).contains(&p) {
            return Err(TideGlassError::Enrichment {
                reason: format!("p-value at index {index} is out of range: {p}"),
            });
        }
    }

    let n = p_values.len();
    let mut indexed: Vec<(usize, f64)> = p_values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut adjusted_sorted = vec![0.0; n];
    let mut min_so_far = 1.0_f64;

    for (rank, &(original_index, p)) in indexed.iter().enumerate().rev() {
        let rank_f = tideglass_core::count_as_f64(rank + 1);
        let n_f = tideglass_core::count_as_f64(n);
        let raw_adjusted = (p * n_f / rank_f).min(1.0);
        min_so_far = min_so_far.min(raw_adjusted);
        adjusted_sorted[original_index] = min_so_far;
    }

    Ok(adjusted_sorted)
}

/// Filters and ranks RGES results by reversal strength with BH FDR correction.
///
/// # Errors
///
/// Propagates errors from [`benjamini_hochberg`].
pub fn screen_compounds(
    results: &[RgesResult],
    config: &ScreenConfig,
) -> Result<Vec<RankedRgesHit>> {
    if results.is_empty() {
        return Ok(Vec::new());
    }

    let prefiltered: Vec<&RgesResult> = results
        .iter()
        .filter(|result| result.p_value <= config.p_value_threshold)
        .filter(|result| result.reversal_strength >= config.min_reversal_strength)
        .collect();

    if prefiltered.is_empty() {
        return Ok(Vec::new());
    }

    let p_values: Vec<f64> = prefiltered.iter().map(|result| result.p_value).collect();
    let adjusted = benjamini_hochberg(&p_values)?;

    let mut hits: Vec<RankedRgesHit> = prefiltered
        .into_iter()
        .zip(adjusted)
        .map(|(result, adjusted_p_value)| RankedRgesHit {
            compound_id: result.compound_id.clone(),
            rges_score: result.rges_score,
            p_value: result.p_value,
            adjusted_p_value,
            reversal_strength: result.reversal_strength,
            n_permutations: result.n_permutations,
        })
        .filter(|hit| hit.adjusted_p_value <= config.fdr_threshold)
        .collect();

    hits.sort_by(|a, b| {
        b.reversal_strength
            .partial_cmp(&a.reversal_strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tideglass_core::types::CompoundId;

    #[test]
    fn bh_correction_monotonic_and_bounded() {
        let p_values = [0.01, 0.04, 0.03, 0.20];
        let adjusted = benjamini_hochberg(&p_values).expect("bh correction");

        assert_eq!(adjusted.len(), 4);
        for &q in &adjusted {
            assert!((0.0..=1.0).contains(&q));
        }
        assert!(adjusted[0] <= adjusted[1] || adjusted[0] <= 0.04);
    }

    #[test]
    fn bh_empty_input_returns_empty() {
        let adjusted = benjamini_hochberg(&[]).expect("empty bh");
        assert!(adjusted.is_empty());
    }

    #[test]
    fn bh_rejects_out_of_range_p_value() {
        let err = benjamini_hochberg(&[0.5, 1.5]).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn screen_ranks_by_reversal_strength() {
        let results = vec![
            RgesResult {
                compound_id: CompoundId::new("A"),
                rges_score: 0.8,
                p_value: 0.01,
                reversal_strength: 0.8,
                n_permutations: 100,
            },
            RgesResult {
                compound_id: CompoundId::new("B"),
                rges_score: 0.5,
                p_value: 0.02,
                reversal_strength: 0.5,
                n_permutations: 100,
            },
        ];

        let config = ScreenConfig {
            fdr_threshold: 0.05,
            ..ScreenConfig::default()
        };

        let hits = screen_compounds(&results, &config).expect("screen");
        assert_eq!(hits.len(), 2);
        assert!(hits[0].reversal_strength >= hits[1].reversal_strength);
    }

    #[test]
    fn screen_filters_by_fdr_threshold() {
        let results = vec![RgesResult {
            compound_id: CompoundId::new("WEAK"),
            rges_score: 0.1,
            p_value: 0.9,
            reversal_strength: 0.1,
            n_permutations: 50,
        }];

        let config = ScreenConfig {
            fdr_threshold: 0.05,
            ..ScreenConfig::default()
        };

        let hits = screen_compounds(&results, &config).expect("screen");
        assert!(hits.is_empty());
    }
}
