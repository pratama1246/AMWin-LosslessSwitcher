# RateMatch privacy notice

Version 0.3.4. Updated 6 November 2026.

RateMatch is developed by Pratama Putra and currently supports Apple Music for Windows. It is not affiliated with Apple. This notice describes the source in this repository.

## Data used locally

The application reads the current media session's title, artist, album, duration, artwork and playback state through Windows. It enumerates output device names/identifiers, formats and capabilities. Settings include the selected device, switching mode, settling delay and always-on-top preference.

Format resolution reads Apple playback diagnostic ETL files in the Apple Music package's local Logs directory, including SavedLogs. Windows tracerpt.exe converts these into a temporary XML file named amwin_etw_<process-id>.xml. RateMatch attempts to remove this temporary file after conversion; crashes or interruptions can leave it behind. The application can inspect downloaded package manifests under the current user's Music/Apple Music/Media/Apple Music folder. Paths and logs may contain music metadata and local account information.

## External requests

Catalog searches send song/artist/album search terms and a storefront code to Apple services. Song IDs may be used for detail queries. The application requests Apple's music.apple.com page and associated scripts to obtain a public web developer token, which is sent in catalog headers. This is not an Apple ID password or a user login token collected by RateMatch.

Source endpoints include music.apple.com, amp-api.music.apple.com and itunes.apple.com. Artwork URLs returned by those services may use Apple content-delivery hosts. Receiving services can observe connection information such as the public IP address. Apple determines its service-side processing and retention; this project cannot promise deletion of Apple's logs. See [Apple's privacy policy](https://www.apple.com/legal/privacy/).

The application source does not upload ETL/XML files or audio samples to a developer-operated server. It does not record or re-encode playback audio. Online lookup and artwork downloads mean that normal use is not fully offline.

## Storage and retention

Normal storage is %APPDATA%/AMWin-LosslessSwitcher; the old folder name is retained for compatibility. Portable mode stores files beside the executable when --portable is supplied, portable.dat exists there, or settings.json already exists there.

- settings.json stores preferences and the selected device ID/name.
- catalog_cache.json can contain verified track keys, formats, song IDs, artwork URLs, correlated format information and the public web developer token. Track keys may reveal music you have played. This is a resolution cache, not a timestamped listening-history export.
- Artwork bytes are cached in memory; artwork URLs may be persisted.
- Startup behavior is configured through a Windows user registry entry.

Files remain until cleared, replaced, removed, or cleaned by installation/uninstallation. Do not assume automatic expiry or encryption. Other users/programs with access to the files may read them.

## Your controls

Use Settings → Clear cache to remove the catalog cache. Playback may repopulate it. Quit RateMatch before manually removing its data folder. Disable Start with Windows before removing a portable copy. Closing the window hides it; exit from the tray menu to stop monitoring. Blocking network access prevents online lookup/artwork retrieval and may reduce accuracy; local telemetry/manifests may still be usable.

The existing installer removes legacy application data folders during install/uninstall. Back up preferences before upgrading if needed. Portable files beside the executable require separate cleanup.

## Support

Use the [issue tracker](https://github.com/pratama1246/AMWin-LosslessSwitcher/issues) for questions. Issues are public: review/redact logs, cache files, tokens and local paths before sharing. Update this notice whenever data sources, outbound requests or storage behavior change.
