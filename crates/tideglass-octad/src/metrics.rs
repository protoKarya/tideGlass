// SPDX-License-Identifier: AGPL-3.0-or-later

//! Classification and ranking metrics for benchmark comparison.

use serde::{Deserialize, Serialize};
use tideglass_core::error::{Result, TideGlassError};

/// A single point on a precision–recall or ROC curve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricPoint {
    /// Threshold or rank position for this point.
    pub x: f64,
    /// Metric value at this point.
    pub y: f64,
}

/// Computes area under the curve using the trapezoidal rule.
///
/// Points must be sorted by ascending `x`. Requires at least two points.
///
/// # Errors
///
/// Returns [`TideGlassError::InsufficientData`] when fewer than two points are provided.
pub fn trapezoidal_auc(points: &[MetricPoint]) -> Result<f64> {
    if points.len() < 2 {
        return Err(TideGlassError::InsufficientData {
            required: 2,
            actual: points.len(),
        });
    }

    let mut area = 0.0_f64;
    for window in points.windows(2) {
        let dx = window[1].x - window[0].x;
        let avg_y = f64::midpoint(window[0].y, window[1].y);
        area += dx * avg_y;
    }

    Ok(area.clamp(0.0, 1.0))
}

/// Computes ROC AUC from ranked scores and binary labels.
///
/// Higher scores should indicate positive class membership.
///
/// # Errors
///
/// Returns [`TideGlassError::InsufficientData`] when inputs are empty or all one class.
pub fn compute_auc(scores: &[f64], labels: &[bool]) -> Result<f64> {
    if scores.len() != labels.len() {
        return Err(TideGlassError::Enrichment {
            reason: format!(
                "scores length {} != labels length {}",
                scores.len(),
                labels.len()
            ),
        });
    }
    if scores.is_empty() {
        return Err(TideGlassError::InsufficientData {
            required: 1,
            actual: 0,
        });
    }

    let n_pos = labels.iter().filter(|&&label| label).count();
    let n_neg = labels.len().saturating_sub(n_pos);

    if n_pos == 0 || n_neg == 0 {
        return Err(TideGlassError::InsufficientData {
            required: 1,
            actual: 0,
        });
    }

    let mut indexed: Vec<(f64, bool)> =
        scores.iter().copied().zip(labels.iter().copied()).collect();
    indexed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut tp = 0_u32;
    let mut fp = 0_u32;
    let mut curve = vec![MetricPoint { x: 0.0, y: 0.0 }];

    let n_pos_f = tideglass_core::count_as_f64(n_pos);
    let n_neg_f = tideglass_core::count_as_f64(n_neg);

    for (_, label) in &indexed {
        if *label {
            tp = tp.saturating_add(1);
        } else {
            fp = fp.saturating_add(1);
        }
        curve.push(MetricPoint {
            x: f64::from(fp) / n_neg_f,
            y: f64::from(tp) / n_pos_f,
        });
    }

    trapezoidal_auc(&curve)
}

/// Precision and recall at a given score threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecisionRecall {
    /// Precision: TP / (TP + FP).
    pub precision: f64,
    /// Recall: TP / (TP + FN).
    pub recall: f64,
}

/// Computes precision and recall at a score threshold.
///
/// # Errors
///
/// Returns [`TideGlassError::Enrichment`] when score and label lengths differ.
pub fn compute_precision_recall(
    scores: &[f64],
    labels: &[bool],
    threshold: f64,
) -> Result<PrecisionRecall> {
    if scores.len() != labels.len() {
        return Err(TideGlassError::Enrichment {
            reason: "scores and labels length mismatch".to_owned(),
        });
    }

    let mut tp = 0_u32;
    let mut fp = 0_u32;
    let mut fn_count = 0_u32;

    for (&score, &label) in scores.iter().zip(labels) {
        let predicted = score >= threshold;
        match (predicted, label) {
            (true, true) => tp = tp.saturating_add(1),
            (true, false) => fp = fp.saturating_add(1),
            (false, true) => fn_count = fn_count.saturating_add(1),
            (false, false) => {}
        }
    }

    let precision = if tp + fp > 0 {
        f64::from(tp) / f64::from(tp + fp)
    } else {
        0.0
    };

    let recall = if tp + fn_count > 0 {
        f64::from(tp) / f64::from(tp + fn_count)
    } else {
        0.0
    };

    Ok(PrecisionRecall { precision, recall })
}

/// Computes F1 score from precision and recall.
#[must_use]
pub fn compute_f1(precision: f64, recall: f64) -> f64 {
    if precision + recall <= f64::EPSILON {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

/// Lin's concordance correlation coefficient between two ranked lists.
///
/// # Errors
///
/// Returns [`TideGlassError::Enrichment`] on length mismatch or zero variance.
pub fn concordance_correlation(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TideGlassError::Enrichment {
            reason: format!("x length {} != y length {}", x.len(), y.len()),
        });
    }
    if x.len() < 2 {
        return Err(TideGlassError::InsufficientData {
            required: 2,
            actual: x.len(),
        });
    }

    let n = tideglass_core::count_as_f64(x.len());
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let mut var_x = 0.0_f64;
    let mut var_y = 0.0_f64;
    let mut cov = 0.0_f64;

    for (&xi, &yi) in x.iter().zip(y) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        var_x += dx * dx;
        var_y += dy * dy;
        cov += dx * dy;
    }

    if var_x <= f64::EPSILON || var_y <= f64::EPSILON {
        return Err(TideGlassError::Enrichment {
            reason: "zero variance in concordance inputs".to_owned(),
        });
    }

    let rho = cov / (var_x * var_y).sqrt();
    Ok((2.0 * rho * var_x.sqrt() * var_y.sqrt())
        / ((mean_x - mean_y).mul_add(mean_x - mean_y, var_x + var_y)))
}

#[cfg(test)]
mod tests {
    use tideglass_core::error::TideGlassError;

    use super::*;

    #[test]
    fn trapezoidal_auc_unit_square() {
        let points = vec![
            MetricPoint { x: 0.0, y: 0.0 },
            MetricPoint { x: 1.0, y: 1.0 },
        ];
        let auc = trapezoidal_auc(&points).expect("auc");
        assert!((auc - 0.5).abs() < 1e-10);
    }

    #[test]
    fn perfect_classifier_auc_is_one() {
        let scores = [0.9, 0.8, 0.2, 0.1];
        let labels = [true, true, false, false];
        let auc = compute_auc(&scores, &labels).expect("auc");
        assert!((auc - 1.0).abs() < 1e-10);
    }

    #[test]
    fn precision_recall_at_threshold() {
        let scores = [0.9, 0.6, 0.3];
        let labels = [true, false, true];
        let pr = compute_precision_recall(&scores, &labels, 0.5).expect("pr");
        assert!((pr.precision - 0.5).abs() < 1e-10);
        assert!((pr.recall - 0.5).abs() < 1e-10);
    }

    #[test]
    fn concordance_perfect_agreement() {
        let values = [1.0, 2.0, 3.0, 4.0];
        let ccc = concordance_correlation(&values, &values).expect("ccc");
        assert!((ccc - 1.0).abs() < 1e-10);
    }

    #[test]
    fn inverted_classifier_auc_near_zero() {
        let scores = [0.1, 0.2, 0.8, 0.9];
        let labels = [true, true, false, false];
        let auc = compute_auc(&scores, &labels).expect("auc");
        assert!(auc < 0.1);
    }

    #[test]
    fn precision_zero_when_no_predictions_above_threshold() {
        let scores = [0.1, 0.2, 0.3];
        let labels = [true, false, true];
        let pr = compute_precision_recall(&scores, &labels, 0.9).expect("pr");
        assert!((pr.precision).abs() < f64::EPSILON);
    }

    #[test]
    fn concordance_negative_for_inverse_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [4.0, 3.0, 2.0, 1.0];
        let ccc = concordance_correlation(&x, &y).expect("ccc");
        assert!(ccc < 0.0);
    }

    #[test]
    fn compute_f1_zero_when_precision_and_recall_zero() {
        let f1 = compute_f1(0.0, 0.0);
        assert!((f1).abs() < f64::EPSILON);
    }

    #[test]
    fn trapezoidal_auc_with_three_points() {
        let points = vec![
            MetricPoint { x: 0.0, y: 0.0 },
            MetricPoint { x: 0.5, y: 0.5 },
            MetricPoint { x: 1.0, y: 1.0 },
        ];
        let auc = trapezoidal_auc(&points).expect("auc");
        assert!((auc - 0.5).abs() < 1e-10);
    }

    #[test]
    fn compute_auc_rejects_empty_input() {
        let err = compute_auc(&[], &[]).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn compute_auc_rejects_all_positive_labels() {
        let scores = [0.5, 0.6];
        let labels = [true, true];
        let err = compute_auc(&scores, &labels).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn compute_auc_rejects_all_negative_labels() {
        let scores = [0.5, 0.6];
        let labels = [false, false];
        let err = compute_auc(&scores, &labels).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn compute_auc_rejects_length_mismatch() {
        let err = compute_auc(&[0.5, 0.6], &[true]).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn precision_recall_length_mismatch() {
        let err = compute_precision_recall(&[0.5], &[true, false], 0.5).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn precision_recall_all_below_threshold() {
        let scores = [0.1, 0.2, 0.3];
        let labels = [true, false, true];
        let pr = compute_precision_recall(&scores, &labels, 0.9).expect("pr");
        assert!(pr.precision.abs() < f64::EPSILON);
        assert!(pr.recall.abs() < f64::EPSILON);
    }

    #[test]
    fn precision_recall_all_above_threshold() {
        let scores = [0.9, 0.8, 0.7];
        let labels = [true, true, false];
        let pr = compute_precision_recall(&scores, &labels, 0.5).expect("pr");
        assert!((pr.precision - 2.0 / 3.0).abs() < 1e-10);
        assert!((pr.recall - 1.0).abs() < 1e-10);
    }

    #[test]
    fn precision_recall_mixed_predictions() {
        let scores = [0.9, 0.8, 0.3, 0.1];
        let labels = [true, false, false, true];
        let pr = compute_precision_recall(&scores, &labels, 0.5).expect("pr");
        assert!((pr.precision - 0.5).abs() < 1e-10);
        assert!((pr.recall - 0.5).abs() < 1e-10);
    }

    #[test]
    fn f1_perfect_precision_and_recall() {
        let f1 = compute_f1(1.0, 1.0);
        assert!((f1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn concordance_length_mismatch() {
        let err = concordance_correlation(&[1.0, 2.0], &[1.0]).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn concordance_too_few_samples() {
        let err = concordance_correlation(&[1.0], &[1.0]).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn concordance_zero_variance() {
        let err = concordance_correlation(&[5.0, 5.0, 5.0], &[1.0, 2.0, 3.0]).unwrap_err();
        assert!(matches!(err, TideGlassError::Enrichment { .. }));
    }

    #[test]
    fn trapezoidal_auc_single_point_insufficient() {
        let err = trapezoidal_auc(&[MetricPoint { x: 0.0, y: 0.5 }]).unwrap_err();
        assert!(matches!(err, TideGlassError::InsufficientData { .. }));
    }

    #[test]
    fn auc_mixed_classifier_between_zero_and_one() {
        let scores = [0.9, 0.7, 0.5, 0.3];
        let labels = [true, false, true, false];
        let auc = compute_auc(&scores, &labels).expect("auc");
        assert!(auc > 0.0 && auc <= 1.0);
    }
}
