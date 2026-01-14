//! Anomaly detection logic

use std::collections::HashMap;
use std::time::Instant;
use tracing::debug;

use crate::config::Config;
use crate::metrics::SystemMetrics;

/// Severity level of detected anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Warning,
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertLevel::Warning => write!(f, "WARNING"),
            AlertLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Type of anomaly (used for stable cooldown keys)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyType {
    Memory,
    Swap,
    Load,
    MemoryGrowthRate,
    ProcessWatchlist,
}

/// Result of anomaly detection
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub anomaly_type: AnomalyType,
    pub level: AlertLevel,
    pub message: String,
    pub details: Vec<String>,
}

/// Anomaly detection with cooldown tracking
pub struct AnomalyDetector {
    config: Config,
    /// Last notification time and level for each anomaly type (for cooldown)
    last_notification: HashMap<String, (Instant, AlertLevel)>,
    /// How many consecutive times an anomaly has been detected
    breach_counters: HashMap<AnomalyType, u32>,
    /// Currently active alerts (for hysteresis)
    active_alerts: HashMap<AnomalyType, AlertLevel>,
    /// When the current high load started (None if load is normal)
    load_start_time: Option<Instant>,
}

impl AnomalyDetector {
    pub fn new(config: &Config) -> Self {
        Self {
            config: Config {
                general: config.general.clone(),
                thresholds: config.thresholds.clone(),
                detection: config.detection.clone(),
                notification: config.notification.clone(),
            },
            last_notification: HashMap::new(),
            breach_counters: HashMap::new(),
            active_alerts: HashMap::new(),
            load_start_time: None,
        }
    }

    /// Check metrics for anomalies, respecting cooldown periods
    pub fn check(&mut self, metrics: &SystemMetrics) -> Option<Anomaly> {
        let mut anomalies_raw: Vec<Anomaly> = Vec::new();

        // Check memory percentage
        if let Some(a) = self.check_memory_percent(metrics) {
            anomalies_raw.push(a);
        }

        // Check swap percentage
        if let Some(a) = self.check_swap_percent(metrics) {
            anomalies_raw.push(a);
        }

        // Check load average
        if let Some(a) = self.check_load_average(metrics) {
            anomalies_raw.push(a);
        }

        // Check memory growth rate
        if let Some(a) = self.check_memory_growth_rate(metrics) {
            anomalies_raw.push(a);
        }

        // Check watchlist processes
        if let Some(a) = self.check_process_watchlist(metrics) {
            anomalies_raw.push(a);
        }

        // Return the most severe anomaly that passes cooldown
        let mut anomalies = anomalies_raw.clone();
        anomalies.sort_by_key(|a| match a.level {
            AlertLevel::Critical => 0,
            AlertLevel::Warning => 1,
        });

        for anomaly in anomalies {
            // Phase 4: Alert Inhibition & Hierarchy
            // If we have a Memory alert, suppress Growth alerts (they are redundant/noisy)
            if anomaly.anomaly_type == AnomalyType::MemoryGrowthRate {
                if anomalies_raw.iter().any(|a| a.anomaly_type == AnomalyType::Memory) {
                    debug!("Inhibiting Growth alert because Memory alert is present");
                    continue;
                }
            }

            // Apply Damping: Increment counter for this anomaly type
            let counter = self.breach_counters.entry(anomaly.anomaly_type).or_insert(0);
            *counter += 1;

            if *counter < self.config.detection.persistent_breach_threshold {
                debug!("Damping {:?}: breach_count={} < threshold={}", 
                    anomaly.anomaly_type, *counter, self.config.detection.persistent_breach_threshold);
                continue;
            }

            // Use stable key based on anomaly type only (ignore level for key)
            let key = format!("{:?}", anomaly.anomaly_type);
            
            if self.check_cooldown(&key, anomaly.level) {
                self.last_notification.insert(key, (Instant::now(), anomaly.level));
                self.active_alerts.insert(anomaly.anomaly_type, anomaly.level);
                debug!("Anomaly detected: {:?}. Details: {:?}", anomaly.message, anomaly.details);
                return Some(anomaly);
            }
        }

        // Reset counters for anomaly types that didn't breach this time
        // Actually, we should only reset if no severity of that type was found
        // This is handled by a separate pass or by cleaning up after the loop
        let detected_types: std::collections::HashSet<_> = anomalies_raw.iter().map(|a| a.anomaly_type).collect();
        self.breach_counters.retain(|t, _| detected_types.contains(t));
        self.active_alerts.retain(|t, _| detected_types.contains(t));

        None
    }

    fn check_memory_percent(&self, metrics: &SystemMetrics) -> Option<Anomaly> {
        let percent = metrics.memory_percent;
        let thresholds = &self.config.thresholds;
        let recovery = thresholds.recovery_margin;

        // Apply Hysteresis: Use lower threshold if already in alert state
        let active_level = self.active_alerts.get(&AnomalyType::Memory);
        
        let (warn_thresh, crit_thresh) = match active_level {
            Some(AlertLevel::Critical) => (thresholds.memory_warning - recovery, thresholds.memory_critical - recovery),
            Some(AlertLevel::Warning) => (thresholds.memory_warning - recovery, thresholds.memory_critical),
            None => (thresholds.memory_warning, thresholds.memory_critical),
        };

        if percent >= crit_thresh {
            let culprit = self.get_memory_culprit(metrics);
            let msg = format!("Mem {:.0}%: {}", percent, culprit);
            
            Some(Anomaly {
                anomaly_type: AnomalyType::Memory,
                level: AlertLevel::Critical,
                message: msg,
                details: vec![],
            })
        } else if percent >= warn_thresh {
            let culprit = self.get_memory_culprit(metrics);
            let msg = format!("Mem {:.0}%: {}", percent, culprit);
            
            Some(Anomaly {
                anomaly_type: AnomalyType::Memory,
                level: AlertLevel::Warning,
                message: msg,
                details: vec![],
            })
        } else {
            None
        }
    }

    fn check_swap_percent(&self, metrics: &SystemMetrics) -> Option<Anomaly> {
        if metrics.swap_total == 0 {
            return None;
        }

        let percent = metrics.swap_percent;
        let thresholds = &self.config.thresholds;
        let recovery = thresholds.recovery_margin;

        // Apply Correlation: Suppress swap alerts if memory pressure is low (macOS opportunistic swapping)
        if metrics.memory_percent <= 80.0 {
            return None;
        }

        // Apply Hysteresis
        let active_level = self.active_alerts.get(&AnomalyType::Swap);
        let (warn_thresh, crit_thresh) = match active_level {
            Some(AlertLevel::Critical) => (thresholds.swap_warning - recovery, thresholds.swap_critical - recovery),
            Some(AlertLevel::Warning) => (thresholds.swap_warning - recovery, thresholds.swap_critical),
            None => (thresholds.swap_warning, thresholds.swap_critical),
        };

        let level = if percent >= crit_thresh {
            Some(AlertLevel::Critical)
        } else if percent >= warn_thresh {
            Some(AlertLevel::Warning)
        } else {
            None
        };

        if let Some(level) = level {
            let culprit = self.get_memory_culprit(metrics);
            let msg = format!("Swap {:.0}%: {}", percent, culprit);
            
            Some(Anomaly {
                anomaly_type: AnomalyType::Swap,
                level,
                message: msg,
                details: vec![],
            })
        } else {
            None
        }
    }

    fn check_load_average(&mut self, metrics: &SystemMetrics) -> Option<Anomaly> {
        let load = metrics.load_1m;
        let thresholds = &self.config.thresholds;
        let recovery = 1.0; // Fixed absolute recovery for load

        if load >= thresholds.load_warning {
            // High load detected, check if it's sustained
            if self.load_start_time.is_none() {
                self.load_start_time = Some(Instant::now());
                debug!("High load detected ({:.1}), starting timer", load);
            }

            let sustained_secs = self.load_start_time.unwrap().elapsed().as_secs();
            let alert_threshold_secs = 120; // 2 minutes

            if sustained_secs >= alert_threshold_secs {
                // Apply Hysteresis
                let active_level = self.active_alerts.get(&AnomalyType::Load);
                let (warn_thresh, crit_thresh) = match active_level {
                    Some(AlertLevel::Critical) => (thresholds.load_warning - recovery, thresholds.load_critical - recovery),
                    Some(AlertLevel::Warning) => (thresholds.load_warning - recovery, thresholds.load_critical),
                    None => (thresholds.load_warning, thresholds.load_critical),
                };

                let level = if load >= crit_thresh {
                    AlertLevel::Critical
                } else if load >= warn_thresh {
                    AlertLevel::Warning
                } else {
                    return None;
                };

                let culprit = self.get_cpu_culprit(metrics);
                let msg = format!("Load {:.1}: {}", load, culprit);

                return Some(Anomaly {
                    anomaly_type: AnomalyType::Load,
                    level,
                    message: msg,
                    details: vec![],
                });
            }
        } else {
            // Load is normal, reset timer
            if self.load_start_time.is_some() {
                debug!("Load returned to normal, resetting timer");
                self.load_start_time = None;
            }
        }

        None
    }

    fn check_memory_growth_rate(&self, metrics: &SystemMetrics) -> Option<Anomaly> {
        let rate = metrics.memory_growth_rate?;
        let thresholds = &self.config.thresholds;

        // Condition 1: Only alert on positive growth (memory increasing)
        if rate <= 0.0 {
            return None;
        }

        // Phase 3: Cross-Metric Correlation
        // Only alert on growth if the system is actually under some pressure,
        // unless growth is so high it's clearly an emergency.
        let high_pressure = metrics.memory_percent > 80.0 || metrics.swap_percent > 80.0;
        let critical_growth = rate >= 10.0;

        if !high_pressure && !critical_growth {
             // System is healthy enough to handle some growth without bothering the user.
             return None;
        }

        // Condition 2: Only alert if system memory usage is significant (> 60%)
        // This avoids noise when memory is fluctuates at low usage levels.
        if metrics.memory_percent < 60.0 {
            return None;
        }

        if rate >= thresholds.memory_growth_rate_critical {
            let culprit = self.get_memory_culprit(metrics);
            let msg = format!("Growth {:.0}GB/h: {}", rate, culprit);
            
            Some(Anomaly {
                anomaly_type: AnomalyType::MemoryGrowthRate,
                level: AlertLevel::Critical,
                message: msg,
                details: vec![],
            })
        } else if rate >= thresholds.memory_growth_rate_warning {
            let culprit = self.get_memory_culprit(metrics);
            let msg = format!("Growth {:.0}GB/h: {}", rate, culprit);
            
            Some(Anomaly {
                anomaly_type: AnomalyType::MemoryGrowthRate,
                level: AlertLevel::Warning,
                message: msg,
                details: vec![],
            })
        } else {
            None
        }
    }

    fn check_process_watchlist(&self, metrics: &SystemMetrics) -> Option<Anomaly> {
        let watchlist = &self.config.detection.process_watchlist;
        let threshold_mb = self.config.detection.process_memory_threshold_mb;

        // Check both raw top processes and aggregated processes
        let mut all_to_check = metrics.top_processes.clone();
        all_to_check.extend(metrics.aggregated_processes.clone());

        for proc in all_to_check {
            let proc_name_lower = proc.name.to_lowercase();

            for watched in watchlist {
                if proc_name_lower.contains(&watched.to_lowercase()) {
                    if proc.memory_mb >= threshold_mb as f64 {
                        // Resolve generic names for watchlist matches too
                        let mut name = proc.name.replace(" (Group)", "");
                        if (name == "Electron" || name == "Electron Helper") && proc.exe.is_some() {
                            if let Some(exe_path) = &proc.exe {
                                if let Some(start) = exe_path.find("/Applications/") {
                                    if let Some(end) = exe_path[start..].find(".app/") {
                                            let app_path = &exe_path[start..start+end];
                                            if let Some(slash) = app_path.rfind('/') {
                                                name = app_path[slash+1..].to_string();
                                            }
                                    }
                                }
                            }
                        }

                        return Some(Anomaly {
                            anomaly_type: AnomalyType::ProcessWatchlist,
                            level: AlertLevel::Warning,
                            message: format!("Heavy App: {} ({:.0}GB)", name, proc.memory_mb / 1024.0),
                            details: vec![],
                        });
                    }
                }
            }
        }

        None
    }

    /// Check if enough time has passed since last notification for this key
    /// Check if enough time has passed since last notification for this key
    /// Returns TRUE if we should notify, FALSE if we should suppress.
    /// Logic:
    /// - If no previous alert: TRUE
    /// - If usage went up (Warning -> Critical): TRUE (escalate immediately)
    /// - If usage is same/lower: Check cooldown
    fn check_cooldown(&self, key: &str, current_level: AlertLevel) -> bool {
        let cooldown_secs = self.config.detection.notification_cooldown_minutes * 60;

        match self.last_notification.get(key) {
            Some((last_time, last_level)) => {
                // Always allow escalation (Warning -> Critical)
                if current_level > *last_level {
                    debug!("Escalating alert for '{}': {:?} -> {:?}", key, last_level, current_level);
                    return true;
                }

                let elapsed = last_time.elapsed().as_secs();
                let passed = elapsed >= cooldown_secs;
                
                if !passed {
                    debug!("Suppressed alert for '{}' ({:?}): elapsed={}s < cooldown={}s", 
                        key, current_level, elapsed, cooldown_secs);
                }
                
                passed
            }
            None => true,
        }
    }

    /// Get string describing the top memory user: "Ghostty (1GB)"
    fn get_memory_culprit(&self, metrics: &SystemMetrics) -> String {
        let mut all_procs = metrics.top_processes.clone();
        all_procs.extend(metrics.aggregated_processes.clone());
        
        // Sort by memory descending
        all_procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

        if let Some(top) = all_procs.first() {
            // Remove "(Group)" for brevity if present
            let mut name = top.name.replace(" (Group)", "");

            // Refine generic names like "Electron" using the executable path
            if (name == "Electron" || name == "Electron Helper") && top.exe.is_some() {
                if let Some(exe_path) = &top.exe {
                    // Try to find /Applications/AppName.app/
                    if let Some(start) = exe_path.find("/Applications/") {
                        if let Some(end) = exe_path[start..].find(".app/") {
                             // Extract "Beeper" from "/Applications/Beeper.app/..."
                             let app_path = &exe_path[start..start+end];
                             if let Some(slash) = app_path.rfind('/') {
                                 name = app_path[slash+1..].to_string();
                             }
                        }
                    }
                }
            }

            // Round to integer GB for brevity
            format!("{} ({:.0}GB)", name, top.memory_mb / 1024.0)
        } else {
            "Unknown".to_string()
        }
    }

    /// Get string describing the top CPU user: "ffmpeg (120%)"
    fn get_cpu_culprit(&self, metrics: &SystemMetrics) -> String {
        let mut all_procs = metrics.top_processes.clone();
        all_procs.extend(metrics.aggregated_processes.clone());
        
        // Sort by CPU descending
        all_procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(top) = all_procs.first() {
            let mut name = top.name.replace(" (Group)", "");
            
            // Refine generic names
            if (name == "Electron" || name == "Electron Helper") && top.exe.is_some() {
                if let Some(exe_path) = &top.exe {
                    if let Some(start) = exe_path.find("/Applications/") {
                        if let Some(end) = exe_path[start..].find(".app/") {
                             let app_path = &exe_path[start..start+end];
                             if let Some(slash) = app_path.rfind('/') {
                                 name = app_path[slash+1..].to_string();
                             }
                        }
                    }
                }
            }

            format!("{} ({:.0}%)", name, top.cpu_usage)
        } else {
            "Unknown".to_string()
        }
    }
}

// Implement Clone for Config components that need it
impl Clone for crate::config::GeneralConfig {
    fn clone(&self) -> Self {
        Self {
            check_interval_seconds: self.check_interval_seconds,
            log_file: self.log_file.clone(),
        }
    }
}

impl Clone for crate::config::ThresholdConfig {
    fn clone(&self) -> Self {
        Self {
            memory_warning: self.memory_warning,
            memory_critical: self.memory_critical,
            swap_warning: self.swap_warning,
            swap_critical: self.swap_critical,
            load_warning: self.load_warning,
            load_critical: self.load_critical,
            memory_growth_rate_warning: self.memory_growth_rate_warning,
            memory_growth_rate_critical: self.memory_growth_rate_critical,
            recovery_margin: self.recovery_margin,
        }
    }
}

impl Clone for crate::config::DetectionConfig {
    fn clone(&self) -> Self {
        Self {
            process_watchlist: self.process_watchlist.clone(),
            process_memory_threshold_mb: self.process_memory_threshold_mb,
            notification_cooldown_minutes: self.notification_cooldown_minutes,
            persistent_breach_threshold: self.persistent_breach_threshold,
        }
    }
}

impl Clone for crate::config::NotificationConfig {
    fn clone(&self) -> Self {
        Self {
            use_hammerspoon: self.use_hammerspoon,
            fallback_to_terminal_notifier: self.fallback_to_terminal_notifier,
            warning_color: self.warning_color.clone(),
            critical_color: self.critical_color.clone(),
        }
    }
}

impl Clone for crate::config::Config {
    fn clone(&self) -> Self {
        Self {
            general: self.general.clone(),
            thresholds: self.thresholds.clone(),
            detection: self.detection.clone(),
            notification: self.notification.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::SystemMetrics;
    use crate::config::Config;
    use std::time::Duration;

    fn mock_metrics(mem: f64, swap: f64, growth: Option<f64>) -> SystemMetrics {
        SystemMetrics {
            timestamp: chrono::Local::now(),
            memory_total: 100 * 1024 * 1024 * 1024, // 100GB
            memory_used: (mem * 1024.0 * 1024.0 * 1024.0) as u64,
            memory_free: ((100.0 - mem) * 1024.0 * 1024.0 * 1024.0) as u64,
            memory_percent: mem,
            swap_total: 100 * 1024 * 1024 * 1024,
            swap_used: (swap * 1024.0 * 1024.0 * 1024.0) as u64,
            swap_percent: swap,
            load_1m: 1.0,
            load_5m: 1.0,
            load_15m: 1.0,
            top_processes: vec![],
            aggregated_processes: vec![],
            memory_growth_rate: growth,
        }
    }

    #[test]
    fn test_damping() {
        let mut config = Config::default();
        config.detection.persistent_breach_threshold = 3;
        config.thresholds.memory_critical = 90.0;
        
        let mut detector = AnomalyDetector::new(&config);
        let metrics = mock_metrics(95.0, 0.0, None);

        // Breach 1
        assert!(detector.check(&metrics).is_none());
        assert_eq!(*detector.breach_counters.get(&AnomalyType::Memory).unwrap(), 1);

        // Breach 2
        assert!(detector.check(&metrics).is_none());
        assert_eq!(*detector.breach_counters.get(&AnomalyType::Memory).unwrap(), 2);

        // Breach 3
        let a = detector.check(&metrics).expect("Should alert now");
        assert_eq!(a.level, AlertLevel::Critical);
    }

    #[test]
    fn test_hysteresis() {
        let mut config = Config::default();
        config.detection.persistent_breach_threshold = 1;
        config.detection.notification_cooldown_minutes = 0; // Disable cooldown for test
        config.thresholds.memory_warning = 80.0;
        config.thresholds.recovery_margin = 5.0; // recovery at 75.0

        let mut detector = AnomalyDetector::new(&config);
        
        // 1. Enter warning
        let m_high = mock_metrics(82.0, 0.0, None);
        detector.check(&m_high).expect("Should fire");

        // 2. Drop below threshold but above recovery (80.0 -> 78.0)
        let m_mid = mock_metrics(78.0, 0.0, None);
        let a = detector.check(&m_mid).expect("Should stay in alert");
        assert_eq!(a.level, AlertLevel::Warning);

        // 3. Drop below recovery (75.0 -> 74.0)
        let m_low = mock_metrics(74.0, 0.0, None);
        assert!(detector.check(&m_low).is_none());
        assert!(detector.active_alerts.get(&AnomalyType::Memory).is_none());
    }

    #[test]
    fn test_inhibition() {
        let mut config = Config::default();
        config.detection.persistent_breach_threshold = 1;
        config.detection.notification_cooldown_minutes = 0;
        config.thresholds.memory_warning = 80.0;
        config.thresholds.memory_growth_rate_warning = 1.0;

        let mut detector = AnomalyDetector::new(&config);
        
        // High memory + high growth
        let m = mock_metrics(85.0, 0.0, Some(5.0));
        let a = detector.check(&m).expect("Should alert");
        
        // Growth should be inhibited by memory
        assert_eq!(a.anomaly_type, AnomalyType::Memory);
    }

    #[test]
    fn test_swap_correlation() {
        let mut config = Config::default();
        config.detection.persistent_breach_threshold = 1;
        config.detection.notification_cooldown_minutes = 0;
        config.thresholds.swap_warning = 80.0;
        config.thresholds.memory_warning = 90.0;

        let mut detector = AnomalyDetector::new(&config);
        
        // High swap (90%) but low memory (70%)
        let m_low_mem = mock_metrics(70.0, 90.0, None);
        assert!(detector.check(&m_low_mem).is_none(), "Swap alert should be suppressed when memory is low");

        // High swap (90%) and high memory (85%)
        let m_high_mem = mock_metrics(85.0, 90.0, None);
        let a = detector.check(&m_high_mem).expect("Should alert when memory is high");
        assert_eq!(a.anomaly_type, AnomalyType::Swap);
    }
}
