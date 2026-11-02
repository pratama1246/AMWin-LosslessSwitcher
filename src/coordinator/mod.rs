use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::audio::{AudioFormat, policy_config::PolicyConfig, wasapi::WasapiManager};
use crate::apple_music::{CatalogResolver, SmtcManager, TrackInfo};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitcherMode {
    Seamless,
    Purist,
}

#[derive(Debug, Clone)]
pub struct CoordinatorStatus {
    pub current_track: Option<TrackInfo>,
    pub current_device_format: Option<AudioFormat>,
    pub target_format: Option<AudioFormat>,
    pub artwork_bytes: Option<Vec<u8>>,
    pub status_message: String,
    pub is_syncing: bool,
    pub is_lossless: bool,
    pub mode: SwitcherMode,
    pub settling_delay_ms: u64,
}

pub struct Coordinator {
    status: Arc<Mutex<CoordinatorStatus>>,
    selected_device_id: Arc<Mutex<Option<String>>>,
    shutdown_flag: Arc<Mutex<bool>>,
    initial_device_formats: Arc<Mutex<std::collections::HashMap<String, AudioFormat>>>,
    thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    catalog: Arc<CatalogResolver>,
    last_processed_track_key: Arc<Mutex<String>>,
}

impl Coordinator {
    pub fn new(mode: SwitcherMode, settling_delay_ms: u64) -> Self {
        let status = Arc::new(Mutex::new(CoordinatorStatus {
            current_track: None,
            current_device_format: None,
            target_format: None,
            artwork_bytes: None,
            status_message: "Initializing...".to_string(),
            is_syncing: false,
            is_lossless: true,
            mode,
            settling_delay_ms,
        }));

        let coord = Self {
            status,
            selected_device_id: Arc::new(Mutex::new(None)),
            shutdown_flag: Arc::new(Mutex::new(false)),
            initial_device_formats: Arc::new(Mutex::new(std::collections::HashMap::new())),
            thread_handle: Arc::new(Mutex::new(None)),
            catalog: Arc::new(CatalogResolver::new()),
            last_processed_track_key: Arc::new(Mutex::new(String::new())),
        };

        coord.capture_initial_formats();
        coord
    }

    pub fn capture_initial_formats(&self) {
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        if let Ok(wasapi) = WasapiManager::new() {
            if let Ok(dev_id) = wasapi.get_default_device_id() {
                if let Ok(policy) = PolicyConfig::new() {
                    if let Ok(fmt) = policy.get_device_format(&dev_id) {
                        let mut map = self.initial_device_formats.lock().unwrap();
                        map.entry(dev_id).or_insert(fmt);
                    }
                }
            }
        }
    }

    pub fn get_status(&self) -> CoordinatorStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn set_mode(&self, mode: SwitcherMode) {
        let mut st = self.status.lock().unwrap();
        st.mode = mode;
    }

    pub fn set_settling_delay(&self, ms: u64) {
        let mut st = self.status.lock().unwrap();
        st.settling_delay_ms = ms;
    }

    pub fn set_device_id(&self, device_id: Option<String>) {
        let mut dev = self.selected_device_id.lock().unwrap();
        *dev = device_id;
    }

    pub fn get_device_id(&self) -> Option<String> {
        self.selected_device_id.lock().unwrap().clone()
    }

    pub fn restore_initial_formats(&self) {
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        let formats: Vec<(String, AudioFormat)> = {
            let guard = self.initial_device_formats.lock().unwrap();
            guard.iter().map(|(k, v)| (k.clone(), *v)).collect()
        };
        if !formats.is_empty() {
            if let Ok(policy_config) = PolicyConfig::new() {
                for (dev_id, orig_fmt) in formats {
                    let _ = policy_config.set_device_format(&dev_id, &orig_fmt);
                }
            }
        }
    }

    pub fn clear_cache(&self) {
        self.catalog.clear_cache();
        if let Ok(mut key) = self.last_processed_track_key.lock() {
            key.clear();
        }
    }

    pub fn shutdown(&self) {
        {
            let mut flag = self.shutdown_flag.lock().unwrap();
            *flag = true;
        }
        let handle = {
            let mut h = self.thread_handle.lock().unwrap();
            h.take()
        };
        if let Some(h) = handle {
            let _ = h.join();
        }
        self.restore_initial_formats();
    }

    pub fn start(&self) {
        self.capture_initial_formats();
        let status_arc = Arc::clone(&self.status);
        let device_id_arc = Arc::clone(&self.selected_device_id);
        let shutdown_arc = Arc::clone(&self.shutdown_flag);
        let initial_fmts_arc = Arc::clone(&self.initial_device_formats);
        let catalog_arc = Arc::clone(&self.catalog);
        let last_track_key_arc = Arc::clone(&self.last_processed_track_key);

        let handle = thread::spawn(move || {
            let _ = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            };

            let wasapi = match WasapiManager::new() {
                Ok(w) => w,
                Err(_) => {
                    let mut st = status_arc.lock().unwrap();
                    st.status_message = "WASAPI Init Failed".to_string();
                    return;
                }
            };

            let policy_config = match PolicyConfig::new() {
                Ok(p) => p,
                Err(_) => {
                    let mut st = status_arc.lock().unwrap();
                    st.status_message = "PolicyConfig Init Failed".to_string();
                    return;
                }
            };

            let smtc = match SmtcManager::new() {
                Ok(s) => s,
                Err(_) => {
                    let mut st = status_arc.lock().unwrap();
                    st.status_message = "SMTC Init Failed".to_string();
                    return;
                }
            };

            {
                let mut st = status_arc.lock().unwrap();
                st.status_message = format!("Active [{}] - Waiting for Apple Music...", catalog_arc.storefront().to_uppercase());
            }
            let mut last_applied_fmt: Option<AudioFormat> = None;
            let mut resolve_cache: Option<(String, AudioFormat, Vec<AudioFormat>)> = None;
            let mut resume_until: Option<Instant> = None;
            let mut last_mode = {
                let st = status_arc.lock().unwrap();
                st.mode
            };

            loop {
                if *shutdown_arc.lock().unwrap() {
                    break;
                }

                let target_device_id = {
                    let guard = device_id_arc.lock().unwrap();
                    guard.clone().or_else(|| wasapi.get_default_device_id().ok())
                };

                let active_device_id = match target_device_id {
                    Some(id) => id,
                    None => {
                        {
                            let mut st = status_arc.lock().unwrap();
                            st.status_message = "No audio endpoint found".to_string();
                        }
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                };

                let current_dev_fmt = policy_config.get_device_format(&active_device_id).ok()
                    .or_else(|| wasapi.get_current_format(&active_device_id).ok());

                // Capture initial format for this device before any modification
                {
                    let mut map = initial_fmts_arc.lock().unwrap();
                    if !map.contains_key(&active_device_id) {
                        if let Some(fmt) = current_dev_fmt {
                            map.insert(active_device_id.clone(), fmt);
                        }
                    }
                }

                let apple_session = smtc.get_apple_music_session();
                let track_info = apple_session.as_ref().and_then(|s| smtc.get_track_info(s));

                // Resume keeps retrying on later polls if the first attempt lost the race against the endpoint rebinding.
                // ponytail: a manual pause inside this window gets overridden; shorten the window if that ever annoys
                if let (Some(until), Some(s)) = (resume_until, apple_session.as_ref()) {
                    if smtc.is_playing(s) || Instant::now() > until {
                        resume_until = None;
                    } else if smtc.is_paused(s) {
                        smtc.try_play(s);
                    }
                }

                let (mode, settling_delay_ms) = {
                    let st = status_arc.lock().unwrap();
                    (st.mode, st.settling_delay_ms)
                };

                let mode_changed = mode != last_mode;
                last_mode = mode;

                match track_info {
                    Some(ref track) => {
                        let track_key = if !track.album.is_empty() {
                            format!("{}|{}|{}", track.artist, track.title, track.album)
                        } else {
                            format!("{}|{}", track.artist, track.title)
                        };
                        let is_new_track = {
                            let last_key = last_track_key_arc.lock().unwrap();
                            track_key != *last_key
                        };

                        // Instant UI update on track transition: show new title and artwork immediately
                        if is_new_track {
                            let mut st = status_arc.lock().unwrap();
                            st.current_track = Some(track.clone());
                            if track.smtc_artwork_bytes.is_some() {
                                st.artwork_bytes = track.smtc_artwork_bytes.clone();
                            }
                            st.status_message = format!("Syncing: {}...", track.title);
                            st.is_syncing = true;
                        }

                        let is_active = apple_session.as_ref().map(|s| smtc.is_active_or_playing(s)).unwrap_or(false);

                        let album_opt = if !track.album.is_empty() { Some(track.album.as_str()) } else { None };
                        let resolution = catalog_arc.resolve_track(&track.artist, &track.title, album_opt, track.duration_secs);
                        let raw_target_fmt = resolution.format;
                        let artwork_bytes = track.smtc_artwork_bytes.clone().or(resolution.artwork_bytes);
                        let is_lossless = resolution.is_lossless;

                        let requested_fmt = match mode {
                            SwitcherMode::Seamless => {
                                AudioFormat::new(raw_target_fmt.sample_rate, 24)
                            }
                            SwitcherMode::Purist => raw_target_fmt,
                        };

                        // WASAPI verified candidates sorted descending (highest rate <= requested_fmt first)
                        let candidates: Vec<AudioFormat> = match resolve_cache {
                            Some((ref d, r, ref list)) if *d == active_device_id && r == requested_fmt => list.clone(),
                            _ => {
                                let mut list = wasapi.supported_candidates(&active_device_id, &requested_fmt);
                                if list.is_empty() {
                                    list.push(requested_fmt);
                                }
                                resolve_cache = Some((active_device_id.clone(), requested_fmt, list.clone()));
                                list
                            }
                        };
                        let effective_target_fmt = candidates.first().copied().unwrap_or(requested_fmt);

                        let cur_rate = current_dev_fmt.map(|f| f.sample_rate).unwrap_or(0);
                        let cur_bits = current_dev_fmt.map(|f| f.bit_depth).unwrap_or(0);

                        let already_applied_for_this_track = !is_new_track && !mode_changed && match last_applied_fmt {
                            Some(applied) => {
                                current_dev_fmt.map_or(false, |cur| {
                                    cur.sample_rate == applied.sample_rate && cur.bit_depth == applied.bit_depth
                                })
                            }
                            None => false,
                        };

                        let needs_switch = mode_changed || (!already_applied_for_this_track && {
                            cur_rate != effective_target_fmt.sample_rate || cur_bits != effective_target_fmt.bit_depth
                        });

                        let format_differs = match last_applied_fmt {
                            Some(last) => last.sample_rate != effective_target_fmt.sample_rate || last.bit_depth != effective_target_fmt.bit_depth,
                            None => true,
                        };

                        if needs_switch && (is_new_track || mode_changed || format_differs) {
                            {
                                let mut st = status_arc.lock().unwrap();
                                st.is_syncing = true;
                                st.current_track = Some(track.clone());
                                st.target_format = Some(raw_target_fmt);
                                st.artwork_bytes = artwork_bytes.clone();
                                st.current_device_format = current_dev_fmt;
                                st.is_lossless = is_lossless;
                                if !is_lossless {
                                    st.status_message = format!("Syncing to {} (AAC Lossy)...", effective_target_fmt.display_str());
                                } else {
                                    st.status_message = format!("Syncing to {}...", effective_target_fmt.display_str());
                                }
                            }

                            let should_resume = is_active || is_new_track;

                            if should_resume {
                                if let Some(ref session) = apple_session {
                                    smtc.try_pause(session);
                                    let pause_deadline = Instant::now() + Duration::from_millis(500);
                                    while !smtc.is_paused(session) && Instant::now() < pause_deadline {
                                        thread::sleep(Duration::from_millis(15));
                                    }
                                    // Strict barrier: never change the endpoint format under a stream that hasn't paused.
                                    if !smtc.is_paused(session) {
                                        status_arc.lock().unwrap().is_syncing = false;
                                        thread::sleep(Duration::from_millis(400));
                                        continue;
                                    }
                                    // Drain guard: let AM release its render buffer after reporting Paused.
                                    thread::sleep(Duration::from_millis(100));
                                }
                            }

                            // Dynamic smart settling delay: sample rates >= 96kHz or clock family transitions require extra PLL clock-lock time
                            let is_high_rate = effective_target_fmt.sample_rate >= 96000 || cur_rate >= 96000;
                            let is_clock_family_change = cur_rate > 0 && (
                                (cur_rate % 44100 == 0 && effective_target_fmt.sample_rate % 48000 == 0)
                                || (cur_rate % 48000 == 0 && effective_target_fmt.sample_rate % 44100 == 0)
                            );
                            let effective_delay = if is_high_rate || is_clock_family_change {
                                settling_delay_ms + 150
                            } else {
                                settling_delay_ms
                            };
                            thread::sleep(Duration::from_millis(effective_delay));

                            // Tries each driver-accepted candidate in descending order
                            let mut switch_res = Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x88890008_u32 as i32)));
                            for c in candidates.iter() {
                                if policy_config.set_device_format(&active_device_id, c).is_ok() {
                                    switch_res = Ok(*c);
                                    break;
                                }
                            }

                            let probe_deadline = Instant::now() + Duration::from_millis(400);
                            while Instant::now() < probe_deadline {
                                if wasapi.probe_device_ready(&active_device_id) {
                                    break;
                                }
                                thread::sleep(Duration::from_millis(25));
                            }

                            // Post-switch settling: give Windows audio engine and DAC PLL extra headroom for clock transitions
                            let post_delay = if is_high_rate || is_clock_family_change {
                                150
                            } else {
                                80
                            };
                            thread::sleep(Duration::from_millis(post_delay));

                            if should_resume {
                                let resume_session = smtc.get_apple_music_session().or_else(|| apple_session.clone());
                                if let Some(ref session) = resume_session {
                                    if !smtc.resume_playback_reliable(session) {
                                        resume_until = Some(Instant::now() + Duration::from_secs(8));
                                    }
                                }
                            }

                            let (applied_fmt, is_clamped) = match switch_res {
                                Ok(clamped) => {
                                    let clamped_flag = clamped.sample_rate < requested_fmt.sample_rate 
                                        || clamped.bit_depth < requested_fmt.bit_depth;
                                    (Some(clamped), clamped_flag)
                                }
                                Err(_) => (None, false),
                            };

                            let new_current_fmt = policy_config.get_device_format(&active_device_id).ok()
                                .or_else(|| wasapi.get_current_format(&active_device_id).ok());

                            if applied_fmt.is_some() {
                                if let Ok(mut key) = last_track_key_arc.lock() {
                                    *key = track_key.clone();
                                }
                                last_applied_fmt = applied_fmt;
                            } else {
                                // Clear cached candidate list so the next poll re-evaluates cleanly
                                resolve_cache = None;
                            }

                            {
                                let mut st = status_arc.lock().unwrap();
                                st.is_syncing = false;
                                st.current_track = Some(track.clone());
                                st.target_format = Some(raw_target_fmt);
                                st.current_device_format = new_current_fmt;
                                st.is_lossless = is_lossless;
                                if let Some(applied) = applied_fmt {
                                    if !is_lossless {
                                        st.status_message = format!("Synchronized: {} (AAC Lossy)", applied.display_str());
                                    } else if is_clamped {
                                        st.status_message = format!("Synchronized: {} (Clamped)", applied.display_str());
                                    } else {
                                        st.status_message = format!("Synchronized: {}", applied.display_str());
                                    }
                                } else if let Some(cur) = new_current_fmt {
                                    st.status_message = format!("Retrying sync ({})...", cur.display_str());
                                } else {
                                    st.status_message = "Audio endpoint unsupported".to_string();
                                }
                            }
                        } else {
                            if let Ok(mut key) = last_track_key_arc.lock() {
                                *key = track_key.clone();
                            }
                            let mut st = status_arc.lock().unwrap();
                            st.is_syncing = false;
                            st.current_track = Some(track.clone());
                            st.target_format = Some(raw_target_fmt);
                            st.artwork_bytes = artwork_bytes;
                            st.current_device_format = current_dev_fmt;
                            st.is_lossless = is_lossless;
                            if !needs_switch {
                                let disp_fmt = current_dev_fmt.unwrap_or(effective_target_fmt);
                                let is_cur_clamped = disp_fmt.sample_rate < requested_fmt.sample_rate 
                                    || disp_fmt.bit_depth < requested_fmt.bit_depth;
                                if !is_lossless {
                                    st.status_message = format!("Synchronized: {} (AAC Lossy)", disp_fmt.display_str());
                                } else if is_cur_clamped {
                                    st.status_message = format!("Synchronized: {} (Clamped)", disp_fmt.display_str());
                                } else {
                                    st.status_message = format!("Synchronized: {}", disp_fmt.display_str());
                                }
                            }
                        }
                    }
                    None => {
                        let mut st = status_arc.lock().unwrap();
                        st.current_track = None;
                        st.target_format = None;
                        st.artwork_bytes = None;
                        st.is_lossless = true;
                        st.current_device_format = current_dev_fmt;
                        st.status_message = "Waiting for Apple Music playback...".to_string();
                    }
                }

                thread::sleep(Duration::from_millis(200));
            }

            // Restore initial format on coordinator thread exit
            let formats: Vec<(String, AudioFormat)> = {
                let mut guard = initial_fmts_arc.lock().unwrap();
                guard.drain().collect()
            };
            for (dev_id, orig_fmt) in formats {
                let _ = policy_config.set_device_format(&dev_id, &orig_fmt);
            }

            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        });
        let mut h = self.thread_handle.lock().unwrap();
        *h = Some(handle);
    }
}
