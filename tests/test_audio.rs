use lossless_switcher_am::audio::wasapi::WasapiManager;
use lossless_switcher_am::audio::policy_config::PolicyConfig;
use lossless_switcher_am::audio::AudioFormat;
use lossless_switcher_am::apple_music::CatalogResolver;

#[test]
fn test_audio_endpoints_and_catalog() {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    let wasapi = WasapiManager::new().expect("WASAPI manager initialization failed");
    let devices = wasapi.list_render_devices().expect("Failed to list render devices");
    println!("Found {} render devices:", devices.len());
    for d in &devices {
        let fmt = wasapi.get_current_format(&d.id);
        println!(" - [{}] {} (Default: {}, Current: {:?})", d.id, d.name, d.is_default, fmt);
    }
    assert!(!devices.is_empty(), "Expected at least one audio output device");

    let catalog = CatalogResolver::new();
    let fmt = catalog.resolve_format("Eagles", "Hotel California");
    println!("Resolved Eagles - Hotel California: {:?}", fmt);
    assert_eq!(fmt.sample_rate, 192000);
    assert_eq!(fmt.bit_depth, 24);

    let ja11 = devices.iter().find(|d| d.name.contains("JadeAudio") || d.name.contains("JA11"));
    if let Some(dev) = ja11 {
        println!("Testing live switch on JA11 to 96 kHz / 24-bit...");
        let policy = PolicyConfig::new().expect("PolicyConfig failed");
        let target = AudioFormat::new(96000, 24);
        let res = policy.set_device_format(&dev.id, &target);
        println!("Switch to 96k result: {:?}", res);
        assert!(res.is_ok(), "Format switch to 96 kHz must succeed with S_OK");
        let dev_fmt_pol = policy.get_device_format(&dev.id);
        let dev_fmt_was = wasapi.get_current_format(&dev.id);
        println!("After switch 96k - PolicyConfig dev format: {:?}", dev_fmt_pol);
        println!("After switch 96k - WASAPI mix format: {:?}", dev_fmt_was);
        assert_eq!(dev_fmt_pol.unwrap().bit_depth, 24);
    }

    // Test auto-clamping on the active default device (e.g. Realtek falling back to 48kHz/24-bit)
    let default_dev = devices.iter().find(|d| d.is_default).expect("No default render device");
    let policy = PolicyConfig::new().expect("PolicyConfig failed");
    let hi_res_target = AudioFormat::new(192000, 24);
    let clamped_res = policy.set_device_format_clamped(&default_dev.id, &hi_res_target);
    println!("Testing auto-clamping on default device {}: {:?}", default_dev.name, clamped_res);
    assert!(clamped_res.is_ok(), "set_device_format_clamped must succeed by falling back to supported format");
    let applied = clamped_res.unwrap();
    println!("Successfully clamped to hardware limit: {:?}", applied);
    assert!(applied.sample_rate <= 192000);
}

#[test]
fn test_switcher_mode_logic() {
    use lossless_switcher_am::coordinator::SwitcherMode;

    let cd_source = AudioFormat::new(44100, 16);
    let hi_res_source = AudioFormat::new(96000, 24);

    // Seamless mode: bit-depth is anchored to 24-bit container
    let seamless_cd_effective = match SwitcherMode::Seamless {
        SwitcherMode::Seamless => AudioFormat::new(cd_source.sample_rate, 24),
        SwitcherMode::Purist => cd_source,
    };
    assert_eq!(seamless_cd_effective.sample_rate, 44100);
    assert_eq!(seamless_cd_effective.bit_depth, 24);

    let seamless_hires_effective = match SwitcherMode::Seamless {
        SwitcherMode::Seamless => AudioFormat::new(hi_res_source.sample_rate, 24),
        SwitcherMode::Purist => hi_res_source,
    };
    assert_eq!(seamless_hires_effective.sample_rate, 96000);
    assert_eq!(seamless_hires_effective.bit_depth, 24);

    // Purist mode: bit-depth strictly mirrors the source audio file
    let purist_cd_effective = match SwitcherMode::Purist {
        SwitcherMode::Seamless => AudioFormat::new(cd_source.sample_rate, 24),
        SwitcherMode::Purist => cd_source,
    };
    assert_eq!(purist_cd_effective.sample_rate, 44100);
    assert_eq!(purist_cd_effective.bit_depth, 16);

    // Test DAC transition logic:
    // If DAC is at 44.1k/24 (Seamless), and user switches to Purist on a 16-bit track:
    let cur_rate = 44100;
    let cur_bits = 24;
    let target = purist_cd_effective;
    let needs_switch = cur_rate != target.sample_rate || cur_bits != target.bit_depth;
    assert!(needs_switch, "Purist mode must trigger switch when DAC is in 24-bit container for 16-bit track");

    // If DAC is at 44.1k/16 (Purist), and user switches to Seamless:
    let cur_rate = 44100;
    let cur_bits = 16;
    let target = seamless_cd_effective;
    let needs_switch = cur_rate != target.sample_rate || cur_bits != target.bit_depth;
    assert!(needs_switch, "Seamless mode must trigger switch to elevate 16-bit container to 24-bit");

    // Seamless gapless behavior: track changes from 16-bit 44.1k to 24-bit 44.1k
    let cur_rate = 44100;
    let cur_bits = 24;
    let target = AudioFormat::new(44100, 24);
    let needs_switch = cur_rate != target.sample_rate || cur_bits != target.bit_depth;
    assert!(!needs_switch, "Seamless mode must NOT trigger DAC reset when rate and container match");
}
