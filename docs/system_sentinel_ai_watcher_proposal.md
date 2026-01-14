# System Sentinel: AI-Triggered Anomaly Response Daemon

**Proposal Date**: 2026-01-11
**Status**: Concept / Evaluation
**Author**: Fredrik Branström (with Claude Code analysis)

---

## Original Prompt (Verbatim)

> Hmmm this gives me an idea to have some kind of watcher in the background in my system as a launch daemon - just like the narration daemon or graphiti daemon (look at the dotfiles repo). It could be Rust-based to be very low-profile / performance focused. It should continually watch for any kind of suspicious escalation in memory pressure or CPU or GPU usage. And when such anomalous conditions appear, it should Claude Code in a terminal window that opens by itself and prompt Claude Code to start looking into the matter. Save a doc outlining this idea with this verbatim prompt included at the start, and evaluate the idea and try to think critically about it. Also do online research to see if others have had the same idea. Maybe there's something readily available out there already, wouldn't surprise me.

---

## Executive Summary

The idea is to create a low-overhead Rust daemon that monitors system health metrics and automatically launches Claude Code to investigate anomalies. This concept sits at the intersection of several emerging trends: self-healing IT systems, AI-powered observability, and autonomous agent orchestration.

**Verdict**: The idea is sound and increasingly mainstream in enterprise IT. However, for a personal workstation, a **notification-based approach** (alert → user decides → optional auto-launch) may be more appropriate than fully autonomous investigation. Several existing tools can be combined to achieve 80% of this vision today.

---

## Critical Evaluation

### Strengths

| Aspect | Analysis |
|--------|----------|
| **Proactive Detection** | Today's incident (70GB Ghostty leak) went unnoticed until system was nearly unusable. Early detection at 5-10GB would have prevented crisis. |
| **Leverages Existing Infrastructure** | Fits naturally with existing daemon pattern (TTS daemon, Graphiti daemon). LaunchAgent architecture is proven. |
| **Claude Code Capability Match** | Claude Code excels at exactly this task: reading logs, checking processes, correlating symptoms, suggesting fixes. The post-mortem we just created demonstrates this. |
| **Rust for Low Overhead** | Rust is ideal for always-on daemons: zero runtime overhead, predictable memory, no GC pauses. Existing crate `sysinfo` provides cross-platform metrics. |
| **Learning Opportunity** | Each investigation creates knowledge that can be captured via `/capture` or `/significance` commands, building institutional memory. |

### Concerns & Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Cost of False Positives** | High | Each Claude Code session costs API tokens. False positive triggering expensive investigation sessions could add up. Implement confidence thresholds and cooldown periods. |
| **The Watcher Paradox** | Medium | A monitoring daemon that itself leaks memory or consumes CPU would be ironic and dangerous. Rust mitigates but doesn't eliminate this. Implement self-monitoring with hard limits. |
| **Anomaly Definition Complexity** | High | What's "anomalous"? A compile job legitimately uses 100% CPU. ML training legitimately uses all GPU. Static thresholds will false-positive; ML-based baselines add complexity. |
| **Security Implications** | Medium | Auto-launching an AI agent with system access based on triggers could be exploited. Malware could intentionally trigger high CPU to get Claude Code to run and be manipulated. |
| **User Disruption** | Medium | Auto-opening terminal windows interrupts workflow. User may be in a meeting, presentation, or deep focus. |
| **Circular Investigation** | Low | Claude Code investigating high memory might itself contribute to memory pressure. Need to exclude the watcher and CC from metrics. |

### Critical Questions

1. **Threshold Calibration**: How do you distinguish "Ghostty leaking 70GB" from "legitimate 70GB video render"?
2. **Cooldown Logic**: After one investigation, how long before another can trigger?
3. **User Consent Model**: Always auto-launch? Notification first? Time-of-day rules?
4. **Investigation Scope**: What exactly should CC be prompted to do? Full forensics or quick triage?
5. **Action Authority**: Should CC just report, or can it take remedial action (kill processes)?

---

## Research Findings: Existing Solutions

### Highly Relevant

| Project | Description | Relevance |
|---------|-------------|-----------|
| **[macmon](https://github.com/vladkens/macmon)** | Rust-based sudoless performance monitoring for Apple Silicon. CPU/GPU/RAM/power/temperature. | Could be the metrics-gathering foundation. Doesn't have alerting or AI integration, but provides the data layer. |
| **[MCP System Monitor](https://github.com/DarkPhilosophy/mcp-system-monitor)** | MCP server designed for AI agents to monitor system info. Provides CPU, memory, disk, network, process data via MCP protocol. | Almost exactly what you're describing, but for Linux servers. Could be adapted for macOS + Claude Code. |
| **[self-healing-agent-system](https://github.com/wuyilun526/self-healing-agent-system)** | "Intelligent fault self-healing agent system that automatically detects, diagnoses, and repairs common system faults using AI-powered analysis." | Proof of concept exists. Python-based, server-focused, but validates the pattern. |
| **[Oracle adaptivemm](https://github.com/oracle/adaptivemm)** | Userspace daemon for proactive free memory management. | Enterprise-grade memory pressure handling. Linux-focused but shows the daemon pattern. |

### Industry Trend: Self-Healing AI Systems

The concept is becoming mainstream in enterprise IT:

- **UiPath Self-Healing Agents**: Detect, diagnose, and repair RPA failures in real-time
- **Klover.ai P.O.D.S.**: Point of Decision Systems with AI agents for IT management
- **The New Stack**: "3 Stages of Building Self-Healing IT Systems With Multiagent AI" (May 2025)
- **SuperAGI**: "Beginner's Guide to Implementing Self-Healing AI Systems" (June 2025)

**Key Insight**: Enterprise is moving toward this model for servers and infrastructure. Personal workstation use is less explored but follows naturally.

### macOS-Specific Considerations

- **Memory Pressure API**: macOS provides `memory_pressure` command and `vm_stat` for metrics
- **LaunchD Integration**: Native daemon management with `KeepAlive`, `ThrottleInterval`, resource limits
- **Notification Center**: Can alert user without opening windows via `osascript` or `terminal-notifier`
- **AppleScript/Shortcuts**: Can open Terminal.app and run commands programmatically

---

## Proposed Architecture

### Option A: Notification-First (Recommended for Personal Use)

```
┌─────────────────────────────────────────────────────────────┐
│                    System Sentinel Daemon                    │
│                         (Rust)                               │
├─────────────────────────────────────────────────────────────┤
│  Metrics Collection (every 30s)                             │
│  ├── Memory: vm_stat, memory_pressure                       │
│  ├── CPU: host_processor_info                               │
│  ├── GPU: IOKit (Apple Silicon)                             │
│  └── Process: libproc (top memory/CPU consumers)            │
├─────────────────────────────────────────────────────────────┤
│  Anomaly Detection                                          │
│  ├── Static thresholds (memory > 90%, swap > 50%)           │
│  ├── Rate-of-change detection (memory growing > 1GB/hour)   │
│  └── Process-specific tracking (known leakers list)         │
├─────────────────────────────────────────────────────────────┤
│  Response Actions                                           │
│  ├── Level 1: Log to file                                   │
│  ├── Level 2: macOS notification                            │
│  ├── Level 3: Notification with "Investigate" button        │
│  └── Level 4: Auto-launch Claude Code (user-configurable)   │
└─────────────────────────────────────────────────────────────┘
```

### Option B: Full Autonomous (Higher Risk, Higher Reward)

```
Anomaly Detected
      │
      ▼
┌─────────────────┐
│ Confidence > 85%│──No──▶ Log only
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Cooldown clear? │──No──▶ Log only
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Open Terminal   │
│ + Claude Code   │
│ with prompt:    │
│ "Investigate    │
│  system anomaly │
│  detected..."   │
└─────────────────┘
```

### Claude Code Prompt Template

```
System Sentinel detected an anomaly:

**Timestamp**: {timestamp}
**Anomaly Type**: {type} (memory_pressure | cpu_spike | gpu_overload)
**Severity**: {severity}/10
**Details**:
- Memory: {mem_used}GB / {mem_total}GB ({mem_percent}%)
- Swap: {swap_used}GB / {swap_total}GB
- Load: {load_1m}, {load_5m}, {load_15m}
- Top consumers: {top_5_processes}

**Your task**:
1. Identify the root cause of this anomaly
2. Determine if it's a legitimate workload or a problem (leak, runaway process)
3. If it's a problem, suggest remediation steps
4. Save findings to ~/Documents/system_sentinel_reports/

Do NOT take destructive action without user confirmation.
```

---

## Implementation Approach

### Phase 1: Metrics Foundation (Week 1)
- Create Rust daemon using `sysinfo` crate
- Implement metrics collection: memory, CPU, swap, top processes
- Log to file, no alerting yet
- LaunchAgent plist for auto-start

### Phase 2: Anomaly Detection (Week 2)
- Implement threshold-based detection
- Add rate-of-change tracking (memory growing over time)
- Process-specific watchlist (Ghostty, Arc, etc.)
- macOS notifications via `terminal-notifier`

### Phase 3: Claude Code Integration (Week 3)
- AppleScript to open Terminal + run `claude` with prompt
- Configurable auto-launch vs notification-only mode
- Cooldown logic (minimum 30 min between auto-launches)
- Cost tracking (log each AI investigation)

### Phase 4: Learning Loop (Week 4)
- Parse Claude Code investigation results
- Build knowledge base of past incidents
- Adjust thresholds based on false positive rate
- Integration with Graphiti for persistent memory

---

## Rust Crates to Use

| Crate | Purpose |
|-------|---------|
| `sysinfo` | Cross-platform system metrics (CPU, memory, processes) |
| `notify-rust` | Desktop notifications |
| `tokio` | Async runtime for daemon |
| `serde` / `toml` | Configuration file parsing |
| `tracing` | Structured logging |
| `clap` | CLI argument parsing |

---

## Configuration File (Proposed)

```toml
# ~/.config/system-sentinel/config.toml

[thresholds]
memory_percent_warning = 80
memory_percent_critical = 90
swap_percent_warning = 50
swap_percent_critical = 80
load_average_warning = 10.0
load_average_critical = 50.0

[detection]
check_interval_seconds = 30
memory_growth_rate_gb_per_hour = 2.0  # Alert if growing faster
process_watchlist = ["ghostty", "Arc", "node"]

[response]
level_1_action = "log"           # Always
level_2_action = "notify"        # Warning threshold
level_3_action = "notify_button" # Critical threshold
level_4_action = "auto_launch"   # Severe (configurable)
auto_launch_enabled = false      # User must opt-in
cooldown_minutes = 30

[claude_code]
prompt_template = "~/.config/system-sentinel/prompt.md"
working_directory = "~"
save_reports_to = "~/Documents/system_sentinel_reports/"
```

---

## Alternatives to Building From Scratch

### Quick Win: Shell Script + Cron

```bash
#!/bin/bash
# ~/bin/memory-watcher.sh
MEM_PRESSURE=$(memory_pressure | grep "System-wide" | awk '{print $NF}' | tr -d '%')
if [ "$MEM_PRESSURE" -lt 20 ]; then
    terminal-notifier -title "Memory Warning" \
        -message "Only ${MEM_PRESSURE}% free. Run 'claude' to investigate?" \
        -execute "open -a Terminal && claude"
fi
```

Add to crontab: `*/5 * * * * ~/bin/memory-watcher.sh`

**Pros**: Works today, 10 minutes to implement
**Cons**: Not sophisticated, no rate-of-change detection, crude thresholds

### Hybrid: macmon + Custom Alerting

1. Install `macmon` for metrics visualization
2. Add alerting layer on top (shell script reading macmon data)
3. Trigger Claude Code when thresholds exceeded

---

## Recommendation

**Start simple, iterate based on experience:**

1. **Today**: Implement the shell script quick win
2. **This week**: Add to cron, tune thresholds based on false positives
3. **If valuable**: Build proper Rust daemon with sophisticated detection
4. **Long-term**: Consider contributing to or forking `mcp-system-monitor` for macOS

The 70GB Ghostty incident proves the value of proactive monitoring. But the right level of automation depends on how often anomalies occur and how disruptive false positives would be.

---

## Open Questions for Future Consideration

1. Should the daemon have **kill authority** for known-bad processes (e.g., auto-kill Ghostty if >10GB)?
2. Should it integrate with the **TTS narration system** to speak alerts?
3. Could it use **local ML** (e.g., Core ML) for anomaly detection instead of static thresholds?
4. Should investigation results feed into **Graphiti** for cross-session learning?
5. Is there value in a **community-shared watchlist** of known leaky apps?

---

## References

- [macmon - Rust macOS monitor](https://github.com/vladkens/macmon)
- [MCP System Monitor for AI agents](https://github.com/DarkPhilosophy/mcp-system-monitor)
- [Self-Healing Agent System](https://github.com/wuyilun526/self-healing-agent-system)
- [Oracle adaptivemm](https://github.com/oracle/adaptivemm)
- [systemd Memory Pressure Handling](https://systemd.io/MEMORY_PRESSURE/)
- [The New Stack: Self-Healing IT Systems](https://thenewstack.io/three-stages-of-building-self-healing-it-systems-with-multiagent-ai/)
- [Building a Real-Time System Monitor in Rust](https://thenewstack.io/building-a-real-time-system-monitor-in-rust-terminal/)

---

*Document generated following Ghostty 70GB memory leak incident (2026-01-11)*
