// SPDX-License-Identifier: AGPL-3.0-or-later

//! Async Unix Domain Socket JSON-RPC server with NDJSON framing.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::data::ModuleData;
use crate::petaltongue::PetalTongueClient;

/// Shared server context passed to each connection handler.
pub struct ServerContext {
    pub module_data: Arc<ModuleData>,
    pub petal_client: Option<Arc<PetalTongueClient>>,
}

/// Binds a UDS listener and serves JSON-RPC requests until a shutdown signal arrives.
///
/// CAS-loaded `ModuleData` and optional `PetalTongueClient` are shared across all
/// connections via `Arc`.
///
/// # Errors
///
/// Returns an error when socket setup, acceptance, or cleanup fails.
pub async fn run_server(
    socket_path: &str,
    ctx: Arc<ServerContext>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file(socket_path);

    if let Some(parent) = std::path::Path::new(socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    eprintln!("tideglass: listening on {socket_path}");

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _addr) = accept_result?;
                let ctx = Arc::clone(&ctx);
                tokio::spawn(handle_connection(stream, ctx));
            }
            () = &mut shutdown => {
                eprintln!("tideglass: shutting down");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(socket_path);
    Ok(())
}

async fn handle_connection(stream: tokio::net::UnixStream, ctx: Arc<ServerContext>) {
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let method = extract_method(&line);
        let response = crate::dispatch::dispatch_request(&line, &ctx.module_data);

        // Forward viz scenes to petalTongue when available (fire-and-forget).
        if let (Some(petal), Some(m)) = (&ctx.petal_client, &method) {
            if crate::petaltongue::is_viz_method(m) {
                if let Some(scene) = response.result.as_ref() {
                    let petal = Arc::clone(petal);
                    let scene = scene.clone();
                    tokio::spawn(async move {
                        if let Err(err) = petal.render_scene(scene).await {
                            eprintln!("tideglass: petalTongue forward failed: {err}");
                        }
                    });
                }
            }
        }

        let response_bytes = match serde_json::to_vec(&response) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                bytes
            }
            Err(_) => continue,
        };
        if writer.write_all(&response_bytes).await.is_err() {
            break;
        }
    }
}

/// Extracts the `method` field from a raw JSON-RPC request without full deserialization.
fn extract_method(raw: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| v.get("method")?.as_str().map(ToOwned::to_owned))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate()).expect("register SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("register SIGINT handler");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_method_from_valid_request() {
        let raw = r#"{"jsonrpc":"2.0","method":"science.rges_screen","params":{},"id":1}"#;
        assert_eq!(extract_method(raw).as_deref(), Some("science.rges_screen"));
    }

    #[test]
    fn extract_method_from_viz_request() {
        let raw = r#"{"jsonrpc":"2.0","method":"viz.rges_volcano","params":{},"id":1}"#;
        assert_eq!(extract_method(raw).as_deref(), Some("viz.rges_volcano"));
    }

    #[test]
    fn extract_method_from_invalid_json() {
        assert!(extract_method("{not valid json").is_none());
    }

    #[test]
    fn extract_method_missing_method_field() {
        let raw = r#"{"jsonrpc":"2.0","params":{},"id":1}"#;
        assert!(extract_method(raw).is_none());
    }

    #[test]
    fn server_context_default_no_petal() {
        let ctx = ServerContext {
            module_data: Arc::new(ModuleData::default()),
            petal_client: None,
        };
        assert!(ctx.petal_client.is_none());
    }
}
