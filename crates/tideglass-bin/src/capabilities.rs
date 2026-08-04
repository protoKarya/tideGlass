// SPDX-License-Identifier: AGPL-3.0-or-later

//! Capability discovery for the tideGlass `UniBin`.

use std::sync::Arc;

use serde_json::{Value, json};
use tideglass_core::discovery::Capability;
use tideglass_core::ipc::methods;
use tideglass_core::{PRIMAL_NAME, VERSION};

/// Returns the canonical `capabilities.list` response envelope.
#[must_use]
pub fn list() -> Value {
    let capabilities = all_capabilities();
    let count = capabilities.len();
    json!({
        "capabilities": capabilities,
        "count": count,
        "primal": PRIMAL_NAME,
        "version": VERSION,
    })
}

fn all_capabilities() -> Vec<Capability> {
    vec![
        capability(
            methods::CAPABILITIES_LIST,
            "List tideGlass JSON-RPC methods",
        ),
        capability(methods::HEALTH_LIVENESS, "Liveness probe"),
        capability(methods::HEALTH_CHECK, "Health check with component status"),
        capability(methods::HEALTH_READINESS, "Readiness probe"),
        capability(
            methods::RGES_SCREEN,
            "RGES compound screening against a disease signature",
        ),
        capability(methods::RCL_SELECT, "Representative cell line selection"),
        capability(
            methods::GPS4DRUG_PREDICT,
            "Structure-to-expression prediction",
        ),
        capability(
            methods::COMPOUND_SCREEN,
            "Large-scale compound library screening",
        ),
        capability(
            methods::MCTS_OPTIMIZE,
            "MCTS-based combination optimization",
        ),
        capability(methods::OCTAD_BENCHMARK, "OCTAD benchmark evaluation"),
        capability(methods::NF_SCORE, "Network fragmentation score computation"),
    ]
}

fn capability(method: &str, description: &str) -> Capability {
    Capability {
        method: Arc::from(method),
        version: Arc::from(VERSION),
        description: Arc::from(description),
    }
}

#[cfg(test)]
mod tests {
    use tideglass_core::ipc::methods;

    use super::*;

    #[test]
    fn list_returns_eleven_capabilities() {
        let response = list();
        assert_eq!(response["count"], 11);
        assert_eq!(response["primal"], "tideglass");
        assert_eq!(response["version"], VERSION);
        assert_eq!(
            response["capabilities"]
                .as_array()
                .expect("capabilities array")
                .len(),
            11
        );
    }

    #[test]
    fn list_includes_all_science_and_health_methods() {
        let response = list();
        let listed_methods: Vec<&str> = response["capabilities"]
            .as_array()
            .expect("capabilities array")
            .iter()
            .filter_map(|cap| cap["method"].as_str())
            .collect();

        for expected in [
            methods::CAPABILITIES_LIST,
            methods::HEALTH_LIVENESS,
            methods::HEALTH_CHECK,
            methods::HEALTH_READINESS,
            methods::RGES_SCREEN,
            methods::RCL_SELECT,
            methods::GPS4DRUG_PREDICT,
            methods::COMPOUND_SCREEN,
            methods::MCTS_OPTIMIZE,
            methods::OCTAD_BENCHMARK,
            methods::NF_SCORE,
        ] {
            assert!(
                listed_methods.contains(&expected),
                "missing capability: {expected}"
            );
        }
    }

    #[test]
    fn each_capability_has_method_version_and_description() {
        let response = list();
        for cap in response["capabilities"]
            .as_array()
            .expect("capabilities array")
        {
            assert!(cap["method"].as_str().is_some());
            assert_eq!(cap["version"], VERSION);
            assert!(cap["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
    }
}
