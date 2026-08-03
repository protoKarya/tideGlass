// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
//! Monte Carlo Tree Search for molecular lead optimization.

pub mod action;
pub mod search;
pub mod tree;

pub use action::{MolecularAction, apply_action, default_actions};
pub use search::{MctsConfig, MctsResult, MctsSearch};
pub use tree::{MctsNode, node_at_path, node_at_path_mut, select_path};
