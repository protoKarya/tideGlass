// SPDX-License-Identifier: AGPL-3.0-or-later

//! Async CAS client for nestGate JSON-RPC over UDS.
//!
//! First primal to wire live CAS data. Divergences from spec are documented
//! in the AAR handoff.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use tideglass_core::cas::{
    CasExistsResponse, CasGetResponse, CasListResponse, CasPutResponse, CasRouting, methods,
};
use tideglass_core::error::TideGlassError;

/// Async CAS client routed via biomeOS Neural API or direct to nestGate.
pub struct CasClient {
    socket_path: Arc<str>,
    routing: CasRouting,
}

impl CasClient {
    /// Creates a new CAS client targeting the given socket with routing mode.
    #[must_use]
    pub fn new(socket_path: &str, routing: CasRouting) -> Self {
        Self {
            socket_path: Arc::from(socket_path),
            routing,
        }
    }

    /// Returns the routing mode (Neural API vs direct).
    #[must_use]
    pub const fn routing(&self) -> CasRouting {
        self.routing
    }

    /// Retrieves a CAS object by BLAKE3 hash. Returns decoded bytes on success.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::DataAccess`] on transport or decode failure.
    /// Returns [`TideGlassError::DataAccess`] if the object requires streaming (> 64 MiB).
    pub async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, TideGlassError> {
        let params = json!({"hash": hash});
        let response: CasGetResponse = self.call(methods::CONTENT_GET, params).await?;

        if response.use_streaming == Some(true) {
            return Err(TideGlassError::DataAccess(format!(
                "CAS object {hash} exceeds inline limit ({} bytes); streaming not yet implemented",
                response.size
            )));
        }

        match response.data {
            Some(b64) => {
                let bytes = BASE64.decode(&b64).map_err(|err| {
                    TideGlassError::DataAccess(format!("base64 decode failed for {hash}: {err}"))
                })?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }

    /// Checks whether a CAS object exists by hash.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::DataAccess`] on transport failure.
    pub async fn exists(&self, hash: &str) -> Result<bool, TideGlassError> {
        let params = json!({"hash": hash});
        let response: CasExistsResponse = self.call(methods::CONTENT_EXISTS, params).await?;
        Ok(response.exists)
    }

    /// Lists all CAS objects, optionally filtered by family.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::DataAccess`] on transport failure.
    pub async fn list(&self, family_id: Option<&str>) -> Result<CasListResponse, TideGlassError> {
        let params = family_id.map_or_else(|| json!({}), |fid| json!({"family_id": fid}));
        self.call(methods::CONTENT_LIST, params).await
    }

    /// Stores bytes in CAS, returning the BLAKE3 hash and metadata.
    ///
    /// # Errors
    ///
    /// Returns [`TideGlassError::DataAccess`] on transport failure.
    #[allow(dead_code)]
    pub async fn put(
        &self,
        data: &[u8],
        content_type: Option<&str>,
        source: Option<&str>,
        pipeline: Option<&str>,
    ) -> Result<CasPutResponse, TideGlassError> {
        let b64 = BASE64.encode(data);
        let mut params = json!({"data": b64});
        if let Some(ct) = content_type {
            params["content_type"] = Value::String(ct.to_owned());
        }
        if let Some(src) = source {
            params["source"] = Value::String(src.to_owned());
        }
        if let Some(pipe) = pipeline {
            params["pipeline"] = Value::String(pipe.to_owned());
        }
        params["stored_by"] = Value::String("tideglass".to_owned());
        self.call(methods::CONTENT_PUT, params).await
    }

    /// Sends a JSON-RPC 2.0 request over UDS and deserializes the response.
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, TideGlassError> {
        let stream = UnixStream::connect(self.socket_path.as_ref())
            .await
            .map_err(|err| {
                TideGlassError::Transport(format!("CAS connect to {}: {err}", self.socket_path))
            })?;

        let (reader, mut writer) = stream.into_split();

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let mut request_bytes = serde_json::to_vec(&request)
            .map_err(|err| TideGlassError::DataAccess(format!("serialize CAS request: {err}")))?;
        request_bytes.push(b'\n');

        writer.write_all(&request_bytes).await.map_err(|err| {
            TideGlassError::Transport(format!("CAS write to {}: {err}", self.socket_path))
        })?;
        writer
            .shutdown()
            .await
            .map_err(|err| TideGlassError::Transport(format!("CAS shutdown write: {err}")))?;

        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await.map_err(|err| {
            TideGlassError::Transport(format!("CAS read from {}: {err}", self.socket_path))
        })?;

        let rpc_response: Value = serde_json::from_str(&line)
            .map_err(|err| TideGlassError::DataAccess(format!("CAS response parse: {err}")))?;

        if let Some(error) = rpc_response.get("error") {
            return Err(TideGlassError::DataAccess(format!(
                "nestGate {method} error: {}",
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            )));
        }

        let result = rpc_response.get("result").cloned().unwrap_or(Value::Null);
        serde_json::from_value(result).map_err(|err| {
            TideGlassError::DataAccess(format!("CAS response deserialize for {method}: {err}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_new_stores_path_and_routing() {
        let client = CasClient::new("/run/membrane/nestgate.sock", CasRouting::Direct);
        assert_eq!(client.socket_path.as_ref(), "/run/membrane/nestgate.sock");
        assert_eq!(client.routing(), CasRouting::Direct);
    }

    #[test]
    fn client_neural_api_routing() {
        let client = CasClient::new(
            "/run/user/1000/biomeos/neural-api-default.sock",
            CasRouting::NeuralApi,
        );
        assert_eq!(client.routing(), CasRouting::NeuralApi);
    }

    #[test]
    fn base64_roundtrip() {
        let original = b"tideGlass CAS test";
        let encoded = BASE64.encode(original);
        let decoded = BASE64.decode(&encoded).expect("decode");
        assert_eq!(&decoded, original);
    }
}
