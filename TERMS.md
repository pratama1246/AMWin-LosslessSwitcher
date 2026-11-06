# RateAlign usage information

Applies to v0.3.5. Updated 6 November 2026.

## License

RateAlign is distributed under GNU GPL version 3 only. [LICENSE](LICENSE) is the controlling license. This document explains operation and limitations; it adds no restrictions to GPL rights. Use, modification and redistribution are governed by that license.

## Operation and limitations

RateAlign changes the selected Windows audio endpoint's sample format based on resolved Apple Music metadata. Other programs and system sounds may share that endpoint. Switching can interrupt audio, produce audible transitions, or fall back to a supported format. Try new devices/modes at a comfortable listening volume.

Normal exit attempts to restore captured endpoint formats. Crashes, forced termination, disconnected devices or driver failures may prevent restoration. Matching metadata does not guarantee bit-perfect playback, higher source quality, Exclusive Mode, ASIO, or bypassing Windows processing. Seamless mode can intentionally use 24-bit output for a source with another depth. Clamped is a supported fallback, not an exact match. RateAlign does not remove DRM or grant rights to music/subscriptions.

Apple Music/services, Windows, devices and drivers are controlled by their respective providers. Compatibility can change. RateAlign is independent and is not endorsed by Apple or Microsoft. Names and artwork belong to their respective owners.

## Privacy, warranty and support

Read [PRIVACY.md](PRIVACY.md) for local data handling and external requests. The GPL's warranty and liability provisions apply to the extent permitted by applicable law. This notice does not override mandatory legal rights or introduce another waiver. No guaranteed device compatibility or service-level commitment is offered here.

Report reproducible issues through the [issue tracker](https://github.com/pratama1246/AMWin-LosslessSwitcher/issues), with sensitive data removed.
