// SPDX-License-Identifier: AGPL-3.0-or-later

//! Health triad probes for biomeOS orchestration.

use serde_json::{Value, json};
use std::time::SystemTime;
use tideglass_core::PRIMAL_NAME;

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
        "primal": PRIMAL_NAME,
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
    let routing_label = cas_routing_label(data);

    json!({
        "status": "healthy",
        "primal": PRIMAL_NAME,
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
            "routing": routing_label,
            "datasets_loaded": data.loaded_datasets.len(),
            "converged_datasets": data.converged_datasets.len(),
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

/// `health.readiness` with CAS routing and convergence status.
#[must_use]
pub fn readiness_with_cas(data: &ModuleData) -> Value {
    let routing_label = cas_routing_label(data);

    json!({
        "ready": true,
        "modules_loaded": 7,
        "cas_connected": data.cas_connected,
        "cas_routing": routing_label,
        "cas_datasets": data.loaded_datasets.len(),
        "converged_datasets": data.converged_datasets.len(),
        "timestamp": timestamp_iso(),
    })
}

fn cas_routing_label(data: &ModuleData) -> &'static str {
    data.cas_routing.map_or("none", |r| match r {
        tideglass_core::cas::CasRouting::NeuralApi => "neural-api",
        tideglass_core::cas::CasRouting::Direct => "direct",
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

    #[test]
    fn check_uses_primal_name_constant() {
        let response = check();
        assert_eq!(response["primal"], tideglass_core::PRIMAL_NAME);
    }

    #[test]
    fn check_with_cas_reports_routing_mode() {
        let data = ModuleData {
            cas_routing: Some(tideglass_core::cas::CasRouting::NeuralApi),
            cas_connected: true,
            ..ModuleData::default()
        };

        let response = check_with_cas(&data);
        assert_eq!(response["cas"]["routing"], "neural-api");
        assert_eq!(response["cas"]["connected"], true);
        assert_eq!(response["primal"], tideglass_core::PRIMAL_NAME);
    }

    #[test]
    fn check_with_cas_direct_routing() {
        let data = ModuleData {
            cas_routing: Some(tideglass_core::cas::CasRouting::Direct),
            ..ModuleData::default()
        };

        let response = check_with_cas(&data);
        assert_eq!(response["cas"]["routing"], "direct");
    }

    #[test]
    fn check_with_cas_no_routing() {
        let data = ModuleData::default();
        let response = check_with_cas(&data);
        assert_eq!(response["cas"]["routing"], "none");
    }

    #[test]
    fn readiness_with_cas_reports_convergence() {
        let data = ModuleData {
            cas_connected: true,
            converged_datasets: vec!["test".to_owned()],
            ..ModuleData::default()
        };

        let response = readiness_with_cas(&data);
        assert_eq!(response["converged_datasets"], 1);
        assert_eq!(response["cas_connected"], true);
    }
}
