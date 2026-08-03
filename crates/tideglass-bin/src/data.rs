// SPDX-License-Identifier: AGPL-3.0-or-later

//! CAS-backed data loading for tideGlass modules.
//!
//! Loads module data from nestGate CAS on server startup. Data is held in an
//! `Arc<ModuleData>` shared across dispatch handlers.

use std::collections::HashMap;

use serde::Deserialize;
use tideglass_core::cas::CasHash;
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
    /// Datasets loaded from CAS.
    pub loaded_datasets: Vec<String>,
    /// Errors encountered during loading (for AAR).
    pub load_errors: Vec<String>,
}

/// `GPS4Drug` pre-trained weights from CAS.
#[derive(Debug, Clone)]
pub struct Gps4DrugWeights {
    pub weights: Vec<Vec<f64>>,
    pub config: tideglass_gps4drug::LinearRegressionConfig,
}

/// Loads module data from nestGate CAS.
///
/// Returns partially-loaded data if some datasets fail — the system remains
/// operational for modules with data while reporting gaps.
pub async fn load_from_cas(client: &CasClient) -> ModuleData {
    let mut data = ModuleData::default();

    match client.list(None).await {
        Ok(list_response) => {
            data.cas_connected = true;
            eprintln!(
                "tideglass: CAS connected — {} objects indexed",
                list_response.count
            );
        }
        Err(err) => {
            let msg = format!("CAS connection failed: {err}");
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
        assert!(data.loaded_datasets.is_empty());
        assert!(data.load_errors.is_empty());
    }

    #[test]
    fn resolve_dataset_hash_returns_none_without_manifest() {
        assert!(resolve_dataset_hash("tideglass.compound_library").is_none());
        assert!(resolve_dataset_hash("tideglass.gps4drug_weights").is_none());
    }
}
