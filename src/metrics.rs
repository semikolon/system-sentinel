//! System metrics collection using sysinfo crate

use std::collections::{VecDeque, HashMap};
use sysinfo::{System, ProcessesToUpdate, MemoryRefreshKind, ProcessRefreshKind};
use serde::{Serialize, Deserialize};
use tracing::debug;

/// Snapshot of system metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Local>,

    // Memory metrics (in bytes)
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_free: u64,
    pub memory_percent: f64,

    // Swap metrics (in bytes)
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_percent: f64,

    // Load averages
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,

    // Top memory-consuming processes
    pub top_processes: Vec<ProcessInfo>,

    // Aggregated memory usage for watched processes (e.g., app families)
    pub aggregated_processes: Vec<ProcessInfo>,

    // Memory growth rate (GB/hour, calculated from history)
    pub memory_growth_rate: Option<f64>,
}

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

impl ProcessInfo {
    /// Returns a human-friendly name for the process.
    /// Resolves generic names like "Electron" or "java" to their app bundle names if possible.
    pub fn human_name(&self) -> String {
        let name = self.name.replace(" (Group)", "");

        // Heuristics for generic names that are often just runners for a real app
        let names_to_resolve = [
            "Electron", "Electron Helper", "java", "Python", "node", "ruby", "Web Content"
        ];
        
        let needs_resolution = names_to_resolve.iter().any(|&n| name.contains(n));

        if needs_resolution && self.exe.is_some() {
            if let Some(exe_path) = &self.exe {
                if let Some(app_name) = extract_app_name(exe_path) {
                    return app_name;
                }
            }
        }

        name
    }
}

/// Helper to extract \"App Name\" from a path containing .app
/// e.g. \"/Applications/Visual Studio Code.app/Contents/MacOS/Electron\" -> \"Visual Studio Code\"
fn extract_app_name(path: &str) -> Option<String> {
    if let Some(idx) = path.find(".app") {
        // Find the last slash before the .app
        let prefix = &path[..idx];
        if let Some(slash_idx) = prefix.rfind('/') {
            return Some(prefix[slash_idx + 1..].to_string());
        }
    }
    None
}

/// Collects system metrics with historical tracking for rate calculations
pub struct MetricsCollector {
    system: System,
    /// Rolling history for growth rate calculation (last 10 minutes)
    memory_history: VecDeque<(chrono::DateTime<chrono::Local>, u64)>,
    /// Maximum history entries (at 30s intervals, 20 entries = 10 minutes)
    max_history: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            memory_history: VecDeque::new(),
            max_history: 60, // 30 minutes at 30s intervals
        }
    }

    /// Collect current system metrics
    pub fn collect(&mut self) -> SystemMetrics {
        let now = chrono::Local::now();

        // Refresh system information
        self.system.refresh_memory_specifics(MemoryRefreshKind::new().with_ram().with_swap());
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,  // refresh_user
            ProcessRefreshKind::everything(),
        );

        // Memory metrics
        let memory_total = self.system.total_memory();
        let memory_used = self.system.used_memory();
        let memory_free = self.system.free_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64) * 100.0
        } else {
            0.0
        };

        // Swap metrics
        let swap_total = self.system.total_swap();
        let swap_used = self.system.used_swap();
        let swap_percent = if swap_total > 0 {
            (swap_used as f64 / swap_total as f64) * 100.0
        } else {
            0.0
        };

        // Load averages
        let load_avg = System::load_average();

        // Top memory-consuming processes
        let mut processes: Vec<ProcessInfo> = self.system
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                parent_pid: p.parent().map(|ppid| ppid.as_u32()),
                name: p.name().to_string_lossy().to_string(),
                memory_bytes: p.memory(),
                memory_mb: p.memory() as f64 / 1024.0 / 1024.0,
                cpu_usage: p.cpu_usage(),
                exe: p.exe().map(|path| path.to_string_lossy().to_string()),
            })
            .collect();

        // Sort by memory usage descending
        processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        let top_processes: Vec<ProcessInfo> = processes.into_iter().take(10).collect();

        // Update history and calculate growth rate
        self.memory_history.push_back((now, memory_used));
        while self.memory_history.len() > self.max_history {
            self.memory_history.pop_front();
        }

        let memory_growth_rate = self.calculate_growth_rate();

        debug!(
            "Metrics collected: mem={:.1}%, swap={:.1}%, load={:.1}",
            memory_percent, swap_percent, load_avg.one
        );

        SystemMetrics {
            timestamp: now,
            memory_total,
            memory_used,
            memory_free,
            memory_percent,
            swap_total,
            swap_used,
            swap_percent,
            load_1m: load_avg.one,
            load_5m: load_avg.five,
            load_15m: load_avg.fifteen,
            top_processes,
            aggregated_processes: Vec::new(), // Initialized as empty, can be populated if needed
            memory_growth_rate,
        }
    }

    /// Automatically discovers and aggregates memory for all application bundles.
    /// Uses heuristics to identify app roots (paths containing .app) and bridges process tree gaps.
    pub fn collect_aggregated(&mut self) -> SystemMetrics {
        let mut metrics = self.collect();
        
        let mut all_procs = HashMap::new();
        let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();

        // High-fidelity parent map fallback for macOS
        let ppid_map = self.get_macos_ppid_map();

        for (pid, proc) in self.system.processes() {
            let pid = pid.as_u32();
            
            // Use ps-provided PPID if available, otherwise trust sysinfo
            let parent_pid = ppid_map.get(&pid).copied()
                .or_else(|| proc.parent().map(|p| p.as_u32()));

            let info = ProcessInfo {
                pid,
                parent_pid,
                name: proc.name().to_string_lossy().to_string(),
                memory_bytes: proc.memory(),
                memory_mb: proc.memory() as f64 / 1024.0 / 1024.0,
                cpu_usage: proc.cpu_usage(),
                exe: proc.exe().map(|p| p.to_string_lossy().to_string()),
            };
            
            all_procs.insert(pid, info);

            if let Some(ppid) = parent_pid {
                children_map.entry(ppid).or_default().push(pid);
            }
        }

        // Auto-discover app roots
        let mut app_roots: HashMap<String, Vec<u32>> = HashMap::new();
        
        for info in all_procs.values() {
            if let Some(exe_path) = &info.exe {
                // Heuristic: If path contains ".app/", it's likely an application
                if let Some(app_name) = extract_app_name(exe_path) {
                    // Check if parent is NOT part of the same app (found a root)
                    let is_root = match info.parent_pid {
                        Some(ppid) => {
                            match all_procs.get(&ppid) {
                                Some(parent) => !parent.exe.as_ref().map_or(false, |p| p.contains(&format!("{}.app", app_name))),
                                None => true // Parent unknown (or pid 1), so this is a root
                            }
                        },
                        None => true
                    };

                    if is_root {
                         app_roots.entry(app_name).or_default().push(info.pid);
                    }
                }
            }
        }

         let mut aggregated = Vec::new();
        for (app_name, roots) in app_roots {
             let mut total_bytes = 0;
             let mut total_cpu = 0.0;
             let mut processed_pids = std::collections::HashSet::new();
             
             for root_pid in roots {
                let mut stack = vec![root_pid];
                
                while let Some(pid) = stack.pop() {
                    if !processed_pids.insert(pid) { continue; }

                    if let Some(proc) = all_procs.get(&pid) {
                        total_bytes += proc.memory_bytes;
                        total_cpu += proc.cpu_usage;
                        if let Some(children) = children_map.get(&pid) {
                            stack.extend(children);
                        }
                    }
                }
             }

             if total_bytes > 0 {
                aggregated.push(ProcessInfo {
                    pid: 0, // Virtual PID for group
                    parent_pid: None,
                    name: format!("{} (Group)", app_name), // e.g. "Ghostty (Group)"
                    memory_bytes: total_bytes,
                    memory_mb: total_bytes as f64 / 1024.0 / 1024.0,
                    cpu_usage: total_cpu,
                    exe: None,
                });
             }
        }

        metrics.aggregated_processes = aggregated;
        metrics
    }

    /// Calculate memory growth rate in GB/hour from historical data
    /// Uses Linear Least Squares Regression to be robust against noise.
    fn calculate_growth_rate(&self) -> Option<f64> {
        let n = self.memory_history.len() as f64;
        if n < 2.0 {
            return None;
        }

        let (oldest_time, _) = self.memory_history.front()?;
        
        // Convert to relative time (hours) and memory (GB) points
        let mut x_sum = 0.0;
        let mut y_sum = 0.0;
        let mut xy_sum = 0.0;
        let mut xx_sum = 0.0;

        for (time, mem) in &self.memory_history {
            let x = (*time - *oldest_time).num_seconds() as f64 / 3600.0; // Hours since start of window
            let y = *mem as f64 / 1024.0 / 1024.0 / 1024.0; // GB

            x_sum += x;
            y_sum += y;
            xy_sum += x * y;
            xx_sum += x * x;
        }

        // Linear least squares slope: (N∑xy - ∑x∑y) / (N∑x² - (∑x)²)
        let numerator = n * xy_sum - x_sum * y_sum;
        let denominator = n * xx_sum - x_sum * x_sum;

        if denominator.abs() < 1e-9 {
            return None; 
        }

        let slope_gb_per_hour = numerator / denominator;
        
        Some(slope_gb_per_hour)
    }


    /// Bridges gaps in the process tree by calling the system 'ps' utility.
    /// 'ps' has special kernel permissions to see parent/child relationships
    /// of root processes that normal libraries (and non-root users) miss.
    fn get_macos_ppid_map(&self) -> HashMap<u32, u32> {
        use std::process::Command;
        
        let mut map = HashMap::new();
        
        let output = Command::new("ps")
            .args(&["-ax", "-o", "pid,ppid"])
            .output();
            
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(pid), Ok(ppid)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                        if ppid != 0 {
                            map.insert(pid, ppid);
                        }
                    }
                }
            }
        }
        
        map
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
