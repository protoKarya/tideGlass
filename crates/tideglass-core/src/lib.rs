// SPDX-License-Identifier: AGPL-3.0-or-later

//! tideGlass core — shared types, enrichment algorithms, and IPC definitions
//! for sovereign drug repurposing within the ecoPrimals ecosystem.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

pub mod cas;
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

/// Canonical primal name for tideGlass — single source of truth used by
/// all identity, health, and IPC responses.
pub const PRIMAL_NAME: &str = "tideglass";

/// Converts a `usize` to `f64` for statistical computation.
///
/// Gene counts, compound counts, and sample sizes in drug repurposing pipelines
/// are always far below 2^52, so this conversion is exact. This centralizes the
/// `cast_precision_loss` allowance rather than scattering `#[allow]` annotations.
#[allow(clippy::cast_precision_loss)]
#[inline]
#[must_use]
pub const fn count_as_f64(n: usize) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_name_is_tideglass() {
        assert_eq!(PRIMAL_NAME, "tideglass");
    }

    #[test]
    fn version_is_nonempty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn count_as_f64_exact_for_small_values() {
        assert!((count_as_f64(0) - 0.0).abs() < f64::EPSILON);
        assert!((count_as_f64(1) - 1.0).abs() < f64::EPSILON);
        assert!((count_as_f64(1000) - 1000.0).abs() < f64::EPSILON);
        assert!((count_as_f64(100_000) - 100_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn count_as_f64_exact_up_to_2_pow_52() {
        let max_exact: usize = 1 << 52;
        let result = count_as_f64(max_exact);
        assert!(result > 0.0);
        assert!((result - 4_503_599_627_370_496.0).abs() < f64::EPSILON);
    }

    #[test]
    fn identity_uses_primal_name() {
        let id = tideglass_identity();
        assert_eq!(id.name.as_ref(), PRIMAL_NAME);
    }
}
