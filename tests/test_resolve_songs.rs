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
}
