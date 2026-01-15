# Issue Analysis: Bluetooth Speaker Volume Control on macOS

**Date**: 2026-01-15
**Affected Device**: Marshall Bluetooth speaker + Mac Mini M2
**Status**: Known limitation with workarounds

## Symptom

When connected to a Marshall Bluetooth speaker, macOS volume keys (F11/F12) show visual feedback (dots go up/down) but the actual speaker volume does not change. Volume must be controlled via:
- Physical controls on the speaker
- Per-app volume sliders (YouTube, Spotify, etc.)

## Root Cause

**This is a Bluetooth audio protocol limitation, not a macOS bug.**

Bluetooth audio devices report their capabilities to macOS via the A2DP (Advanced Audio Distribution Profile) protocol. Some speakers report themselves as having **independent volume control**, which causes macOS to:
1. Disable software volume mixing for that device
2. Send a fixed-level audio stream to the speaker
3. Expect the speaker to handle its own volume

Marshall speakers (like Stanmore, Acton, Kilburn) typically fall into this category—they're designed as "pro audio" devices where volume is managed on the device itself.

**Why some devices work and others don't:**
- Cheap earbuds: Often report no volume capability → macOS controls volume
- Marshall/high-end speakers: Report independent volume → macOS defers to device
- Some devices: Support both → Volume controllable from either end

## Workarounds

### 1. Use Physical Volume Knob (Intended Design)
Marshall speakers have prominent analog volume knobs. This is the intended user experience.

### 2. Audio MIDI Setup Multi-Output Device
Create a virtual audio device that allows software volume control:
1. Open **Audio MIDI Setup** (Applications → Utilities)
2. Click **+** → **Create Multi-Output Device**
3. Check your Marshall speaker AND a virtual output (like BlackHole or Zoom Audio Device)
4. Select this new device as your sound output

This routes audio through a mixer where macOS can apply volume before sending to the speaker.

### 3. Proxy Audio Device (Homebrew)
```bash
brew install --cask proxy-audio-device
```
After installation, configure via **Proxy Audio Device Settings** app. Enables media key control for problematic devices.

### 4. SoundSource (Paid App)
[Rogue Amoeba SoundSource](https://rogueamoeba.com/soundsource/) provides per-app and per-device volume control that bypasses this limitation.

## Why This Isn't a Bug

Apple's position: The speaker manufacturer decides how volume should be handled. If Marshall reports "I handle my own volume," macOS respects that. This is by design for professional audio workflows where consistent output levels matter.

**Comparison:**
- Windows: Often ignores device capabilities and applies software volume anyway
- macOS: Respects device-reported capabilities
- iOS: Uses different Bluetooth profiles, may work differently

## References

- [Apple Community: Volume Control not working With Bluetooth Speaker](https://discussions.apple.com/thread/252364494)
- [Apple Community: Mac volume control disabled when using external devices](https://discussions.apple.com/thread/255028132)
- [MacPaw: External speakers not working on Mac](https://macpaw.com/how-to/external-speakers-not-working-mac)

## Recommendation

**Use the physical volume knob.** This is how Marshall designed the speaker to work. The workarounds above add complexity and potential audio quality issues. If software volume control is essential, consider speakers that advertise macOS compatibility (like Sonos or HomePod).
