# Issue Analysis: Bluetooth Speaker Volume Control on macOS

**Date**: 2026-01-15
**Affected Device**: Marshall Bluetooth speaker + Mac Mini M2
**Status**: Intermittent - AVRCP negotiation issue

## Symptom

macOS volume keys (F11/F12) **sometimes work** and **sometimes don't** with the Marshall Bluetooth speaker. When broken:
- Volume slider moves visually but actual output doesn't change
- Must use physical speaker knob or per-app volume

**Key observation**: This is INTERMITTENT, not constant. It works sometimes.

## Root Cause

**This is an AVRCP "Absolute Volume" negotiation issue, not a fixed protocol limitation.**

### What's Happening

Bluetooth audio uses two profiles:
- **A2DP**: Audio streaming (the actual sound)
- **AVRCP**: Remote control (volume, play/pause, track skip)

AVRCP 1.4+ introduced "Absolute Volume" - a feature where the host (Mac) and speaker coordinate volume levels. When negotiation succeeds, macOS volume controls work. When it fails, they don't.

### Why It's Intermittent

The AVRCP handshake can fail due to:
1. **Connection timing**: If profiles initialize in wrong order
2. **Bluetooth interference**: Wi-Fi, other devices on 2.4GHz
3. **Pairing state corruption**: Cached connection parameters become stale
4. **Firmware bugs**: On either Mac or speaker side
5. **macOS updates**: Sonoma 14.4+ introduced regressions for some users

**This explains why it works sometimes**: When the AVRCP negotiation succeeds, volume works. When it fails, volume is disabled for that session.

## Fixes (In Order of Effectiveness)

### 1. Re-pair the Device (Most Effective)
Forces fresh AVRCP negotiation:
1. System Settings → Bluetooth
2. Right-click Marshall speaker → **Remove**
3. Turn speaker off, wait 10 seconds
4. Turn speaker on, put in pairing mode
5. Re-pair from Mac

### 2. Reset Bluetooth Module
Clears macOS Bluetooth stack state:
1. Hold **Shift + Option** and click Bluetooth icon in menu bar
2. Click **Reset the Bluetooth module** (if available)
3. Or: `sudo pkill bluetoothd` in Terminal
4. Reconnect device

### 3. Power Cycle in Specific Order
Sometimes fixes negotiation timing:
1. Disconnect speaker from Mac
2. Turn speaker OFF
3. Turn speaker ON
4. Wait 5 seconds
5. Connect from Mac

### 4. Check for Firmware Updates
- Marshall speakers: Check Marshall Bluetooth app (iOS/Android)
- macOS: Ensure running latest version (some fixes in point releases)

### 5. Reduce Interference
- Move speaker closer to Mac
- Disable unused Bluetooth devices
- Check for Wi-Fi congestion on 2.4GHz

## Workarounds (If Fixes Don't Work)

### Audio MIDI Setup Multi-Output Device
Bypasses AVRCP volume entirely:
1. Open **Audio MIDI Setup** (Applications → Utilities)
2. Click **+** → **Create Multi-Output Device**
3. Check Marshall speaker AND a virtual output (BlackHole, Zoom Audio Device)
4. Select this new device as sound output

### Proxy Audio Device (Homebrew)
```bash
brew install --cask proxy-audio-device
```
Enables media key control for problematic devices.

### SoundSource (Paid App)
[Rogue Amoeba SoundSource](https://rogueamoeba.com/soundsource/) - per-app/per-device volume control.

## macOS Limitation

**Unlike Windows and Android, macOS has NO setting to disable Absolute Volume.**

- Windows: Registry key `DisableAbsoluteVolume`
- Android: Developer Settings → "Disable absolute volume"
- macOS: No equivalent option exists

This means you cannot force macOS to always control volume locally.

## References

- [Apple Community: Volume control after Sonoma 14.4](https://discussions.apple.com/thread/255523029)
- [Apple Community: Cannot change volume on Bluetooth devices](https://discussions.apple.com/thread/253711192)
- [UMA Technology: Bluetooth Speaker Volume Control Not Working](https://umatechnology.org/solved-bluetooth-speaker-volume-control-not-working/)

## Summary

| Behavior | Cause |
|----------|-------|
| Works sometimes | AVRCP negotiation succeeded |
| Doesn't work | AVRCP negotiation failed |
| Fix | Re-pair device to force fresh negotiation |

**Bottom line**: Try re-pairing first. If the problem recurs frequently, use the Audio MIDI Setup workaround for consistent behavior.
