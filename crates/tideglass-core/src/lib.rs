// SPDX-License-Identifier: AGPL-3.0-or-later

//! tideGlass core — shared types, enrichment algorithms, and IPC definitions
//! for sovereign drug repurposing within the ecoPrimals ecosystem.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

pub mod discovery;
pub mod enrichment;
pub mod error;
pub mod ipc;
pub mod types;

pub use discovery::{Capability, CapabilityRegistry, InMemoryCapabilityRegistry, PrimalIdentity};
pub use enrichment::{compute_rges, permutation_p_value, weighted_ks_enrichment};
pub use error::{Result, TideGlassError};
pub use ipc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, tideglass_identity};
pub use types::{
    CellLineId, Comparison, CompoundId, DiseaseSignature, EnrichmentConfig, GeneId,
    PerturbationSignature, RgesResult, ValidationTarget,
};

/// Crate version string from `CARGO_PKG_VERSION`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
