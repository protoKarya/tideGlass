// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structure-to-expression prediction from molecular features.

use serde::{Deserialize, Serialize};
use tideglass_core::error::{Result, TideGlassError};
use tideglass_core::types::GeneId;

use crate::features::MolecularFeatures;

/// Predicted expression change for a single gene.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneExpressionPrediction {
    /// Gene whose expression change is predicted.
    pub gene_id: GeneId,
    /// Predicted log2 fold change (positive = up, negative = down).
    pub log2_fold_change: f64,
}

/// Full predicted perturbation signature from structure alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpressionPrediction {
    /// Per-gene predicted fold changes.
    pub genes: Vec<GeneExpressionPrediction>,
    /// Model confidence score in `[0, 1]`.
    pub confidence: f64,
}

/// Predicts gene expression changes from molecular structure features.
pub trait ExpressionPredictor: Send + Sync {
    /// Predicts expression changes for the given molecular features.
    ///
    /// # Errors
    ///
    /// Returns an error when feature dimensionality does not match the model.
    fn predict(&self, features: &MolecularFeatures) -> Result<ExpressionPrediction>;
}

/// Configuration for linear regression expression prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegressionConfig {
    /// Target genes to predict (order must match weight matrix rows).
    pub target_genes: Vec<GeneId>,
    /// Intercept term added to each gene prediction.
    pub intercept: f64,
}

/// Multivariate linear regression predictor: `y = intercept + W · x`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegressionPredictor {
    /// Per-gene regression weights aligned with feature vector dimensions.
    pub weights: Vec<Vec<f64>>,
    /// Model configuration including target genes and intercept.
    pub config: LinearRegressionConfig,
}

impl LinearRegressionPredictor {
    /// Creates a predictor from weight matrix and configuration.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::Enrichment`] if weight row count mismatches target genes
    /// or column count is zero.
    pub fn new(weights: Vec<Vec<f64>>, config: LinearRegressionConfig) -> Result<Self> {
        if weights.len() != config.target_genes.len() {
            return Err(TideGlassError::Enrichment {
                reason: format!(
                    "weight rows {} do not match target gene count {}",
                    weights.len(),
                    config.target_genes.len()
                ),
            });
        }
        if weights.iter().any(Vec::is_empty) {
            return Err(TideGlassError::Enrichment {
                reason: "weight matrix contains empty rows".to_owned(),
            });
        }
        Ok(Self { weights, config })
    }

    fn dot_product(row: &[f64], features: &[f64]) -> Result<f64> {
        if row.len() != features.len() {
            return Err(TideGlassError::Enrichment {
                reason: format!(
                    "feature dimension {} does not match weight dimension {}",
                    features.len(),
                    row.len()
                ),
            });
        }
        Ok(row.iter().zip(features).map(|(w, x)| w * x).sum())
    }
}

impl ExpressionPredictor for LinearRegressionPredictor {
    fn predict(&self, features: &MolecularFeatures) -> Result<ExpressionPrediction> {
        let feature_vector = features.to_feature_vector();

        let genes: Vec<GeneExpressionPrediction> = self
            .weights
            .iter()
            .zip(&self.config.target_genes)
            .map(|(row, gene_id)| {
                let raw = Self::dot_product(row, &feature_vector)?;
                Ok(GeneExpressionPrediction {
                    gene_id: gene_id.clone(),
                    log2_fold_change: raw + self.config.intercept,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mean_abs: f64 = genes
            .iter()
            .map(|gene| gene.log2_fold_change.abs())
            .sum::<f64>()
            / tideglass_core::count_as_f64(genes.len());
        let confidence = (mean_abs / (mean_abs + 1.0)).clamp(0.0, 1.0);

        Ok(ExpressionPrediction { genes, confidence })
    }
}

/// Computes coefficient of determination R² between predicted and observed values.
///
/// # Errors
///
/// Returns [`TideGlassError::InsufficientData`] when fewer than two samples are provided,
/// or [`TideGlassError::Enrichment`] when vectors differ in length.
pub fn compute_r_squared(predicted: &[f64], observed: &[f64]) -> Result<f64> {
    if predicted.len() != observed.len() {
        return Err(TideGlassError::Enrichment {
            reason: format!(
                "predicted length {} != observed length {}",
                predicted.len(),
                observed.len()
            ),
        });
    }
    if predicted.len() < 2 {
        return Err(TideGlassError::InsufficientData {
            required: 2,
            actual: predicted.len(),
        });
    }

    let n = tideglass_core::count_as_f64(predicted.len());
    let mean_observed = observed.iter().sum::<f64>() / n;

    let ss_res: f64 = predicted
        .iter()
        .zip(observed)
        .map(|(pred, obs)| {
            let diff = obs - pred;
            diff * diff
        })
        .sum();

    let ss_tot: f64 = observed
        .iter()
        .map(|obs| {
            let diff = obs - mean_observed;
            diff * diff
        })
        .sum();

    if ss_tot <= f64::EPSILON {
        return Ok(1.0);
    }

    Ok(1.0 - ss_res / ss_tot)
}

#[cfg(test)]
mod tests {
    use tideglass_core::types::CompoundId;

    use super::*;
    use crate::features::PhysicochemicalProperties;

    fn sample_features() -> MolecularFeatures {
        MolecularFeatures {
            compound_id: CompoundId::new("TEST"),
            smiles: None,
            fingerprint_bits: vec![],
            properties: PhysicochemicalProperties {
                molecular_weight: 300.0,
                log_p: 2.5,
                tpsa: 60.0,
                hba: 4,
                hbd: 2,
                rotatable_bonds: 3,
                aromatic_rings: 2,
            },
        }
    }

    #[test]
    fn linear_regression_predicts_all_target_genes() {
        let genes = vec![GeneId::new("G1"), GeneId::new("G2")];
        let weights = vec![vec![0.01; 7], vec![0.02; 7]];
        let config = LinearRegressionConfig {
            target_genes: genes,
            intercept: 0.0,
        };

        let predictor = LinearRegressionPredictor::new(weights, config).expect("predictor");
        let prediction = predictor.predict(&sample_features()).expect("predict");

        assert_eq!(prediction.genes.len(), 2);
        assert!(prediction.genes[0].log2_fold_change > 0.0);
    }

    #[test]
    fn r_squared_perfect_prediction() {
        let values = [1.0, 2.0, 3.0, 4.0];
        let r2 = compute_r_squared(&values, &values).expect("r2");
        assert!((r2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn r_squared_rejects_length_mismatch() {
        let err = compute_r_squared(&[1.0, 2.0], &[1.0]).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn new_rejects_weight_row_count_mismatch() {
        let config = LinearRegressionConfig {
            target_genes: vec![GeneId::new("G1"), GeneId::new("G2")],
            intercept: 0.0,
        };
        let weights = vec![vec![0.01; 7]];
        let err = LinearRegressionPredictor::new(weights, config).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn predict_errors_on_feature_dimension_mismatch() {
        let genes = vec![GeneId::new("G1")];
        let weights = vec![vec![0.01; 3]];
        let config = LinearRegressionConfig {
            target_genes: genes,
            intercept: 0.0,
        };
        let predictor = LinearRegressionPredictor::new(weights, config).expect("predictor");
        let err = predictor.predict(&sample_features()).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn r_squared_one_for_constant_observed_values() {
        let predicted = [2.0, 3.0, 4.0];
        let observed = [5.0, 5.0, 5.0];
        let r2 = compute_r_squared(&predicted, &observed).expect("r2");
        assert!((r2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn r_squared_rejects_fewer_than_two_samples() {
        let err = compute_r_squared(&[1.0], &[2.0]).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn intercept_shifts_predictions() {
        let genes = vec![GeneId::new("G1")];
        let weights = vec![vec![0.0; 7]];
        let config_no_intercept = LinearRegressionConfig {
            target_genes: genes.clone(),
            intercept: 0.0,
        };
        let config_with_intercept = LinearRegressionConfig {
            target_genes: genes,
            intercept: 2.5,
        };

        let predictor_base =
            LinearRegressionPredictor::new(weights.clone(), config_no_intercept).expect("base");
        let predictor_shifted =
            LinearRegressionPredictor::new(weights, config_with_intercept).expect("shifted");

        let base = predictor_base
            .predict(&sample_features())
            .expect("base predict");
        let shifted = predictor_shifted
            .predict(&sample_features())
            .expect("shifted predict");

        assert!(
            (shifted.genes[0].log2_fold_change - base.genes[0].log2_fold_change - 2.5).abs()
                < 1e-10
        );
    }
}
