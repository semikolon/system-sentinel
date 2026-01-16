use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tauri::{AppHandle, Emitter, Manager};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, TrayIconId};
use tracing::{error, info};

const TRAY_ID: &str = "sentinel-tray";

// Mirroring the daemon's structs for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub memory_bytes: u64,
    pub memory_mb: f64,
    pub cpu_usage: f32,
    pub exe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_free: u64,
    pub memory_percent: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_percent: f64,
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,
    pub top_processes: Vec<ProcessInfo>,
    pub aggregated_processes: Vec<ProcessInfo>,
    pub memory_growth_rate: Option<f64>,
}

/// System health state for tray icon
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Healthy,
    Warning,
    Critical,
}

impl HealthState {
    /// Determine health state from system metrics
    pub fn from_metrics(metrics: &SystemMetrics) -> Self {
        // Critical: Memory > 90% OR (Swap > 50% AND Memory > 80%)
        if metrics.memory_percent > 90.0
            || (metrics.swap_percent > 50.0 && metrics.memory_percent > 80.0)
        {
            return HealthState::Critical;
        }

        // Warning: Memory > 80% OR high growth rate
        if metrics.memory_percent > 80.0
            || metrics.swap_percent > 30.0
            || metrics.memory_growth_rate.map_or(false, |r| r > 2.0)
        {
            return HealthState::Warning;
        }

        HealthState::Healthy
    }
}

/// Load tray icon for a given health state (decodes PNG to RGBA)
fn load_tray_icon(state: HealthState) -> Image<'static> {
    let icon_bytes: &[u8] = match state {
        HealthState::Healthy => include_bytes!("../icons/tray/healthy@2x.png"),
        HealthState::Warning => include_bytes!("../icons/tray/warning@2x.png"),
        HealthState::Critical => include_bytes!("../icons/tray/critical@2x.png"),
    };

    // Decode PNG to RGBA using image crate
    let img = image::load_from_memory(icon_bytes)
        .expect("Failed to decode tray icon PNG")
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    Image::new_owned(rgba, width, height)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            // Setup Tray Menu
            let tray_menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit Sentinel", true, None::<&str>)?,
                ],
            )?;

            // Build tray with initial healthy icon
            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(load_tray_icon(HealthState::Healthy))
                .tooltip("System Sentinel - Healthy")
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show().and_then(|_| window.set_focus());
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = if window.is_visible().unwrap_or(false) {
                                window.hide()
                            } else {
                                window.show().and_then(|_| window.set_focus())
                            };
                        }
                    }
                })
                .build(app)?;

            // Start IPC Client using Tauri's async runtime
            tauri::async_runtime::spawn(async move {
                if let Err(e) = run_ipc_client(handle).await {
                    error!("IPC client fatal error: {}", e);
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Update tray icon and tooltip based on health state
fn update_tray_state(app: &AppHandle, state: HealthState) {
    let tray_id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = app.tray_by_id(&tray_id) {
        // Update icon
        let _ = tray.set_icon(Some(load_tray_icon(state)));

        // Update tooltip
        let tooltip = match state {
            HealthState::Healthy => "System Sentinel - Healthy",
            HealthState::Warning => "System Sentinel - Warning",
            HealthState::Critical => "System Sentinel - Critical!",
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

async fn run_ipc_client(app: AppHandle) -> Result<(), anyhow::Error> {
    let socket_path = "/tmp/system-sentinel.soc";
    let mut current_state = HealthState::Healthy;

    loop {
        info!("Connecting to IPC socket at {}...", socket_path);
        match UnixStream::connect(socket_path).await {
            Ok(stream) => {
                info!("Connected to Sentinel daemon.");
                let mut reader = BufReader::new(stream).lines();

                while let Ok(Some(line)) = reader.next_line().await {
                    if let Ok(metrics) = serde_json::from_str::<SystemMetrics>(&line) {
                        // Emit to frontend
                        let _ = app.emit("metrics-update", metrics.clone());

                        // Update tray icon if state changed
                        let new_state = HealthState::from_metrics(&metrics);
                        if new_state != current_state {
                            info!("Health state changed: {:?} -> {:?}", current_state, new_state);
                            update_tray_state(&app, new_state);
                            current_state = new_state;
                        }
                    }
                }
                error!("IPC connection lost. Retrying in 5 seconds...");
            }
            Err(e) => {
                error!("Failed to connect to IPC socket: {}. Retrying in 5 seconds...", e);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
