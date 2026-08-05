// SPDX-License-Identifier: AGPL-3.0-or-later

//! Async petalTongue IPC client for visualization scene rendering over UDS.
//!
//! Mirrors the `CasClient` pattern: discovers petalTongue socket via Neural API
//! `membrane/` scan, sends declarative scene JSON via `visualization.render.scene`,
//! and returns scene handles for interactive updates.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use tideglass_core::error::TideGlassError;

/// petalTongue visualization method constants.
#[allow(dead_code)]
pub mod methods {
    pub const RENDER_SCENE: &str = "visualization.render.scene";
    pub const RENDER_STREAM: &str = "visualization.render.stream";
    pub const SCENE_UPDATE: &str = "visualization.scene.update";
    pub const SCENE_CLOSE: &str = "visualization.scene.close";
}

/// Environment variable for petalTongue socket override.
pub const PETALTONGUE_SOCKET_ENV: &str = "PETALTONGUE_SOCKET";

/// Directories to scan for NUCLEUS sockets, in priority order.
const SOCKET_DIRS: &[&str] = &["membrane", "biomeos"];

/// How petalTongue was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetalTongueRouting {
    /// Via biomeOS Neural API — capability-based routing.
    NeuralApi,
    /// Direct UDS connection to petalTongue.
    Direct,
}

/// Discovered petalTongue socket with routing mode.
#[derive(Debug, Clone)]
pub struct PetalTongueSocketInfo {
    pub path: String,
    pub routing: PetalTongueRouting,
}

/// Scene handle returned after a successful `visualization.render.scene` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneHandle {
    pub scene_id: String,
    #[serde(default)]
    pub status: String,
}

/// Async client for petalTongue scene rendering over UDS.
///
/// Not yet instantiated in the dispatch loop — visualization methods currently
/// return scene JSON directly. When petalTongue is co-deployed on westGate,
/// the server startup will create a client and forward scenes.
#[allow(dead_code)]
pub struct PetalTongueClient {
    socket_path: Arc<str>,
    routing: PetalTongueRouting,
}

#[allow(dead_code)]
impl PetalTongueClient {
    #[must_use]
    pub fn new(socket_path: &str, routing: PetalTongueRouting) -> Self {
        Self {
            socket_path: Arc::from(socket_path),
            routing,
        }
    }

    #[must_use]
    pub const fn routing(&self) -> PetalTongueRouting {
        self.routing
    }

    #[must_use]
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// Sends a scene to petalTongue for WebGL rendering.
    ///
    /// The scene JSON follows the petalTongue declarative format:
    /// `{ "scene": "<name>", "data": {...}, "format": "webgl", "interactive": true }`
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::Transport`] on socket failure.
    pub async fn render_scene(&self, scene: Value) -> Result<SceneHandle, TideGlassError> {
        self.call(methods::RENDER_SCENE, scene).await
    }

    /// Streams a scene to petalTongue for live-updating visualization.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::Transport`] on socket failure.
    #[allow(dead_code)]
    pub async fn render_stream(&self, scene: Value) -> Result<SceneHandle, TideGlassError> {
        self.call(methods::RENDER_STREAM, scene).await
    }

    /// Lightweight connectivity check via `health.check`.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::Transport`] if the socket is unreachable.
    pub async fn check_health(&self) -> Result<String, TideGlassError> {
        let response: Value = self.call("health.check", json!({})).await?;
        let version = response
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        Ok(version)
    }

    /// Sends a JSON-RPC 2.0 request over UDS and deserializes the response.
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, TideGlassError> {
        let mut stream = UnixStream::connect(self.socket_path.as_ref())
            .await
            .map_err(|err| {
                TideGlassError::Transport(format!(
                    "petalTongue connect to {}: {err}",
                    self.socket_path
                ))
            })?;

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let mut request_bytes = serde_json::to_vec(&request).map_err(|err| {
            TideGlassError::Transport(format!("serialize petalTongue request: {err}"))
        })?;
        request_bytes.push(b'\n');

        stream.write_all(&request_bytes).await.map_err(|err| {
            TideGlassError::Transport(format!("petalTongue write to {}: {err}", self.socket_path))
        })?;
        stream
            .shutdown()
            .await
            .map_err(|err| TideGlassError::Transport(format!("petalTongue shutdown: {err}")))?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.map_err(|err| {
            TideGlassError::Transport(format!("petalTongue read from {}: {err}", self.socket_path))
        })?;

        if buf.is_empty() {
            return Err(TideGlassError::Transport(format!(
                "petalTongue {method}: empty response from {}",
                self.socket_path
            )));
        }

        let rpc_response: Value = serde_json::from_slice(&buf).map_err(|err| {
            TideGlassError::Transport(format!("petalTongue response parse: {err}"))
        })?;

        if let Some(error) = rpc_response.get("error") {
            return Err(TideGlassError::Transport(format!(
                "petalTongue {method} error: {}",
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            )));
        }

        let result = rpc_response.get("result").cloned().unwrap_or(Value::Null);
        serde_json::from_value(result).map_err(|err| {
            TideGlassError::Transport(format!(
                "petalTongue response deserialize for {method}: {err}"
            ))
        })
    }
}

/// Discovers the petalTongue socket using Neural API or direct scan.
///
/// Priority:
/// 1. `PETALTONGUE_SOCKET` env var (explicit override)
/// 2. `$XDG_RUNTIME_DIR/{membrane,biomeos}/petaltongue-*.sock` (direct)
/// 3. Neural API routing (petalTongue is a capability, not a separate socket
///    in some deployments — the Neural API routes `visualization.*` calls)
#[must_use]
pub fn discover_petaltongue_socket() -> Option<PetalTongueSocketInfo> {
    if let Ok(path) = std::env::var(PETALTONGUE_SOCKET_ENV) {
        if !path.is_empty() {
            return Some(PetalTongueSocketInfo {
                path,
                routing: PetalTongueRouting::Direct,
            });
        }
    }

    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        for dir in SOCKET_DIRS {
            if let Some(path) =
                tideglass_core::cas::find_socket_by_prefix(&format!("{xdg}/{dir}"), "petaltongue-")
            {
                return Some(PetalTongueSocketInfo {
                    path,
                    routing: PetalTongueRouting::Direct,
                });
            }
        }

        // Neural API can route visualization.* calls to petalTongue
        for dir in SOCKET_DIRS {
            if let Some(path) =
                tideglass_core::cas::find_socket_by_prefix(&format!("{xdg}/{dir}"), "neural-api-")
            {
                return Some(PetalTongueSocketInfo {
                    path,
                    routing: PetalTongueRouting::NeuralApi,
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_stores_path_and_routing() {
        let client =
            PetalTongueClient::new("/run/membrane/petaltongue.sock", PetalTongueRouting::Direct);
        assert_eq!(client.socket_path(), "/run/membrane/petaltongue.sock");
        assert_eq!(client.routing(), PetalTongueRouting::Direct);
    }

    #[test]
    fn client_neural_api_routing() {
        let client = PetalTongueClient::new(
            "/run/user/1000/membrane/neural-api-westgate-tower-155f.sock",
            PetalTongueRouting::NeuralApi,
        );
        assert_eq!(client.routing(), PetalTongueRouting::NeuralApi);
    }

    #[test]
    fn discover_does_not_panic() {
        let _ = discover_petaltongue_socket();
    }

    #[test]
    fn routing_variants_eq() {
        assert_eq!(PetalTongueRouting::Direct, PetalTongueRouting::Direct);
        assert_ne!(PetalTongueRouting::Direct, PetalTongueRouting::NeuralApi);
    }

    #[test]
    fn method_constants_are_visualization_domain() {
        assert!(methods::RENDER_SCENE.starts_with("visualization."));
        assert!(methods::RENDER_STREAM.starts_with("visualization."));
        assert!(methods::SCENE_UPDATE.starts_with("visualization."));
        assert!(methods::SCENE_CLOSE.starts_with("visualization."));
    }

    #[test]
    fn scene_handle_deserializes() {
        let json = serde_json::json!({
            "scene_id": "rges-volcano-001",
            "status": "rendered"
        });
        let handle: SceneHandle = serde_json::from_value(json).expect("deserialize");
        assert_eq!(handle.scene_id, "rges-volcano-001");
        assert_eq!(handle.status, "rendered");
    }

    #[test]
    fn scene_handle_deserializes_minimal() {
        let json = serde_json::json!({ "scene_id": "test-001" });
        let handle: SceneHandle = serde_json::from_value(json).expect("deserialize");
        assert_eq!(handle.scene_id, "test-001");
        assert!(handle.status.is_empty());
    }
}
