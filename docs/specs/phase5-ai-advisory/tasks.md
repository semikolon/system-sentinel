# Phase 5: AI Advisory Layer - Implementation Tasks

**Version**: 1.0
**Created**: 2026-01-16
**Status**: Ready for Implementation

---

## Task Groups

### TG-1: NSPopover Integration (Priority: High)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T1.1 | Add `tauri-nspopover-plugin` dependency | 15m | None |
| T1.2 | Configure plugin in tauri.conf.json | 15m | T1.1 |
| T1.3 | Replace window toggle with popover toggle | 30m | T1.2 |
| T1.4 | Test popover opens/closes on tray click | 15m | T1.3 |
| T1.5 | Verify auto-hide on focus loss | 15m | T1.4 |

**Deliverable**: Native macOS popover opens from tray icon

---

### TG-2: Chat UI Implementation (Priority: High)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T2.1 | Design chat interface HTML/CSS | 1h | T1.5 |
| T2.2 | Add message history component | 30m | T2.1 |
| T2.3 | Add input field with send button | 30m | T2.1 |
| T2.4 | Implement message state management | 30m | T2.2, T2.3 |
| T2.5 | Style user vs AI messages | 30m | T2.4 |
| T2.6 | Add "thinking" indicator | 15m | T2.4 |
| T2.7 | Add scroll-to-bottom behavior | 15m | T2.5 |

**Deliverable**: Functional chat interface in popover

---

### TG-3: Claude Agent Bridge (Priority: High)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T3.1 | Add `claude-agent-sdk` crate dependency | 15m | None |
| T3.2 | Create ClaudeAgent struct with spawn logic | 1h | T3.1 |
| T3.3 | Implement query submission method | 30m | T3.2 |
| T3.4 | Implement stdout streaming/parsing | 1h | T3.3 |
| T3.5 | Create Tauri command: `submit_query` | 30m | T3.4 |
| T3.6 | Create Tauri command: `get_response` | 30m | T3.5 |
| T3.7 | Test basic query→response flow | 30m | T3.6 |

**Deliverable**: Rust bridge that spawns claude CLI and returns responses

---

### TG-4: Context Packaging (Priority: Medium)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T4.1 | Define QueryContext struct | 15m | None |
| T4.2 | Collect metrics from daemon IPC | 30m | T4.1 |
| T4.3 | Collect process list | 15m | T4.2 |
| T4.4 | Collect recent alerts | 15m | T4.2 |
| T4.5 | Build system prompt template | 30m | T4.1-T4.4 |
| T4.6 | Format prompt with context injection | 30m | T4.5 |

**Deliverable**: Queries sent with full system context

---

### TG-5: Action Suggestion System (Priority: Medium)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T5.1 | Define SuggestedAction enum | 15m | None |
| T5.2 | Define RiskLevel enum | 15m | T5.1 |
| T5.3 | Parse AI response for action suggestions | 1h | T5.2, T3.4 |
| T5.4 | Create action card UI component | 30m | T5.1 |
| T5.5 | Implement Confirm/Cancel buttons | 30m | T5.4 |
| T5.6 | Add typed confirmation for high-risk | 30m | T5.5 |
| T5.7 | Connect confirmation to action execution | 30m | T5.6 |

**Deliverable**: User can review and confirm suggested actions

---

### TG-6: Notification System (Priority: Medium)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T6.1 | Integrate with existing TTS daemon | 30m | T3.7 |
| T6.2 | Add macOS notification on response ready | 30m | T3.7 |
| T6.3 | Add tray icon pulse/badge animation | 30m | T3.7 |
| T6.4 | Make notification method configurable | 15m | T6.1-T6.3 |

**Deliverable**: User notified when AI response ready

---

### TG-7: Polish & Error Handling (Priority: Low)

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| T7.1 | Handle claude CLI not found | 15m | T3.2 |
| T7.2 | Add query timeout handling | 15m | T3.3 |
| T7.3 | Add response parse error handling | 15m | T3.4 |
| T7.4 | Add action audit logging | 30m | T5.7 |
| T7.5 | Add keyboard shortcuts (Cmd+Enter send) | 15m | T2.3 |
| T7.6 | Add compact metrics bar to popover | 30m | T2.1 |

**Deliverable**: Robust error handling and UX polish

---

## Implementation Order

```
Phase 5a: Core UI (TG-1 + TG-2)
    ↓
Phase 5b: AI Integration (TG-3 + TG-4)
    ↓
Phase 5c: Action Flow (TG-5)
    ↓
Phase 5d: Notifications (TG-6)
    ↓
Phase 5e: Polish (TG-7)
```

---

## Estimated Total

| Group | Tasks | Estimate |
|-------|-------|----------|
| TG-1: NSPopover | 5 | 1.5h |
| TG-2: Chat UI | 7 | 3.5h |
| TG-3: Claude Bridge | 7 | 4h |
| TG-4: Context | 6 | 2h |
| TG-5: Actions | 7 | 3.5h |
| TG-6: Notifications | 4 | 2h |
| TG-7: Polish | 6 | 2h |
| **Total** | **42** | **~18.5h** |

With 8-15x velocity multiplier: **1-2 hours actual implementation time**

---

## Verification Checklist

- [ ] Popover opens on tray icon click
- [ ] Popover auto-hides on focus loss
- [ ] User can type and send message
- [ ] AI response displays in chat
- [ ] Suggested actions show with risk level
- [ ] Confirmation required for risky actions
- [ ] Notification when response ready
- [ ] All actions logged to audit trail
- [ ] Works without separate API key
- [ ] Inherits CLAUDE.md and MCP servers
