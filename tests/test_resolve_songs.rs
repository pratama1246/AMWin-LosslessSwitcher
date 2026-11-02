use lossless_switcher_am::apple_music::CatalogResolver;

#[test]
fn test_resolve_songs_with_artwork() {
    let catalog = CatalogResolver::new();
    println!("Storefront: {}", catalog.storefront());
    assert_eq!(catalog.storefront(), "id");

    // 1. Regional track (DxS - Blue)
    let res_blue = catalog.resolve_track("DxS", "Blue", None, None);
    println!("Resolved DxS - Blue: format={:?}, artwork_bytes len={:?}", res_blue.format, res_blue.artwork_bytes.as_ref().map(|b| b.len()));
    assert_eq!(res_blue.format.sample_rate, 44100);
    assert_eq!(res_blue.format.bit_depth, 16);
    assert!(res_blue.artwork_bytes.is_some(), "Expected artwork bytes to be fetched in-memory for DxS - Blue");
    let art_bytes = res_blue.artwork_bytes.unwrap();
    assert!(!art_bytes.is_empty(), "Artwork in-memory bytes must not be empty");

    // 2. High-res classic with Album precision (Eagles - Hotel California, 2013 Remaster)
    let res_eagles = catalog.resolve_track("Eagles", "Hotel California", Some("Hotel California (2013 Remaster)"), Some(391));
    println!("Resolved Eagles: format={:?}, artwork_bytes len={:?}", res_eagles.format, res_eagles.artwork_bytes.as_ref().map(|b| b.len()));
    assert_eq!(res_eagles.format.sample_rate, 192000);
    assert_eq!(res_eagles.format.bit_depth, 24);

    // 3. JOY - Hello without album (Lossy AAC track)
    let res_joy_no_alb = catalog.resolve_track("JOY", "Hello", None, None);
    println!("Resolved JOY - Hello (NO ALBUM): format={:?}, is_lossless={}", res_joy_no_alb.format, res_joy_no_alb.is_lossless);
    assert!(!res_joy_no_alb.is_lossless, "JOY - Hello must be detected as Lossy/AAC");

    // 4. Coldplay - Yellow without album (192 kHz / 24-bit Hi-Res via ETW telemetry)
    let res_yellow_no_alb = catalog.resolve_track("Coldplay", "Yellow", None, None);
    println!("Resolved Coldplay - Yellow (NO ALBUM): format={:?}, is_lossless={}", res_yellow_no_alb.format, res_yellow_no_alb.is_lossless);
    assert_eq!(res_yellow_no_alb.format.sample_rate, 192000);
    assert_eq!(res_yellow_no_alb.format.bit_depth, 24);
    assert!(res_yellow_no_alb.is_lossless);

    // 4b. What Apple Music Windows SMTC actually sends:
    let res_yellow_smtc = catalog.resolve_track("Coldplay - Parachutes", "Yellow", None, None);
    println!("Resolved 'Coldplay - Parachutes' - 'Yellow': format={:?}, is_lossless={}", res_yellow_smtc.format, res_yellow_smtc.is_lossless);
    assert_eq!(res_yellow_smtc.format.sample_rate, 192000);
    assert_eq!(res_yellow_smtc.format.bit_depth, 24);
    assert!(res_yellow_smtc.is_lossless);

    // 5. The Rose - Back To Me (24/48k)
    let res_rose = catalog.resolve_track("The Rose", "Back To Me", Some("DUAL"), None);
    println!("Resolved The Rose - Back To Me: format={:?}, is_lossless={}", res_rose.format, res_rose.is_lossless);
    assert_eq!(res_rose.format.sample_rate, 48000);
    assert_eq!(res_rose.format.bit_depth, 24);
    assert!(res_rose.is_lossless);

    // 6. RIIZE - Behind The Shine (96 kHz / 24-bit Hi-Res)
    catalog.clear_cache();
    let res_riize = catalog.resolve_track("RIIZE", "Behind The Shine", Some("Perfect Crown (Original Soundtrack) Pt. 4 - Single"), None);
    println!("Resolved RIIZE - Behind The Shine: format={:?}, is_lossless={}", res_riize.format, res_riize.is_lossless);
    assert_eq!(res_riize.format.sample_rate, 96000);
    assert_eq!(res_riize.format.bit_depth, 24);
    assert!(res_riize.is_lossless);

    // 7. DxS - Rockstar (DK Solo) from Serenade (Lossless 44.1 kHz / 16-bit, NOT Hi-Res 96k)
    let res_dxs = catalog.resolve_track("DxS", "Rockstar (DK Solo)", Some("Serenade"), None);
    println!("Resolved DxS - Rockstar (DK Solo): format={:?}, is_lossless={}", res_dxs.format, res_dxs.is_lossless);
    assert_eq!(res_dxs.format.sample_rate, 44100);
    assert_eq!(res_dxs.format.bit_depth, 16);
    assert!(res_dxs.is_lossless);
}
