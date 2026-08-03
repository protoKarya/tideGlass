// SPDX-License-Identifier: AGPL-3.0-or-later

//! Async Unix Domain Socket JSON-RPC server with NDJSON framing.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::data::ModuleData;

/// Binds a UDS listener and serves JSON-RPC requests until a shutdown signal arrives.
///
/// CAS-loaded `ModuleData` is shared across all connections via `Arc`.
///
/// # Errors
///
/// Returns an error when socket setup, acceptance, or cleanup fails.
pub async fn run_server(
    socket_path: &str,
    module_data: Arc<ModuleData>,
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
                let data = Arc::clone(&module_data);
                tokio::spawn(handle_connection(stream, data));
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

async fn handle_connection(stream: tokio::net::UnixStream, module_data: Arc<ModuleData>) {
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let response = crate::dispatch::dispatch_request(&line, &module_data);
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
