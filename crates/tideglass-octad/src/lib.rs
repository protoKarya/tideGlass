// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! OCTAD benchmark comparison with AUC, precision-recall, and concordance metrics.

pub mod benchmark;
pub mod metrics;

pub use benchmark::{BenchmarkConfig, BenchmarkResult, OctadComparison, RankedCompound};
pub use metrics::{
    MetricPoint, PrecisionRecall, compute_auc, compute_f1, compute_precision_recall,
    concordance_correlation, trapezoidal_auc,
};
