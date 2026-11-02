================================================================================
  LosslessSwitcher For Apple Music (Windows x64) - Version 0.3.0
================================================================================
  Author  : Pratama Putra
  GitHub  : https://github.com/pratama1246/AMWin-LosslessSwitcher
  License : GNU General Public License v3.0 (GPL-3.0)
================================================================================

[OVERVIEW]
LosslessSwitcher For Apple Music (AMWin-LosslessSwitcher) is a lightweight,
native Windows 64-bit utility that monitors Apple Music playback and
automatically synchronizes your selected Windows audio endpoint format
(DAC sample rate) with the currently playing track.

By default, Apple Music for Windows does not adjust the Windows audio
endpoint format to match each song's native sample rate. This tool resolves
the track format using Apple Music metadata and local telemetry, then updates
the Windows audio endpoint accordingly to prevent unwanted OS resampling.

The application changes the Windows endpoint format only. It does not
intercept, capture, re-encode, or alter the digital audio stream.


--------------------------------------------------------------------------------
[AUDIO MODES EXPLAINED]
--------------------------------------------------------------------------------

1. SEAMLESS MODE (Recommended):
   - Keeps your DAC/audio endpoint fixed at 24-bit while dynamically switching
     sample rates (44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, 176.4 kHz, 192 kHz).
   - Ideal for modern USB DACs, preventing clicks, pops, and driver re-locks
     when transitioning between 16-bit and 24-bit tracks.

2. PURIST MODE:
   - Strictly switches both the sample rate AND the bit depth to match the
     exact track specification (e.g. 44.1 kHz / 16-bit for standard CD lossless,
     96 kHz / 24-bit for Hi-Res).
   - Some DAC drivers may produce an audible pop when changing bit depth containers.


--------------------------------------------------------------------------------
[AUDIO SETTINGS & GUIDELINES]
--------------------------------------------------------------------------------

- Output Device:
  Choose your specific DAC / sound card from the dropdown, or select
  "Auto (Follow Windows Default)" to automatically switch whichever device is
  currently active in Windows.

- Settling Delay (50 - 800 ms, default: 200 ms):
  Provides buffer time for your DAC's internal clock (PLL) to stabilize
  between sample rate transitions. If your DAC occasionally drops the start
  of a song or clicks during track changes, increase this value to 300-400 ms.

- Optional Apple Music Settings:
  1. Open Apple Music -> Settings -> Playback.
  2. Turn on "Lossless Audio" and set it to "Hi-Res Lossless (up to 24-bit/192 kHz)".
  3. For standard stereo playback without DSP alterations, consider disabling
     "Sound Check", "Dolby Atmos", and "Song Transitions".

- Optional Windows Settings:
  1. Open Windows Settings -> System -> Sound -> More sound settings.
  2. Open your DAC's Properties -> Advanced / Enhancements tab.
  3. Disable "Audio Enhancements" and "Spatial Audio" for a clean audio path.


--------------------------------------------------------------------------------
[IMPORTANT TECHNICAL NOTICE]
--------------------------------------------------------------------------------
Sample Rate Synchronization Is NOT "Bit-Perfect" Exclusive Mode:

AMWin-LosslessSwitcher matches the Windows audio endpoint format to the active
track to eliminate sample rate conversion (SRC) by the Windows Audio Engine.
However, please note:
- Apple Music for Windows operates exclusively through the Windows Shared
  Audio Engine (WASAPI Shared). Apple Music does not offer ASIO or WASAPI
  Exclusive Mode.
- System volume, notification sounds, Windows audio enhancements, and driver
  DSP can still interact with playback.
- This tool changes endpoint formats; it does not bypass the Windows audio stack.


--------------------------------------------------------------------------------
[COMMAND LINE ARGUMENTS & PORTABLE MODE]
--------------------------------------------------------------------------------

The executable accepts the following command-line flags:

  (no arguments)        Launch the application window normally.
  --minimized / --tray  Start the application minimized directly to system tray.
  --portable            Store settings and cache files next to the executable
                        instead of %APPDATA%\AMWin-LosslessSwitcher.

Portable Mode:
You can also enable portable mode simply by placing an empty file named
"portable.dat" in the same folder as the executable.


--------------------------------------------------------------------------------
[UNINSTALLATION & SUPPORT]
--------------------------------------------------------------------------------

- Installed Version:
  Go to Windows Settings -> Apps -> Installed apps -> "LosslessSwitcher For
  Apple Music" and click Uninstall.

- Portable Version:
  Simply delete the folder containing the executable and its adjacent files.

- Report Issues / Feature Requests:
  https://github.com/pratama1246/AMWin-LosslessSwitcher/issues


================================================================================
  LosslessSwitcher for Apple Music is licensed under GNU GPL v3.0.
  Copyright (C) 2026 Pratama Putra.
  Not affiliated with Apple Inc. Apple and Apple Music are trademarks of Apple Inc.
================================================================================
