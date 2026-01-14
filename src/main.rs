//! System Sentinel - Low-overhead system health monitor
//!
//! Monitors memory, swap, load average, and per-process memory usage.
//! Sends notifications via Hammerspoon when anomalies are detected.

mod config;
mod detector;
mod metrics;
mod notifier;

use anyhow::Result;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::detector::AnomalyDetector;
use crate::metrics::MetricsCollector;
use crate::notifier::Notifier;

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
