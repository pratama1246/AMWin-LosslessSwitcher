use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::coordinator::SwitcherMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub mode: SwitcherMode,
    pub settling_delay_ms: u64,
    pub is_always_on_top: bool,
    pub selected_device_id: Option<String>,
    #[serde(default)]
    pub selected_device_name: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mode: SwitcherMode::Seamless,
            settling_delay_ms: 200,
            is_always_on_top: false,
            selected_device_id: None,
            selected_device_name: None,
        }
    }
}

pub fn get_app_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let is_explicit_portable = std::env::args().any(|a| a == "--portable")
                || exe_dir.join("portable.dat").exists()
                || exe_dir.join("settings.json").exists();
            if is_explicit_portable {
                return exe_dir.to_path_buf();
            }
        }
    }

    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(&app_data).join("AMWin-LosslessSwitcher");
    let legacy_dir = PathBuf::from(&app_data).join("LosslessSwitcher-AM");
    if legacy_dir.exists() {
        let _ = fs::remove_dir_all(&legacy_dir);
    }
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn load_settings() -> AppSettings {
    let settings_path = get_app_dir().join("settings.json");
    if let Ok(content) = fs::read_to_string(&settings_path) {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
            return settings;
        }
    }
    AppSettings::default()
}

pub fn save_settings(settings: &AppSettings) {
    let settings_path = get_app_dir().join("settings.json");
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(settings_path, json);
    }
}
