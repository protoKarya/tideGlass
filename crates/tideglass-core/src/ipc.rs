// SPDX-License-Identifier: AGPL-3.0-or-later

//! JSON-RPC 2.0 request/response types and tideGlass method constants.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::VERSION;
use crate::discovery::{Capability, PrimalIdentity};

/// JSON-RPC 2.0 method name constants following semantic `domain.operation` naming.
pub mod methods {
    /// RGES compound screening against a disease signature.
    pub const RGES_SCREEN: &str = "science.rges_screen";
    /// Representative cell line selection for perturbation matching.
    pub const RCL_SELECT: &str = "science.rcl_select";
    /// `GPS4Drug` deep learning prediction endpoint.
    pub const GPS4DRUG_PREDICT: &str = "science.gps4drug_predict";
    /// Large-scale compound library screening.
    pub const COMPOUND_SCREEN: &str = "science.compound_screen";
    /// MCTS-based combination optimization.
    pub const MCTS_OPTIMIZE: &str = "science.mcts_optimize";
    /// OCTAD benchmark evaluation.
    pub const OCTAD_BENCHMARK: &str = "science.octad_benchmark";
    /// Network fragmentation score computation.
    pub const NF_SCORE: &str = "science.nf_score";

    /// Capability discovery — list methods this primal implements.
    pub const CAPABILITIES_LIST: &str = "capabilities.list";
    /// Liveness probe for orchestrators.
    pub const HEALTH_LIVENESS: &str = "health.liveness";
    /// Detailed health check with component status.
    pub const HEALTH_CHECK: &str = "health.check";
    /// Readiness probe — true when the primal can accept work.
    pub const HEALTH_READINESS: &str = "health.readiness";
}

/// Standard JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version, typically `"2.0"`.
    pub jsonrpc: Arc<str>,
    /// Method name in `domain.operation` form.
    pub method: Arc<str>,
    /// Optional parameters payload.
    pub params: Option<serde_json::Value>,
    /// Request correlation id.
    pub id: serde_json::Value,
}

/// Standard JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version, typically `"2.0"`.
    pub jsonrpc: Arc<str>,
    /// Result payload on success.
    pub result: Option<serde_json::Value>,
    /// Error object on failure.
    pub error: Option<JsonRpcError>,
    /// Correlation id matching the request.
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i64,
    /// Short error message.
    pub message: String,
    /// Optional structured error data.
    pub data: Option<serde_json::Value>,
}

/// Builds the tideGlass core primal identity from crate version and IPC method constants.
#[must_use]
pub fn tideglass_identity() -> PrimalIdentity {
    PrimalIdentity {
        name: Arc::from("tideglass-core"),
        version: Arc::from(VERSION),
        capabilities: vec![
            capability(
                methods::CAPABILITIES_LIST,
                "List tideGlass core capabilities",
            ),
            capability(methods::HEALTH_LIVENESS, "Liveness probe"),
            capability(methods::HEALTH_CHECK, "Health check with component status"),
            capability(methods::HEALTH_READINESS, "Readiness probe"),
        ],
    }
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
    use super::*;

    #[test]
    fn method_constants_use_domain_operation_naming() {
        assert!(methods::RGES_SCREEN.contains('.'));
        assert!(methods::CAPABILITIES_LIST.contains('.'));
        assert!(methods::HEALTH_LIVENESS.starts_with("health."));
    }

    #[test]
    fn tideglass_identity_includes_health_and_capabilities() {
        let identity = tideglass_identity();
        assert_eq!(identity.name.as_ref(), "tideglass-core");
        assert_eq!(identity.version.as_ref(), VERSION);

        let methods: Vec<&str> = identity
            .capabilities
            .iter()
            .map(|cap| cap.method.as_ref())
            .collect();

        assert!(methods.contains(&super::methods::CAPABILITIES_LIST));
        assert!(methods.contains(&super::methods::HEALTH_LIVENESS));
        assert!(methods.contains(&super::methods::HEALTH_READINESS));
    }

    #[test]
    fn json_rpc_request_round_trip() {
        let request = JsonRpcRequest {
            jsonrpc: Arc::from("2.0"),
            method: Arc::from(methods::RGES_SCREEN),
            params: Some(serde_json::json!({"disease": "melanoma"})),
            id: serde_json::json!(1),
        };

        let json = serde_json::to_string(&request).expect("serialize request");
        let back: JsonRpcRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(back.method.as_ref(), methods::RGES_SCREEN);
    }

    #[test]
    fn json_rpc_error_response_serializes() {
        let response = JsonRpcResponse {
            jsonrpc: Arc::from("2.0"),
            result: None,
            error: Some(JsonRpcError {
                code: -32_600,
                message: "Invalid Request".to_owned(),
                data: None,
            }),
            id: serde_json::json!(null),
        };

        let json = serde_json::to_string(&response).expect("serialize response");
        assert!(json.contains("Invalid Request"));
    }
}
