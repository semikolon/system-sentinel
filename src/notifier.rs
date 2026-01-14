//! Notification sending via Hammerspoon or fallback

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::detector::{AlertLevel, Anomaly};

pub struct Notifier {
    use_hammerspoon: bool,
    fallback_to_terminal_notifier: bool,
    warning_color: String,
    critical_color: String,
    narrator: crate::narration::Narrator,
}

impl Notifier {
    pub fn new(config: &Config) -> Self {
        Self {
            use_hammerspoon: config.notification.use_hammerspoon,
            fallback_to_terminal_notifier: config.notification.fallback_to_terminal_notifier,
            warning_color: config.notification.warning_color.clone(),
            critical_color: config.notification.critical_color.clone(),
            narrator: crate::narration::Narrator::new(),
        }
    }

    /// Send notification for detected anomaly
    pub fn send(&self, anomaly: &Anomaly) -> Result<()> {
        info!("Sending {} notification: {}", anomaly.level, anomaly.message);

        // Trigger narration (non-blocking on audio, but blocks on socket IPC)
        if let Err(e) = self.narrator.narrate(&anomaly.narration_message) {
            warn!("Narration failed: {}", e);
        }

        if self.use_hammerspoon {
            match self.send_hammerspoon(anomaly) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    warn!("Hammerspoon notification failed: {}", e);
                    if !self.fallback_to_terminal_notifier {
                        return Err(e);
                    }
                }
            }
        }

        if self.fallback_to_terminal_notifier {
            self.send_terminal_notifier(anomaly)?;
        }

        Ok(())
    }

    /// Send notification via Hammerspoon's `hs` CLI
    fn send_hammerspoon(&self, anomaly: &Anomaly) -> Result<()> {
        let (icon, color, duration) = match anomaly.level {
            AlertLevel::Warning => ("⚠️", &self.warning_color, 10),
            AlertLevel::Critical => ("🚨", &self.critical_color, 15),
        };

        // Format message with details
        let details_str = if anomaly.details.is_empty() {
            String::new()
        } else {
            format!("\\n{}", anomaly.details.join("\\n"))
        };

        let message = format!("{} {}{}", icon, anomaly.message, details_str);

        // Build Hammerspoon Lua command
        // Style matches existing TTS hotkeys alerts
        let lua_cmd = format!(
            r#"hs.alert.show("{}", {{
                strokeColor = {{ white = 0, alpha = 0.75 }},
                fillColor = {{ hex = "{}", alpha = 0.95 }},
                textColor = {{ white = 1, alpha = 1 }},
                strokeWidth = 2,
                radius = 10,
                textSize = 18,
                fadeInDuration = 0.15,
                fadeOutDuration = 0.15,
                atScreenEdge = 0
            }}, {})"#,
            message.replace('"', r#"\""#),
            color,
            duration
        );

        debug!("Hammerspoon command: {}", lua_cmd);

        let output = Command::new("hs")
            .arg("-c")
            .arg(&lua_cmd)
            .output()
            .context("Failed to execute hs command")?;

        if output.status.success() {
            debug!("Hammerspoon notification sent successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hs command failed: {}", stderr)
        }
    }

    /// Fallback to terminal-notifier
    fn send_terminal_notifier(&self, anomaly: &Anomaly) -> Result<()> {
        let title = match anomaly.level {
            AlertLevel::Warning => "System Sentinel Warning",
            AlertLevel::Critical => "System Sentinel CRITICAL",
        };

        let message = format!("{}\n{}", anomaly.message, anomaly.details.join("\n"));

        let output = Command::new("terminal-notifier")
            .arg("-title")
            .arg(title)
            .arg("-message")
            .arg(&message)
            .arg("-sound")
            .arg("default")
            .output()
            .context("Failed to execute terminal-notifier")?;

        if output.status.success() {
            debug!("terminal-notifier notification sent successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("terminal-notifier failed: {}", stderr)
        }
    }
}
