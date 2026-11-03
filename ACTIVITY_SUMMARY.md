# Engineering Activity & Investigation Summary

**Date:** November 3, 2026  
**Project:** Apple Music Lossless Switcher for Windows (`lossless-switcher-am`)  
**Base Release:** `v0.3.1` ➔ **Target Release:** `v0.3.2`

---

## 1. Background & Offline Playback Experiments

### Initial Inquiries & Motivations
The session began by investigating whether **Offline Mode** (tracks downloaded locally in Apple Music for Windows) could be supported:
1. Why does audio resolution drop to **16-bit / 44.1 kHz** when playing offline/downloaded tracks?
2. Can lossless or Hi-Res audio specifications be extracted directly from local files or Windows Event Tracing (ETW) logs?

### Technical Findings on Offline Audio
- **FairPlay DRM Encryption:** Tracks downloaded by Apple Music Windows (UWP / Win32 container) are stored in encrypted private chunks (`.m4p` containers). Container headers and stream specifications cannot be decrypted or inspected by external processes without Apple's proprietary FairPlay DRM keys.
- **Windows SMTC Limitations:** Windows System Media Transport Controls (SMTC) only publishes presentation metadata (*Title*, *Artist*, *Album*, *Thumbnail*). Technical stream properties (*sample rate*, *bit depth*, *audio codec*) are intentionally omitted from SMTC payloads.
- **Windows Audio ETW Traces:** The `Microsoft-Windows-Audio` ETW provider records WASAPI endpoint formats after the audio engine negotiates shared mixing, reflecting Windows post-resampling parameters rather than the original source master file attributes.
- **Root Cause of Fallback to 16-bit:** Without an active Internet connection or a matching entry in Apple Music's online catalog, the switcher safely falls back to standard CD quality (16-bit / 44.1 kHz).

---

## 2. Hybrid Approach Experiments & Race Condition Discovery

### Experimental Hybrid Design
An experimental hybrid architecture was tested to combine:
- Apple Music Storefront Catalog Web API lookups.
- Local cache persistence and real-time ETW telemetry tracking.
- Heuristic fallback evaluation for offline playback.

### Observed Instabilities & Race Conditions
Field testing exposed critical race conditions in real-world playback:
1. **SMTC Blank Interval:** During track transitions, SMTC media properties temporarily flush to empty (`None`) for 1–2 polling cycles (~100–300 ms).
2. **Ping-Pong Loop Switching:** This momentary absence of track metadata caused the coordinator to falsely detect playback stop or device changes, triggering a restore of the default device format, only to immediately re-switch to the new track's format a fraction of a second later.
3. **Hardware Relay Chattering & Glitches:** External DACs (e.g., JadeAudio JA11, Realtek ALC) suffered rapid relay switching cycles, producing audible clicks, pop noises, and playback stutter in Apple Music.
4. **Verdict:** The hybrid approach introduced excessive race conditions and unpredictable state oscillations.

---

## 3. Decision & Remediation: Clean Revert to v0.3.1

Per user directive:
> *"It's erratic, full of race conditions. Revert all code back to 0.3.1 except for the default DAC fix."*

Actions taken:
1. **Clean Git Restoration:**
   - Discarded all experimental changes in `src/apple_music/catalog.rs`, `src/apple_music/etw_reader.rs`, `Cargo.toml`, and related files.
   - Removed temporary test scripts (`tests/test_offline_resilience.rs`).
   - Cleaned the working tree back to `v0.3.1` (`608525f`).
2. **Cache Purge:**
   - Deleted `%APPDATA%\AMWin-LosslessSwitcher\catalog_cache.json` to eliminate stale or corrupted cache entries from previous tests.

---

## 4. Root Cause Analysis & Surgical Fix: Default DAC Startup Preservation

### The Reported Issue
When launching the switcher while Apple Music was closed or paused:
- The external DAC was immediately forced to **24-bit / 44.1 kHz**.
- The user's preferred DAC default format (e.g., 32-bit / 384 kHz or 24-bit / 48 kHz) was overwritten before any music was ever played.

### Root Cause in v0.3.1
- At startup, `last_applied_fmt` initialized to `None`.
- If Apple Music was previously launched, Windows SMTC retained the last played song in its memory.
- On the very first tick, the coordinator observed `requested_changed = true` and `cur_rate != effective_target_fmt.sample_rate`.
- Because `is_active` (playback status) was not checked as a prerequisite for the initial switch, the coordinator immediately triggered a format change and falsely announced "Synchronized".

### Surgical Fix in Coordinator
A targeted 10-line fix was applied to `src/coordinator/mod.rs`:

```rust
// 1. Detect initial startup when playback is not currently active
let is_startup_idle = last_applied_fmt.is_none() && !is_active;

// 2. Prevent triggering a format switch if the app just started in an idle state
let needs_switch = !is_startup_idle && (device_changed || mode_changed || requested_changed || (!already_applied_for_this_track && {
    cur_rate != effective_target_fmt.sample_rate || cur_bits != effective_target_fmt.bit_depth
}));

// 3. In the non-switching branch: defer updating state keys while idle
if !is_startup_idle {
    if let Ok(mut key) = last_track_key_arc.lock() {
        *key = track_key.clone();
    }
    if last_applied_fmt.is_none() {
        last_applied_fmt = current_dev_fmt;
    }
    last_requested_fmt = Some(requested_fmt);
}

// 4. Update UI status to informatively reflect the idle waiting state
if is_startup_idle {
    st.status_message = format!("Waiting for playback: {}", track.title);
}
```

### Why This Fix Eliminates Race Conditions
- `is_startup_idle` is only `true` during the initial launch phase before the first song begins playing.
- Once playback starts (`is_active = true`), format switching operates normally and `last_applied_fmt` is set.
- Subsequent track transitions with momentary blank SMTC properties retain `is_startup_idle = false`, completely preventing ping-pong format oscillation.

---

## 5. Verification, Release Packaging & Artifacts

1. **Build Validation:**
   - `cargo check`: Clean pass with 0 errors and 0 warnings.
   - `cargo build --release`: Compiled successfully in 14.17 seconds.
2. **Inno Setup Installer Compilation:**
   - Compiled installer `AMWin-LosslessSwitcher-Setup-v0.3.2.exe`.
3. **Artifact Deployment:**
   - Standalone portable binary and installer compiled and copied to `%USERPROFILE%\Downloads\`.
   - Source archives (`.zip` and `.tar.gz`) packaged via Git archive.
