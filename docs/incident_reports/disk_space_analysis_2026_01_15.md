# Deep Disk Space Analysis - Mac Mini M2

**Date**: 2026-01-15
**Boot Disk**: 228GB total, 178GB used (78%), 16GB free after reboot
**Triggered by**: Docker crash due to disk exhaustion

## Executive Summary

The 228GB boot disk is chronically near capacity due to:
1. **Development artifacts** (~8GB reclaimable): Rust target folders, node_modules
2. **Application caches** (~6GB reclaimable): Spotify, Playwright, browsers
3. **Claude Code data** (~2GB reclaimable): Debug logs, hook caches
4. **Docker** (11GB): Mostly Graphiti data, not easily reclaimable
5. **Application Support** (19GB): Chrome, BeeperTexts, Zed, Discord

**Quick wins can reclaim ~12-15GB** with minimal risk.

---

## Detailed Breakdown

### 1. Docker (11GB physical)

```
Docker.raw logical size: 228GB (sparse file illusion)
Docker.raw physical size: 11GB (actual disk usage)

TYPE            SIZE        RECLAIMABLE
Images          1.182GB     165.9kB (0%)
Containers      90.11kB     20.48kB (22%)
Local Volumes   9.859GB     3.756kB (0%)
Build Cache     0B          0B
```

**Note**: Docker.raw is an APFS sparse file. The 228GB "logical size" is NOT actual disk consumption. Physical size is ~11GB.

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
sentinel-ui/src-tauri/target: 3.9GB ✅ Safe to delete
system-sentinel/target: 617MB ✅ Safe to delete
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

### Tier 1: Safe, Immediate (~8-10GB)

```bash
# Rust build artifacts (will rebuild on next compile)
rm -rf ~/Projects/system-sentinel/sentinel-ui/src-tauri/target
rm -rf ~/Projects/system-sentinel/target

# Caches (apps will recreate as needed)
rm -rf ~/Library/Caches/com.spotify.client
rm -rf ~/Library/Caches/ms-playwright
rm -rf ~/Library/Caches/Arc

# Claude Code debug/cache
rm -rf ~/.claude/debug/*
rm -rf ~/.claude/hooks/cache
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

1. **Set Docker disk limit**: Docker Desktop → Settings → Resources → Disk image size → 32GB max
2. **Periodic cache clearing**: Monthly clear browser/app caches
3. **Clean Rust targets**: Add `cargo clean` to project maintenance routine
4. **Archive old projects**: Move to external drive or delete node_modules in archived projects
5. **Consider larger boot disk**: 228GB is tight for active development work

---

## Post-Analysis Status

After this analysis, the user can reclaim **12-15GB immediately** with safe commands above, bringing free space from 16GB to ~28-31GB (87% → 78% utilization).

For sustainable disk management, consider the 228GB constraint when deciding which development tools and apps to keep active.
