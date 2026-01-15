//! System Sentinel - Low-overhead system health monitor
//!
//! Monitors memory, swap, load average, and per-process memory usage.
//! Sends notifications via Hammerspoon when anomalies are detected.

mod config;
mod detector;
mod metrics;
mod narration;
mod notifier;
mod server;

use anyhow::Result;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::detector::AnomalyDetector;
use crate::metrics::MetricsCollector;
use crate::notifier::Notifier;
use crate::server::IpcServer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("system_sentinel=info".parse()?),
        )
        .with_target(false)
        .init();

    info!("System Sentinel starting...");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: check interval = {}s", config.general.check_interval_seconds);

    // Initialize components
    let mut metrics_collector = MetricsCollector::new();
    let mut detector = AnomalyDetector::new(&config);
    let notifier = Notifier::new(&config);

    // Initialise IPC Server
    let socket_path = "/tmp/system-sentinel.soc";
    let (server, tx) = IpcServer::new(socket_path);
    
    // Start IPC server in background
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("IPC Server failed: {}", e);
        }
    });

    // Main monitoring loop
    let mut check_interval = interval(Duration::from_secs(config.general.check_interval_seconds));

    info!("Entering main monitoring loop");

    loop {
        tokio::select! {
            _ = check_interval.tick() => {
                // Collect metrics (with aggregation for watchlist)
                // Collect metrics (with auto-aggregation)
                let metrics = metrics_collector.collect_aggregated();

                // Detect anomalies
                if let Some(anomaly) = detector.check(&metrics) {
                    warn!("Anomaly detected: {} - {}", anomaly.level, anomaly.message);

                    // Send notification
                    if let Err(e) = notifier.send(&anomaly) {
                        error!("Failed to send notification: {}", e);
                    }
                }

                // Broadcast metrics to UI
                let _ = tx.send(Arc::new(metrics));
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received, exiting...");
                break;
            }
        }
    }

    info!("System Sentinel stopped");
    Ok(())
}
