// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! NF1 tumor reversal scoring with tissue geometry weighting.

pub mod scoring;
pub mod tissue;

pub use scoring::{
    NfReversalScore, NfScoringConfig, compute_gene_weights, compute_nf_scores, to_rges_results,
};
pub use tissue::{CompartmentDistance, GeneCompartmentMap, TissueCompartment, TissueWeight};
