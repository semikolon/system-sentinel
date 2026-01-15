//! Unix Domain Socket server for IPC with the Tauri UI
//! 
//! Broadcasts current system metrics to all connected clients.

use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tracing::{error, info};
use crate::metrics::SystemMetrics;

pub struct IpcServer {
    socket_path: String,
    tx: broadcast::Sender<Arc<SystemMetrics>>,
}

impl IpcServer {
    pub fn new(socket_path: &str) -> (Self, broadcast::Sender<Arc<SystemMetrics>>) {
        let (tx, _rx) = broadcast::channel(16);
        (
            Self {
                socket_path: socket_path.to_string(),
                tx: tx.clone(),
            },
            tx,
        )
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        // Remove existing socket if it exists
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC Server listening on {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let rx = self.tx.subscribe();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, rx).await {
                            error!("IPC client error: {}", e);
                        }
                    });
                }
                Err(e) => error!("IPC accept error: {}", e),
            }
        }
    }
}

async fn handle_client(mut stream: UnixStream, mut rx: broadcast::Receiver<Arc<SystemMetrics>>) -> Result<(), anyhow::Error> {
    while let Ok(metrics) = rx.recv().await {
        let json = serde_json::to_string(&*metrics)?;
        stream.write_all(json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;
    }
    Ok(())
}
