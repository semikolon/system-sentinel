# Docker Infrastructure Migration Plan

> **Created**: 2026-01-15
> **Status**: Planned
> **Goal**: Eliminate Docker Desktop pain points while maintaining local development capability

## Executive Summary

Two-phase migration to improve Docker experience:

1. **Phase 1: OrbStack** - Replace Docker Desktop with lighter, auto-reclaiming alternative
2. **Phase 2: Knot** - Self-hosted registry on Dell, eliminating Mac Docker dependency for deploys

Both phases are independent - either can be done alone, but together they provide maximum flexibility.

---

## Background: Why This Migration?

### The Problem (2026-01-15 Incident)

Docker Desktop's BuildKit cache grew from 11GB → 27GB in 4 hours during Kamal deployments, crashing Docker and nearly filling the 228GB boot disk. Root cause: BuildKit stores cache in **volumes** that survive `docker builder prune`.

**See**: `docs/incident_reports/disk_space_analysis_2026_01_15.md` for full incident analysis.

### Current Architecture

```
Mac Mini (Docker Desktop REQUIRED)     Dell Optiplex
┌────────────────────────────────┐    ┌─────────────────────┐
│ registry:2 on localhost:5555   │◄───│ Remote Builder      │
│ Kamal starts this container    │    │ Builds AMD64 image  │
│ SSH tunnel back to Mac         │    │ Pulls via tunnel    │
└────────────────────────────────┘    └─────────────────────┘
```

**Pain points:**
- Docker Desktop uses 2.4GB RAM idle
- Disk never auto-reclaims (manual fstrim required)
- 30-60s startup time
- BuildKit volumes grow unbounded
- Mac must be running for deploys

---

## Phase 1: OrbStack Migration

### Why OrbStack?

| Feature | Docker Desktop | OrbStack |
|---------|---------------|----------|
| RAM usage | 2.4GB idle | 0.8GB idle |
| Disk reclaim | Manual fstrim | **Automatic** |
| Startup time | 30-60s | ~2s |
| BuildKit volumes | Grow unbounded | Auto-cleanup |
| Price | Free (personal) | Free (personal) |

### Community Experience Research

**Positive:**
- 7.9k GitHub stars, active development
- v2.0.5 (Nov 2025) is stable after v2.0.0-2.0.1 teething issues
- Full Docker/buildx/Compose compatibility
- Automatic disk reclaim is the killer feature

**Reported Issues (resolved):**
- v2.0.0-2.0.1 had crash issues → Fixed in v2.0.2+
- One migration data loss report (GitHub #1530) → Mitigation: backup volumes first
- `xterm-ghostty` warnings → Fixed in v2.0.5

**Kamal Compatibility:** Confirmed working - same buildx, multi-arch builds, local registry.

### Migration Steps

**Pre-flight (5 min):**
```bash
# 1. Verify Docker Desktop data to migrate
docker volume ls
docker images
docker ps -a

# 2. Backup critical volumes (Graphiti is tiny, but be safe)
mkdir -p ~/docker_backup
docker run --rm -v falkordb_data:/data -v ~/docker_backup:/backup alpine \
  tar czf /backup/falkordb_data.tar.gz /data
```

**Installation (10 min):**
```bash
# 3. Install OrbStack
brew install orbstack

# 4. Open OrbStack - it will prompt to migrate Docker Desktop data
open -a OrbStack

# 5. When prompted, click "Migrate" to copy Docker Desktop data
#    (Original data stays intact - this is a COPY)
```

**Verification (5 min):**
```bash
# 6. Verify migration
docker volume ls                    # Should see falkordb_data
docker images                       # Should see your images
docker ps                           # Start containers as needed

# 7. Test Graphiti
docker start falkordb-graphiti      # Or however you start it
# Verify Graphiti MCP connects
```

**Cleanup (5 min):**
```bash
# 8. Stop Docker Desktop (don't uninstall yet - keep as backup)
osascript -e 'quit app "Docker Desktop"'

# 9. Verify OrbStack is handling Docker
docker context ls                   # Should show orbstack as current
which docker                        # Should be OrbStack's docker
```

### Rollback Plan

If OrbStack doesn't work:
```bash
# Switch back to Docker Desktop
docker context use desktop-linux

# Or start Docker Desktop
open -a "Docker Desktop"
```

### Post-Migration: Uninstall Docker Desktop (Optional, After 1 Week)

After confirming OrbStack works reliably for a week:
```bash
# Uninstall Docker Desktop
brew uninstall --cask docker

# Remove Docker Desktop data (ONLY after confirming OrbStack works!)
rm -rf ~/Library/Containers/com.docker.docker
rm -rf ~/Library/Application\ Support/Docker\ Desktop
```

---

## Phase 2: Knot Registry (Dell Self-Hosted)

### Why Knot?

Eliminates Mac Docker dependency for deployments entirely.

| Aspect | Current (localhost) | Knot |
|--------|--------------------|----- |
| Mac Docker required | Yes | **No** |
| External accounts | None | None |
| Setup time | Done | ~15 min |
| Maintenance | None | Low (self-hosted) |

**Use case:** Deploy from anywhere - iPad, another machine, CI/CD - without Mac.

### Community Experience Research

**Source:** https://knot.deployto.dev/

**Positive:**
- Official tool from DeployTo.Dev, designed specifically for Kamal 2
- "Zero external dependencies" - no Docker Hub account needed
- Automatic HTTPS via Let's Encrypt
- Active development, good documentation
- Video walkthrough available

**Requirements:**
- DNS A record pointing to Dell
- Port 443 open on Dell (for HTTPS)
- One-time 15-minute setup

### Prerequisites

1. **DNS Record** (do first - needed for Let's Encrypt):
   ```
   Type: A
   Name: registry.fredrikbranstrom.se
   Value: 213.164.219.201
   TTL: 300

   # If using Cloudflare: Turn OFF proxy (gray cloud)
   ```

2. **Dell SSH Access:**
   ```bash
   ssh pop  # Or: ssh -p 2222 fredrik@213.164.219.201
   ```

### Installation Steps

**On Dell (15 min):**
```bash
# 1. Clone Knot
git clone https://github.com/deployTo-Dev/knot.git
cd knot

# 2. Run setup (interactive)
./knot setup
# Answer prompts:
#   - Server IP: 213.164.219.201
#   - Registry domain: registry.fredrikbranstrom.se
#   - SSH user: fredrik

# 3. Deploy registry
./knot deploy
# This bootstraps the registry container with Let's Encrypt HTTPS
```

**Update brf-auto config:**

Edit `~/Projects/brf-auto/config/deploy.yml`:
```yaml
# BEFORE (localhost registry - requires Mac Docker)
registry:
  server: localhost:5555
  username: "<%= ENV['KAMAL_REGISTRY_USERNAME'] %>"
  password: "<%= ENV['KAMAL_REGISTRY_PASSWORD'] %>"

# AFTER (Knot registry - no Mac Docker needed)
registry:
  server: registry.fredrikbranstrom.se
  username: admin
  password: <%= ENV["KNOT_REGISTRY_PASSWORD"] %>
```

Add to `.env` and `.kamal/secrets`:
```bash
KNOT_REGISTRY_PASSWORD=<password-from-knot-setup>
```

**Verification:**
```bash
# Test registry access
docker login registry.fredrikbranstrom.se

# Test deploy (from Mac OR any machine with SSH access to Dell)
cd ~/Projects/brf-auto
kamal deploy
```

### Rollback Plan

Revert `config/deploy.yml` to localhost:5555 registry config.

---

## Migration Checklist

### Phase 1: OrbStack
- [ ] Backup falkordb_data volume
- [ ] Install OrbStack (`brew install orbstack`)
- [ ] Run migration (automatic prompt)
- [ ] Verify Graphiti container starts
- [ ] Verify Kamal deploy works
- [ ] Stop Docker Desktop
- [ ] Run for 1 week to confirm stability
- [ ] (Optional) Uninstall Docker Desktop

### Phase 2: Knot
- [ ] Create DNS A record for registry.fredrikbranstrom.se
- [ ] Wait for DNS propagation (~5-15 min)
- [ ] SSH to Dell and run Knot setup
- [ ] Update brf-auto deploy.yml
- [ ] Add KNOT_REGISTRY_PASSWORD to secrets
- [ ] Test deploy from Mac
- [ ] (Optional) Test deploy from another machine

---

## Architecture After Both Phases

```
Mac Mini (OrbStack)                    Dell Optiplex
┌────────────────────────────────┐    ┌─────────────────────────────┐
│ Local development              │    │ Knot Registry               │
│ - FalkorDB/Graphiti           │    │ registry.fredrikbranstrom.se│
│ - Auto disk reclaim           │    │ HTTPS via Let's Encrypt     │
│ - 0.8GB RAM                   │    │                             │
│                                │    │ Remote Builder              │
│ Optional for deploys           │───▶│ Builds AMD64 images         │
│ (Knot eliminates requirement)  │    │ Pushes to local registry    │
└────────────────────────────────┘    │                             │
                                      │ Production Apps             │
                                      │ - brf-auto                  │
                                      │ - Temporal                  │
                                      └─────────────────────────────┘
```

**Benefits achieved:**
- ✅ Auto disk reclaim (no more 27GB crises)
- ✅ Lower RAM usage (0.8GB vs 2.4GB)
- ✅ Faster startup (2s vs 30-60s)
- ✅ Can deploy without Mac Docker running
- ✅ Local Graphiti/FalkorDB preserved
- ✅ No external registry accounts needed

---

## References

- OrbStack Docs: https://docs.orbstack.dev/
- OrbStack Migration: https://docs.orbstack.dev/install
- Knot: https://knot.deployto.dev/
- Kamal Registry Docs: https://kamal-deploy.org/docs/configuration/docker-registry/
- Incident Report: `docs/incident_reports/disk_space_analysis_2026_01_15.md`
- Kamal Local Docker Dependency: `brf-auto/docs/architecture/kamal_local_docker_dependency.md`
