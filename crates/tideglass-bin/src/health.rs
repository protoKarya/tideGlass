// SPDX-License-Identifier: AGPL-3.0-or-later

//! Health triad probes for biomeOS orchestration.

use serde_json::{Value, json};
use std::time::SystemTime;

use crate::data::ModuleData;

/// `health.liveness` — "is the process alive?" Always true if responding.
#[must_use]
pub fn liveness() -> Value {
    json!({
        "alive": true,
        "timestamp": timestamp_iso(),
    })
}

/// `health.check` — detailed component status (no CAS info).
#[cfg(test)]
#[must_use]
pub fn check() -> Value {
    json!({
        "status": "healthy",
        "primal": "tideglass",
        "version": tideglass_core::VERSION,
        "components": {
            "rges": "ready",
            "rcl": "ready",
            "gps4drug": "ready",
            "screen": "ready",
            "molsearch": "ready",
            "octad": "ready",
            "nf": "ready",
        },
        "timestamp": timestamp_iso(),
    })
}

/// `health.check` with CAS connection and data-loading status.
#[must_use]
pub fn check_with_cas(data: &ModuleData) -> Value {
    json!({
        "status": "healthy",
        "primal": "tideglass",
        "version": tideglass_core::VERSION,
        "components": {
            "rges": "ready",
            "rcl": "ready",
            "gps4drug": if data.gps4drug_weights.is_some() { "ready (CAS)" } else { "ready (caller-supplied)" },
            "screen": if data.compound_library.is_some() { "ready (CAS)" } else { "ready (caller-supplied)" },
            "molsearch": "ready",
            "octad": if data.known_actives.is_some() { "ready (CAS)" } else { "ready (caller-supplied)" },
            "nf": "ready",
        },
        "cas": {
            "connected": data.cas_connected,
            "datasets_loaded": data.loaded_datasets.len(),
            "load_errors": data.load_errors.len(),
        },
        "timestamp": timestamp_iso(),
    })
}

/// `health.readiness` — "can the primal accept work?" True when server is listening.
#[cfg(test)]
#[must_use]
pub fn readiness() -> Value {
    json!({
        "ready": true,
        "modules_loaded": 7,
        "timestamp": timestamp_iso(),
    })
}

/// `health.readiness` with CAS status.
#[must_use]
pub fn readiness_with_cas(data: &ModuleData) -> Value {
    json!({
        "ready": true,
        "modules_loaded": 7,
        "cas_connected": data.cas_connected,
        "cas_datasets": data.loaded_datasets.len(),
        "timestamp": timestamp_iso(),
    })
}

fn timestamp_iso() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or_else(
            |_| "0".to_owned(),
            |duration| duration.as_secs().to_string(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liveness_returns_alive() {
        let response = liveness();
        assert_eq!(response["alive"], true);
    }

    #[test]
    fn check_returns_healthy_with_seven_components() {
        let response = check();
        assert_eq!(response["status"], "healthy");
        assert_eq!(
            response["components"]
                .as_object()
                .expect("components object")
                .len(),
            7
        );
    }

    #[test]
    fn readiness_returns_ready_with_modules_loaded() {
        let response = readiness();
        assert_eq!(response["ready"], true);
        assert_eq!(response["modules_loaded"], 7);
    }

    #[test]
    fn all_responses_contain_timestamp() {
        for response in [liveness(), check(), readiness()] {
            assert!(response.get("timestamp").is_some());
        }
    }
}
