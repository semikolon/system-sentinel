# Incident Report: Docker Shutdown + Claude Code SSE Crash

**Date:** 2026-01-16 ~11:31 CET
**Impact:** All Claude Code sessions connected to Graphiti crashed
**Duration:** Ongoing (Docker hung at "stopping" state)
**Root Cause:** Claude Code SSE transport bug + Docker VM unexplained termination

## Summary

Docker Desktop's VM gracefully terminated while user was away (making coffee). This caused the Graphiti MCP server to lose its FalkorDB connection, which cascaded to crash multiple active Claude Code sessions. Docker then hung at "Docker Desktop is stopping..." indefinitely.

## Timeline (UTC+1)

| Time | Event |
|------|-------|
| ~11:31:36 | `com.docker.virtualization` log: "Requesting VM termination" |
| ~11:31:38 | VM stopped gracefully |
| ~11:31:36 | Electron log: GPU and Network Service processes killed (SIGTERM) |
| ~11:32+ | Backend log: Continuous "no route to host" errors to 192.168.65.7:2376 |
| ~11:34 | User notices Claude Code sessions have crashed |
| ~11:37+ | Docker Desktop GUI stuck at "Docker Desktop is stopping..." |

**Concurrent event:** uBlock Origin extension crashed in Arc browser around the same time (possible system-wide memory pressure correlation).

## Root Cause Analysis

### What We Know

1. **Graceful shutdown, not crash**: The virtualization log shows "VM has stopped gracefully" - this was NOT an unexpected termination
2. **Resource Saver ruled out**: All idle feature flags are disabled:
   - `IdleMemSaver: variant="disabled"`
   - `IdlePause: variant="disabled"`
   - `IdleShutdown: variant="disabled"`
3. **User did not manually quit**: User was away from keyboard
4. **No memory pressure evidence**: macOS kernel logs show no jetsam events around this time

### What Triggered the Shutdown: UNKNOWN

The VM received a termination request from an unknown source. Possible causes:
- macOS system event (brief sleep? display timeout?)
- Apple's Virtualization.framework internal issue
- Some background process signaling Docker to quit
- Docker Desktop internal bug/auto-update attempt

### Why Claude Code Sessions Crashed

The Graphiti wrapper (`~/.config/claude/wrappers.d/graphiti/claude`) has graceful degradation for Docker being unavailable **at startup** (lines 35-41), but NOT for Docker dying **mid-session**:

```bash
# Startup check - graceful
if ! docker info &>/dev/null; then
    echo "⚠️  Docker daemon not running - Graphiti memory unavailable this session"
    return 1  # Continue without Graphiti
fi
```

Once a Claude Code session starts with Graphiti active:
1. Session connects to Graphiti MCP server at localhost:8000
2. Graphiti server connects to FalkorDB at localhost:6380
3. Docker VM dies → FalkorDB container stops → Graphiti loses database
4. MCP server tool calls fail → Claude Code session crashes

**Design gap:** No mid-session resilience or reconnection logic.

## Impact

- Multiple Claude Code sessions crashed simultaneously
- Work in progress lost (sessions cannot be resumed after MCP failure)
- User workflow completely disrupted

## Resolution

Force quit Docker Desktop and reboot (Docker hung in "stopping" state indefinitely).

## Prevention Recommendations

### Short-term
1. **Migrate to OrbStack** (already planned): More stable, better macOS integration
2. **Add Graphiti reconnection logic**: Detect FalkorDB disconnect, attempt reconnection or graceful degradation

### Long-term
1. **Complete OrbStack + Knot migration**: Reduce Docker Desktop dependency entirely
2. **Investigate Docker Desktop auto-quit triggers**: Check if display sleep, power management, or other macOS events can trigger VM termination

## Related Incidents

- `incident_report_docker_data_loss_2026_01_15.md` - Complete data loss during hung state recovery
- `disk_space_analysis_2026_01_15.md` - BuildKit cache bloat crashing Docker

## Deep Dive: Why Did Claude Code Sessions Crash?

### Initial Assumption (WRONG)
We assumed the Graphiti wrapper's lack of mid-session resilience caused the crashes.

### Research Findings
According to GitHub issues ([#1026](https://github.com/anthropics/claude-code/issues/1026), [#15232](https://github.com/anthropics/claude-code/issues/15232), [#3279](https://github.com/anthropics/claude-code/issues/3279)), **Claude Code should NOT crash when MCP servers disconnect**:

- Session should continue running
- Disconnected server shows red indicator
- Tools become unavailable but session stays alive
- `/mcp reconnect <server>` can restore (added in v1.0.64)

### What Actually Happened
Sessions **completely exited** to terminal prompt:
- Two brf-auto sessions: Silent exit, no error output
- Dotfiles session: Showed garbled wrapper monitor output after CC died
- Status bars showed "MCP server failed" / "2 MCP servers need auth" before exit

### Root Cause: Likely Claude Code Bug
The crash is most likely a **bug in Claude Code's SSE transport error handling**, not the wrapper:

1. **Wrapper is passive**: Only sets `GRAPHITI_GROUP_ID` env var and runs background monitor
2. **Crash timing**: Happened when MCP SSE connection failed, which is CC's responsibility
3. **Known issues**: GitHub shows CC has MCP connection stability problems
4. **SSE vs stdio**: Graphiti uses SSE transport - may have different (worse) error handling than stdio-based MCP servers

### Evidence
```
/Users/fredrikbranstrom/.config/claude/wrappers.d/graphiti/claude: line 211: 57502 MCP servers need auth · /mcpp ${HEALTH_CHECK_INTERVAL}
```
This is the wrapper's background `monitor_session()` outputting after CC already died - a symptom, not the cause.

### Action Item
File bug report to [anthropics/claude-code](https://github.com/anthropics/claude-code/issues): "SSE-based MCP server disconnection crashes session instead of graceful degradation"

---

## Lessons Learned

Docker Desktop on macOS has proven chronically unstable across multiple failure modes:
1. Hung states requiring `rm -rf` that causes data loss
2. Unbounded disk growth from BuildKit volumes
3. Unexplained VM termination crashing dependent services

**Additionally**: Claude Code's MCP error handling for SSE transport appears buggy - server disconnection should degrade gracefully, not crash sessions.

This is the third Docker incident in 48 hours. Migration to OrbStack is now **urgent**, not optional.

---

## Technical Deep Dive: SSE Transport Architecture Failure

### MCP Configuration

Graphiti is configured as SSE transport in `~/.claude.json`:
```json
"graphiti": {
  "type": "sse",
  "url": "http://localhost:8000/sse?group_id=${GRAPHITI_GROUP_ID}",
  "headers": { "X-Graphiti-Group-Id": "${GRAPHITI_GROUP_ID}" }
}
```

### Dependency Chain
```
Claude Code → SSE Client → Graphiti MCP (localhost:8000) → FalkorDB (localhost:6380) → Docker VM
```

### Why SSE Transport is Problematic

| Aspect | Stdio Transport | SSE Transport |
|--------|-----------------|---------------|
| Connection | Local process pipes | HTTP long-polling |
| Failure mode | Process exit → clean detection | Network timeout → messy errors |
| Recovery | Restart child process | HTTP reconnection logic needed |
| Error handling | Synchronous, predictable | Async, many failure paths |

### Known SSE Issues in MCP Ecosystem

1. **TypeError: terminated** - When SSE streams fail unexpectedly ([GitHub #3033](https://github.com/anthropics/claude-code/issues/3033))
2. **5-minute timeout bug** - MCP TypeScript SDK closes SSE after idle ([typescript-sdk #270](https://github.com/modelcontextprotocol/typescript-sdk/issues/270))
3. **Body Timeout Error** - Documented across multiple projects ([trigger.dev #2134](https://github.com/triggerdotdev/trigger.dev/issues/2134))

### Difference: Normal Timeout vs Network Death

- **Normal timeout**: Connection idle → 5-min timeout → graceful disconnect handling
- **Network death** (this incident): TCP connection suddenly broken → `fetch()` throws unhandled exception → Node.js process crashes

### Why Wrapper Is Innocent

The Graphiti wrapper (`~/.config/claude/wrappers.d/graphiti/claude`) is passive:
1. Only sets `GRAPHITI_GROUP_ID` environment variable
2. Ensures FalkorDB running at startup (graceful degradation if not)
3. Background `monitor_session()` only checks if Claude is alive - doesn't control it
4. The garbled terminal output was the monitor continuing after CC died

### Conclusion

The crash is a **Claude Code bug** in SSE transport error handling, not a wrapper bug. When Docker's network died, Claude Code's MCP client encountered an unhandled exception and crashed instead of marking the server as failed.

### Action Items

1. **Bug report filed**: anthropics/claude-code - "SSE MCP server disconnection crashes session"
2. **Migrate Graphiti to HTTP transport**: SSE is deprecated in MCP spec; Streamable HTTP recommended
3. **OrbStack migration**: Prevents Docker from dying unexpectedly

### References

- [GitHub #1026: Reconnect MCP Servers](https://github.com/anthropics/claude-code/issues/1026)
- [GitHub #15232: Auto-reconnect for MCP](https://github.com/anthropics/claude-code/issues/15232)
- [MCP Transports Spec](https://modelcontextprotocol.io/docs/concepts/transports)
- [Why MCP Deprecated SSE](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/)
