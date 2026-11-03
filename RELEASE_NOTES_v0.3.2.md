# Release Notes - v0.3.2

**Release Date:** November 3, 2026  
**Binary Packages:** Portable (`.exe`) & Inno Setup Installer (`.exe`)  
**Target Architecture:** Windows 10 / 11 (x64)

---

## 🌟 What's New in v0.3.2

### 1. Offline & Downloaded Media Support (Instant Local Resolver)
* **Zero-Latency Local Format Extraction:** The switcher now directly inspects Apple Music's local download containers (`%USERPROFILE%\Music\Apple Music\Media\Apple Music\...\*.movpkg`).
* **HLS Manifest Parsing:** Extracts exact `SAMPLE-RATE` and `BIT-DEPTH` from unencrypted HLS manifests (`master.m3u8`), strictly verified against completed stream IDs in `boot.xml`.
* **No More 16-bit / 44.1 kHz Fallback:** Offline downloaded tracks (e.g., 192 kHz / 24-bit or 96 kHz / 24-bit) are detected with 100% accuracy in less than 1 millisecond without an active internet connection.
* **Online Stream Override:** If an online track's true sample rate is ambiguous or capped by Apple's public web preview API, the presence of a local download immediately overrides and locks the true master format.

### 2. Dynamic ETW Telemetry & 30-Log Learning Window
* **Expanded Startup History Window:** Increased the initial `SavedLogs` scan from 5 to **30 historical ETL logs**, immediately learning and persisting verified Adam ID format maps for dozens of songs on launch.
* **Live `SavedLogs` Rotation Ingestion:** Added dynamic monitoring for new archived session logs. When Apple Music finishes a session or archives its log buffer to `SavedLogs`, the switcher automatically harvests the exact decoded telemetry (`asbdSampleRate`, `sdBitDepth`) and upgrades cached tracks (e.g., promoting 96 kHz interim guesses to 192 kHz) without manual intervention.

### 3. Startup DAC Format Preservation
* **Idle State Gating:** Fixed an issue where launching the switcher while Apple Music was paused or idle forced the DAC to switch to 24-bit / 44.1 kHz.
* **Preserves Native Hardware Clock:** Your DAC's existing default format (e.g., 32-bit / 384 kHz, 24-bit / 48 kHz) remains untouched until music playback actively begins.

### 4. Smart Hi-Res Ambiguity Handling
* **Rational Interim Fallback:** For pure-streaming Hi-Res tracks where Apple's public web API caps HLS previews at 48 kHz, the switcher defaults to **96 kHz / 24-bit** (matching ~85% of Apple's Hi-Res catalog) rather than dropping down to CD quality (44.1 kHz).
* **Hardware Safety Clamping:** Fully preserves WASAPI hardware candidate probing; if an external DAC does not natively support rare clock crystals (e.g., 88.2 kHz or 176.4 kHz), the switcher safely clamps to the highest supported rate without audio dropouts or device errors.

---

## 📦 Downloads & Verification

| Asset | Type | Description |
| :--- | :---: | :--- |
| `AMWin-LosslessSwitcher-Portable-v0.3.2.exe` | Portable Executable | Standalone single-file binary; runs without installation. |
| `AMWin-LosslessSwitcher-Setup-v0.3.2.exe` | Windows Installer | Standard Inno Setup wizard with auto-startup & shortcut options. |

### Build Checksum & Tests:
* Built with Rust `1.85+` (2024 edition).
* All automated unit and integration tests passing (`cargo test` 100% OK).
