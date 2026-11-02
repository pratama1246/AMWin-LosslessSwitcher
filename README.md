<div align="center">
  <img src="assets/logo.svg" width="130" height="130" alt="LosslessSwitcher For Apple Music Logo" />
  <h1>LosslessSwitcher For Apple Music</h1>
  <p><b>Apple Music Audio Format Synchronizer for Windows</b></p>
  <p>Automatically adjusts the Windows audio output format to match the track format reported by Apple Music metadata.</p>

  [![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D6.svg)](https://microsoft.com/windows)
  [![Language](https://img.shields.io/badge/language-Rust-DEA584.svg)](https://www.rust-lang.org/)
  [![License: GPL-3.0](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
</div>

<div align="center">
  <img width="540" alt="Recording_11_1_2026_7_00_09_PM3" src="https://github.com/user-attachments/assets/e4ec6293-9da6-4090-bcae-e04ba9490c69" />
</div>

---

## Overview

Apple Music for Windows does not provide an option to automatically change the Windows audio endpoint format for each track. As a result, the output device may remain configured to a single sample rate and bit depth while Apple Music plays tracks with different formats.

**AMWin-LosslessSwitcher** is a lightweight Windows utility that follows Apple Music playback and changes the selected Windows audio endpoint format based on track metadata. It uses Windows System Media Transport Controls (SMTC) to detect playback information and resolves track format information using Apple Music Web and iTunes metadata.

The application changes the Windows endpoint format; it does not decode, capture, or modify Apple Music audio.

<div align="center">
  <img width="320" alt="LosslessSwitcher For Apple Music main window" src="https://github.com/user-attachments/assets/8c8e2c54-0a2f-43cc-afd9-5cb99100e0c1" />
  <img width="320" alt="LosslessSwitcher For Apple Music audio settings" src="https://github.com/user-attachments/assets/88edf31a-f9b1-4f50-b66b-91edb6d01d8a" />
</div>

## Features

- **Automatic format switching** — Adjusts the selected Windows audio endpoint when the detected track format changes.
- **Seamless Mode** — Keeps the endpoint bit depth at 24-bit while following the track sample rate.
- **Purist Mode** — Follows both the track sample rate and bit depth.
- **Output device selection** — Select a specific audio endpoint or use the Windows default device.
- **Hardware format checking** — Checks whether the selected endpoint supports a requested format and handles unsupported formats.
- **Adjustable settling delay** — Configure a delay to accommodate audio devices when changing formats.
- **System tray support** — Keep the utility running in the background.
- **Settings persistence** — Saves application preferences between launches.
- **Portable mode** — Run the executable without installing the application.

## How It Works

1. AMWin-LosslessSwitcher detects the active Apple Music session through Windows SMTC.
2. It uses track information to look up matching metadata from Apple Music Web and iTunes.
3. The resolved sample rate and bit depth are used as the target Windows audio endpoint format.
4. The application checks whether the selected endpoint supports the requested format before applying the change.

The format shown by the application is based on matched metadata. It is not a direct measurement of the decoded audio stream or the signal received by the DAC.

## Installation

### Option A: Windows installer

Download `AMWin-LosslessSwitcher-Setup-v0.2.0.exe` from [GitHub Releases](../../releases).

The installer:
- Installs the application under `%LOCALAPPDATA%\Programs\AMWin-LosslessSwitcher`.
- Creates Start Menu and Desktop shortcuts.
- Offers an option to launch the application minimized to the system tray at Windows startup.
- Registers an uninstaller in Windows Settings.

### Option B: Portable executable

Download `AMWin-LosslessSwitcher-Portable-v0.2.0.exe` from [GitHub Releases](../../releases) and run it from a folder of your choice.

To keep settings and catalog cache files beside the executable, use `--portable` or create an empty `portable.dat` file in the same folder.

## Usage & Settings

### Now Playing

The Now Playing view displays:
- Track title, artist, and album artwork.
- The format resolved from track metadata.
- The current format of the selected Windows audio endpoint.
- The current synchronization status.

The application can be minimized to the system tray while it monitors playback.

### Audio Settings

- **Output Device** — Choose an audio endpoint or follow the Windows default device.
- **Switcher Mode** — Choose Seamless Mode or Purist Mode.
- **Settling Delay** — Set the delay used during endpoint format changes (50–800 ms).
- **Always on Top** — Keep the application window above other windows.
- **Run on Windows Startup** — Start the application when you sign in to Windows.
- **Clear Cache** — Clear locally stored catalog cache data.

### Recommended Apple Music & Windows Settings

These settings may help keep the audio path consistent, but they do not turn Windows Shared Mode into Exclusive Mode.

- Enable **Lossless Audio** in Apple Music playback settings. Choose **Hi-Res Lossless** if you want Apple Music to make high-resolution tracks available.
- Disable **Sound Check**, **Dolby Atmos**, and **Song Transitions** if you prefer standard stereo lossless playback without those features.
- If you use external volume control, you may set Apple Music and Windows volume to 100%. Be aware that this affects system volume and other applications too.
- Disable Windows audio enhancements, spatial sound, or driver DSP effects if you do not want those effects applied.

The exact settings available may vary by Apple Music and Windows version.

## Known Limitations

- **No Exclusive Mode:** Apple Music for Windows does not expose an Exclusive Mode option through this utility. Playback continues through the Windows Shared Audio Engine.
- **No bit-perfect guarantee:** Changing the Windows endpoint format does not guarantee bit-perfect output. Other applications, system sounds, Windows audio processing, volume controls, drivers, and device behavior may affect the final signal.
- **Metadata-based format information:** The target format is resolved from Apple Music Web and iTunes metadata. It is not read directly from the decoded audio stream.
- **Device and driver support:** Available formats and switching behavior depend on the selected audio endpoint and its driver. Unsupported formats may not be applied.
- **Track matching:** Metadata lookup depends on matching the detected track information with Apple's catalog entries. Incorrect or unavailable matches may affect the resolved format.

## Tested Hardware

The following combination has been tested by the project maintainer:

| Operating System | Audio Device / DAC | Tested Format Range | Result |
| :--- | :--- | :--- | :--- |
| Windows 11 (x64) | **FiiO JadeAudio JA11** (USB-C DAC) | `44.1 kHz`–`192 kHz`, `16-bit` / `24-bit` | Tested with Seamless and Purist modes |

This is a record of the maintainer's test setup, not a guarantee of compatibility with every device or driver.

## Command Line Arguments

| Parameter | Action |
| :--- | :--- |
| `(none)` | Opens the main application window. |
| `--minimized` or `--tray` | Starts the application minimized to the system tray. |
| `--portable` | Stores settings and catalog cache files beside the executable. |

---

## Development

### Requirements

- Windows 10 or Windows 11 (64-bit)
- Rust stable toolchain: `stable-x86_64-pc-windows-msvc`
- Inno Setup 6 (optional, for building the installer)

### Build

```powershell
# Check the project
cargo check

# Run tests
cargo test

# Build the release executable
cargo build --release

# Optional: build the installer
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" installer\setup.iss
```

The main application source is under `src/`, with tests under `tests/`. The installer configuration is in `installer/setup.iss`.

## Privacy

AMWin-LosslessSwitcher requests track-related metadata from public Apple services, including Apple Music Web and iTunes. Application settings and catalog cache are stored locally under `%APPDATA%\AMWin-LosslessSwitcher`, or beside the executable in portable mode.

This README does not make a zero-telemetry claim. That claim should only be added after the application source and its dependencies have been audited for analytics, telemetry, crash reporting, and other outbound data.

## Acknowledgments & Special Thanks

- [Jordan Gibson — WindowsLosslessSwitcher](https://github.com/jordanmgibson/WindowsLosslessSwitcher), for the Windows project concept and inspiration.
- [Vincent Neo — LosslessSwitcher](https://github.com/vincentneo/LosslessSwitcher), for pioneering automatic Apple Music sample rate switching on macOS.

## License & Legal

This project is licensed under the [GNU General Public License v3.0](LICENSE).

- © 2026 Pratama Putra.
- Not affiliated with Apple Inc. Apple and Apple Music are trademarks of Apple Inc.
