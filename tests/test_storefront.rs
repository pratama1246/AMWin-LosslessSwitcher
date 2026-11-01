use lossless_switcher_am::apple_music::get_system_storefront;

#[test]
fn test_system_storefront() {
    let sf = get_system_storefront();
    println!("Detected system storefront: {}", sf);
    assert_eq!(sf.len(), 2);
    assert_eq!(sf, "id");
}
