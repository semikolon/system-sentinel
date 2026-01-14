# System Sentinel

A low-overhead, Rust-native system health monitor for macOS. It watches your system resources and alerts you via Hammerspoon when things go wrong.

## Features
- **Sustained Load Detection**: Alerts only after 2 minutes of high CPU load to avoid false positives.
- **Auto-Discovery Process Aggregation**: Automatically groups and tracks memory for process families (e.g., "Ghostty", "Electron", "Arc") without manual configuration.
- **Smart Alerts**: Distinguishes between transient spikes and real problems.

## Configuration
Config file location: `~/.config/system-sentinel/config.toml`

The Sentinel automatically discovers applications to watch. You can tune thresholds in the config file, but you do NOT need to list applications manually.

## 🚀 Deployment & Updates

**CRITICAL**: Since System Sentinel runs as a macOS Launch Daemon (background service), simply running `cargo build` is not enough. You must update the running service binary.

### How to Update
After making code changes, run this sequence:

```bash
# 1. Build optimized release binary
cargo build --release

# 2. **REQUIRED**: Copy binary to the service location
cp target/release/system-sentinel ~/.local/bin/system-sentinel

# 3. **REQUIRED**: Restart the Launch Daemon to pick up changes
launchctl unload ~/Library/LaunchAgents/com.fredrikbranstrom.system-sentinel.plist
launchctl load ~/Library/LaunchAgents/com.fredrikbranstrom.system-sentinel.plist
```

### Checking Status
To see if the service is running:
```bash
launchctl list | grep sentinel
```

To view logs:
```bash
tail -f ~/.local/share/system-sentinel/stdout.log
```
