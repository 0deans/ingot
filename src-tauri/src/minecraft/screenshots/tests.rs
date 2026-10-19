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
    let files_d1 = scan_screenshot_files(root, 1).unwrap();
    assert_eq!(files_d1.len(), 1);
    assert_eq!(
        files_d1[0].file_name().and_then(|s| s.to_str()),
        Some("root.png")
    );

    // Depth 2 -> root.png and sub1.jpg
    let files_d2 = scan_screenshot_files(root, 2).unwrap();
    assert_eq!(files_d2.len(), 2);

    // Depth 3 -> root.png, sub1.jpg, and sub2.webp
    let files_d3 = scan_screenshot_files(root, 3).unwrap();
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
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "../secret.png",
        "foo/bar.png",
        "foo\\bar.png",
        "file:stream.png",
        "CON.png",
    ] {
        let result = delete_screenshot(&[], temp.path(), "inst1", name);
        assert!(matches!(result, Err(ScreenshotError::InvalidFilename(_))));
    }
}
#[test]
#[ignore = "opt-in discovery smoke test reads installed Bedrock directories"]
fn test_bedrock_screenshot_dirs_detection() {
    let dirs = get_bedrock_screenshot_dirs();
    for d in &dirs {
        let files = scan_screenshot_files(d, 3).unwrap();
        assert!(d.exists(), "Detected directory must exist");
        for f in files {
            assert!(f.is_file(), "Scanned item must be a file");
        }
    }
}

#[test]
fn corrupt_image_does_not_publish_partial_thumbnail() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("invalid.png");
    fs::write(&source, b"invalid image").unwrap();
    let destination = temp.path().join("cache/thumb.jpg");
    assert!(matches!(
        create_thumbnail(&source, &destination),
        Err(ScreenshotError::Image { .. })
    ));
    assert!(!destination.exists());
}

#[test]
fn index_validates_thumbnail_identity_and_deletion_cleans_cached_capture() {
    use super::service::{
        get_all_screenshots, validate_thumbnail_request, ScreenshotKind, ScreenshotSource,
    };
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("captures");
    let cache = temp.path().join("cache");
    fs::create_dir_all(&directory).unwrap();
    fs::create_dir_all(&cache).unwrap();
    let image = directory.join("shot.png");
    fs::write(
        &image,
        b"image bytes are decoded only for thumbnail requests",
    )
    .unwrap();
    fs::write(
        directory.join("shot.json"),
        br#"{"captureTime":1700000000}"#,
    )
    .unwrap();
    fs::write(directory.join("shot.mc"), b"companion").unwrap();
    let sources = vec![ScreenshotSource {
        instance_id: "bedrock".into(),
        instance_name: "Bedrock".into(),
        directory,
        kind: ScreenshotKind::Bedrock,
    }];
    let items = get_all_screenshots(&sources, &cache).unwrap();
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.modified_at, 1700000000000);
    assert!(validate_thumbnail_request(item, &image, item.modified_at).is_ok());
    assert!(matches!(
        validate_thumbnail_request(item, &image, item.modified_at + 1),
        Err(ScreenshotError::StaleMetadata)
    ));
    assert!(matches!(
        validate_thumbnail_request(item, &temp.path().join("outside.png"), item.modified_at),
        Err(ScreenshotError::StaleMetadata)
    ));
    let cached = get_thumbnail_file_path(&cache, "bedrock", "shot.png", item.modified_at);
    fs::write(&cached, b"cached").unwrap();
    delete_screenshot(&sources, &cache, "bedrock", "shot.png").unwrap();
    assert!(!image.exists());
    assert!(!cached.exists());
    assert!(!image.with_extension("json").exists());
    assert!(!image.with_extension("mc").exists());
}

#[test]
fn scan_distinguishes_missing_directory_from_invalid_directory() {
    let temp = tempfile::tempdir().unwrap();
    assert!(scan_screenshot_files(&temp.path().join("missing"), 3)
        .unwrap()
        .is_empty());
    let file = temp.path().join("not-directory");
    fs::write(&file, b"file").unwrap();
    assert!(scan_screenshot_files(&file, 3).is_err());
}

#[test]
fn duplicate_bedrock_capture_names_do_not_delete_an_arbitrary_capture() {
    use super::service::{ScreenshotKind, ScreenshotSource};
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("captures");
    for user in ["one", "two"] {
        let user_directory = directory.join(user);
        fs::create_dir_all(&user_directory).unwrap();
        fs::write(user_directory.join("shot.png"), user).unwrap();
    }
    let sources = vec![ScreenshotSource {
        instance_id: "bedrock".into(),
        instance_name: "Bedrock".into(),
        directory: directory.clone(),
        kind: ScreenshotKind::Bedrock,
    }];
    let result = delete_screenshot(&sources, &temp.path().join("cache"), "bedrock", "shot.png");
    assert!(matches!(result, Err(ScreenshotError::AmbiguousFilename(_))));
    assert_eq!(fs::read(directory.join("one/shot.png")).unwrap(), b"one");
    assert_eq!(fs::read(directory.join("two/shot.png")).unwrap(), b"two");
}

#[test]
fn thumbnail_cache_directory_is_not_reported_as_an_existing_image() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("thumb.jpg");
    fs::create_dir(&destination).unwrap();
    let result = create_thumbnail(&temp.path().join("unused.png"), &destination);
    assert!(matches!(result, Err(ScreenshotError::Io(_))));
    assert!(destination.is_dir());
}

#[test]
fn thumbnail_destination_without_a_parent_is_an_explicit_path_failure() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("capture.png");
    ImageBuffer::<Rgb<u8>, Vec<u8>>::new(1, 1)
        .save(&source)
        .unwrap();
    let result = create_thumbnail(&source, std::path::Path::new(""));
    assert!(
        matches!(result, Err(ScreenshotError::MissingCacheParent(path)) if path.as_os_str().is_empty())
    );
    assert!(source.is_file());
}
