# UI refresh — v0.3.3

Status: planned; application code and release version have not been changed.

## Scope

Refresh the existing Rust/Win32 interface for Windows 10 and 11. Keep the current logo for this release. Preserve audio switching, settings persistence, device selection, startup behavior, notifications, and tray behavior.

## Visual direction

A compact audio utility with a light enclosure and one recessed charcoal display for source and output formats. Use subtle depth on action buttons, native input controls, and flat navigation. Avoid decorative meters, animated waveforms, continuous blur, and a large sidebar.

Palette: background #F3F5F7, content #FFFFFF, display #20262E, primary text #18212B, secondary text #5C6673, active accent #006B78. Reserve status colors for actual states and always accompany them with text. Use installed Segoe UI; no downloaded fonts.

Use 8-unit spacing increments, about 24 units of outer padding, restrained 6–8 unit corner radii, and a native title bar. Size the window around the settings content rather than enlarging it to fill space. All dimensions are logical units at 96 DPI and must scale with the existing DPI handling.

## Now Playing

```text
LosslessSwitcher                            [Settings]

[ Artwork ]   NOW PLAYING
[         ]   Track title
[         ]   Artist
              Waiting / Detecting / Switching / Result

+------------------------------------------------+
| SOURCE                    OUTPUT               |
| 96 kHz / 24-bit    ->      96 kHz / 24-bit      |
+------------------------------------------------+

Output device name
Seamless mode                       [Hide to tray]
```

- Separate track title and artist; allow two lines for the title and use width-aware ellipsis rather than fixed character limits.
- Show both resolved source/target format and current endpoint format clearly. The current target_format field is not an independent PCM measurement; verify its semantics before naming the source label.
- Waiting state: “Play a song in Apple Music to begin.” Show a neutral artwork placeholder and unavailable values as an em dash.
- Represent detecting, switching, unsupported-format fallback, AAC, and errors distinctly. Do not derive a successful state from format equality alone when the coordinator reports an error or fallback.
- Use “Format matched” only when justified; never show a bit-perfect guarantee. Seamless mode can intentionally use a different bit depth.
- Display the selected device and active mode without duplicating all settings on this screen.

## Settings

Replace nested group-box borders with section headings, whitespace, and quiet dividers. Keep native dropdowns, radio buttons, checkboxes, and slider behavior.

1. Audio output: device dropdown and Refresh.
2. Switching mode: Seamless and Purist, each with a short description on a separate line. Avoid promising pop-free switching on every device.
3. Behavior: Always on top; Run at Windows startup, with tray behavior explained.
4. Advanced: settling delay with visible millisecond value; Clear cache as a separate action.
5. About: current version, attribution, disclaimer, and GitHub link, reachable from Settings without a permanent third top-level tab.

## Implementation sequence

1. Inspect the existing navigation, control lifetime, DPI scaling, and coordinator status transitions. Capture a baseline before changing the UI.
2. Update layout, navigation, typography, title/artist labels, and the format display. Reuse the existing GDI/GDI+ drawing and bitmap loading path.
3. Reorganize Settings and About. Add keyboard tab order, visible focus, and accessible labels to navigation and controls. Keep native button semantics; custom drawing must preserve focus and pressed states.
4. Update only changed text/bitmaps during the existing status refresh. Avoid introducing a rendering loop or continuous animation. Retain existing audio timing.
5. Set package, lockfile, installer, resource, and manifest versions consistently to 0.3.3 where applicable. Add release notes describing the delivered changes.

Keep changes concentrated in src/ui/window.rs initially. Extract drawing/layout helpers only if the implementation needs them. No new GUI framework or dependency is planned. Optional Mica and dark mode are deferred.

## Verification

- Build and run the existing relevant tests; distinguish pre-existing failures from regressions.
- Inspect live Now Playing, Settings, and About at 100%, 125%, 150%, and 200% scaling where available. Check monitor DPI changes, long titles, long device names, and empty artwork.
- Verify keyboard navigation, radio selection, dropdown, slider, focus visibility, and high-contrast behavior. Preserve standard window controls.
- Check waiting, playback, format changes, AAC, unavailable device, and fallback states where the environment permits; report any untested cases.
- Verify settings survive restart, startup behavior remains intact, and tray hide/restore/exit works.
- Compare idle CPU and memory against the baseline, including hidden-to-tray behavior. Report measurements rather than promising unchanged performance.
- Deliver a reviewable build and screenshots. Do not publish a release or commit changes without a separate request.

## Deferred

New logo, v1 branding, playback controls, visualizers, theme switching, and window backdrop effects.
