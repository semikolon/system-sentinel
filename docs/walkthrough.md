# Walkthrough: Intelligent Monitoring Upgrade

System Sentinel has been upgraded from simple threshold alerts to an intelligent, context-aware monitoring system. This significantly reduces "false positive" noise while remaining vigilant for true system crises.

## New Intelligence Features

### 1. Damping (Persistence)
Sentinel now ignores short "blips" or startup spikes.
- **How it works**: A breach must be detected **3 consecutive times** (~1.5 minutes) before an alert is fired.
- **Benefit**: Opening a heavy app like Arc or starting a build won't trigger immediate growth or swap alerts.

### 2. Hysteresis (Sticky States)
Prevents "flapping" alerts when a metric hovers exactly at the threshold.
- **How it works**: Once an alert fires (e.g., at 90%), it will **not clear** until the system drops below the **recovery margin** (e.g., 85%).
- **Benefit**: You won't get multiple notifications for the same event as usage fluctuates by 0.1%.

### 3. Cross-Metric Correlation
Sentinel now understands the difference between "using memory" and "running out of memory."
- **How it works**: **Memory Growth** alerts are now **muted** unless the system is also under pressure (Memory > 80% or Swap > 80%). 
- **Exception**: If growth is extreme (> 10GB/h), it will always alert.
- **Benefit**: Normal development work that temporarily uses more RAM is ignored until it actually impacts system health.

### 4. Correlated Swap Alerting
Specifically addresses macOS opportunistic swapping.
- **How it works**: Swap alerts are now **muting** unless your memory usage is also high (**> 80%**).
- **Benefit**: You will no longer be notified about 90% swap usage if the system has plenty of RAM headroom.

### 6. Narration Daemon Integration
Auditory awareness via your existing `tts_daemon`.
- **How it works**: Sentinel now talks to `/tmp/claude-tts-daemon.sock` to queue brief (4-5 word) alerts.
- **Benefit**: You'll hear "Memory critical, Ghostty 3 gigabytes" even if you're not looking at the screen.
- **Queueing**: Narrations are queued and serialized; Sentinel will never speak over other narrations.

### 7. Auditory Distinction (Warning vs. Critical)
The sentinel now provides sound "hints" to the daemon for immediate severity awareness.
- **Warning**: Plays `sounds/subtle/alien_button.wav` (A light, high-tech interface click).
- **Critical**: Plays `sounds/unused/Futuristic Hum 2133.wav` (The ominous, heavy atmosphere you selected).
- **Intelligence**: Just like the visual alerts, these sounds are governed by the damping and correlation logic to ensure they only fire when truly necessary.

---

## Verification Results

### Unit Tests Passed
I've added a test suite to [detector.rs](file:///Users/fredrikbranstrom/Projects/system-sentinel/src/detector.rs) to verify these behaviors:
- [x] **test_damping**: Verified 3-step firing logic.
- [x] **test_hysteresis**: Verified recovery margin clears the alert correctly.
- [x] **test_inhibition**: Verified Growth is muted when Memory is high.
- [x] **test_swap_correlation**: Verified swap alerts are suppressed when memory is low.

### Live Status
The updated sentinel service is running with all phases (1-5) active. It is successfully monitoring system health, correlating swap usage with memory pressure, inhibiting redundant alerts, and narrating critical events via the background daemon.
```bash
launchctl list | grep system-sentinel
# Result: Running with PID 31869
```

## Next Steps
- **Phase 4: Percentile Baselines**: The system is ready for auto-tuning logic. Once it collects 24 hours of data, we can implement the logic to automatically set thresholds based on your specific "98th percentile" of usage.
