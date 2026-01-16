use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command as TokioCommand;
use tauri::{AppHandle, Emitter, Manager};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, TrayIconId};
use tracing::{error, info, warn};

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

            // Build tray with initial healthy icon (disable template mode for colored icons)
            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(load_tray_icon(HealthState::Healthy))
                .icon_as_template(false)
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
        .invoke_handler(tauri::generate_handler![submit_query, execute_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Update tray icon and tooltip based on health state
fn update_tray_state(app: &AppHandle, state: HealthState) {
    let tray_id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = app.tray_by_id(&tray_id) {
        // Update icon and ensure template mode is disabled for colored icons
        let _ = tray.set_icon(Some(load_tray_icon(state)));
        let _ = tray.set_icon_as_template(false);

        // Update tooltip
        let tooltip = match state {
            HealthState::Healthy => "System Sentinel - Healthy",
            HealthState::Warning => "System Sentinel - Warning",
            HealthState::Critical => "System Sentinel - Critical!",
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

// ========== CLAUDE INTEGRATION ==========

/// Suggested action from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub description: String,
    pub risk: String, // "low", "moderate", "high"
    pub command: Option<String>,
    pub pid: Option<u32>,
}

/// Submit a query to Claude CLI and stream response
#[tauri::command]
async fn submit_query(app: AppHandle, prompt: String, metrics_json: Option<String>) -> Result<(), String> {
    // Check if claude CLI exists
    if which::which("claude").is_err() {
        let _ = app.emit("claude-error", "Claude CLI not found. Install with: npm install -g @anthropic-ai/claude-code");
        return Err("Claude CLI not found".into());
    }

    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        // Build system prompt with context
        let system_context = if let Some(metrics) = &metrics_json {
            format!(
                r#"You are System Sentinel, an AI advisor for macOS system health.

CURRENT SYSTEM STATE:
{}

USER QUESTION: {}

Provide concise, actionable advice. If you recommend killing a process,
mention the risk level (low/moderate/high) and what might be lost."#,
                metrics, prompt
            )
        } else {
            format!(
                r#"You are System Sentinel, an AI advisor for macOS system health.

USER QUESTION: {}

Provide concise, actionable advice."#,
                prompt
            )
        };

        // Spawn claude CLI
        let result = TokioCommand::new("claude")
            .args(["--print", &system_context])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match result {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to spawn claude: {}", e);
                let _ = app_clone.emit("claude-error", format!("Failed to start Claude: {}", e));
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                let _ = app_clone.emit("claude-error", "Failed to capture stdout");
                return;
            }
        };

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Stream lines to frontend
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_clone.emit("claude-stream", &line);
            let _ = app_clone.emit("claude-stream", "\n");
        }

        // Wait for process to complete
        match child.wait().await {
            Ok(status) => {
                if status.success() {
                    let _ = app_clone.emit("claude-done", serde_json::json!({}));
                } else {
                    warn!("Claude exited with status: {}", status);
                    let _ = app_clone.emit("claude-done", serde_json::json!({}));
                }
            }
            Err(e) => {
                let _ = app_clone.emit("claude-error", format!("Claude process error: {}", e));
            }
        }
    });

    Ok(())
}

/// Execute a confirmed action
#[tauri::command]
async fn execute_action(app: AppHandle, action: SuggestedAction) -> Result<String, String> {
    info!("Executing action: {:?}", action);

    match action.action_type.as_str() {
        "kill_process" => {
            if let Some(pid) = action.pid {
                // Safety check - don't kill protected processes
                let _protected = ["Terminal", "Ghostty", "Code", "Zed", "Safari", "Arc", "Claude", "Finder"];
                // TODO: Check process name against protected list before killing

                let output = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .output()
                    .map_err(|e| format!("Kill failed: {}", e))?;

                if output.status.success() {
                    let _ = app.emit("action-result", serde_json::json!({"success": true, "message": "Process terminated"}));
                    Ok("Process terminated".into())
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    Err(format!("Kill failed: {}", err))
                }
            } else {
                Err("No PID specified".into())
            }
        }
        "clear_cache" => {
            // Safe operation: clear system caches
            let _ = std::process::Command::new("sudo")
                .args(["purge"])
                .output();
            Ok("Cache cleared".into())
        }
        _ => Err(format!("Unknown action type: {}", action.action_type)),
    }
}

// ========== IPC CLIENT ==========

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
