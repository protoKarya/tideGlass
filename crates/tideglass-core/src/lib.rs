// SPDX-License-Identifier: AGPL-3.0-or-later
// tideGlass core — Gen5 drug repurposing via RGES pipeline
//
// Phase 0: Reproduce GPS4Drug RGES pipeline using local ChEMBL + LINCS data
// from westGate CAS (362 GB federation, 13 datasets in tideGlass domain).

pub mod rges;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
