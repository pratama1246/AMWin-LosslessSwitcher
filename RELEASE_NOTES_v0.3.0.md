## What's New in v0.3.0

### 🚀 Hybrid Detection Engine (Native ETW Telemetry Snooping)
In previous versions, resolving Hi-Res Lossless tracks relied solely on Apple Music's public Web API (`amp-api`), where stream manifests are capped at 48 kHz for browser playback, leaving a guessing dilemma between 96 kHz and 192 kHz.

**v0.3.0 introduces a native diagnostic snooper (`EtwLogReader`):**
- Reads real-time Event Tracing for Windows (`.etl`) telemetry emitted by Apple Music Windows' playback agent (`AMPLibraryAgent.exe:cmplayer`).
- Extracts the exact decoded audio specification directly from Apple's `mediaFormatinfo` events (`asbdFormatID = qlac`, `sdFormatID = alac`, exact `asbdSampleRate` such as 96.0 kHz or 192.0 kHz, and `sdBitDepth = 24 bit`).
- Filters out fast-start transitional buffering (`qaac -> qlac`) to ensure only true decoded ALAC rates are applied.
- Dynamically correlates track identities (`adamID`) across playback boundaries (`song changed`, `/syllable-lyrics`, and FairPlay leases).

### ⚡ Live Format Refinement
If an uncached Hi-Res track is played for the first time and the local log buffer takes a few hundred milliseconds to flush, the Coordinator applies an initial catalog estimate and automatically refines the DAC endpoint format the moment verified telemetry arrives—without interrupting the user.

### 🗄️ Cache Architecture v4
- Upgraded the local catalog cache schema to **v4** (`catalog_cache.json`).
- Stores verified track ID (`adamID`) to format mappings locally to ensure near-instant switching on repeated plays.

### 🛡️ Strict Audio Transparency (No Bit-Perfect Overclaims)
- Added clear documentation clarifying that AMWin-LosslessSwitcher performs **strict sample rate and bit depth matching** to avoid Windows Audio Engine (WASAPI Shared) resampling.
- Explicitly documents that Apple Music Windows routes through Windows Shared Mode; this utility does not provide WASAPI Exclusive Mode, ASIO bypass, or bit-perfect guarantees.

---

### 🧪 Verified Resolution Tests

| Track & Artist | Catalog Trait | Resolved Endpoint Format | Lossless Status |
| :--- | :--- | :--- | :--- |
| **Coldplay — Yellow** (*Parachutes*) | Hi-Res Lossless | **192 kHz / 24-bit** | Lossless (ALAC) |
| **Eagles — Hotel California** (*2013 Remaster*) | Hi-Res Lossless | **192 kHz / 24-bit** | Lossless (ALAC) |
| **RIIZE — Behind The Shine** (*ODYSSEY*) | Hi-Res Lossless | **96 kHz / 24-bit** | Lossless (ALAC) |
| **The Rose — Back To Me** (*DUAL*) | Lossless | **48 kHz / 24-bit** | Lossless (ALAC) |
| **DxS — Blue** (*Serenade - EP*) | Lossless | **44.1 kHz / 16-bit** | Lossless (ALAC) |
| **JOY — Hello** (*Special Album*) | Lossy AAC | **44.1 kHz / 16-bit** | Lossy (AAC) |

---

### 📦 Downloads & Checksums

| File | Type | Size |
| :--- | :--- | :--- |
| **`AMWin-LosslessSwitcher-Setup-v0.3.0.exe`** | Windows Installer (Start Menu, Desktop, Autostart) | ~3.1 MB |
| **`AMWin-LosslessSwitcher-Portable-v0.3.0.exe`** | Self-Contained Standalone Executable | ~2.8 MB |
