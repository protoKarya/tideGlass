// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! Robust cell line selection via signal-to-noise ranking of RGES across compounds.

pub mod selection;

pub use selection::{CellLineRanking, RclConfig, compute_snr, group_by_cell_line, rank_cell_lines};
