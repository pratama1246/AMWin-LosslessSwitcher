use lossless_switcher_am::config::AppSettings;
use lossless_switcher_am::coordinator::SwitcherMode;

#[test]
fn test_settings_serde_roundtrip() {
    let settings = AppSettings {
        mode: SwitcherMode::Seamless,
        settling_delay_ms: 250,
        is_always_on_top: true,
        selected_device_id: Some("{0.0.0.00000000}.{mock-dac-id}".to_string()),
    };

    let json = serde_json::to_string(&settings).expect("Serialization failed");
    let loaded: AppSettings = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.mode, SwitcherMode::Seamless);
    assert_eq!(loaded.settling_delay_ms, 250);
    assert_eq!(loaded.is_always_on_top, true);
    assert_eq!(loaded.selected_device_id, Some("{0.0.0.00000000}.{mock-dac-id}".to_string()));
}
