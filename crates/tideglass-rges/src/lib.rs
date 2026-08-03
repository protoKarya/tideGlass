// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! RGES reversal gene expression screening pipeline with Benjamini–Hochberg FDR correction.

pub mod pipeline;
pub mod screen;

pub use pipeline::RgesPipeline;
pub use screen::{RankedRgesHit, ScreenConfig, benjamini_hochberg, screen_compounds};
