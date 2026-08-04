// SPDX-License-Identifier: AGPL-3.0-or-later

//! CAS-backed data loading for tideGlass modules.
//!
//! Loads module data from CAS (routed via biomeOS Neural API or direct to
//! nestGate) on server startup. Data is held in an `Arc<ModuleData>` shared
//! across dispatch handlers.

use std::collections::HashMap;

use serde::Deserialize;
use tideglass_core::cas::{CasHash, CasRouting};
use tideglass_core::error::TideGlassError;
use tideglass_core::types::CompoundId;
use tideglass_screen::{CompoundLibrary, LibraryCompound};

use crate::cas_client::CasClient;

/// Pre-loaded module data from CAS, shared across all dispatch handlers.
#[derive(Debug, Default)]
pub struct ModuleData {
    /// Compound library for screening (Module 4).
    pub compound_library: Option<CompoundLibrary>,
    /// `GPS4Drug` weight matrix + config (Module 3).
    pub gps4drug_weights: Option<Gps4DrugWeights>,
    /// OCTAD known actives for benchmarking (Module 6).
    pub known_actives: Option<HashMap<CompoundId, bool>>,
    /// CAS connection status for health reporting.
    pub cas_connected: bool,
    /// How CAS is routed (`NeuralApi` or `Direct`).
    pub cas_routing: Option<CasRouting>,
    /// Datasets loaded from CAS.
    pub loaded_datasets: Vec<String>,
    /// Errors encountered during loading (for AAR).
    pub load_errors: Vec<String>,
    /// Datasets that have been verified as fully converged (braided provenance).
    pub converged_datasets: Vec<String>,
    /// CAS socket path — retained so dispatch can create write-back clients for
    /// storing pipeline results.
    #[allow(dead_code)]
    pub cas_socket_path: Option<String>,
    /// Routing mode for write-back clients.
    #[allow(dead_code)]
    pub cas_write_routing: Option<CasRouting>,
}

/// `GPS4Drug` pre-trained weights from CAS.
#[derive(Debug, Clone)]
pub struct Gps4DrugWeights {
    pub weights: Vec<Vec<f64>>,
    pub config: tideglass_gps4drug::LinearRegressionConfig,
}

/// Loads module data from CAS (Neural API or direct nestGate).
///
/// Returns partially-loaded data if some datasets fail — the system remains
/// operational for modules with data while reporting gaps.
pub async fn load_from_cas(client: &CasClient) -> ModuleData {
    let mut data = ModuleData {
        cas_routing: Some(client.routing()),
        cas_socket_path: Some(client.socket_path().to_owned()),
        cas_write_routing: Some(client.routing()),
        ..ModuleData::default()
    };

    match client.check_health().await {
        Ok(version) => {
            data.cas_connected = true;
            let route_label = match client.routing() {
                CasRouting::NeuralApi => "Neural API",
                CasRouting::Direct => "direct",
            };
            eprintln!(
                "tideglass: CAS healthy via {route_label} — nestGate v{version}",
            );
        }
        Err(err) => {
            let msg = format!("CAS health check failed: {err}");
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg);
            return data;
        }
    }

    load_compound_library(client, &mut data).await;
    load_gps4drug_weights(client, &mut data).await;
    load_known_actives(client, &mut data).await;

    eprintln!(
        "tideglass: CAS load complete — {} datasets, {} errors",
        data.loaded_datasets.len(),
        data.load_errors.len()
    );

    data
}

/// Attempts to load compound library from CAS.
///
/// Looks for a JSON-serialized `CompoundLibrary` stored under a well-known
/// pipeline key. If not found, logs the gap — screening will require
/// caller-supplied library data.
async fn load_compound_library(client: &CasClient, data: &mut ModuleData) {
    let result =
        try_load_json::<Vec<LibraryCompound>>(client, "tideglass.compound_library", data).await;

    match result {
        Ok(Some(compounds)) => {
            let mut library = CompoundLibrary::new();
            for compound in compounds {
                library.add(compound);
            }
            eprintln!(
                "tideglass: compound library loaded — {} compounds",
                library.compounds.len()
            );
            data.compound_library = Some(library);
            data.loaded_datasets.push("compound_library".to_owned());
        }
        Ok(None) => {
            let msg = "compound library not found in CAS — screening requires caller-supplied data";
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg.to_owned());
        }
        Err(err) => {
            let msg = format!("compound library load failed: {err}");
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg);
        }
    }
}

/// Attempts to load `GPS4Drug` weight matrix from CAS.
async fn load_gps4drug_weights(client: &CasClient, data: &mut ModuleData) {
    let result = try_load_json::<Gps4DrugWeights>(client, "tideglass.gps4drug_weights", data).await;

    match result {
        Ok(Some(weights)) => {
            eprintln!(
                "tideglass: GPS4Drug weights loaded — {} target genes",
                weights.weights.len()
            );
            data.gps4drug_weights = Some(weights);
            data.loaded_datasets.push("gps4drug_weights".to_owned());
        }
        Ok(None) => {
            let msg =
                "GPS4Drug weights not found in CAS — prediction requires caller-supplied weights";
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg.to_owned());
        }
        Err(err) => {
            let msg = format!("GPS4Drug weights load failed: {err}");
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg);
        }
    }
}

/// Attempts to load OCTAD known actives from CAS.
async fn load_known_actives(client: &CasClient, data: &mut ModuleData) {
    let result =
        try_load_json::<HashMap<CompoundId, bool>>(client, "tideglass.octad_known_actives", data)
            .await;

    match result {
        Ok(Some(actives)) => {
            eprintln!(
                "tideglass: OCTAD known actives loaded — {} compounds",
                actives.len()
            );
            data.known_actives = Some(actives);
            data.loaded_datasets.push("octad_known_actives".to_owned());
        }
        Ok(None) => {
            let msg = "OCTAD known actives not found in CAS — benchmarking requires caller data";
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg.to_owned());
        }
        Err(err) => {
            let msg = format!("OCTAD known actives load failed: {err}");
            eprintln!("tideglass: {msg}");
            data.load_errors.push(msg);
        }
    }
}

/// Attempts to load a JSON-serialized type from CAS by hash.
///
/// Uses `content.exists` first to check, then `content.get` to retrieve.
/// Returns `Ok(None)` if the hash is not found in CAS.
async fn try_load_json<T: serde::de::DeserializeOwned>(
    client: &CasClient,
    dataset_key: &str,
    data: &mut ModuleData,
) -> Result<Option<T>, TideGlassError> {
    let Some(hash) = resolve_dataset_hash(dataset_key) else {
        let msg = format!(
            "no CAS hash configured for {dataset_key} — \
             awaiting data manifest or CAS ingest with pipeline tag"
        );
        data.load_errors.push(msg);
        return Ok(None);
    };

    if !client.exists(hash.as_str()).await? {
        return Ok(None);
    }

    let bytes = client.get(hash.as_str()).await?.ok_or_else(|| {
        TideGlassError::DataAccess(format!(
            "CAS object {hash} exists but content.get returned null"
        ))
    })?;

    let value: T = serde_json::from_slice(&bytes).map_err(|err| {
        TideGlassError::DataAccess(format!("deserialize {dataset_key} from CAS: {err}"))
    })?;

    Ok(Some(value))
}

/// Resolves a dataset key to its CAS hash.
///
/// In the current implementation, this returns `None` for all keys — the GPS
/// platform data is CAS-indexed but we need the specific BLAKE3 hashes from
/// the data manifest. This is a documented Phase 4 integration gap.
///
/// Future: read from `data_manifest.toml` or query CAS by pipeline tag.
const fn resolve_dataset_hash(_dataset_key: &str) -> Option<CasHash> {
    // AAR DIVERGENCE: The GPS platform data (8 files, 1.4 GB) is CAS-indexed
    // on westGate, but the individual BLAKE3 hashes for each dataset component
    // are not yet enumerated in tideGlass configuration. The CAS ingest pipeline
    // stores files with provenance metadata (source, pipeline) but there's no
    // query-by-tag API — only query-by-hash via content.get.
    //
    // Options:
    // 1. content.list + iterate to find objects with matching metadata (expensive)
    // 2. Store a data_manifest.toml with known hashes after ingest (preferred)
    // 3. Add content.query_by_tag to nestGate (upstream request)
    //
    // For now, dispatch handlers fall through to caller-supplied params when
    // CAS data is not pre-loaded.
    None
}

/// Stores a pipeline result in CAS for provenance tracking.
///
/// Creates an ephemeral `CasClient` from the stored socket path and writes
/// the serialized JSON result. Returns the BLAKE3 hash of the stored object.
/// Called by async dispatch handlers when provenance write is enabled.
///
/// # Errors
///
/// Returns an error if CAS is not connected or the write fails.
#[allow(dead_code)]
pub async fn store_pipeline_result(
    data: &ModuleData,
    result_json: &[u8],
    pipeline: &str,
) -> Result<String, TideGlassError> {
    let socket = data
        .cas_socket_path
        .as_deref()
        .ok_or_else(|| TideGlassError::DataAccess("CAS not connected for result storage".into()))?;
    let routing = data.cas_write_routing.unwrap_or(CasRouting::NeuralApi);

    let client = CasClient::new(socket, routing);
    let response = client
        .put(
            result_json,
            Some("application/json"),
            Some(tideglass_core::PRIMAL_NAME),
            Some(pipeline),
        )
        .await?;

    Ok(response.hash)
}

/// Checks whether a dataset has converged to fully braided provenance.
///
/// Called by dispatch handlers before trusting pipeline outputs for the
/// provenance chain. Not yet wired — awaiting Neural API routing to
/// provenance trio primals (rhizoCrypt, loamSpine, sweetGrass).
#[allow(dead_code)]
///
/// westGate data exists in three provenance states:
/// - **primordial**: on disk, no CAS hash
/// - **CAS-only**: BLAKE3 hash in CAS, but no DAG event or spine entry
/// - **fully braided**: CAS hash + rhizoCrypt DAG + loamSpine spine + sweetGrass braid
///
/// Science modules should only trust computation results when input data is
/// fully braided. This gate prevents computation on partially-provenance'd data
/// that may later be invalidated during convergence.
///
/// Currently checks CAS existence only (CAS-only state). Full convergence
/// requires querying rhizoCrypt `dag.session.query` — blocked on Neural API
/// routing to provenance trio primals.
pub async fn is_dataset_converged(client: &CasClient, dataset_key: &str) -> DatasetConvergence {
    let Some(hash) = resolve_dataset_hash(dataset_key) else {
        return DatasetConvergence::Unknown;
    };

    match client.exists(hash.as_str()).await {
        Ok(true) => DatasetConvergence::CasOnly,
        Ok(false) => DatasetConvergence::NotFound,
        Err(_) => DatasetConvergence::Unknown,
    }
}

/// Provenance convergence state for a dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DatasetConvergence {
    /// Not in CAS at all.
    NotFound,
    /// In CAS with BLAKE3 hash but no provenance chain (DAG/spine/braid).
    CasOnly,
    /// Fully braided: CAS + `rhizoCrypt` DAG + loamSpine + sweetGrass.
    FullyBraided,
    /// Cannot determine convergence state (no hash configured or CAS unreachable).
    Unknown,
}

impl DatasetConvergence {
    /// Whether computation is safe on this dataset.
    ///
    /// Currently permits `CasOnly` and `FullyBraided` — in production, only
    /// `FullyBraided` should be trusted for results that enter the provenance chain.
    #[must_use]
    #[allow(dead_code)]
    pub const fn is_computation_safe(self) -> bool {
        matches!(self, Self::CasOnly | Self::FullyBraided)
    }
}

// Gps4DrugWeights needs Deserialize for try_load_json
impl<'de> Deserialize<'de> for Gps4DrugWeights {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            weights: Vec<Vec<f64>>,
            #[serde(default)]
            intercept: f64,
            #[serde(default)]
            target_genes: Vec<String>,
        }

        let inner = Inner::deserialize(deserializer)?;
        Ok(Self {
            config: tideglass_gps4drug::LinearRegressionConfig {
                intercept: inner.intercept,
                target_genes: inner
                    .target_genes
                    .into_iter()
                    .map(|g| tideglass_core::types::GeneId::new(&g))
                    .collect(),
            },
            weights: inner.weights,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_data_defaults_to_empty() {
        let data = ModuleData::default();
        assert!(data.compound_library.is_none());
        assert!(data.gps4drug_weights.is_none());
        assert!(data.known_actives.is_none());
        assert!(!data.cas_connected);
        assert!(data.cas_routing.is_none());
        assert!(data.loaded_datasets.is_empty());
        assert!(data.converged_datasets.is_empty());
        assert!(data.load_errors.is_empty());
        assert!(data.cas_socket_path.is_none());
        assert!(data.cas_write_routing.is_none());
    }

    #[test]
    fn resolve_dataset_hash_returns_none_without_manifest() {
        assert!(resolve_dataset_hash("tideglass.compound_library").is_none());
        assert!(resolve_dataset_hash("tideglass.gps4drug_weights").is_none());
        assert!(resolve_dataset_hash("tideglass.octad_known_actives").is_none());
        assert!(resolve_dataset_hash("nonexistent.key").is_none());
    }

    #[test]
    fn convergence_computation_safety() {
        assert!(!DatasetConvergence::NotFound.is_computation_safe());
        assert!(DatasetConvergence::CasOnly.is_computation_safe());
        assert!(DatasetConvergence::FullyBraided.is_computation_safe());
        assert!(!DatasetConvergence::Unknown.is_computation_safe());
    }

    #[test]
    fn convergence_equality() {
        assert_eq!(DatasetConvergence::CasOnly, DatasetConvergence::CasOnly);
        assert_ne!(
            DatasetConvergence::CasOnly,
            DatasetConvergence::FullyBraided
        );
    }
}
