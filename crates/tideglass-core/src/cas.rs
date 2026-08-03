// SPDX-License-Identifier: AGPL-3.0-or-later

//! Content-Addressable Storage (CAS) types for nestGate integration.
//!
//! Defines the JSON-RPC request/response types for `content.get`, `content.put`,
//! `content.exists`, and `content.list` per the nestGate NG-1 contract.
//! The async client lives in `tideglass-bin`; these are transport-agnostic types.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// BLAKE3 hash identifying a CAS object (64-char lowercase hex).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CasHash(pub Arc<str>);

impl CasHash {
    #[must_use]
    pub fn new(hash: &str) -> Self {
        Self(Arc::from(hash))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CasHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Parameters for `content.get`.
#[derive(Debug, Serialize)]
pub struct CasGetParams<'a> {
    pub hash: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<&'a str>,
}

/// Response from `content.get` (inline, <= 64 MiB).
#[derive(Debug, Deserialize)]
pub struct CasGetResponse {
    /// Base64-encoded content, or `null` if missing.
    pub data: Option<String>,
    pub hash: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub family_id: Option<String>,
    #[serde(default)]
    pub retrieved_in_ms: Option<f64>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub stored_at: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub pipeline: Option<String>,
    #[serde(default)]
    pub stored_by: Option<String>,
    /// When true, object exceeds inline limit; use streaming API.
    #[serde(default)]
    pub use_streaming: Option<bool>,
    #[serde(default)]
    pub streaming_method: Option<String>,
}

/// Parameters for `content.exists`.
#[derive(Debug, Serialize)]
pub struct CasExistsParams<'a> {
    pub hash: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<&'a str>,
}

/// Response from `content.exists`.
#[derive(Debug, Deserialize)]
pub struct CasExistsResponse {
    pub exists: bool,
    pub hash: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub family_id: Option<String>,
}

/// Parameters for `content.list`.
#[derive(Debug, Serialize)]
pub struct CasListParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<&'a str>,
}

/// Single entry in a `content.list` response.
#[derive(Debug, Deserialize)]
pub struct CasListEntry {
    pub hash: String,
    #[serde(default)]
    pub size: u64,
}

/// Response from `content.list`.
#[derive(Debug, Deserialize)]
pub struct CasListResponse {
    /// Note: the live handler returns `hashes`, not `items`.
    pub hashes: Vec<CasListEntry>,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub family_id: Option<String>,
}

/// Parameters for `content.put`.
#[derive(Debug, Serialize)]
pub struct CasPutParams<'a> {
    /// Base64-encoded content.
    pub data: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<&'a str>,
}

/// Response from `content.put`.
#[derive(Debug, Deserialize)]
pub struct CasPutResponse {
    pub hash: String,
    pub size: u64,
    pub stored: bool,
    #[serde(default)]
    pub deduplicated: bool,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub family_id: Option<String>,
}

/// CAS method constants matching the nestGate NG-1 contract.
pub mod methods {
    pub const CONTENT_GET: &str = "content.get";
    pub const CONTENT_EXISTS: &str = "content.exists";
    pub const CONTENT_LIST: &str = "content.list";
    pub const CONTENT_PUT: &str = "content.put";
}

/// Well-known socket paths for nestGate discovery, in priority order.
pub const NESTGATE_SOCKET_ENV: &str = "NESTGATE_SOCKET";

/// Discovers the nestGate socket path using the standard resolution chain.
///
/// Priority: `NESTGATE_SOCKET` env var → `$XDG_RUNTIME_DIR/biomeos/nestgate.sock`
/// → `/run/membrane/nestgate.sock` → `None`.
#[must_use]
pub fn discover_nestgate_socket() -> Option<String> {
    if let Ok(path) = std::env::var(NESTGATE_SOCKET_ENV) {
        if !path.is_empty() {
            return Some(path);
        }
    }

    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        let path = format!("{xdg}/biomeos/nestgate.sock");
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    let membrane_path = "/run/membrane/nestgate.sock";
    if std::path::Path::new(membrane_path).exists() {
        return Some(membrane_path.to_owned());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cas_hash_roundtrip() {
        let hash = CasHash::new("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2");
        assert_eq!(hash.as_str().len(), 64);
    }

    #[test]
    fn cas_get_params_serializes() {
        let params = CasGetParams {
            hash: "abc123",
            family_id: None,
        };
        let json = serde_json::to_value(&params).expect("serialize");
        assert_eq!(json["hash"], "abc123");
        assert!(json.get("family_id").is_none());
    }

    #[test]
    fn cas_get_params_with_family_serializes() {
        let params = CasGetParams {
            hash: "abc123",
            family_id: Some("westgate"),
        };
        let json = serde_json::to_value(&params).expect("serialize");
        assert_eq!(json["family_id"], "westgate");
    }

    #[test]
    fn cas_get_response_deserializes_inline() {
        let json = serde_json::json!({
            "data": "aGVsbG8=",
            "hash": "abc123",
            "size": 5,
            "family_id": "default",
            "retrieved_in_ms": 1.23,
            "content_type": "text/plain"
        });
        let response: CasGetResponse = serde_json::from_value(json).expect("deserialize");
        assert_eq!(response.data.as_deref(), Some("aGVsbG8="));
        assert_eq!(response.size, 5);
        assert!(response.use_streaming.is_none());
    }

    #[test]
    fn cas_get_response_deserializes_missing() {
        let json = serde_json::json!({
            "data": null,
            "hash": "abc123",
            "family_id": "default"
        });
        let response: CasGetResponse = serde_json::from_value(json).expect("deserialize");
        assert!(response.data.is_none());
    }

    #[test]
    fn cas_get_response_deserializes_streaming_redirect() {
        let json = serde_json::json!({
            "data": null,
            "hash": "abc123",
            "size": 293_000_000_000_u64,
            "use_streaming": true,
            "streaming_method": "content.retrieve_stream"
        });
        let response: CasGetResponse = serde_json::from_value(json).expect("deserialize");
        assert_eq!(response.use_streaming, Some(true));
        assert_eq!(
            response.streaming_method.as_deref(),
            Some("content.retrieve_stream")
        );
    }

    #[test]
    fn cas_list_response_deserializes() {
        let json = serde_json::json!({
            "hashes": [
                {"hash": "aaa", "size": 100},
                {"hash": "bbb", "size": 200}
            ],
            "count": 2,
            "family_id": "default"
        });
        let response: CasListResponse = serde_json::from_value(json).expect("deserialize");
        assert_eq!(response.hashes.len(), 2);
        assert_eq!(response.count, 2);
    }

    #[test]
    fn cas_put_params_serializes_minimal() {
        let params = CasPutParams {
            data: "aGVsbG8=",
            content_type: None,
            source: None,
            pipeline: None,
            stored_by: None,
            family_id: None,
        };
        let json = serde_json::to_value(&params).expect("serialize");
        assert_eq!(json["data"], "aGVsbG8=");
        assert!(json.get("content_type").is_none());
    }

    #[test]
    fn cas_put_response_deserializes() {
        let json = serde_json::json!({
            "hash": "abc123",
            "size": 5,
            "stored": true,
            "deduplicated": false,
            "content_type": "text/plain"
        });
        let response: CasPutResponse = serde_json::from_value(json).expect("deserialize");
        assert!(response.stored);
        assert!(!response.deduplicated);
    }

    #[test]
    fn discover_socket_does_not_panic() {
        let result = discover_nestgate_socket();
        // May return Some on westGate with live socket, or None otherwise
        let _ = result;
    }
}
