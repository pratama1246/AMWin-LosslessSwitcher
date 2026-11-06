use crate::coordinator::CoordinatorStatus;

pub fn summary(status: &CoordinatorStatus) -> (String, &'static str, u32) {
    let output = status.current_device_format.map(|f| f.display_str()).unwrap_or_else(|| "— kHz / — bit".into());
    let message = status.status_message.as_str();
    if status.current_track.is_none() {
        return ("— kHz / — bit".into(), "Idle", 0x0073665C);
    }
    if message.contains("unavailable") || message.contains("unsupported") || message.contains("Failed") {
        return (output, "Output unavailable", 0x003333B0);
    }
    if status.is_syncing || message.starts_with("Retrying") {
        return (output, "Switching…", 0x000080A0);
    }
    if message.contains("(Clamped)") {
        let format = match (status.target_format, status.current_device_format) {
            (Some(source), Some(applied)) if source.sample_rate != applied.sample_rate => format!("{} → {}", source.sample_rate as f64 / 1000.0, applied.display_str()),
            _ => output,
        };
        return (format, "Limited by device", 0x000080A0);
    }
    if message.starts_with("Synchronized:") {
        if !status.is_lossless { return (output, "AAC playback", 0x0073665C); }
        if status.target_format == status.current_device_format {
            return (output, "Format matched", 0x00786B00);
        }
        return (output, "Rate synchronized", 0x00786B00);
    }
    (output, "Waiting for playback", 0x0073665C)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{audio::AudioFormat, coordinator::SwitcherMode, apple_music::TrackInfo};

    #[test]
    fn mini_summary_preserves_fallback_and_seamless_states() {
        let mut s = CoordinatorStatus {
            current_track: Some(TrackInfo { title: "Test".into(), artist: "Artist".into(), album: "Album".into(), duration_secs: None, smtc_artwork_bytes: None }),
            current_device_format: Some(AudioFormat::new(96000, 24)),
            target_format: Some(AudioFormat::new(192000, 24)),
            artwork_bytes: None, status_message: "Synchronized: 96 kHz (Clamped)".into(),
            is_syncing: false, is_lossless: true, mode: SwitcherMode::Seamless, settling_delay_ms: 300,
        };
        assert_eq!(summary(&s).1, "Limited by device");
        assert!(summary(&s).0.contains("192 → 96"));
        s.is_syncing = true;
        assert_eq!(summary(&s).1, "Switching…");
        s.is_syncing = false;
        s.status_message = "Synchronized: 96 kHz".into();
        s.target_format = Some(AudioFormat::new(96000, 16));
        assert_eq!(summary(&s).1, "Rate synchronized");
        s.target_format = s.current_device_format;
        assert_eq!(summary(&s).1, "Format matched");
        s.current_track = None;
        assert_eq!(summary(&s).1, "Idle");
    }
}
