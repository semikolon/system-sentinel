# System Sentinel - Implementation Plan v2.1

**Project**: system-sentinel
**Created**: 2026-01-11
**Updated**: 2026-01-15 10:25 CET (v2.2 - Intelligent Tier 0 live)
**Status**: Phase 2 Complete - Intelligent Damping & Correlation Operational
**Origin**: Post-mortem from Ghostty 70GB memory leak incident

---

## Development Session Info

**Primary Session Location**: `~/Documents/superwhisper` (CC session ID: `85154bc8-b414-46eb-8f1d-9c14f48d4377`)
**Original Proposal**: Moved to `docs/system_sentinel_ai_watcher_proposal.md`

---

## ⚠️ Claude Code Integration Status: DISABLED

**AI/LLM integration (Phases 2+) is NOT enabled.** The current daemon is Tier 0 only:
- ✅ Rust-native threshold detection
- ✅ Hammerspoon notifications
- ❌ NO Gemini CLI calls
- ❌ NO Claude Agent SDK calls
- ❌ NO token spending

**Enable AI when**: User explicitly requests in future session.

---

## Vision

An AI-powered system guardian that monitors macOS health, intelligently diagnoses anomalies, and can take autonomous protective action in extreme cases. Unlike simple threshold alerts, System Sentinel uses tiered intelligence to distinguish true outliers from normal high-usage patterns.

**Core Goals**:
1. **Never let another meltdown happen** - Auto-detect and mitigate before crisis
2. **Token-conscious** - Escalate intelligence only when needed
3. **User-friendly** - Menu bar UI with simple chat interface
4. **Autonomous but safe** - Take action only when rational, with safeguards

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         System Sentinel Suite                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────┐          ┌─────────────────────────────────────┐  │
│  │   Metrics Daemon    │          │      Menu Bar App (Tauri)           │  │
│  │      (Rust)         │◀────────▶│                                     │  │
│  │                     │   IPC    │  ┌─────────────────────────────┐    │  │
│  │  • sysinfo metrics  │          │  │   System Tray Icon          │    │  │
│  │  • Tier 0 detection │          │  │   🟢 🟡 🔴 (health status) │    │  │
│  │  • Rate tracking    │          │  └─────────────────────────────┘    │  │
│  │                     │          │              │                       │  │
│  └─────────────────────┘          │              ▼                       │  │
│           │                       │  ┌─────────────────────────────┐    │  │
│           │ anomaly               │  │   Dropdown Panel            │    │  │
│           ▼                       │  │                             │    │  │
│  ┌─────────────────────┐          │  │  • Current metrics summary  │    │  │
│  │  Intelligence Hub   │          │  │  • Recent alerts            │    │  │
│  │                     │          │  │  • Chat input to Claude     │    │  │
│  │  Tier 1: Haiku     │──────────│  │  • Suggested actions        │    │  │
│  │  Tier 2: Sonnet    │          │  └─────────────────────────────┘    │  │
│  │  Tier 3: Actions   │          │                                     │  │
│  └─────────────────────┘          └─────────────────────────────────────┘  │
│           │                                                                 │
│           ▼                                                                 │
│  ┌─────────────────────┐                                                   │
│  │  Claude Agent SDK   │                                                   │
│  │  (Python wrapper)   │                                                   │
│  │                     │                                                   │
│  │  • Tool execution   │                                                   │
│  │  • System diagnosis │                                                   │
│  │  • Action execution │                                                   │
│  └─────────────────────┘                                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Tiered Intelligence Architecture (Token-Conscious)

### Tier 0: Rust-Native Detection (Always On, Zero Tokens)

Pure Rust threshold and rate-of-change detection. No LLM calls.

**Triggers**:
- Memory > 80% (warning) / 90% (critical)
- Swap > 50% (warning) / 75% (critical)
- Load average > 10 (warning) / 50 (critical)
- Memory growth > 2 GB/hr (warning) / 5 GB/hr (critical)
- Watchlist process > configured threshold

**Actions**: Update menu bar icon, show Hammerspoon notification

**Cost**: $0.00

### Tier 1: Quick Classification (On Anomaly, FREE with Gemini)

Light LLM call (Gemini 3 Flash) to classify the anomaly.

**Default Model**: Gemini 3 Flash ($0.50/$3.00 per 1M tokens - FREE within daily limits)
**Alternative**: Claude Haiku ($0.25/$1.25 per 1M tokens)

**Triggers**: Any Tier 0 warning or critical alert

**Purpose**:
- Distinguish false positives (legitimate high usage)
- Classify urgency: "expected workload" vs "potential issue" vs "crisis"
- Decide whether to escalate to Tier 2

**Prompt Template** (~500 tokens):
```
System metrics: {memory_pct}% RAM, {swap_pct}% swap, load {load_1m}
Top processes: {top_5_processes}
Historical context: {growth_rate}GB/hr over {window}
User context: {time_of_day}, {active_apps}

Classify: [normal_high_usage | investigate | urgent | critical]
Confidence: [low | medium | high]
Brief reason (10 words max):
```

**Cost**: FREE (within Gemini free tier: 1,000 RPD, 250K TPM)

### Tier 2: Full Diagnosis (On Significant Anomaly, FREE or ~$0.02/call)

Full LLM call for comprehensive system diagnosis.

**Default Model**: Gemini 3 Flash (FREE within limits) or Gemini 3 Pro ($2.00/$8.00)
**Premium Option**: Claude Sonnet 4 ($3.00/$15.00) with Agent SDK tools

**Triggers**:
- Tier 1 returns "investigate", "urgent", or "critical"
- User explicitly requests diagnosis via chat
- Critical threshold exceeded (skip Tier 1)

**Capabilities**:
- Run diagnostic commands (`vm_stat`, `top`, `ps`, `sample`)
- Read process info and logs
- Analyze memory patterns
- Identify root cause
- Suggest specific remediation steps

**Prompt Template** (~2000 tokens):
```
You are System Sentinel, a macOS system guardian.

Current state:
{detailed_metrics}

Historical trend (last 10 min):
{memory_history}

Top processes by memory:
{top_processes_detailed}

Recent alerts:
{recent_alerts}

Task: Diagnose the root cause and suggest actions.
Available actions: [notify_user, suggest_action, recommend_kill_process, request_reboot]
```

**Cost**: FREE (Gemini free tier) or ~$0.02/call (Gemini 3 Pro paid)

### Tier 3: Autonomous Action (Critical Only, With Safeguards)

Take protective action without waiting for user.

**Triggers**:
- Tier 2 recommends action with high confidence
- System is in imminent danger (memory > 95%, load > 100)
- Specific runaway process identified

**Actions** (with safeguards):
1. **Kill runaway process** - Only if:
   - Process is NOT in protected list (IDE, browser tabs with unsaved work)
   - Memory usage is clearly anomalous (>10GB and growing)
   - Tier 2 confirms it's safe to kill
   - User has not interacted with process in last 5 min

2. **Force garbage collection** - Safe operations:
   - Clear browser caches
   - Purge system caches (`sudo purge`)
   - Restart non-critical services

3. **Emergency notification** - Always:
   - Log all actions taken
   - Send notification explaining what was done
   - Offer to undo if possible

**Safeguards**:
- **Protected process list**: Terminal, IDE, browser (configurable)
- **Confirmation for destructive actions**: Unless system is in true crisis
- **Rate limiting**: Max 1 autonomous action per 5 minutes
- **Audit log**: Every action logged with full context

**Cost**: Same as Tier 2 (part of same call)

---

## Token Budget Management

### Provider Economics Comparison (Jan 2026)

| Model | Input $/1M | Output $/1M | Context | Free Tier |
|-------|-----------|-------------|---------|-----------|
| Gemini 2.0 Flash Lite | $0.08 | $0.30 | 1M | Yes |
| **Gemini 3 Flash** | $0.50 | $3.00 | 1M | **Yes (recommended)** |
| Gemini 3 Pro | $2.00 | $8.00 | 1M | Limited |
| Claude Haiku | $0.25 | $1.25 | 200K | No |
| Claude Sonnet 4 | $3.00 | $15.00 | 200K | No |
| Claude Opus 4.5 | $5.00 | $25.00 | 200K | No |

### Gemini Free Tier (Default for System Sentinel)

- **1,000 requests/day** (RPD)
- **250,000 tokens/minute** (TPM)
- **5-15 requests/minute** (RPM)
- **No credit card required**
- **1M token context window**

### Estimated Daily Usage

With Gemini 3 Flash (free tier):
- Tier 0: Unlimited (free, Rust-native)
- Tier 1: ~50 calls/day × ~550 tokens = ~27K tokens → **FREE**
- Tier 2: ~5 calls/day × ~2,500 tokens = ~12.5K tokens → **FREE**
- Chat queries: ~10/day × ~1,000 tokens = ~10K tokens → **FREE**

**Total under normal usage: $0.00/day** ✅

Fallback to paid tier only when:
- Free tier rate limits exceeded
- User explicitly requests Claude Agent SDK
- Premium diagnosis required

### Budget Tracking

```toml
[budget]
# Only tracks paid API usage (Gemini free tier doesn't count)
daily_limit_cents = 50
warn_at_percent = 80
pause_at_percent = 100
reset_hour_utc = 8        # Reset at 8 AM UTC
prefer_free_tier = true   # Always try Gemini free tier first
```

---

## Menu Bar UI (Tauri)

### Technology Stack

- **Framework**: Tauri 2.0 (Rust backend + WebView frontend)
- **UI**: HTML/CSS/JS (or React/Svelte for more structure)
- **Tray**: `tauri-plugin-system-tray` or direct `tray-icon` crate
- **IPC**: Unix socket between metrics daemon and Tauri app

### System Tray Icon States

| State | Icon | Meaning |
|-------|------|---------|
| Healthy | 🟢 | All metrics normal |
| Elevated | 🟡 | Warning threshold exceeded |
| Critical | 🔴 | Critical threshold or active investigation |
| AI Active | 🔵 | Claude is analyzing/responding |

### Dropdown Panel Design

```
┌────────────────────────────────────────────┐
│ System Sentinel                    ⚙️ ✕   │
├────────────────────────────────────────────┤
│ Memory   ████████░░ 78%   8.1/10.4 GB     │
│ Swap     ██░░░░░░░░ 15%   0.3/2.0 GB      │
│ Load     3.2 (1m) / 2.8 (5m) / 2.4 (15m)  │
├────────────────────────────────────────────┤
│ Recent Alerts                              │
│ • 2:34 PM - Memory spike (resolved)        │
│ • 1:15 PM - Ghostty at 4.2GB (watching)    │
├────────────────────────────────────────────┤
│ ┌────────────────────────────────────────┐ │
│ │ Ask about your system...              │ │
│ └────────────────────────────────────────┘ │
│                                     Send → │
├────────────────────────────────────────────┤
│ Today: $0.12 / $0.50 budget               │
└────────────────────────────────────────────┘
```

### Chat Interface

Simple text input that sends queries to Claude Agent SDK.

**Example Interactions**:
- "Why is Ghostty using so much memory?"
- "Is it safe to kill this process?"
- "What's causing the high load?"
- "Run full system diagnosis"

---

## Intelligence Backend: Gemini CLI (Primary) + Claude Agent SDK (Premium)

### Why Gemini CLI?

| Feature | Gemini CLI | Claude Agent SDK |
|---------|------------|------------------|
| **Free tier** | 1,000/day, 60 RPM | None |
| **Context** | 1M tokens | 200K tokens |
| **Tool use** | Built-in ReAct loop | Built-in tools |
| **Auth** | Google account (browser once) | API key required |
| **Shell integration** | Excellent | Python/TypeScript |

**Decision**: Use Gemini CLI as primary backend (free), Claude Agent SDK as premium fallback.

### Gemini CLI Integration (Shell-based)

The Rust daemon shells out to `gemini` CLI for intelligence:

```bash
# Tier 1: Quick classification
echo "System metrics: 85% RAM, 60% swap, load 15.2
Top processes: ghostty (8GB), Arc (2GB), node (1.5GB)
Memory growth: 3GB/hr over last 10 min

Classify: [normal_high_usage | investigate | urgent | critical]
Respond in JSON: {\"classification\": \"...\", \"confidence\": \"high|medium|low\", \"reason\": \"...\"}" | gemini --model gemini-2.0-flash

# Tier 2: Full diagnosis
gemini --model gemini-2.0-flash "You are System Sentinel. Diagnose this:
$(cat /tmp/sentinel_metrics.json)
Run vm_stat, ps aux, and analyze. Suggest actions."
```

### Rust Integration Code

```rust
// src/intelligence.rs
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub classification: String,  // normal_high_usage, investigate, urgent, critical
    pub confidence: String,      // high, medium, low
    pub reason: String,
}

pub fn classify_anomaly(metrics_json: &str) -> Result<ClassificationResult, anyhow::Error> {
    let prompt = format!(r#"
System metrics: {}

Classify urgency: [normal_high_usage | investigate | urgent | critical]
Respond ONLY with JSON: {{"classification": "...", "confidence": "high|medium|low", "reason": "..."}}
"#, metrics_json);

    let output = Command::new("gemini")
        .args(["--model", "gemini-2.0-flash", &prompt])
        .output()?;

    let response = String::from_utf8_lossy(&output.stdout);
    // Parse JSON from response
    let result: ClassificationResult = serde_json::from_str(&response)?;
    Ok(result)
}
```

### Premium Fallback: Claude Agent SDK

For complex diagnosis or when user requests, shell out to Python:

```python
# sentinel_claude.py - Only used for premium tier
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions

async def diagnose(metrics_json: str) -> dict:
    async for message in query(
        prompt=f"Diagnose system issue: {metrics_json}",
        options=ClaudeAgentOptions(
            allowed_tools=["Bash", "Read"],
            max_turns=3,
            model="claude-sonnet-4"
        )
    ):
        if hasattr(message, "result"):
            return {"diagnosis": message.result}
    return {}
```

### Custom MCP Server (Future)

System Sentinel can expose its own MCP server for richer tool integration:

```python
# sentinel_mcp_server.py
from claude_agent_sdk import tool, create_sdk_mcp_server

@tool("get_sentinel_metrics", "Get current system metrics from Sentinel", {})
async def get_sentinel_metrics(args):
    # Read from Sentinel's metrics store
    pass

@tool("get_memory_timeline", "Get memory history over N minutes", {"minutes": int})
async def get_memory_timeline(args):
    pass

@tool("suggest_kill", "Suggest killing a process", {"pid": int, "reason": str})
async def suggest_kill(args):
    pass
```

---

## Implementation Phases

### Phase 1: Foundation (COMPLETE ✅)
- [x] Project structure
- [x] Cargo.toml with dependencies
- [x] Metrics collection (sysinfo)
- [x] Anomaly detection (thresholds, rate-of-change)
- [x] Hammerspoon notification
- [x] LaunchAgent plist (`launchd/com.fredrikbranstrom.system-sentinel.plist`)
- [x] Default config file (`config/default.toml`)
- [x] Build release binary (`~/.local/bin/system-sentinel`)
- [x] Install config (`~/.config/system-sentinel/config.toml`)
- [x] Test run - **verified working** (detected swap at 84.2% from incident recovery)

**Status (2026-01-11 19:06)**: Tier 0 daemon ready. Not yet installed as LaunchAgent.
Next step: `launchctl load` the plist, then test notifications work.

### Phase 2: Local Intelligence Upgrade (COMPLETE ✅)
- [x] **Damping (Persistence)**: Alerts fire only after 3 consecutive breaches.
- [x] **Hysteresis (Sticky States)**: Recovery margins prevent alert flapping.
- [x] **Cross-Metric Correlation**: Mute growth alerts unless system is under pressure.
- [x] **Correlated Swap Alerting**: Suppress swap alerts if memory usage is low.
- [x] **Auditory Awareness**: Integration with `tts_daemon` for spoken alerts.
- [x] **Sound Hints**: Warning vs Critical sound effects.

### Phase 2.5: Intelligence Polish (Next)
- [ ] **Bundle Name Resolution**: Resolve generic names (Electron, java) to human names (Code, PyCharm).
- [ ] **Dynamic Thresholding**: Store baseline metrics to auto-adjust thresholds.

### Phase 3: Menu Bar App (Tauri 2.0) - COMPLETE ✅
- [x] Tauri project setup and IPC infrastructure (Unix socket `/tmp/system-sentinel.soc`)
- [x] System tray icon with health states (shield icons: green/amber/red)
- [x] Dashboard UI with Memory/Swap/Growth metrics and top processes
- [x] Click-to-toggle visibility on tray icon
- [x] Dynamic icon/tooltip updates based on HealthState

**Known Issue**: Tray icon renders as white rectangle instead of colored shield.
Research indicates need to call `set_icon_as_template(false)` after each `set_icon()` update.
See: `sentinel-ui/src-tauri/src/lib.rs:155` - fix deferred.

**Status (2026-01-16)**: UI functional, daemon→UI IPC working, icon color fix pending.

### Phase 4: Autonomous Actions
- [ ] Protected process whitelist.
- [ ] Kill process capability and safety confirmation flow.
- [ ] System-wide audit log of actions taken.

### Phase 5: AI Advisory Layer (Spec'd 2026-01-16)

**Spec Files**: `docs/specs/phase5-ai-advisory/`
- `requirements.md` - Functional and non-functional requirements
- `design.md` - Architecture, SDK selection, UI design
- `tasks.md` - 42 implementation tasks (~18.5h estimated, 1-2h with velocity)

**Mode**: Advisory only - AI suggests, human confirms all actions with risk.

**Decisions captured via /spec:**
| Question | Answer |
|----------|--------|
| Trigger | Threshold-based OR user-initiated |
| Privacy | Full context (trust Claude) |
| Surface | Menu bar popover chat UI (CleanMyMac-style) |
| Cost | Route through Claude Code session (no separate API cost) |
| Suggestions | AI asks permission for any risky actions |
| Backend | Claude Agent SDK / ACP under the hood |
| Latency | Async: notify when ready (TTS/notification/icon animation) |

**UI Vision**: Native-feeling macOS menu bar popover that auto-hides on focus loss.
Self-contained chat interface, not requiring separate CC window for simple queries.

**Reference screenshots** (CleanMyMac menu bar UI for inspiration):
- `/Users/fredrikbranstrom/Screenshots/Screenshot 2026-01-16 at 14.40.16.png`
- `/Users/fredrikbranstrom/Screenshots/Screenshot 2026-01-16 at 14.38.29.png`

**Complements CleanMyMac**: CMM license owned but not proactive/alerting enough.
System Sentinel fills the gap with intelligent anomaly detection + AI diagnosis.

**Research completed (2026-01-16)**:
- [x] macOS menu bar popover: **`tauri-nspopover-plugin`** (135★, MIT)
  - https://github.com/freethinkel/tauri-nspopover-plugin
  - Native NSPopover with auto-hide on focus loss
  - YouTube tutorial: "Mastering Menu Bar Apps: Using Rust and Tauri for macOS"
- [x] Claude Agent SDK / ACP integration options:

  | Solution | Stars | Approach | CLAUDE.md/MCP? |
  |----------|-------|----------|----------------|
  | **claude-agent-sdk** (crates.io) | - | CLI wrapper | ✅ Yes - spawns `claude` CLI |
  | **ACP Rust SDK** | 36★ | Protocol | ⚠️ Depends on impl |
  | **zed-industries/claude-code-acp** | 744★ | TypeScript | ✅ Yes (needs bridge) |
  | Direct API | - | Raw HTTP | ❌ No context |

  **Recommendation: `claude-agent-sdk` crate** because:
  - Pure Rust (native Tauri)
  - Spawns CC CLI → inherits global CLAUDE.md + MCP servers
  - Hook system for permission callbacks
  - No separate API cost

  **Critical requirement**: Agent must inherit user's full CC environment:
  - `~/.claude/CLAUDE.md` (global rules, system context)
  - `~/.claude.json` MCP servers (Graphiti, PAL, Exa, etc.)
  - Project CLAUDE.md when in project context

  CLI wrapper approach (claude-agent-sdk) satisfies this by spawning actual `claude` process.
  Direct API approaches would lose all this context.

- [x] Streaming responses in popover UI (Tauri events + SDK streaming)

**Implementation tasks**:
- [ ] Popover UI with chat interface
- [ ] SDK/ACP integration for Claude queries
- [ ] Context packaging (metrics, processes, history)
- [ ] Async response handling with notification options
- [ ] Permission flow for risky action suggestions

### Phase 6: Polish & Integration
- [ ] Graphiti integration (store incidents)
- [ ] GPU/Neural Engine monitoring
- [ ] User preference learning
- [ ] Cross-session pattern recognition

### Phase 7: Intelligent Automation (Future Vision)
- [ ] **Auto-GitHub Issue Submission**: When detecting patterns matching known bugs
  - Detect Ghostty memory leak pattern → draft GitHub issue with diagnostic data
  - Include system specs, memory timeline, repro conditions
  - User confirms before submission
- [ ] **Crash Auto-Recovery**: Automatically respond to application crashes
  - Detect crash via log monitoring or process exit
  - Collect crash report, logs, system state
  - Optionally restart crashed apps
  - Generate post-mortem
- [ ] **Pattern Learning**: Train on local incident history
  - Correlate process behavior with eventual issues
  - Learn "normal" baselines per-application
  - Predict issues before they become critical
- [ ] **Cross-App Intelligence**: Connect incidents across software
  - "Every time Arc + Claude Code + Ghostty run together, memory pressure rises"
  - Suggest workflow optimizations

---

## Configuration (Expanded)

Location: `~/.config/system-sentinel/config.toml`

```toml
[general]
check_interval_seconds = 30
log_file = "~/.local/share/system-sentinel/sentinel.log"

[thresholds]
memory_warning = 80
memory_critical = 90
swap_warning = 50
swap_critical = 75
load_warning = 10.0
load_critical = 50.0
memory_growth_rate_warning = 2.0
memory_growth_rate_critical = 5.0

[detection]
process_watchlist = ["ghostty", "Arc", "node", "Electron", "Claude"]
process_memory_threshold_mb = 2000
notification_cooldown_minutes = 10

[notification]
use_hammerspoon = true
fallback_to_terminal_notifier = true
warning_color = "#FFA500"
critical_color = "#FF4444"

[intelligence]
enabled = true
# Primary provider (free tier available)
primary_provider = "gemini"
tier1_model = "gemini-3-flash"
tier2_model = "gemini-3-flash"      # or "gemini-3-pro" for complex cases

# Premium fallback (for Agent SDK tools or user preference)
premium_provider = "claude"
premium_model = "claude-sonnet-4"

# Skip Tier 1 classification on critical alerts
skip_tier1_on_critical = true

[budget]
# Only tracks PAID API usage (Gemini free tier not counted)
daily_limit_cents = 50
warn_at_percent = 80
pause_at_percent = 100
prefer_free_tier = true             # Always try Gemini free tier first

[safety]
# Processes that should NEVER be killed automatically
protected_processes = ["Terminal", "Ghostty", "Code", "Zed", "Safari", "Arc", "Claude"]
# Require confirmation before any destructive action
require_confirmation = true
# Override confirmation only in true crisis (memory > 95%, unresponsive)
crisis_override_threshold_memory = 95
crisis_override_threshold_load = 100
# Max autonomous actions per hour
max_autonomous_actions_per_hour = 3

[ui]
show_menu_bar_icon = true
dropdown_width = 400
dropdown_height = 500
```

---

## Security Considerations

1. **Process Kill Safety**
   - Never kill processes with unsaved data without confirmation
   - Protected list is user-configurable
   - Crisis override has very high thresholds

2. **API Key Storage**
   - Use macOS Keychain for Anthropic API key
   - Never log API key

3. **Audit Trail**
   - Every autonomous action logged with full context
   - Logs stored locally, optionally synced to Graphiti

4. **Rate Limiting**
   - Budget caps prevent runaway API costs
   - Action rate limiting prevents thrashing

---

## Related Documents

- **Post-Mortem**: `~/Documents/ghostty_memory_incident_postmortem_2026-01-11.md`
- **Original Proposal**: `docs/system_sentinel_ai_watcher_proposal.md` (moved from ~/Documents)
- **Claude Agent SDK**: https://platform.claude.com/docs/en/agent-sdk/overview
- **Tauri System Tray**: https://tauri.app/learn/system-tray

---

*Plan v2.0 created 2026-01-11 - Expanded from notification-only to AI-powered guardian*
