// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! Compound library screening with Lipinski, RGES, and structural alert filters.

pub mod filter;
pub mod library;

pub use filter::{
    FilterRejection, ScreenFilterConfig, StructuralAlertConfig, explain_rejections,
    filter_ranked_hits,
};
pub use library::{CompoundLibrary, LibraryCompound, LipinskiConfig, LipinskiViolation};
