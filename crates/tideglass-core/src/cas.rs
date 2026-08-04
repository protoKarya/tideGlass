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

/// Environment variable for Neural API socket override.
pub const NEURAL_API_SOCKET_ENV: &str = "NEURAL_API_SOCKET";
/// Fallback: direct nestGate socket override (bypasses Neural API routing).
pub const NESTGATE_SOCKET_ENV: &str = "NESTGATE_SOCKET";

/// Directories to scan for NUCLEUS sockets, in priority order.
/// Live NUCLEUS uses `membrane/`; earlier doc drafts referenced `biomeos/`.
const SOCKET_DIRS: &[&str] = &["membrane", "biomeos"];

/// Discovers the CAS transport socket using the G56 Neural API routing pattern.
///
/// Priority:
/// 1. `NEURAL_API_SOCKET` env var (explicit override)
/// 2. `$XDG_RUNTIME_DIR/{membrane,biomeos}/neural-api-*.sock` (Neural API, glob)
/// 3. `NESTGATE_SOCKET` env var (direct, bypasses routing)
/// 4. `$XDG_RUNTIME_DIR/{membrane,biomeos}/nestgate-*.sock` (direct, glob)
/// 5. `/run/membrane/nestgate*.sock` (system membrane fallback)
///
/// Steps 1–2 route through biomeOS Neural API (capability-based).
/// Steps 3–5 connect directly to nestGate (no routing, no capability discovery).
///
/// Socket names use the NUCLEUS family-ID convention (e.g.
/// `neural-api-westgate-tower-155f.sock`) and are discovered by prefix glob
/// rather than fixed filenames.
#[must_use]
pub fn discover_cas_socket() -> Option<CasSocketInfo> {
    if let Ok(path) = std::env::var(NEURAL_API_SOCKET_ENV) {
        if !path.is_empty() {
            return Some(CasSocketInfo {
                path,
                routing: CasRouting::NeuralApi,
            });
        }
    }

    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        for dir in SOCKET_DIRS {
            if let Some(path) = find_socket_by_prefix(&format!("{xdg}/{dir}"), "neural-api-") {
                return Some(CasSocketInfo {
                    path,
                    routing: CasRouting::NeuralApi,
                });
            }
        }
    }

    if let Ok(path) = std::env::var(NESTGATE_SOCKET_ENV) {
        if !path.is_empty() {
            return Some(CasSocketInfo {
                path,
                routing: CasRouting::Direct,
            });
        }
    }

    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        for dir in SOCKET_DIRS {
            if let Some(path) = find_socket_by_prefix(&format!("{xdg}/{dir}"), "nestgate-") {
                return Some(CasSocketInfo {
                    path,
                    routing: CasRouting::Direct,
                });
            }
        }
    }

    if let Some(path) = find_socket_by_prefix("/run/membrane", "nestgate") {
        return Some(CasSocketInfo {
            path,
            routing: CasRouting::Direct,
        });
    }

    None
}

/// Scans `dir` for the first `.sock` file whose name starts with `prefix`.
#[must_use]
pub fn find_socket_by_prefix(dir: &str, prefix: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(prefix) && name.ends_with(".sock") {
            return Some(entry.path().to_string_lossy().into_owned());
        }
    }
    None
}

/// How CAS requests are routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasRouting {
    /// Via biomeOS Neural API — capability-based routing to nestGate.
    NeuralApi,
    /// Direct UDS connection to nestGate — no capability routing.
    Direct,
}

/// Discovered CAS socket with routing mode.
#[derive(Debug, Clone)]
pub struct CasSocketInfo {
    /// Socket path.
    pub path: String,
    /// Whether this routes via Neural API or connects directly.
    pub routing: CasRouting,
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
    fn discover_cas_socket_does_not_panic() {
        let result = discover_cas_socket();
        if let Some(info) = &result {
            assert!(!info.path.is_empty());
            assert!(
                std::path::Path::new(&info.path)
                    .extension()
                    .is_some_and(|ext| ext == "sock")
            );
        }
    }

    #[test]
    fn find_socket_by_prefix_returns_none_for_missing_dir() {
        assert!(find_socket_by_prefix("/nonexistent/dir", "neural-api-").is_none());
    }

    #[test]
    fn cas_routing_variants() {
        assert_eq!(CasRouting::NeuralApi, CasRouting::NeuralApi);
        assert_ne!(CasRouting::NeuralApi, CasRouting::Direct);
    }
}
