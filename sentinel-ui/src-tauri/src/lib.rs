use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tracing::{error, info};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Setup Tray
            let tray_menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "quit", "Quit Sentinel", true, None::<&str>)?,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
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

            // Start IPC Client
            tokio::spawn(async move {
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

async fn run_ipc_client(app: AppHandle) -> Result<(), anyhow::Error> {
    let socket_path = "/tmp/system-sentinel.soc";
    
    loop {
        info!("Connecting to IPC socket at {}...", socket_path);
        match UnixStream::connect(socket_path).await {
            Ok(stream) => {
                info!("Connected to Sentinel daemon.");
                let mut reader = BufReader::new(stream).lines();
                
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Ok(metrics) = serde_json::from_str::<SystemMetrics>(&line) {
                        // Emit to frontend
                        let _ = app.emit("metrics-update", metrics);
                        
                        // TODO: Update tray icon based on severity
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
