// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! `GPS4Drug` structure-to-expression prediction from molecular descriptors.

pub mod features;
pub mod prediction;

pub use features::{MolecularFeatures, PhysicochemicalProperties};
pub use prediction::{
    ExpressionPrediction, ExpressionPredictor, GeneExpressionPrediction, LinearRegressionConfig,
    LinearRegressionPredictor, compute_r_squared,
};
