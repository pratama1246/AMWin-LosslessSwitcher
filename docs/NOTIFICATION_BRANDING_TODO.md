# Notification sender branding

Saved: 6 November 2026. Subsequently implemented in local v0.3.5 at the user's request.

## Reported issue

The Windows notification header identifies the sender as **lossless-switcher-am.exe**. The requested fix concerns this sender attribution, not the notification title or message body.

## Desired behavior

Show the chosen product name as the notification sender. The current released name is RateMatch; RateAlign is a saved candidate only. Confirm the final branding from the current project decision when implementing.

## Follow-up investigation

- Verify how Windows attributes the existing Shell_NotifyIconW notifications for both standalone/portable and installed builds.
- Check executable metadata, Windows application identity, and shortcut/registration requirements. Do not assume changing the message title or VERSIONINFO alone fixes sender attribution.
- Test an actual notification on Windows after restarting the new build, including any effect of cached shell identity.
- Preserve existing settings and startup behavior when changing identifiers or executable names.

## Existing uncommitted work

Before this clarification, lossless-switcher.rc received RateMatch VERSIONINFO and the notification title in src/ui/window.rs was changed to “RateMatch — Output format changed.” These changes have not been verified to fix sender attribution. The attempted release build was blocked because the running executable could not be replaced. Do not report the sender issue as resolved.

## Applied follow-up

Local v0.3.5 uses RateAlign.exe, RateAlign executable metadata, and the PratamaPutra.RateAlign process AppUserModelID. It registers DisplayName and IconUri under HKCU/Software/Classes/AppUserModelId/PratamaPutra.RateAlign. Installer shortcuts use the same ID. Settings includes Test notification; the user confirmed that sender attribution now displays RateAlign. A custom balloon icon uses the new embedded logo. The original investigation notes above are retained as history. No new commit, push or release is included.
