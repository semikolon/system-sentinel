# Phase 5: AI Advisory Layer - Design

**Version**: 1.0
**Created**: 2026-01-16
**Status**: Spec Complete

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Sentinel UI (Tauri 2.0)                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────┐     ┌──────────────────────────────────┐  │
│  │   Tray Icon         │     │   NSPopover (native)             │  │
│  │   🟢 🟡 🔴          │────▶│                                  │  │
│  └─────────────────────┘     │  ┌────────────────────────────┐  │  │
│                              │  │ Metrics Summary            │  │  │
│                              │  │ Memory: 78% | Swap: 15%    │  │  │
│                              │  └────────────────────────────┘  │  │
│                              │                                  │  │
│                              │  ┌────────────────────────────┐  │  │
│                              │  │ Chat Interface             │  │  │
│                              │  │ ┌────────────────────────┐ │  │  │
│                              │  │ │ Message history        │ │  │  │
│                              │  │ └────────────────────────┘ │  │  │
│                              │  │ ┌────────────────────────┐ │  │  │
│                              │  │ │ Ask about system...    │ │  │  │
│                              │  │ └────────────────────────┘ │  │  │
│                              │  └────────────────────────────┘  │  │
│                              │                                  │  │
│                              │  ┌────────────────────────────┐  │  │
│                              │  │ Suggested Actions          │  │  │
│                              │  │ [Kill ghostty] ⚠️ [Confirm] │  │  │
│                              │  └────────────────────────────┘  │  │
│                              └──────────────────────────────────┘  │
│                                         │                          │
│                                         │ query                    │
│                                         ▼                          │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                  Claude Agent Bridge (Rust)                  │  │
│  │                                                              │  │
│  │  • Spawns `claude` CLI process                               │  │
│  │  • Inherits ~/.claude/CLAUDE.md                              │  │
│  │  • Inherits ~/.claude.json MCP servers                       │  │
│  │  • Streams responses via stdout                              │  │
│  │  • Hook callbacks for permission requests                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                         │                          │
│                                         │ IPC                      │
│                                         ▼                          │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                  Metrics Daemon (existing)                   │  │
│  │  • Provides current metrics                                   │  │
│  │  • Provides historical trends                                 │  │
│  │  • Provides process list                                      │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### Decision 1: SDK Selection

**Chosen**: `claude-agent-sdk` crate (CLI wrapper approach)

**Rationale**:
- Spawns actual `claude` CLI process → inherits full CC environment
- Pure Rust integration with Tauri
- No separate API key required
- Hook system for permission callbacks

**Rejected alternatives**:
- Direct API: No CLAUDE.md/MCP inheritance
- ACP TypeScript SDK: Would need Node.js bridge
- Gemini CLI: Doesn't support MCP integration

### Decision 2: UI Component

**Chosen**: `tauri-nspopover-plugin` for native macOS NSPopover

**Rationale**:
- Native macOS feel (auto-hide on focus loss)
- 135★ GitHub, actively maintained
- Direct Tauri 2.0 integration
- MIT license

**Reference implementation**:
- https://github.com/freethinkel/tauri-nspopover-plugin
- YouTube: "Mastering Menu Bar Apps: Using Rust and Tauri for macOS"

### Decision 3: Human-in-the-Loop Pattern

**Pattern**: Advisory mode with explicit confirmation

```rust
enum SuggestedAction {
    KillProcess { pid: u32, name: String, risk: RiskLevel },
    RestartService { name: String, risk: RiskLevel },
    ClearCache { target: String, risk: RiskLevel },
    Custom { description: String, command: String, risk: RiskLevel },
}

enum RiskLevel {
    Safe,      // Green - auto-executable with user awareness
    Moderate,  // Yellow - requires confirmation dialog
    High,      // Red - requires explicit typed confirmation
}
```

**Flow**:
1. AI suggests action with risk level
2. UI displays action with appropriate warning
3. User clicks Confirm/Reject
4. If High risk: user types confirmation text
5. Action executes only after confirmation

### Decision 4: Async Response Pattern

**Pattern**: Background processing with notification

```rust
// User submits query
let query_id = submit_query(prompt, context);
// UI shows "Thinking..." indicator
// Background: claude process runs
// On completion:
notify_user(query_id, NotificationMethod::All); // TTS + notification + icon pulse
```

**Notification methods**:
1. TTS daemon speaks summary (via existing integration)
2. macOS notification center
3. Tray icon animation/badge

---

## Data Flow

### Query Submission

```
User Input → Popover UI → Tauri Command → Claude Agent Bridge
                                              ↓
                                         Spawn `claude` CLI
                                              ↓
                                         Stream stdout
                                              ↓
                                         Parse response
                                              ↓
                                         Update UI state
                                              ↓
                                         Notify user
```

### Context Packaging

```rust
struct QueryContext {
    // Current state
    metrics: SystemMetrics,
    top_processes: Vec<ProcessInfo>,

    // Historical
    memory_trend_10m: Vec<f64>,
    recent_alerts: Vec<Alert>,

    // Environment
    hostname: String,
    os_version: String,
    uptime_hours: f64,
}

fn build_prompt(user_query: &str, context: &QueryContext) -> String {
    format!(r#"
You are System Sentinel, an AI advisor for macOS system health.

CURRENT STATE:
- Memory: {mem_pct}% ({mem_used}/{mem_total} GB)
- Swap: {swap_pct}%
- Load: {load_1m} / {load_5m} / {load_15m}

TOP PROCESSES:
{processes}

RECENT ALERTS:
{alerts}

USER QUESTION: {user_query}

Provide actionable advice. If suggesting process termination,
indicate the risk level and what data might be lost.
"#, ...)
}
```

---

## UI Components

### Popover Layout (320x480 default)

```
┌────────────────────────────────────────┐
│ System Sentinel            [─] [×]     │  ← Header with minimize/close
├────────────────────────────────────────┤
│ ████████░░ 78% Memory  (12.4/16 GB)    │  ← Compact metrics bar
│ ██░░░░░░░░ 15% Swap                    │
├────────────────────────────────────────┤
│                                        │
│ 🤖 What would you like to know?        │  ← Chat history (scrollable)
│                                        │
│ 👤 Why is Ghostty using 3GB?           │
│                                        │
│ 🤖 Ghostty is showing elevated memory  │
│    due to scrollback buffer. This is   │
│    expected with heavy terminal use.   │
│                                        │
│    Suggested action:                   │
│    ┌──────────────────────────────┐   │
│    │ ⚠️ Restart Ghostty          │   │
│    │ Risk: Moderate               │   │
│    │ [Cancel] [Confirm]           │   │
│    └──────────────────────────────┘   │
│                                        │
├────────────────────────────────────────┤
│ ┌────────────────────────────────────┐ │  ← Input field
│ │ Ask about your system...           │ │
│ └────────────────────────────────────┘ │
│                                  [→]   │  ← Send button
└────────────────────────────────────────┘
```

---

## Error Handling

| Error | Handling |
|-------|----------|
| Claude CLI not found | Show setup instructions, disable AI features |
| Query timeout (>60s) | Cancel with "Claude is taking longer than expected" |
| Parse error | Show raw response, log for debugging |
| Permission denied | Show action blocked, explain why |

---

## Security Considerations

1. **Process termination**: Always require confirmation, log action
2. **Command execution**: Never execute arbitrary commands from AI
3. **Allowed actions**: Whitelist of safe operations only
4. **Audit trail**: Every action logged with timestamp, context, and user confirmation

---

---

## Streaming Response Pattern (Researched 2026-01-16)

### Tauri + Async Stdout Streaming

Pattern from tauri-apps examples and Stack Overflow:

```rust
// Rust backend (lib.rs)
use tokio::io::{AsyncBufReadExt, BufReader};
use tauri::Emitter;

#[tauri::command]
async fn submit_query(app: tauri::AppHandle, prompt: String) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        // Spawn claude CLI process
        let mut child = tokio::process::Command::new("claude")
            .args(["--print", &prompt])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Stream each line to frontend
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("claude-stream", &line);
        }

        // Signal completion
        let _ = app.emit("claude-done", ());
        Ok::<(), String>(())
    });

    Ok(())
}
```

```javascript
// Frontend (chat.js)
import { listen } from '@tauri-apps/api/event';

let responseText = '';

await listen('claude-stream', (event) => {
    responseText += event.payload;
    updateChatUI(responseText);
});

await listen('claude-done', () => {
    finalizeResponse();
});
```

### Claude Agent SDK Streaming

Both `claude-agent-sdk` and `claude-agents-sdk` crates support async streaming:

```rust
use claude_agent_sdk::{query, Message};
use futures::StreamExt;

let stream = query(&prompt, Some(options)).await?;
let mut stream = Box::pin(stream);

while let Some(message) = stream.next().await {
    match message? {
        Message::Assistant(msg) => {
            app.emit("claude-stream", msg.text())?;
        }
        Message::Result(result) => {
            app.emit("claude-done", result)?;
        }
        _ => {}
    }
}
```

### UI Update Strategy

1. **Optimistic UI**: Show "thinking" state immediately
2. **Incremental rendering**: Append text as tokens arrive
3. **Markdown parsing**: Render markdown after complete sentence/paragraph
4. **Auto-scroll**: Keep latest content visible
5. **Action parsing**: Detect structured action suggestions in stream

---

## Future Extensions

- **Streaming responses**: ✅ Researched - use Tauri events + SDK streaming
- **Action history**: View past actions and their outcomes
- **Quick actions**: Pre-defined queries ("Why is memory high?", "Full diagnosis")
- **Graphiti integration**: Store incidents for pattern learning
