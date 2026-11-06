# About layout draft

Saved: 6 November 2026. Subsequently implemented in local v0.3.5 at the user's request.

## Problem

The current About page mixes left-aligned text with a centered standalone logo. Name/version spacing is tight, actions are scattered, and the Apple Music optimization line repeats the description.

## Proposed layout

```text
← Settings

[Logo]  RateMatch
        Version 0.3.4 · Windows x64

Automatically matches your output audio format
to Apple Music track metadata.

[GitHub]  [License]  [Privacy]

Developed by Pratama Putra
© 2026 · GPL-3.0-only

Not affiliated with Apple Inc.
```

- Align logo, name and version as one identity block.
- Separate description, links and attribution with consistent spacing.
- Remove the redundant “Optimized for Apple Music for Windows” line.
- Replace the displayed “All rights reserved” line with useful GPL license information; retain attribution and the controlling LICENSE text.
- Move Back to settings to the top.
- Show Mini mode on Now Playing rather than About.
- Provide functional GitHub, License and Privacy actions with keyboard focus.
- Use the package version dynamically. RateMatch is the current released name; RateAlign remains a candidate only.

Implemented using the existing Win32 layout and controls in src/ui/window.rs. The current product name is RateAlign and the displayed version is read from Cargo. Main/mini behavior is retained. This document preserves the original proposal; there is no new commit, push or release in this task.
