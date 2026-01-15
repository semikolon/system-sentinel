# Incident Report: APFS Time Machine "Illegal Byte Sequence" (Error 92)

**Status**: Resolved
**Date**: 2026-01-15
**Affected System**: external USB (APFS) backup volume `/Volumes/FredrikBackup`

## 1. Symptom
Persistent, loud mechanical disk activity ("crackling" or "grinding") on the external USB drive starting immediately after a Time Machine backup reports as finished. Disk I/O monitoring (`iostat`) showed 100% saturation with small, repetitive metadata writes.

## 2. Diagnosis
Detailed analysis of `backupd` logs via `log show` revealed the following cycle:
1. `backupd` initiates **Backup Thinning** (space-based).
2. It attempts to delete 6 older snapshots from **January/February 2025**.
3. It fails with `Error Domain=NSPOSIXErrorDomain Code=92 "Illegal byte sequence"`.
4. It retries indefinitely, causing the disk thrashing.

### Root Cause: Decomposed Unicode (NFD)
Cross-referencing the local disk with `convmv` revealed **1822 files** with "legacy" decomposed Unicode characters (e.g., `a` + `̈` instead of `ä`). These files originated from a 2011 MacBook Pro backup archive.
- **The Conflict**: Files created with NFD on older versions of macOS or specific apps conflict with the strict expectations of the APFS Time Machine thinning engine.
- **The Desync**: Due to this error, Time Machine's catalog (`backup_manifest.plist`) became desynchronized from the actual APFS snapshot index, leading to "Ghost Backups" that the system tried to delete but couldn't "see" correctly.

## 3. Remediation

### Immediate Fix (Clearing the Queue)
The "Illegal byte sequence" stops the thinning process from ever completing. The safest way to clear the corrupt queue is:
1. **Remove Disk**: In System Settings > Time Machine, remove the backup destination.
2. **Re-add Disk**: Add it back and select **"Claim existing backups"**.
This flushes the local manifest and rebuilds the connection, clearing the "Ghost" thinning tasks.

### Root Remedy (Local Machine)
To prevent the problematic names from being re-uploaded in future backups, the local filenames must be normalized to Unicode NFC:
```bash
sudo convmv -r -f utf8 -t utf8 --nfc --nfd --notest /Users
```

### Long-term Health
- **First Aid**: Run Disk Utility First Aid on the backup volume while high-priority tasks are not running.
- **Space Management**: Ensure the backup volume has at least 15-20% free space to reduce the frequency of aggressive thinning.

## 4. Key Learnings
- **Error 92** in Time Machine on APFS is almost always a filename encoding or metadata collision, not a hardware failure.
- **Resetting the association** (Remove/Re-add) is a non-destructive way to clear manifest corruption.

---

## 5. Follow-up Incident: Disk Space Crisis (2026-01-15 15:00 CET)

### Symptom
- Docker Desktop crashed with "write ... init.log: no space left on device"
- Boot disk at **93% capacity** (only 15GB free of 228GB)
- Time Machine backup restarted after previous failure due to low disk space
- Docker daemon hung: `docker ps` does not respond

### Current Status (15:25 CET)
- **Time Machine**: 92.4% complete, ~2 minutes remaining, writing to FredrikBackup (332GB free)
- **Docker**: Daemon unresponsive. Error dialog displayed. Multiple zombie processes visible.
- **Root cause**: Boot disk exhaustion prevented Docker VM writes

### Disk Space Analysis
| Location | Size | Notes |
|----------|------|-------|
| Docker container | 11GB | Fresh after yesterday's data loss incident |
| ~/Library/Caches | 8.5GB | Browser/app caches |
| ~/Library/Containers | 12GB | Docker + sandboxed apps |
| Local TM snapshot | ~1GB | Will be released after backup completes |

### Recovery Plan
1. ✅ **Wait for Time Machine to complete** - DONE (15:28 CET, ThinningPostBackup)
2. ✅ **Time Machine fully idle** - Confirmed 15:35 CET (`Running = 0`)
3. **Reboot** - Cleanest recovery path. Will:
   - Kill all zombie Docker processes
   - Release local TM snapshot (~1GB)
   - Clear temporary files and stuck file handles
   - Give Docker daemon a fresh start

### Correction: Docker.raw Size Misconception

**Original claim was wrong.** Docker.raw is a **sparse file** on APFS:
- **Logical size** (`ls -lh`): Shows maximum allocation (can appear as 64GB+)
- **Physical size** (`du -h`): Actual disk consumption (often 50-70% less)

The deleted Docker.raw likely consumed **10-30GB actual disk space**, not 228GB. The 228GB figure is the total boot disk capacity. APFS sparse files compress empty blocks, so the file's apparent size vastly overstates real disk usage.

**Reference**: [Docker Desktop Mac FAQs](https://docs.docker.com/desktop/troubleshoot-and-support/faqs/macfaqs/)

### Risk Assessment
- **HIGH**: 15GB free is critically low for development work
- **MEDIUM**: Docker may need Docker.raw deletion if daemon won't recover post-reboot
- **LOW**: Time Machine backup completed successfully

### Post-Reboot Actions
1. ✅ Verify Docker Desktop launches cleanly - DONE
2. Run `docker system prune -a` if Docker has accumulated cruft
3. ✅ Deep disk space analysis - DONE (see `disk_space_analysis_2026_01_15.md`)
4. Consider setting Docker Desktop disk limit (Settings → Resources → Disk image size)

### Post-Reboot Status (16:00 CET)
- **Docker**: ✅ Running, both containers healthy (FalkorDB, BuildKit)
- **Disk space**: 19GB free (up from 11GB after cache clearing)
- **Time Machine**: Idle, last backup 15:50

---

## 6. Follow-up Incident: xcode_select_link Backup Warning (2026-01-15 16:15 CET)

### Symptom
Time Machine error dialog:
> "Time Machine couldn't complete the backup to 'FredrikBackup'"
> "/private/var/db/xcode_select_link" could not be backed up.

### Diagnosis

**This is a MINOR warning, not a critical failure.**

```bash
$ ls -la /private/var/db/xcode_select_link
lrwxr-xr-x  1 root  wheel  35 Oct  8  2024 /private/var/db/xcode_select_link@ -> /Library/Developer/CommandLineTools
```

The file is just a **symlink** pointing to Xcode Command Line Tools. Time Machine sometimes has trouble backing up certain system-level symlinks in `/var/db/`.

### Why This Isn't Critical

1. **Easily recreatable**: Run `xcode-select --install` to regenerate
2. **Not user data**: Just a pointer to developer tools
3. **Backup otherwise succeeded**: The warning is informational, not fatal
4. **The destination exists**: `/Library/Developer/CommandLineTools` IS backed up

### Resolution Options

**Option A: Ignore it** (recommended)
The symlink can be recreated in seconds. Your actual data is backed up.

**Option B: Exclude from backups**
1. System Settings → Time Machine → Options
2. Add `/private/var/db/xcode_select_link` to exclusions
3. Prevents the warning on future backups

**Option C: Recreate the symlink**
```bash
sudo xcode-select --reset
```
This rebuilds the symlink, which may resolve TM's issue with it.

### Risk Assessment
- **SEVERITY**: Low (informational warning)
- **DATA LOSS RISK**: None (symlink is not user data)
- **ACTION REQUIRED**: None (or exclude to silence warning)
