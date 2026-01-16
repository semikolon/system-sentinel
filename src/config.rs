//! Configuration loading and defaults

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub thresholds: ThresholdConfig,
    #[serde(default)]
    pub detection: DetectionConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    #[allow(dead_code)] // Reserved for future use
    #[serde(default = "default_log_file")]
    pub log_file: String,
    #[allow(dead_code)] // Reserved for future use
    #[serde(default = "default_ipc_socket")]
    pub ipc_socket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThresholdConfig {
    #[serde(default = "default_memory_warning")]
    pub memory_warning: f64,
    #[serde(default = "default_memory_critical")]
    pub memory_critical: f64,
    #[serde(default = "default_swap_warning")]
    pub swap_warning: f64,
    #[serde(default = "default_swap_critical")]
    pub swap_critical: f64,
    #[serde(default = "default_load_warning")]
    pub load_warning: f64,
    #[serde(default = "default_load_critical")]
    pub load_critical: f64,
    #[serde(default = "default_memory_growth_rate_warning")]
    pub memory_growth_rate_warning: f64,
    #[serde(default = "default_memory_growth_rate_critical")]
    pub memory_growth_rate_critical: f64,
    #[serde(default = "default_recovery_margin")]
    pub recovery_margin: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DetectionConfig {
    #[serde(default = "default_process_watchlist")]
    pub process_watchlist: Vec<String>,
    #[serde(default = "default_process_memory_threshold_mb")]
    pub process_memory_threshold_mb: u64,
    #[serde(default = "default_notification_cooldown_minutes")]
    pub notification_cooldown_minutes: u64,
    #[serde(default = "default_persistent_breach_threshold")]
    pub persistent_breach_threshold: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationConfig {
    #[serde(default = "default_use_hammerspoon")]
    pub use_hammerspoon: bool,
    #[serde(default = "default_fallback_to_terminal_notifier")]
    pub fallback_to_terminal_notifier: bool,
    #[serde(default = "default_warning_color")]
    pub warning_color: String,
    #[serde(default = "default_critical_color")]
    pub critical_color: String,
}

// Default value functions
fn default_check_interval() -> u64 { 30 }
fn default_log_file() -> String { "~/.local/share/system-sentinel/sentinel.log".to_string() }
fn default_memory_warning() -> f64 { 80.0 }
fn default_memory_critical() -> f64 { 90.0 }
fn default_swap_warning() -> f64 { 80.0 }
fn default_swap_critical() -> f64 { 95.0 }
fn default_load_warning() -> f64 { 10.0 }
fn default_load_critical() -> f64 { 50.0 }
fn default_memory_growth_rate_warning() -> f64 { 3.0 }
fn default_memory_growth_rate_critical() -> f64 { 8.0 }
fn default_recovery_margin() -> f64 { 5.0 }
fn default_process_watchlist() -> Vec<String> {
    vec!["ghostty".to_string(), "Arc".to_string(), "node".to_string(), "Electron".to_string()]
}
fn default_process_memory_threshold_mb() -> u64 { 2000 }
fn default_notification_cooldown_minutes() -> u64 { 20 }
fn default_persistent_breach_threshold() -> u32 { 3 }
fn default_use_hammerspoon() -> bool { true }
fn default_fallback_to_terminal_notifier() -> bool { true }
fn default_warning_color() -> String { "#FFA500".to_string() }
fn default_critical_color() -> String { "#FF4444".to_string() }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval(),
            log_file: default_log_file(),
            ipc_socket: default_ipc_socket(),
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            memory_warning: default_memory_warning(),
            memory_critical: default_memory_critical(),
            swap_warning: default_swap_warning(),
            swap_critical: default_swap_critical(),
            load_warning: default_load_warning(),
            load_critical: default_load_critical(),
            memory_growth_rate_warning: default_memory_growth_rate_warning(),
            memory_growth_rate_critical: default_memory_growth_rate_critical(),
            recovery_margin: default_recovery_margin(),
        }
    }
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            process_watchlist: default_process_watchlist(),
            process_memory_threshold_mb: default_process_memory_threshold_mb(),
            notification_cooldown_minutes: default_notification_cooldown_minutes(),
            persistent_breach_threshold: default_persistent_breach_threshold(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            use_hammerspoon: default_use_hammerspoon(),
            fallback_to_terminal_notifier: default_fallback_to_terminal_notifier(),
            warning_color: default_warning_color(),
            critical_color: default_critical_color(),
        }
    }
}

impl Config {
    /// Load configuration from default location or create defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
            toml::from_str(&content)
                .with_context(|| "Failed to parse config file")
        } else {
            // Use defaults if no config file exists
            Ok(Self::default())
        }
    }

    /// Get the default config file path
    /// Prioritizes ~/.config/system-sentinel/config.toml even on macOS
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/Users/fredrikbranstrom"))
            .join(".config")
            .join("system-sentinel")
            .join("config.toml")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            thresholds: ThresholdConfig::default(),
            detection: DetectionConfig::default(),
            notification: NotificationConfig::default(),
        }
    }
}
fn default_ipc_socket() -> String { "/tmp/system-sentinel.soc".to_string() }
