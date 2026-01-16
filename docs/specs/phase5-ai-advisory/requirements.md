# Phase 5: AI Advisory Layer - Requirements

**Version**: 1.0
**Created**: 2026-01-16
**Status**: Spec Complete

---

## Functional Requirements

### FR-1: Trigger Mechanisms
- **FR-1.1**: System SHALL trigger AI advisory on threshold-based anomalies (memory/swap/load alerts)
- **FR-1.2**: System SHALL allow user-initiated queries via chat interface
- **FR-1.3**: System SHALL support both automatic and manual trigger modes

### FR-2: Privacy & Context
- **FR-2.1**: System SHALL send full system context to Claude (no data filtering)
- **FR-2.2**: System SHALL include current metrics, process list, and historical trends
- **FR-2.3**: System SHALL leverage existing CC CLAUDE.md rules and MCP servers

### FR-3: User Interface
- **FR-3.1**: System SHALL display advisory interface as native macOS menu bar popover
- **FR-3.2**: Popover SHALL auto-hide when user clicks outside (focus loss)
- **FR-3.3**: System SHALL support chat-style conversation within popover
- **FR-3.4**: System SHALL display AI response with clear formatting

### FR-4: Advisory Mode (Human-in-the-Loop)
- **FR-4.1**: AI SHALL suggest actions, never execute automatically
- **FR-4.2**: All risky actions SHALL require explicit user confirmation
- **FR-4.3**: System SHALL display clear risk indicators for destructive actions
- **FR-4.4**: User SHALL be able to approve, reject, or modify suggested actions

### FR-5: Response Handling
- **FR-5.1**: System SHALL handle async AI responses (no UI blocking)
- **FR-5.2**: System SHALL notify user when AI response is ready (TTS/notification/icon animation)
- **FR-5.3**: System SHALL display streaming responses when available

### FR-6: Cost Management
- **FR-6.1**: AI queries SHALL route through Claude Code session (no separate API cost)
- **FR-6.2**: System SHALL inherit user's existing CC environment and MCP tools
- **FR-6.3**: System SHALL NOT require separate Anthropic API key

---

## Non-Functional Requirements

### NFR-1: Performance
- **NFR-1.1**: Popover SHALL open within 200ms of tray icon click
- **NFR-1.2**: AI response latency is acceptable (async notification pattern)
- **NFR-1.3**: Background processing SHALL NOT impact system metrics collection

### NFR-2: Usability
- **NFR-2.1**: UI SHALL feel native to macOS (NSPopover behavior)
- **NFR-2.2**: Chat interface SHALL be familiar/intuitive
- **NFR-2.3**: System SHALL complement (not replace) CleanMyMac functionality

### NFR-3: Security
- **NFR-3.1**: No process kill operations without user confirmation
- **NFR-3.2**: Protected process list SHALL be respected
- **NFR-3.3**: All suggested actions SHALL be logged

### NFR-4: Integration
- **NFR-4.1**: SHALL inherit ~/.claude/CLAUDE.md global rules
- **NFR-4.2**: SHALL have access to user's configured MCP servers
- **NFR-4.3**: SHALL integrate with existing Tauri menu bar infrastructure

---

## Design Reference

- CleanMyMac menu bar UI: `/Users/fredrikbranstrom/Screenshots/Screenshot 2026-01-16 at 14.40.16.png`
- CleanMyMac expanded view: `/Users/fredrikbranstrom/Screenshots/Screenshot 2026-01-16 at 14.38.29.png`

---

## Acceptance Criteria

1. User can click tray icon to open popover with chat interface
2. User can type question and receive AI response
3. AI response includes actionable suggestions with risk indicators
4. User must confirm any destructive action before execution
5. System uses existing CC session (no API key configuration)
6. Popover auto-hides on focus loss
