use std::collections::HashMap;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtwMediaFormat {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub is_lossless: bool,
    pub has_explicit_bit_depth: bool,
}

struct EtwState {
    initialized_saved_logs: bool,
    last_active_path: Option<PathBuf>,
    last_active_len: u64,
    last_active_scan: Option<Instant>,
    by_adam_id: HashMap<i64, EtwMediaFormat>,
    latest_active_adam_id: Option<i64>,
    latest_active_format: Option<(Option<i64>, EtwMediaFormat, u64)>,
    event_seq: u64,
}

pub struct EtwLogReader {
    logs_dir: Option<PathBuf>,
    state: Mutex<EtwState>,
}

impl EtwLogReader {
    pub fn new(initial_adam_map: HashMap<i64, EtwMediaFormat>) -> Self {
        let logs_dir = find_apple_music_logs_dir();
        Self {
            logs_dir,
            state: Mutex::new(EtwState {
                initialized_saved_logs: false,
                last_active_path: None,
                last_active_len: 0,
                last_active_scan: None,
                by_adam_id: initial_adam_map,
                latest_active_adam_id: None,
                latest_active_format: None,
                event_seq: 0,
            }),
        }
    }

    pub fn clear(&self) {
        if let Ok(mut st) = self.state.lock() {
            st.initialized_saved_logs = false;
            st.last_active_path = None;
            st.last_active_len = 0;
            st.last_active_scan = None;
            st.by_adam_id.clear();
            st.latest_active_adam_id = None;
            st.latest_active_format = None;
            st.event_seq = 0;
        }
    }

    pub fn export_adam_map(&self) -> HashMap<i64, EtwMediaFormat> {
        self.state
            .lock()
            .map(|st| st.by_adam_id.clone())
            .unwrap_or_default()
    }

    pub fn is_active_log_recent(&self) -> bool {
        let Some(ref dir) = self.logs_dir else {
            return false;
        };
        let Some((path, _)) = newest_etl_in_dir(dir) else {
            return false;
        };
        if let Ok(meta) = fs::metadata(path)
            && let Ok(modified) = meta.modified()
            && let Ok(elapsed) = SystemTime::now().duration_since(modified)
        {
            return elapsed <= Duration::from_secs(180);
        }
        false
    }

    pub fn get_lossless_format_for_adam_id(&self, adam_id: i64) -> Option<EtwMediaFormat> {
        self.refresh_logs_if_needed();
        let st = self.state.lock().ok()?;
        st.by_adam_id.get(&adam_id).copied().filter(|f| f.is_lossless)
    }

    pub fn latest_active_adam_id(&self) -> Option<i64> {
        self.refresh_logs_if_needed();
        let st = self.state.lock().ok()?;
        st.latest_active_adam_id
    }

    pub fn current_event_seq(&self) -> u64 {
        self.refresh_logs_if_needed();
        self.state.lock().map(|st| st.event_seq).unwrap_or(0)
    }

    pub fn get_latest_lossless_since(&self, min_seq: u64) -> Option<(Option<i64>, EtwMediaFormat)> {
        self.refresh_logs_if_needed();
        let st = self.state.lock().ok()?;
        if let Some((adam_id, fmt, seq)) = st.latest_active_format
            && seq > min_seq && fmt.is_lossless {
                return Some((adam_id, fmt));
            }
        None
    }

    fn refresh_logs_if_needed(&self) {
        let Some(ref logs_dir) = self.logs_dir else {
            return;
        };

        let need_saved = self
            .state
            .lock()
            .map(|st| !st.initialized_saved_logs)
            .unwrap_or(false);

        if need_saved {
            let saved_dir = logs_dir.join("SavedLogs");
            let mut etl_files = recent_etls_in_dir(&saved_dir, 5);
            if let Some((active_path, _)) = newest_etl_in_dir(logs_dir) {
                etl_files.push(active_path);
            }

            if !etl_files.is_empty()
                && let Some(xml) = run_tracerpt(&etl_files) {
                    if let Ok(mut st) = self.state.lock() {
                        parse_tracerpt_xml(&xml, &mut st);
                        st.initialized_saved_logs = true;
                        if let Some((active_path, active_len)) = newest_etl_in_dir(logs_dir) {
                            st.last_active_path = Some(active_path);
                            st.last_active_len = active_len;
                            st.last_active_scan = Some(Instant::now());
                        }
                    }
                    return;
                }

            if let Ok(mut st) = self.state.lock() {
                st.initialized_saved_logs = true;
            }
        }

        // Check if active ETL file grew or rotated
        let Some((active_path, active_len)) = newest_etl_in_dir(logs_dir) else {
            return;
        };

        let should_scan_active = if let Ok(st) = self.state.lock() {
            let path_changed = st.last_active_path.as_ref() != Some(&active_path);
            let size_changed = active_len != st.last_active_len && active_len > 0;
            let throttle_ok = st
                .last_active_scan
                .is_none_or(|t| t.elapsed() >= Duration::from_millis(1200));
            (path_changed || size_changed) && throttle_ok
        } else {
            false
        };

        if should_scan_active
            && let Some(xml) = run_tracerpt(std::slice::from_ref(&active_path))
                && let Ok(mut st) = self.state.lock() {
                    parse_tracerpt_xml(&xml, &mut st);
                    st.last_active_path = Some(active_path);
                    st.last_active_len = active_len;
                    st.last_active_scan = Some(Instant::now());
                }
    }
}

fn find_apple_music_logs_dir() -> Option<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let packages_dir = PathBuf::from(local_app_data).join("Packages");
    let entries = fs::read_dir(packages_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("AppleInc.AppleMusicWin_") {
            let logs = entry.path().join("LocalState").join("Logs");
            if logs.is_dir() {
                return Some(logs);
            }
        }
    }
    None
}

fn newest_etl_in_dir(dir: &Path) -> Option<(PathBuf, u64)> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(PathBuf, SystemTime, u64)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if !fname_str.starts_with("AMPLibraryAgent_") || !fname_str.ends_with(".etl") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            let len = meta.len();
            if len == 0 {
                continue;
            }
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            match best {
                Some((_, best_mod, _)) if modified <= best_mod => {}
                _ => best = Some((path, modified, len)),
            }
        }
    }

    best.map(|(p, _, l)| (p, l))
}

fn recent_etls_in_dir(dir: &Path, max_count: usize) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if !fname_str.starts_with("AMPLibraryAgent_") || !fname_str.ends_with(".etl") {
            continue;
        }
        if let Ok(meta) = entry.metadata()
            && meta.len() > 1024 {
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                files.push((path, modified));
            }
    }

    files.sort_by_key(|a| std::cmp::Reverse(a.1));
    files.truncate(max_count);
    files.reverse();
    files.into_iter().map(|(p, _)| p).collect()
}

fn run_tracerpt(etl_paths: &[PathBuf]) -> Option<String> {
    if etl_paths.is_empty() {
        return None;
    }

    let pid = std::process::id();
    let temp_xml = std::env::temp_dir().join(format!("amwin_etw_{}.xml", pid));
    let _ = fs::remove_file(&temp_xml);

    let mut cmd = Command::new("tracerpt.exe");
    for p in etl_paths {
        cmd.arg(p);
    }
    cmd.arg("-o")
        .arg(&temp_xml)
        .arg("-of")
        .arg("XML")
        .arg("-lr")
        .arg("-y")
        .creation_flags(CREATE_NO_WINDOW);

    let status = cmd.output().ok()?;
    if !status.status.success() && !temp_xml.exists() {
        let _ = fs::remove_file(&temp_xml);
        return None;
    }

    let bytes = fs::read(&temp_xml).ok()?;
    let _ = fs::remove_file(&temp_xml);
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_tracerpt_xml(xml: &str, state: &mut EtwState) {
    let mut current_adam_id: Option<i64> = None;

    for line in xml.lines() {
        let Some(start_idx) = line.find("<Data Name=\"StringMessage\">") else {
            continue;
        };
        let rest = &line[start_idx + "<Data Name=\"StringMessage\">".len()..];
        let Some(end_idx) = rest.find("</Data>") else {
            continue;
        };
        let msg = &rest[..end_idx];

        if msg.contains("song changed") || msg.contains("RemoveAllItems") {
            current_adam_id = None;
            state.latest_active_format = None;
            continue;
        }

        if let Some(id_str) = msg.strip_prefix("adamID:") {
            if let Ok(id) = id_str.trim().parse::<i64>()
                && id > 0 {
                    current_adam_id = Some(id);
                    state.latest_active_adam_id = Some(id);
                }
            continue;
        }

        if let Some(pos) = msg.find("/songs/") {
            let after_songs = &msg[pos + 7..];
            if let Some(slash_pos) = after_songs.find("/syllable-lyrics") {
                if let Ok(id) = after_songs[..slash_pos].trim().parse::<i64>()
                    && id > 0 {
                        current_adam_id = Some(id);
                        state.latest_active_adam_id = Some(id);
                    }
                continue;
            }
        }

        if msg.contains("mediaFormatinfo")
            && let Some(parsed) = parse_media_format_line(msg) {
                state.event_seq = state.event_seq.wrapping_add(1);
                state.latest_active_format = Some((current_adam_id, parsed, state.event_seq));

                if let Some(adam_id) = current_adam_id {
                    let replace = match state.by_adam_id.get(&adam_id) {
                        None => true,
                        Some(prev) => should_replace_format(prev, &parsed),
                    };
                    if replace {
                        state.by_adam_id.insert(adam_id, parsed);
                    }
                }
            }
    }
}

pub fn parse_media_format_line(msg: &str) -> Option<EtwMediaFormat> {
    let is_qlac = msg.contains("asbdFormatID = qlac");
    let is_qaac = msg.contains("asbdFormatID = qaac");
    let sd_alac = msg.contains("sdFormatID = alac");
    let sd_aac = msg.contains("sdFormatID = aac");
    let is_hires = msg.contains("high res lossless");
    let is_lossless_label = msg.contains("lossless") && !msg.contains("(lossy)");

    // Filter out transitional stream handoff events where ASBD and SampleDescription formats disagree
    if (is_qaac && (sd_alac || is_hires || is_lossless_label)) || (is_qlac && sd_aac) {
        return None;
    }

    let is_lossless = is_qlac || sd_alac || is_hires || is_lossless_label;

    let rate_prefix = "asbdSampleRate = ";
    let rate_start = msg.find(rate_prefix)? + rate_prefix.len();
    let rate_rest = &msg[rate_start..];
    let rate_end = rate_rest.find(" kHz")?;
    let khz_val: f64 = rate_rest[..rate_end].trim().parse().ok()?;
    let sample_rate = (khz_val * 1000.0).round() as u32;
    if sample_rate == 0 {
        return None;
    }

    let depth_prefix = "sdBitDepth = ";
    let (bit_depth, has_explicit_bit_depth) = if let Some(pos) = msg.find(depth_prefix) {
        let depth_rest = &msg[pos + depth_prefix.len()..];
        if let Some(bit_end) = depth_rest.find(" bit") {
            if let Ok(bits) = depth_rest[..bit_end].trim().parse::<u16>() {
                (bits, true)
            } else {
                (if is_hires || sample_rate > 48000 { 24 } else { 16 }, false)
            }
        } else {
            (if is_hires || sample_rate > 48000 { 24 } else { 16 }, false)
        }
    } else {
        (if is_hires || sample_rate > 48000 { 24 } else { 16 }, false)
    };

    Some(EtwMediaFormat {
        sample_rate,
        bit_depth,
        is_lossless,
        has_explicit_bit_depth,
    })
}

fn should_replace_format(prev: &EtwMediaFormat, next: &EtwMediaFormat) -> bool {
    if next.is_lossless && !prev.is_lossless {
        return true;
    }
    if !next.is_lossless && prev.is_lossless {
        return false;
    }
    if next.has_explicit_bit_depth && !prev.has_explicit_bit_depth {
        return true;
    }
    if !next.has_explicit_bit_depth && prev.has_explicit_bit_depth {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_media_format_lines() {
        let sample_192k_line = "play&gt; cm&gt;&gt; mediaFormatinfo &apos;000002CCAA29FF4C&apos; , songEnhanced, audioCapabilities: 0x0 -&gt; 0x10, 0x0 -&gt; 0x10, asbdFormatID = qlac, sdFormatID = alac, high res lossless, asbdNumChannels = 2, sdNumChannels = 2, sdBitDepth = 24 bit, asbdSampleRate = 192.0 kHz, is not rendering spatial audio";
        let parsed_192k = parse_media_format_line(sample_192k_line).unwrap();
        assert_eq!(parsed_192k.sample_rate, 192000);
        assert_eq!(parsed_192k.bit_depth, 24);
        assert!(parsed_192k.is_lossless);
        assert!(parsed_192k.has_explicit_bit_depth);

        let sample_96k_line = "play&gt; cm&gt;&gt; mediaFormatinfo &apos;000001DDB43ABF2C&apos; , songEnhanced, audioCapabilities: 0x0 -&gt; 0x10, 0x0 -&gt; 0x10, asbdFormatID = qlac, sdFormatID = alac, high res lossless, asbdNumChannels = 2, sdNumChannels = 2, sdBitDepth = 24 bit, asbdSampleRate = 96.0 kHz, is not rendering spatial audio";
        let parsed_96k = parse_media_format_line(sample_96k_line).unwrap();
        assert_eq!(parsed_96k.sample_rate, 96000);
        assert_eq!(parsed_96k.bit_depth, 24);
        assert!(parsed_96k.is_lossless);

        // Transitional qaac -> alac line should be ignored
        let transitional = "play&gt; cm&gt;&gt; mediaFormatinfo &apos;000002CCA8C0326C&apos; , songEnhanced, audioCapabilities: 0x8 -&gt; 0x10, 0x8 -&gt; 0x10, asbdFormatID = qaac, sdFormatID = alac, high res lossless, asbdNumChannels = 2, sdNumChannels = 2, sdBitDepth = 24 bit, asbdSampleRate = 48.0 kHz, is not rendering spatial audio";
        assert!(parse_media_format_line(transitional).is_none());
    }
}
