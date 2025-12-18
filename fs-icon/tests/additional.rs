use fs_icon::{icon_of_path, scale_with_aspect_ratio};

#[test]
fn scale_extreme_aspect_ratios() {
    // Very wide
    let (w, h) = scale_with_aspect_ratio(1000.0, 10.0, 64.0, 64.0);
    assert!((w - 64.0).abs() < 0.001);
    assert!(h < 1.0, "height should shrink proportionally");

    // Very tall
    let (w, h) = scale_with_aspect_ratio(10.0, 1000.0, 64.0, 64.0);
    assert!((h - 64.0).abs() < 0.001);
    assert!(w < 1.0, "width should shrink proportionally");
}

#[test]
fn scale_zero_width_graceful() {
    // Zero width yields zero scaled width and proportionally scaled height (no panic).
    let (w, h) = scale_with_aspect_ratio(0.0, 100.0, 10.0, 10.0);
    assert_eq!(w, 0.0);
    assert_eq!(h, 10.0);
}

#[test]
fn icon_of_path_fallback_for_non_image() {
    // Non-image path should still return some data via NSWorkspace fallback.
    let cwd = std::env::current_dir().unwrap();
    let data = icon_of_path(cwd.to_str().unwrap(), 64.0).expect("fallback icon should exist");
    assert!(!data.is_empty());
}
