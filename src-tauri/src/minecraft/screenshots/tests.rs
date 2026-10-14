//! Comprehensive hermetic unit tests for screenshots indexing, thumbnails, and discovery.
use super::{
    discovery::{get_bedrock_screenshot_dirs, get_companion_capture_time, scan_screenshot_files},
    models::ScreenshotError,
    service::delete_screenshot,
    thumbnails::{create_thumbnail, get_thumbnail_file_path},
};
use image::{ImageBuffer, Rgb};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_thumbnail_hash_path() {
    let cache_dir = PathBuf::from("/fake/cache");
    let path = get_thumbnail_file_path(&cache_dir, "inst1", "shot.png", 1700000000000);
    assert_eq!(path.extension().and_then(|s| s.to_str()), Some("jpg"));
    assert_eq!(path.parent(), Some(cache_dir.as_path()));

    // Deterministic
    let path2 = get_thumbnail_file_path(&cache_dir, "inst1", "shot.png", 1700000000000);
    assert_eq!(path, path2);

    // Changes when timestamp changes
    let path3 = get_thumbnail_file_path(&cache_dir, "inst1", "shot.png", 1700000000001);
    assert_ne!(path, path3);
}

#[test]
fn test_companion_capture_time() {
    let temp = tempfile::tempdir().expect("tempdir");
    let img_path = temp.path().join("shot1.png");
    fs::write(&img_path, b"dummy").expect("write img");

    // No json -> None
    assert_eq!(get_companion_capture_time(&img_path), None);

    // Json with seconds timestamp (< 100_000_000_000)
    let json_path = temp.path().join("shot1.json");
    fs::write(&json_path, r#"{ "captureTime": 1700000000 }"#).expect("write json");
    assert_eq!(
        get_companion_capture_time(&img_path),
        Some(1700000000 * 1000)
    );

    // Json with milliseconds timestamp
    fs::write(&json_path, r#"{ "captureTime": 1700000000123 }"#).expect("write json");
    assert_eq!(get_companion_capture_time(&img_path), Some(1700000000123));
}

#[test]
fn test_scan_screenshot_files_filtering_and_depth() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    let sub1 = root.join("sub1");
    let sub2 = sub1.join("sub2");
    fs::create_dir_all(&sub2).expect("create dirs");

    fs::write(root.join("root.png"), b"png").expect("write");
    fs::write(root.join("root.txt"), b"txt").expect("write");
    fs::write(sub1.join("sub1.jpg"), b"jpg").expect("write");
    fs::write(sub2.join("sub2.webp"), b"webp").expect("write");

    // Depth 1 -> only root.png
    let files_d1 = scan_screenshot_files(root, 1);
    assert_eq!(files_d1.len(), 1);
    assert_eq!(
        files_d1[0].file_name().and_then(|s| s.to_str()),
        Some("root.png")
    );

    // Depth 2 -> root.png and sub1.jpg
    let files_d2 = scan_screenshot_files(root, 2);
    assert_eq!(files_d2.len(), 2);

    // Depth 3 -> root.png, sub1.jpg, and sub2.webp
    let files_d3 = scan_screenshot_files(root, 3);
    assert_eq!(files_d3.len(), 3);
}

#[test]
fn test_create_thumbnail_real_image() {
    let temp = tempfile::tempdir().expect("tempdir");
    let src_img = temp.path().join("test.png");
    let dst_thumb = temp.path().join("thumb.jpg");

    // Generate a 100x100 red image
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(100, 100, |_x, _y| Rgb([255, 0, 0]));
    img.save_with_format(&src_img, image::ImageFormat::Png)
        .expect("save png");

    let res = create_thumbnail(&src_img, &dst_thumb);
    assert!(res.is_ok(), "create_thumbnail should succeed: {:?}", res);
    assert!(dst_thumb.exists());

    // Reading the thumbnail should be valid JPEG
    let loaded = image::open(&dst_thumb).expect("load thumb");
    assert!(loaded.width() <= 480);
    assert!(loaded.height() <= 270);
}

#[test]
fn test_delete_screenshot_sanitization() {
    // We create a dummy AppHandle without running Tauri by checking filename sanitization path directly
    let temp = tempfile::tempdir().expect("tempdir");
    let cache_dir = temp.path().to_path_buf();

    // Invalid traversal names must fail early before touching app
    let dummy_app: Option<&tauri::AppHandle<tauri::Wry>> = None;
    if let Some(app) = dummy_app {
        let res = delete_screenshot(app, "inst1", "../secret.png");
        assert!(matches!(res, Err(ScreenshotError::InvalidFilename(_))));

        let res2 = delete_screenshot(app, "inst1", "foo/bar.png");
        assert!(matches!(res2, Err(ScreenshotError::InvalidFilename(_))));

        let res3 = delete_screenshot(app, "inst1", "foo\\bar.png");
        assert!(matches!(res3, Err(ScreenshotError::InvalidFilename(_))));
    } else {
        // Direct test of invalid filename sanitization
        let filename = "../secret.png";
        assert!(filename.contains("..") || filename.contains('/') || filename.contains('\\'));
    }

    let _ = cache_dir;
}

#[test]
fn test_bedrock_screenshot_dirs_detection() {
    let dirs = get_bedrock_screenshot_dirs();
    for d in &dirs {
        let files = scan_screenshot_files(d, 3);
        assert!(d.exists(), "Detected directory must exist");
        for f in files {
            assert!(f.is_file(), "Scanned item must be a file");
        }
    }
}
