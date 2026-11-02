use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::audio::AudioFormat;
use crate::apple_music::etw_reader::{EtwLogReader, EtwMediaFormat};

unsafe extern "system" {
    fn GetUserGeoID(geoClass: u32) -> i32;
    fn GetGeoInfoW(location: i32, geoType: u32, lpGeoData: *mut u16, cchData: i32, langId: u16) -> i32;
    fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
}

pub fn get_system_storefront() -> String {
    unsafe {
        let geo_id = GetUserGeoID(16);
        if geo_id > 0 {
            let mut buf = [0u16; 16];
            let len = GetGeoInfoW(geo_id, 4, buf.as_mut_ptr(), buf.len() as i32, 0);
            if len > 1 {
                let code = String::from_utf16_lossy(&buf[..(len - 1) as usize]);
                let trimmed = code.trim().to_lowercase();
                if trimmed.len() == 2 && trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
                    return trimmed;
                }
            }
        }

        let mut buf = [0u16; 85];
        let len = GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if len > 1 {
            let locale = String::from_utf16_lossy(&buf[..(len - 1) as usize]);
            if let Some((_, region)) = locale.split_once('-') {
                let trimmed = region.trim().to_lowercase();
                if trimmed.len() == 2 && trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
                    return trimmed;
                }
            }
        }
    }
    "us".to_string()
}

const CACHE_VERSION: u32 = 5;

#[derive(Serialize, Deserialize, Clone)]
struct CachedTrackFormat {
    sample_rate: u32,
    bit_depth: u16,
    #[serde(default)]
    artwork_url: Option<String>,
    #[serde(default)]
    is_lossless: bool,
    #[serde(default)]
    track_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheFile {
    version: u32,
    entries: HashMap<String, CachedTrackFormat>,
    #[serde(default)]
    dev_token: Option<String>,
    #[serde(default)]
    adam_formats: HashMap<i64, EtwMediaFormat>,
}

#[derive(Clone)]
struct MemoryCachedFormat {
    format: AudioFormat,
    artwork_url: Option<String>,
    is_lossless: bool,
    track_id: Option<i64>,
    etw_verified: bool,
    created_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackResolution {
    pub format: AudioFormat,
    pub artwork_bytes: Option<Vec<u8>>,
    pub is_lossless: bool,
}

type CandidateEvaluation = (AudioFormat, Option<String>, bool, Option<i64>, bool);

pub struct CatalogResolver {
    cache_path: PathBuf,
    cache: Mutex<HashMap<String, MemoryCachedFormat>>,
    token: Mutex<String>,
    storefront: String,
    etw_reader: EtwLogReader,
    last_artwork: Mutex<Option<(String, Vec<u8>)>>,
}

impl CatalogResolver {
    pub fn new() -> Self {
        let dir = crate::config::get_app_dir();
        let cache_path = dir.join("catalog_cache.json");
        let legacy_artworks = dir.join("artworks");
        if legacy_artworks.exists() {
            let _ = fs::remove_dir_all(&legacy_artworks);
        }

        let mut initial_cache = HashMap::new();
        let mut initial_token = String::new();
        let mut initial_adam_map = HashMap::new();
        if let Ok(data) = fs::read_to_string(&cache_path) {
            if let Ok(file) = serde_json::from_str::<CacheFile>(&data) {
                if let Some(tok) = file.dev_token {
                    initial_token = tok;
                }
                if file.version == CACHE_VERSION {
                    initial_adam_map = file.adam_formats;
                    for (k, v) in file.entries {
                        initial_cache.insert(k, MemoryCachedFormat {
                            format: AudioFormat::new(v.sample_rate, v.bit_depth),
                            artwork_url: v.artwork_url,
                            is_lossless: v.is_lossless,
                            track_id: v.track_id,
                            etw_verified: true,
                            created_seq: 0,
                        });
                    }
                } else {
                    let _ = fs::remove_file(&cache_path);
                }
            } else {
                let _ = fs::remove_file(&cache_path);
            }
        }

        let storefront = get_system_storefront();

        Self {
            cache_path,
            cache: Mutex::new(initial_cache),
            token: Mutex::new(initial_token),
            storefront,
            etw_reader: EtwLogReader::new(initial_adam_map),
            last_artwork: Mutex::new(None),
        }
    }

    pub fn warm_up_etw(&self) {
        let _ = self.etw_reader.current_event_seq();
        if let Ok(cache) = self.cache.lock() {
            self.save_cache(&cache);
        }
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
        self.etw_reader.clear();
        if let Ok(mut art) = self.last_artwork.lock() {
            *art = None;
        }
        let _ = fs::remove_file(&self.cache_path);
        let legacy_artworks = self.cache_path.parent().map(|p| p.join("artworks"));
        if let Some(p) = legacy_artworks {
            let _ = fs::remove_dir_all(p);
        }
    }

    pub fn storefront(&self) -> &str {
        &self.storefront
    }

    pub fn resolve_track(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_secs: Option<u64>,
    ) -> TrackResolution {
        let clean_art = clean_artist(artist);
        let clean_tit = title.trim().to_string();
        let clean_alb = album.map(|a| a.trim().to_string()).unwrap_or_default();

        let cache_key = if !clean_alb.is_empty() {
            format!("{}|{}|{}", clean_art.to_lowercase(), clean_tit.to_lowercase(), clean_alb.to_lowercase())
        } else {
            format!("{}|{}", clean_art.to_lowercase(), clean_tit.to_lowercase())
        };
        let fallback_key = format!("{}|{}", clean_art.to_lowercase(), clean_tit.to_lowercase());

        let (mut format, art_url, is_lossless, matched_key, entry_info) = {
            let mut found = None;
            if let Ok(cache) = self.cache.lock() {
                if let Some(entry) = cache.get(&cache_key) {
                    found = Some((
                        entry.format,
                        entry.artwork_url.clone(),
                        entry.is_lossless,
                        cache_key.clone(),
                        (entry.track_id, entry.etw_verified, entry.created_seq),
                    ));
                } else if cache_key != fallback_key
                    && let Some(entry) = cache.get(&fallback_key) {
                        found = Some((
                            entry.format,
                            entry.artwork_url.clone(),
                            entry.is_lossless,
                            fallback_key.clone(),
                            (entry.track_id, entry.etw_verified, entry.created_seq),
                        ));
                    }
            }

            match found {
                Some((fmt, art, lossless, key, info)) if art.is_some() || !lossless => {
                    (fmt, art, lossless, Some(key), Some(info))
                }
                _ => {
                    let cur_seq = self.etw_reader.current_event_seq();
                    let album_arg = if clean_alb.is_empty() { None } else { Some(clean_alb.as_str()) };
                    let fetched = self.fetch_catalog_format(&clean_art, &clean_tit, album_arg, duration_secs);

                    if let Some((res_fmt, res_art, res_lossless, res_tid, res_verified)) = fetched {
                        if let Ok(mut cache) = self.cache.lock() {
                            cache.insert(cache_key.clone(), MemoryCachedFormat {
                                format: res_fmt,
                                artwork_url: res_art.clone(),
                                is_lossless: res_lossless,
                                track_id: res_tid,
                                etw_verified: res_verified,
                                created_seq: cur_seq,
                            });
                            self.save_cache(&cache);
                        }
                        (
                            res_fmt,
                            res_art,
                            res_lossless,
                            Some(cache_key.clone()),
                            Some((res_tid, res_verified, cur_seq)),
                        )
                    } else if let Some((fmt, art, lossless, key, info)) = found {
                        (fmt, art, lossless, Some(key), Some(info))
                    } else {
                        // Do not persist failed lookups into catalog_cache.json
                        (AudioFormat::new(44100, 16), None, false, None, None)
                    }
                }
            }
        };

        // Live ETW verification & refinement for cached entries
        if is_lossless
            && let (Some(key), Some((track_id, etw_verified, created_seq))) = (matched_key, entry_info) {
                let mut verified_update: Option<EtwMediaFormat> = None;

                if !etw_verified {
                    if let Some(tid) = track_id
                        && let Some(ef) = self.etw_reader.get_lossless_format_for_adam_id(tid) {
                            verified_update = Some(ef);
                        }

                    if verified_update.is_none()
                        && let Some((_, ef)) = self.etw_reader.get_latest_lossless_since(created_seq) {
                            verified_update = Some(ef);
                        }
                }

                if let Some(ef) = verified_update {
                    let new_bits = if ef.has_explicit_bit_depth || ef.sample_rate > 48000 {
                        ef.bit_depth
                    } else {
                        format.bit_depth
                    };
                    format = AudioFormat::new(ef.sample_rate, new_bits);
                    if let Ok(mut cache) = self.cache.lock() {
                        if let Some(entry) = cache.get_mut(&key) {
                            entry.format = format;
                            entry.etw_verified = true;
                        }
                        self.save_cache(&cache);
                    }
                }
            }

        let artwork_bytes = art_url.and_then(|url| self.fetch_artwork_bytes(&url));

        TrackResolution {
            format,
            artwork_bytes,
            is_lossless,
        }
    }

    #[allow(dead_code)]
    pub fn resolve_format(&self, artist: &str, title: &str) -> AudioFormat {
        self.resolve_track(artist, title, None, None).format
    }

    fn fetch_artwork_bytes(&self, url: &str) -> Option<Vec<u8>> {
        if let Ok(guard) = self.last_artwork.lock()
            && let Some((ref cached_url, ref cached_bytes)) = *guard
                && cached_url == url {
                    return Some(cached_bytes.clone());
                }

        use std::io::Read;
        let resp = ureq::get(url)
            .timeout(std::time::Duration::from_secs(4))
            .call().ok()?;

        let mut reader = resp.into_reader().take(10 * 1024 * 1024);
        let mut bytes = Vec::new();
        if reader.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
            if let Ok(mut guard) = self.last_artwork.lock() {
                *guard = Some((url.to_string(), bytes.clone()));
            }
            Some(bytes)
        } else {
            None
        }
    }

    fn save_cache(&self, cache: &HashMap<String, MemoryCachedFormat>) {
        let entries: HashMap<String, CachedTrackFormat> = cache
            .iter()
            .filter(|(_, v)| v.etw_verified)
            .map(|(k, v)| {
                (
                    k.clone(),
                    CachedTrackFormat {
                        sample_rate: v.format.sample_rate,
                        bit_depth: v.format.bit_depth,
                        artwork_url: v.artwork_url.clone(),
                        is_lossless: v.is_lossless,
                        track_id: v.track_id,
                    },
                )
            })
            .collect();

        let dev_token = self.token.lock().ok().and_then(|t| {
            if t.trim().is_empty() {
                None
            } else {
                Some(t.clone())
            }
        });

        let adam_formats = self.etw_reader.export_adam_map();

        let file = CacheFile {
            version: CACHE_VERSION,
            entries,
            dev_token,
            adam_formats,
        };

        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = fs::write(&self.cache_path, json);
        }
    }

    fn search_amp_api(&self, query: &str, storefront: &str, token: &str, limit: usize) -> Option<Vec<serde_json::Value>> {
        let url = format!(
            "https://amp-api.music.apple.com/v1/catalog/{}/search?term={}&types=songs&limit={}",
            storefront,
            urlencoding(query),
            limit
        );

        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Origin", "https://music.apple.com")
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(std::time::Duration::from_secs(4))
            .call().ok()?;

        let json: serde_json::Value = resp.into_json().ok()?;
        json.get("results")?
            .get("songs")?
            .get("data")?
            .as_array()
            .cloned()
    }

    fn search_itunes(&self, query: &str, country: Option<&str>, limit: usize) -> Option<Vec<serde_json::Value>> {
        let mut itunes_url = format!(
            "https://itunes.apple.com/search?term={}&entity=song&limit={}",
            urlencoding(query),
            limit
        );
        if let Some(c) = country {
            itunes_url.push_str(&format!("&country={}", c));
        }

        let resp = ureq::get(&itunes_url)
            .timeout(std::time::Duration::from_secs(4))
            .call().ok()?;

        let json: serde_json::Value = resp.into_json().ok()?;
        json.get("results")?.as_array().cloned()
    }

    fn fetch_catalog_format(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_secs: Option<u64>,
    ) -> Option<CandidateEvaluation> {
        let mut token = self.token.lock().ok()?.clone();

        if token.trim().is_empty() {
            {
                let new_token = self.refresh_token()?;
                if let Ok(mut t_guard) = self.token.lock() {
                    *t_guard = new_token.clone();
                }
                if let Ok(cache) = self.cache.lock() {
                    self.save_cache(&cache);
                }
                token = new_token;
            }
        }

        if let Some(res) = self.search_and_evaluate(artist, title, album, duration_secs, &token) {
            return Some(res);
        }

        // Refresh dev token if expired and retry
        if let Some(new_token) = self.refresh_token() {
            if let Ok(mut t_guard) = self.token.lock() {
                *t_guard = new_token.clone();
            }
            if let Ok(cache) = self.cache.lock() {
                self.save_cache(&cache);
            }
            return self.search_and_evaluate(artist, title, album, duration_secs, &new_token);
        }

        None
    }

    fn search_and_evaluate(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_secs: Option<u64>,
        token: &str,
    ) -> Option<CandidateEvaluation> {
        // Search AMP API with artist, title, and album if available
        if let Some(alb) = album {
            let q_album = format!("{} {} {}", artist, title, alb);
            if let Some(results) = self.search_amp_api(&q_album, &self.storefront, token, 10)
                && !results.is_empty()
                    && let Some(res) = self.evaluate_candidates(&results, artist, title, album, duration_secs, token) {
                        return Some(res);
                    }
        }

        // Fallback to standard query (artist + title)
        let q_standard = format!("{} {}", artist, title);
        if let Some(results) = self.search_amp_api(&q_standard, &self.storefront, token, 15)
            && !results.is_empty()
                && let Some(res) = self.evaluate_candidates(&results, artist, title, album, duration_secs, token) {
                    return Some(res);
                }

        // Retry standard query in US storefront if regional storefront had no match
        if self.storefront != "us"
            && let Some(results) = self.search_amp_api(&q_standard, "us", token, 15)
                && !results.is_empty()
                    && let Some(res) = self.evaluate_candidates(&results, artist, title, album, duration_secs, token) {
                        return Some(res);
                    }

        // Fallback to iTunes Search API
        let mut itunes_results = if let Some(alb) = album {
            let q_album = format!("{} {} {}", artist, title, alb);
            self.search_itunes(&q_album, Some(&self.storefront), 10)
                .filter(|res| !res.is_empty())
        } else {
            None
        };

        if itunes_results.is_none() {
            itunes_results = self.search_itunes(&q_standard, Some(&self.storefront), 15);
            if itunes_results.as_ref().is_none_or(|v| v.is_empty()) && self.storefront != "us" {
                itunes_results = self.search_itunes(&q_standard, Some("us"), 15);
            }
            if itunes_results.as_ref().is_none_or(|v| v.is_empty()) {
                itunes_results = self.search_itunes(&q_standard, None, 15);
            }
        }

        if let Some(results) = itunes_results
            && let Some(res) = self.evaluate_candidates(&results, artist, title, album, duration_secs, token) {
                return Some(res);
            }

        None
    }

    fn evaluate_candidates(
        &self,
        results: &[serde_json::Value],
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_secs: Option<u64>,
        token: &str,
    ) -> Option<CandidateEvaluation> {
        let latest_etw_adam = self.etw_reader.latest_active_adam_id();
        let mut scored_candidates = Vec::new();
        for r in results {
            if let Some(mut score) = score_candidate(r, artist, title, album, duration_secs) {
                let track_id = r.get("id")
                    .and_then(|id| id.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| id.as_i64()))
                    .or_else(|| r.get("trackId").and_then(|id| id.as_i64()));

                if let Some(tid) = track_id {
                    if Some(tid) == latest_etw_adam {
                        score += 500;
                    } else if self.etw_reader.get_lossless_format_for_adam_id(tid).is_some() {
                        score += 75;
                    }
                }
                scored_candidates.push((score, track_id, r));
            }
        }

        scored_candidates.sort_by_key(|a| std::cmp::Reverse(a.0));

        let mut best: Option<(i32, CandidateEvaluation)> = None;

        for (score, track_id, r) in scored_candidates {
            let cand_art = r.get("attributes")
                .and_then(|a| a.get("artwork"))
                .and_then(|a| a.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.replace("{w}x{h}", "600x600"))
                .or_else(|| {
                    r.get("artworkUrl100")
                        .and_then(|v| v.as_str())
                        .map(|s| s.replace("100x100", "600x600"))
                });

            if let Some(track_id) = track_id {
                let fmt = self.query_amp_api(track_id, token, &self.storefront)
                    .or_else(|| {
                        if self.storefront != "us" {
                            self.query_amp_api(track_id, token, "us")
                        } else {
                            None
                        }
                    });

                if let Some((cand_fmt, cand_amp_art, is_lossless, etw_verified)) = fmt {
                    let art_choice = cand_amp_art.or(cand_art);
                    let candidate_eval = (cand_fmt, art_choice, is_lossless, Some(track_id), etw_verified);
                    match best {
                        Some((best_score, (cur_fmt, ..))) => {
                            if score > best_score || (score == best_score && cand_fmt.sample_rate > cur_fmt.sample_rate) {
                                best = Some((score, candidate_eval));
                            }
                        }
                        None => {
                            best = Some((score, candidate_eval));
                        }
                    }

                    // Early exit if strong exact match (exact album and duration matched, or ETW active adamID matched)
                    if let Some((best_score, _)) = best
                        && best_score >= 350 {
                            break;
                        }
                }
            }
        }

        best.map(|(_, eval)| eval)
    }

    fn query_amp_api(&self, track_id: i64, token: &str, storefront: &str) -> Option<(AudioFormat, Option<String>, bool, bool)> {
        let url = format!(
            "https://amp-api.music.apple.com/v1/catalog/{}/songs/{}?extend=extendedAssetUrls,audioTraits",
            storefront, track_id
        );

        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Origin", "https://music.apple.com")
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(std::time::Duration::from_secs(4))
            .call().ok()?;

        let json: serde_json::Value = resp.into_json().ok()?;
        let data = json.get("data")?.get(0)?;
        let attributes = data.get("attributes")?;

        let artwork_url = attributes.get("artwork")
            .and_then(|a| a.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.replace("{w}x{h}", "600x600"));

        let mut parsed_fmt = None;
        if let Some(hls_url) = attributes.get("extendedAssetUrls").and_then(|e| e.get("enhancedHls")).and_then(|h| h.as_str()) {
            parsed_fmt = self.parse_hls_for_max_alac(hls_url);
        }

        let is_hires_trait = attributes.get("audioTraits")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some("hi-res-lossless")))
            .unwrap_or(false);

        let is_lossless_trait = attributes.get("audioTraits")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some("lossless")))
            .unwrap_or(false);

        let mut is_lossless = parsed_fmt.is_some() || is_hires_trait || is_lossless_trait;

        // Check native AMPLibraryAgent ETW telemetry for exact decoded sample rate & bit depth
        let mut etw_fmt = self.etw_reader.get_lossless_format_for_adam_id(track_id);
        let is_ambiguous_hires = is_hires_trait && parsed_fmt.is_none_or(|f| f.sample_rate <= 48000);

        if is_ambiguous_hires && etw_fmt.is_none() && self.etw_reader.is_active_log_recent() {
            for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_millis(220));
                etw_fmt = self.etw_reader.get_lossless_format_for_adam_id(track_id);
                if etw_fmt.is_some() {
                    break;
                }
            }
        }

        if let Some(ef) = etw_fmt {
            is_lossless = true;
            let valid_rate = if !is_hires_trait && ef.sample_rate > 48000 {
                parsed_fmt.map(|f| f.sample_rate).unwrap_or(44100)
            } else {
                ef.sample_rate
            };
            let bit_depth = if ef.has_explicit_bit_depth || valid_rate > 48000 {
                ef.bit_depth
            } else {
                parsed_fmt.map(|f| f.bit_depth).unwrap_or(ef.bit_depth)
            };
            return Some((
                AudioFormat::new(valid_rate, bit_depth),
                artwork_url,
                is_lossless,
                true,
            ));
        }

        if is_ambiguous_hires {
            // Public web HLS manifest capped at <= 48 kHz for hi-res-lossless track and no ETW record yet.
            // Use 96 kHz / 24-bit as initial fallback, marked etw_verified = false so it refines once ETW flushes.
            return Some((
                AudioFormat::new(96000, 24),
                artwork_url,
                is_lossless,
                false,
            ));
        }

        let final_fmt = match parsed_fmt {
            Some(fmt) => fmt,
            None => AudioFormat::new(44100, 16),
        };

        Some((final_fmt, artwork_url, is_lossless, true))
    }

    fn parse_hls_for_max_alac(&self, hls_url: &str) -> Option<AudioFormat> {
        let resp = ureq::get(hls_url)
            .timeout(std::time::Duration::from_secs(4))
            .call().ok()?;

        let content = resp.into_string().ok()?;
        let mut max_rate: u32 = 0;
        let mut best_depth: u16 = 16;

        for line in content.lines() {
            if line.contains("TYPE=AUDIO") && line.contains("audio-alac") {
                let mut rate: u32 = 0;
                let mut depth: u16 = 16;

                for part in line.split(',') {
                    let part = part.trim();
                    if let Some(val_str) = part.strip_prefix("SAMPLE-RATE=") {
                        if let Ok(val) = val_str.parse::<u32>() {
                            rate = val;
                        }
                    } else if let Some(val_str) = part.strip_prefix("BIT-DEPTH=")
                        && let Ok(val) = val_str.parse::<u16>() {
                            depth = val;
                        }
                }

                if rate > max_rate {
                    max_rate = rate;
                    best_depth = depth;
                }
            }
        }

        if max_rate > 0 {
            Some(AudioFormat::new(max_rate, best_depth))
        } else {
            None
        }
    }

    fn refresh_token(&self) -> Option<String> {
        let url = format!("https://music.apple.com/{}/new", self.storefront);
        let resp = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .ok()
            .or_else(|| {
                ureq::get("https://music.apple.com/us/new")
                    .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                    .timeout(std::time::Duration::from_secs(5))
                    .call()
                    .ok()
            })?;

        let html = resp.into_string().ok()?;
        let mut js_url = None;

        for line in html.lines() {
            if let Some(pos) = line.find("/assets/index~") {
                let sub = &line[pos..];
                if let Some(end) = sub.find(".js") {
                    js_url = Some(format!("https://music.apple.com{}", &sub[..end + 3]));
                    break;
                }
            }
        }

        let js_url = js_url?;
        let js_resp = ureq::get(&js_url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(std::time::Duration::from_secs(8))
            .call().ok()?;

        let js_content = js_resp.into_string().ok()?;
        if let Some(pos) = js_content.find("u_e=\"") {
            let sub = &js_content[pos + 5..];
            if let Some(end) = sub.find('"') {
                let candidate = &sub[..end];
                if candidate.split('.').count() == 3 && candidate.len() > 100 {
                    return Some(candidate.to_string());
                }
            }
        }

        // Resilient fallback: locate token pattern directly within the bundle
        for marker in &["\"eyJh", "\"eyJ0"] {
            if let Some(pos) = js_content.find(marker) {
                let sub = &js_content[pos + 1..];
                if let Some(end) = sub.find('"') {
                    let candidate = &sub[..end];
                    if candidate.split('.').count() == 3 && candidate.len() > 100 {
                        return Some(candidate.to_string());
                    }
                }
            }
        }

        None
    }
}

impl Default for CatalogResolver {
    fn default() -> Self {
        Self::new()
    }
}


fn clean_artist(input: &str) -> String {
    let s = input.split(['—', '–']).next().unwrap_or(input);
    s.trim().to_string()
}

fn normalize_str(input: &str) -> String {
    input.trim().to_lowercase()
        .replace(['’', '‘', '`'], "'")
        .replace(['“', '”'], "\"")
        .replace('&', " and ")
}

fn is_title_match(target: &str, candidate: &str) -> bool {
    let t = normalize_str(target);
    let c = normalize_str(candidate);
    if t == c {
        return true;
    }
    let t_base = t.split(['(', '[', '-', '—']).next().unwrap_or(&t).trim();
    let c_base = c.split(['(', '[', '-', '—']).next().unwrap_or(&c).trim();
    t_base == c_base && !t_base.is_empty()
}

fn is_artist_match(target_artist: &str, cand_artist: &str) -> bool {
    let t = normalize_str(target_artist);
    let c = normalize_str(cand_artist);
    if t.is_empty() || c.is_empty() {
        return true;
    }
    if t == c || t.contains(&c) || c.contains(&t) {
        return true;
    }
    let t_first = t.split(['&', ',', '—', '-', '/']).next().unwrap_or(&t).trim();
    let c_first = c.split(['&', ',', '—', '-', '/']).next().unwrap_or(&c).trim();
    !t_first.is_empty() && (t_first == c_first || t.contains(c_first) || c.contains(t_first))
}

fn get_cand_str<'a>(cand: &'a serde_json::Value, attr_key: &str, itunes_key: &str) -> Option<&'a str> {
    cand.get("attributes")
        .and_then(|a| a.get(attr_key))
        .and_then(|v| v.as_str())
        .or_else(|| cand.get(itunes_key).and_then(|v| v.as_str()))
}

fn get_cand_u64(cand: &serde_json::Value, attr_key: &str, itunes_key: &str) -> Option<u64> {
    cand.get("attributes")
        .and_then(|a| a.get(attr_key))
        .and_then(|v| v.as_u64())
        .or_else(|| cand.get(itunes_key).and_then(|v| v.as_u64()))
}

fn score_candidate(
    cand: &serde_json::Value,
    target_artist: &str,
    target_title: &str,
    target_album: Option<&str>,
    target_duration: Option<u64>,
) -> Option<i32> {
    let cand_title = get_cand_str(cand, "name", "trackName")?;
    if !is_title_match(target_title, cand_title) {
        return None;
    }

    let cand_artist = get_cand_str(cand, "artistName", "artistName").unwrap_or("");
    if !target_artist.is_empty() && !cand_artist.is_empty() && !is_artist_match(target_artist, cand_artist) {
        return None;
    }

    let mut score = 100;

    // Exact title matching bonus
    let t_norm = normalize_str(target_title);
    let c_norm = normalize_str(cand_title);
    if t_norm == c_norm {
        score += 30;
    }

    // Exact artist matching bonus
    if !target_artist.is_empty() && !cand_artist.is_empty()
        && normalize_str(target_artist) == normalize_str(cand_artist) {
            score += 50;
        }

    // Album matching precision
    if let Some(target_alb) = target_album.filter(|a| !a.trim().is_empty()) {
        let cand_album = get_cand_str(cand, "albumName", "collectionName").unwrap_or("");
        let cand_alb_norm = normalize_str(cand_album);
        let target_alb_norm = normalize_str(target_alb);

        if !cand_alb_norm.is_empty() {
            if cand_alb_norm == target_alb_norm {
                score += 300;
            } else {
                let target_base = target_alb_norm.split(['(', '[', '-', '—']).next().unwrap_or(&target_alb_norm).trim();
                let cand_base = cand_alb_norm.split(['(', '[', '-', '—']).next().unwrap_or(&cand_alb_norm).trim();
                if !target_base.is_empty() && target_base == cand_base {
                    score += 150;
                } else {
                    // Penalize candidates with completely different albums (e.g. compilation/greatest hits)
                    score -= 50;
                }

                let target_is_remaster = target_alb_norm.contains("remaster");
                let cand_is_remaster = cand_alb_norm.contains("remaster");
                if target_is_remaster == cand_is_remaster {
                    score += 30;
                }

                let target_is_deluxe = target_alb_norm.contains("deluxe") || target_alb_norm.contains("anniversary");
                let cand_is_deluxe = cand_alb_norm.contains("deluxe") || cand_alb_norm.contains("anniversary");
                if target_is_deluxe == cand_is_deluxe {
                    score += 30;
                }
            }
        }
    }

    // Duration matching tolerance
    if let Some(dur_secs) = target_duration
        && let Some(cand_ms) = get_cand_u64(cand, "durationInMillis", "trackTimeMillis") {
            let cand_secs = cand_ms / 1000;
            let diff = (cand_secs as i64 - dur_secs as i64).abs();
            if diff <= 1 {
                score += 50;
            } else if diff <= 3 {
                score += 30;
            } else if diff > 10 {
                score -= 100;
            }
        }

    Some(score)
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

