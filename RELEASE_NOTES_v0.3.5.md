# RateAlign v0.3.5

Local build; not yet published.

- Renames the application and executable from RateMatch to RateAlign.
- Applies the stepped signal-alignment logo to the executable, window, tray, About and notification icon.
- Sets a consistent Windows AppUserModelID and registers the RateAlign display name/icon for notification attribution. Installer shortcuts use the same identity.
- Rebuilds About around one identity block with version, description, GitHub, License, Privacy and attribution. Documents open locally in Notepad where available, with an online fallback.
- Reorganizes Settings with separate mode descriptions and more vertical spacing.
- Unifies action-button drawing, height, borders, pressed state and keyboard focus. Radio buttons, checkboxes, sliders and device selection remain native.
- Adds Settings → Test notification to check attribution without changing audio formats.

Audio switching behavior, settling delays, legacy settings storage, startup registry key and single-instance mutex are retained. The GitHub repository is not renamed. Version 0.3.4 remains a historical release.

Validation: release build and local tests; main, Settings, About and mini mode are checked manually as available. The user confirmed that the notification sender displays RateAlign. This confirmation does not constitute verification of every Windows version, DPI setting or audio device.
