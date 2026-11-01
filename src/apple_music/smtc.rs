use windows::core::Result;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: Option<u64>,
    pub smtc_artwork_bytes: Option<Vec<u8>>,
}

pub struct SmtcManager {
    manager: GlobalSystemMediaTransportControlsSessionManager,
}

impl SmtcManager {
    pub fn new() -> Result<Self> {
        let async_op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?;
        let manager = async_op.get()?;
        Ok(Self { manager })
    }

    pub fn get_apple_music_session(&self) -> Option<GlobalSystemMediaTransportControlsSession> {
        if let Ok(sessions) = self.manager.GetSessions() {
            let count = sessions.Size().unwrap_or(0);
            for i in 0..count {
                if let Ok(session) = sessions.GetAt(i) {
                    if let Ok(app_id) = session.SourceAppUserModelId() {
                        let app_id_str = app_id.to_string();
                        let app_id_lower = app_id_str.to_lowercase();
                        if app_id_lower.contains("applemusic") 
                            || app_id_lower.contains("amplibraryagent")
                            || app_id_lower.contains("appleinc.applemusic") {
                            return Some(session);
                        }
                    }
                }
            }
        }
        // Strict isolation: never fallback to GetCurrentSession()
        // to prevent other media apps (Spotify, YouTube in Chrome, VLC) from triggering the switcher.
        None
    }

    pub fn get_track_info(&self, session: &GlobalSystemMediaTransportControlsSession) -> Option<TrackInfo> {
        let props_op = session.TryGetMediaPropertiesAsync().ok()?;
        let props = props_op.get().ok()?;
        let title = props.Title().ok()?.to_string();
        let artist_raw = props.Artist().ok()?.to_string();
        let mut album = props.AlbumTitle().ok().map(|s| s.to_string()).unwrap_or_default();

        if title.is_empty() || artist_raw.is_empty() {
            return None;
        }

        let mut artist = artist_raw.clone();
        if album.is_empty() {
            // Apple Music Windows formats artist as "Artist — Album" or "Artist - Album"
            for sep in [" — ", " – ", " —", " –", " - "] {
                if let Some((a, alb)) = artist_raw.split_once(sep) {
                    let a_trim = a.trim();
                    let alb_trim = alb.trim();
                    if !a_trim.is_empty() && !alb_trim.is_empty() {
                        artist = a_trim.to_string();
                        album = alb_trim.to_string();
                        break;
                    }
                }
            }
        }

        let duration_secs = session.GetTimelineProperties().ok()
            .and_then(|tl| tl.EndTime().ok())
            .map(|t| (t.Duration / 10_000_000).max(0) as u64)
            .filter(|&d| d > 0);

        let smtc_artwork_bytes = extract_thumbnail_bytes(&props);

        Some(TrackInfo {
            title,
            artist,
            album,
            duration_secs,
            smtc_artwork_bytes,
        })
    }

    pub fn is_playing(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(playback_info) = session.GetPlaybackInfo() {
            if let Ok(status) = playback_info.PlaybackStatus() {
                return status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
            }
        }
        false
    }

    pub fn is_active_or_playing(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(playback_info) = session.GetPlaybackInfo() {
            if let Ok(status) = playback_info.PlaybackStatus() {
                return status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                    || status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing;
            }
        }
        false
    }

    pub fn is_changing(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(playback_info) = session.GetPlaybackInfo() {
            if let Ok(status) = playback_info.PlaybackStatus() {
                return status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing;
            }
        }
        false
    }

    pub fn is_paused(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(playback_info) = session.GetPlaybackInfo() {
            if let Ok(status) = playback_info.PlaybackStatus() {
                return status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused;
            }
        }
        false
    }

    pub fn try_pause(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(op) = session.TryPauseAsync() {
            if let Ok(success) = op.get() {
                return success;
            }
        }
        false
    }

    pub fn try_play(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        if let Ok(op) = session.TryPlayAsync() {
            if let Ok(success) = op.get() {
                return success;
            }
        }
        false
    }


    pub fn resume_playback_reliable(&self, session: &GlobalSystemMediaTransportControlsSession) -> bool {
        // If Apple Music is currently buffering or changing tracks, allow buffer to stabilize
        let mut wait_count = 0;
        while self.is_changing(session) && wait_count < 8 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            wait_count += 1;
        }

        // Send gentle play commands with retry backoff (strictly TryPlayAsync, never Toggle)
        // This handles cases where the DAC was switching clock crystals (e.g. 96k -> 44.1k)
        // and Apple Music's audio output stream was still re-binding during the first call.
        for _ in 0..4 {
            let _ = self.try_play(session);

            let wait_deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
            while std::time::Instant::now() < wait_deadline {
                if self.is_playing(session) {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }

            if self.is_playing(session) {
                return true;
            }

            if self.is_changing(session) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        self.is_playing(session)
    }
}

fn extract_thumbnail_bytes(
    props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Option<Vec<u8>> {
    let thumb_ref = props.Thumbnail().ok()?;
    let stream_op = thumb_ref.OpenReadAsync().ok()?;
    let stream = stream_op.get().ok()?;
    let size = stream.Size().ok()? as usize;
    if size == 0 || size > 10 * 1024 * 1024 {
        return None;
    }

    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream).ok()?;
    let load_op = reader.LoadAsync(size as u32).ok()?;
    let _ = load_op.get().ok()?;

    let mut bytes = vec![0u8; size];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}
