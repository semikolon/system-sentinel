# Deep Disk Space Analysis - Mac Mini M2

**Date**: 2026-01-15 (Updated 20:00 CET - Second Incident)
**Boot Disk**: 228GB total
**Status**: RECURRING CRISIS - Docker grew from 11GB to 27GB in ONE DAY

## Critical Finding: Docker.raw Growth Root Cause

**Docker.raw grew from 11GB → 27GB in ~4 hours** due to:
1. **BuildKit cache** from Kamal deployments (builds multi-arch images, caches ALL layers)
2. **Graphiti/FalkorDB** knowledge graph data accumulation

**Symptoms**: "com.docker.virtualization: process terminated unexpectedly: use of closed network connection" = disk exhaustion killing Docker VM.

## Solution: Kamal Cleanup Strategy

### Local Mac (BuildKit cache - the BIG one)
```bash
docker builder prune -f --keep-storage=5GB  # After each deploy
docker builder prune -af                     # Nuclear: remove ALL build cache
```

**⚠️ NEVER prune a hung daemon**: If `docker system df` times out or Docker is unresponsive, reboot first. Pruning against a degraded daemon risks corruption or inconsistent state. Always verify Docker is healthy (green icon, containers responsive) before cleanup commands.

### Remote Server (old containers/images)
```bash
kamal prune all  # Keeps last 5 deployments
```

### ✅ IMPLEMENTED: brf-auto Post-Deploy Hook
Both cleanups now run automatically after each Kamal deploy via `.kamal/hooks/post-deploy` (commit d2c90c9). Fails gracefully - cleanup errors won't block deploys.

### Alternative to Docker Desktop: OrbStack
- 0.8GB RAM (vs 2.4GB Docker Desktop)
- Automatic disk reclaim (Docker Desktop NEVER auto-shrinks)
- `brew install orbstack` - drop-in replacement

---

## Executive Summary

The 228GB boot disk is chronically near capacity due to:
1. **Docker** (27GB): BuildKit cache + Graphiti data - CLEAN BUILDKIT AFTER DEPLOYS
2. **Development artifacts** (~8GB reclaimable): Rust target folders, node_modules
3. **Application caches** (~6GB reclaimable): Spotify, Playwright, browsers
4. **Claude Code data** (~2GB reclaimable): Debug logs, hook caches
5. **Application Support** (19GB): Chrome, BeeperTexts, Zed, Discord

**Quick wins can reclaim ~12-15GB** with minimal risk.

---

## Detailed Breakdown

### 1. Docker (27GB after BuildKit bloat)

```
Docker.raw: APFS sparse file (logical size misleading)
Physical size: 11GB → 27GB (after Kamal deploys)

Breakdown (pre-bloat):
TYPE            SIZE        RECLAIMABLE
Images          1.182GB     165.9kB (0%)
Containers      90.11kB     20.48kB (22%)
Local Volumes   9.859GB     0B (Graphiti data!)
Build Cache     ~16GB       ~16GB (the culprit)
```

**Note**: Docker.raw is an APFS sparse file. Run `docker builder prune -af` after reboot to reclaim BuildKit cache.

**Warning**: Volumes contain FalkorDB/Graphiti data. Do NOT prune volumes.

---

### 2. ~/Library/Caches (8.5GB)

| Item | Size | Safe to Delete |
|------|------|----------------|
| com.spotify.client | 2.3GB | ✅ Yes |
| ms-playwright | 1.6GB | ✅ Yes |
| Arc | 1.2GB | ✅ Yes |
| claude-cli-nodejs | 911MB | ✅ Yes |
| company.thebrowser.Browser | 843MB | ✅ Yes |
| beepertexts-updater | 383MB | ✅ Yes |
| Homebrew | 264MB | ⚠️ Careful |
| pip | 252MB | ✅ Yes |
| SiriTTS | 222MB | ✅ Yes |
| com.openai.chat | 154MB | ✅ Yes |
| ms-playwright-go | 127MB | ✅ Yes |

**Quick clean command:**
```bash
rm -rf ~/Library/Caches/com.spotify.client
rm -rf ~/Library/Caches/ms-playwright
rm -rf ~/Library/Caches/Arc
rm -rf ~/Library/Caches/claude-cli-nodejs
rm -rf ~/Library/Caches/company.thebrowser.Browser
```

---

### 3. ~/Library/Application Support (19GB)

| Item | Size | Notes |
|------|------|-------|
| Google (Chrome) | 5.5GB | Profiles, extensions, history |
| BeeperTexts | 3.5GB | Message history, attachments |
| Zed | 2.3GB | Editor state, extensions |
| discord | 869MB | Chat history, cache |
| Code (VS Code) | 724MB | Extensions, workspace data |
| Spotify | 693MB | Offline music cache |
| Claude | 633MB | Desktop app data |
| Kegworks | 629MB | Unknown app |
| Arc | 622MB | Browser data |
| superwhisper | 525MB | Voice recordings? |
| Comet | 476MB | Unknown app |
| Dia | 369MB | Unknown app |
| 1Password | 358MB | Vault data |
| Signal | 357MB | Messages, attachments |

**Review candidates**: BeeperTexts (3.5GB), Discord (869MB) - may contain old attachments.

---

### 4. ~/dotfiles/current/.claude (6.1GB)

| Item | Size | Safe to Delete |
|------|------|----------------|
| projects/ | 4.1GB | ⚠️ Session history |
| debug/ | 909MB | ✅ Yes |
| hooks/ | 754MB | ✅ cache subfolder |
| logs/ | 199MB | ✅ Old logs |
| file-history/ | 101MB | ⚠️ Useful for undo |

**Projects breakdown:**
| Project | Size |
|---------|------|
| brf-auto | 2.7GB |
| kimonokittens | 797MB |
| dotfiles | 448MB |
| karabiner-mods | 73MB |

**Safe clean commands:**
```bash
rm -rf ~/.claude/debug/*
rm -rf ~/.claude/hooks/cache
rm -rf ~/.claude/logs/*.log  # Keep recent
```

---

### 5. ~/Library/Containers (12GB)

| Item | Size | Notes |
|------|------|-------|
| com.docker.docker | 11GB | Contains Docker.raw |
| com.apple.iChat | 252MB | Messages app data |
| com.apple.geod | 228MB | Location services |
| com.apple.mediaanalysisd | 189MB | Photo analysis |

**Docker dominates.** Other containers are minimal.

---

### 6. ~/Projects (17GB)

| Project | Size | Notes |
|---------|------|-------|
| system-sentinel | 4.5GB | 3.9GB is Tauri target |
| brf-auto | 3.0GB | Active project |
| Archived Code | 2.9GB | Old projects |
| kimonokittens | 2.1GB | Active project |
| Archived Projects | 952MB | Old projects |
| agent-loom | 812MB | Experimental |

**Rust target folders:**
```
sentinel-ui/src-tauri/target: 3.9GB ✅ DELETED 2026-01-15
system-sentinel/target: 617MB ✅ DELETED 2026-01-15
```

**node_modules found:**
```
cursor-chat-browser: 582MB
kimonokittens: 540MB
yoga-pose-simulation (archived): 471MB ✅ Safe to delete
brf-auto/frontend: 324MB
sinan/unitedpeople: 295MB
```

---

### 7. /Applications (13GB)

Standard macOS + development apps. Review for unused apps via:
```bash
du -sh /Applications/* | sort -hr | head -20
```

---

## Quick Recovery Commands

### Tier 1: Safe, Immediate

**Already done (2026-01-15):**
- ✅ Rust target folders (~4.5GB)
- ✅ Claude debug/hooks cache (~1.7GB)

**Still available:**
```bash
# Caches (apps will recreate as needed)
rm -rf ~/Library/Caches/com.spotify.client  # 2.3GB
rm -rf ~/Library/Caches/ms-playwright       # 1.6GB
rm -rf ~/Library/Caches/Arc                 # 1.2GB
```

### Tier 2: Review First (~4-5GB)

```bash
# Old archived node_modules
rm -rf ~/Projects/Archived\ Code/yoga-pose-simulation/node_modules

# Old Claude session data (check if needed first)
# ls -la ~/.claude/projects/
```

### Tier 3: Manual Review

- BeeperTexts (3.5GB) - export/archive old messages?
- Chrome data (5.5GB) - clear browsing data in Chrome settings
- Zed (2.3GB) - check for old workspace caches

---

## Long-term Recommendations

1. ✅ **Auto-clean Kamal deploys**: Implemented in brf-auto post-deploy hook
2. **Set Docker disk limit**: Docker Desktop → Settings → Resources → Disk image size → 32GB max
3. **Consider OrbStack**: Drop-in Docker Desktop replacement with auto disk reclaim
4. **Periodic cache clearing**: Monthly clear browser/app caches
5. **Clean Rust targets**: Add `cargo clean` to project maintenance routine
6. **Archive old projects**: Move to external drive or delete node_modules in archived projects

---

## Incident Recovery Timeline (2026-01-15)

### Trigger
~20:00 CET: Docker Desktop crashed with "com.docker.virtualization: process terminated unexpectedly: use of closed network connection". Boot disk at 6.6GB free (97% full).

### Investigation
1. Discovered Docker.raw grew from 11GB → 27GB in ~4 hours
2. Root cause: BuildKit cache accumulation from Kamal multi-arch builds
3. Docker daemon hung - `docker system df` unresponsive

### Recovery Steps Completed
| Step | Action | Space Reclaimed |
|------|--------|-----------------|
| 1 | Deleted Rust target folders (system-sentinel) | ~4.5GB |
| 2 | Deleted Claude debug + hooks cache | ~1.7GB |
| 3 | Implemented brf-auto post-deploy auto-cleanup | (prevention) |

**Disk free after cleanup**: 12GB (up from 6.6GB)

### Pending Recovery (After Reboot)
```bash
# 1. Reboot to restore Docker daemon to healthy state
# 2. Wait for Docker Desktop green icon + containers responsive
# 3. Then run:
docker builder prune -af   # Reclaims ~16GB BuildKit cache
```

**Expected final state**: ~25GB+ free, Docker.raw back to ~11GB

### Prevention Implemented
- brf-auto `.kamal/hooks/post-deploy` now auto-cleans after each deploy
- Future consideration: OrbStack migration for automatic disk reclaim

---

## Post-Analysis Status

For sustainable disk management, consider OrbStack migration (auto disk reclaim) or the 228GB constraint when deciding which development tools to keep active.
