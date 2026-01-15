# Incident Report: Docker Desktop Data Loss
**Date:** 2026-01-14 (Night) / 2026-01-15 (Morning)
**Impact:** Total loss of local Docker images, containers, and volumes.
**Note:** Docker.raw is a sparse file on APFS—logical size (shown by `ls -lh`) may appear as 64GB+ but actual disk consumption (`du -h`) is typically 50-70% less.
**Duration:** ~20 minutes from hang discovery to data loss confirmation.

## Summary
During troubleshooting of a "Dead" BuildKit container and a frozen Docker Desktop GUI, a destructive filesystem command (`rm -rf`) was executed on the Docker container's sandbox directory. Although the command initially reported "Operation not permitted" on metadata files protected by macOS SIP, it successfully deleted the underlying virtual disk file (`Docker.raw`) before being blocked. Upon restart, Docker Desktop initialized a fresh, empty disk image, resulting in the loss of all local state.

## Root Cause Analysis
1. **Primary Trigger:** Docker Desktop entered a corrupted state where the `com.docker.backend` was unresponsive and the UI wouldn't launch.
2. **Hidden Blocker:** A "Zombie" networking process (`com.docker.vmnetd`) from Sunday was holding a system-level lock, preventing new Docker instances from taking control of the networking stack.
3. **Faulty Step:** I (AI Assistant) recommended deleting `~/Library/Containers/com.docker.docker` to clear application state. This recommendation failed to account for the fact that the entire virtual drive (where images/volumes live) is stored inside that specific sandbox.
4. **SIP Behavior:** macOS protected the folder structure itself from being fully deleted, giving a false sense that the command had "failed" safely, when in fact the un-protected `Docker.raw` file inside was deleted immediately.
5. **Snapshot Exclusion:** Investigation into local APFS snapshots revealed that while the parent container folder might appear in the file hierarchy, Docker's default setting "Exclude VM from Time Machine backups" applies a "sticky" exclusion attribute to the virtual disk data. Consequently, the Jan 14 snapshot did not capture the original `Docker.raw` file, rendering a point-in-time recovery impossible.

## Recovery Plan
1. **Time Machine Restoration:** Identify the latest backup of `~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw`.
2. **Surgical Placement:** Restore the file while Docker Desktop is **Quit**.
3. **Lock Release:** Ensure all Docker processes (`vmnetd`, `vpnkit`, `com.docker.backend`) are killed before starting the app with the restored disk.

## Prevention & Lessons Learned
- **Never `rm` Docker Sandboxes:** On macOS, the sandbox *is* the disk. Never delete `~/Library/Containers/com.docker.docker` without an explicit backup of the `Docker.raw` file inside.
- **Identify Zombies First:** Use `ps aux` and `grep` to find stale processes from previous days before attempting filesystem resets.
- **Reboot Priority:** When Docker's UI and backend are out of sync, a system reboot is the only 100% safe way to release networking locks without risking data integrity.
- **AI Accountability:** Assistant must emphasize that `rm -rf` on application containers in macOS is a "Factory Reset" equivalent, not a simple "Clear Cache" move.
