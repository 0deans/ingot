use super::models::{
    resolve_mojang_url, validate_content_path, RawNewsFeed, RawPatchNoteDetail, RawPatchNotesFeed,
};
use super::service::NewsService;

#[test]
fn validates_patch_note_content_paths_and_rejects_traversal() {
    assert!(validate_content_path("javaPatchNotes/26-4-snapshot-6.json"));
    assert!(validate_content_path("javaPatchNotes/1.21.4.json"));
    assert!(validate_content_path(
        "javaPatchNotes/8df071b98ac92911ae6caae35feb68506bca66894d16f41797164b9a9c6d1824.json"
    ));

    assert!(!validate_content_path("../javaPatchNotes/escape.json"));
    assert!(!validate_content_path("javaPatchNotes/../../escape.json"));
    assert!(!validate_content_path("javaPatchNotes/escape.txt"));
    assert!(!validate_content_path("otherPrefix/1.21.json"));
    assert!(!validate_content_path("javaPatchNotes/.json"));
    assert!(!validate_content_path(""));
    assert!(!validate_content_path("javaPatchNotes/nested/sub.json"));
}

#[test]
fn resolves_relative_and_absolute_mojang_image_urls() {
    assert_eq!(
        resolve_mojang_url("/v2/images/snapshot.jpg"),
        "https://launchercontent.mojang.com/v2/images/snapshot.jpg"
    );
    assert_eq!(
        resolve_mojang_url("v2/images/snapshot.jpg"),
        "https://launchercontent.mojang.com/v2/images/snapshot.jpg"
    );
    assert_eq!(
        resolve_mojang_url("https://example.com/custom.png"),
        "https://example.com/custom.png"
    );
}

#[test]
fn deserializes_raw_news_payload() {
    let raw_json = r#"{
        "version": 1,
        "entries": [
            {
                "title": "Halloween on Realms",
                "tag": "News",
                "category": "Minecraft: Java Edition",
                "date": "2026-10-30",
                "text": "12 new maps to send a shiver down your spine!",
                "newsPageImage": {
                    "title": "halloweenrealms_772x350.jpg",
                    "url": "/v2/images/halloweenrealms772x350.jpg"
                },
                "readMoreLink": "https://www.minecraft.net/article/halloween-on-realms",
                "newsType": ["Java", "News page"],
                "id": "82411625b4ff68af6a5d7cefed84c45ec777208b9dadab44ec6bd571882a547c"
            }
        ]
    }"#;

    let feed: RawNewsFeed = serde_json::from_str(raw_json).expect("should parse news feed");
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.title, "Halloween on Realms");
    assert_eq!(entry.tag.as_deref(), Some("News"));
    assert_eq!(entry.news_type, vec!["Java", "News page"]);
}

#[test]
fn deserializes_raw_patch_notes_payload() {
    let raw_json = r#"{
        "version": 1,
        "entries": [
            {
                "title": "Minecraft 26.4 Snapshot 6",
                "version": "26.4-snapshot-6",
                "type": "snapshot",
                "image": {
                    "title": "26.4-snapshot-6 540x540.jpg",
                    "url": "/v2/images/26.4snapshot6540x540.jpg"
                },
                "contentPath": "javaPatchNotes/26-4-snapshot-6.json",
                "id": "26-4-snapshot-6",
                "date": "2026-10-27T13:40:29.000Z",
                "shortText": "Kai is ice fishing..."
            }
        ]
    }"#;

    let feed: RawPatchNotesFeed =
        serde_json::from_str(raw_json).expect("should parse patch notes feed");
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.version, "26.4-snapshot-6");
    assert_eq!(entry.patch_type, "snapshot");
    assert_eq!(entry.content_path, "javaPatchNotes/26-4-snapshot-6.json");
}

#[test]
fn deserializes_raw_patch_note_detail() {
    let raw_json = r#"{
        "title": "Minecraft 26.4 Snapshot 6",
        "version": "26.4-snapshot-6",
        "type": "snapshot",
        "image": {
            "title": "26.4-snapshot-6 540x540.jpg",
            "url": "/v2/images/26.4snapshot6540x540.jpg"
        },
        "body": "<h1>New Features</h1><p>Ice bricks</p>",
        "id": "26-4-snapshot-6",
        "date": "2026-10-27T13:40:29.000Z"
    }"#;

    let detail: RawPatchNoteDetail =
        serde_json::from_str(raw_json).expect("should parse patch note detail");
    assert_eq!(detail.title.as_deref(), Some("Minecraft 26.4 Snapshot 6"));
    assert_eq!(
        detail.body.as_deref(),
        Some("<h1>New Features</h1><p>Ice bricks</p>")
    );
}

#[test]
fn deserializes_legacy_patch_note_detail_without_date() {
    let raw_json = r#"{
        "title": "Minecraft Snapshot 22w07a",
        "type": "snapshot",
        "version": "22w07a",
        "image": {
            "url": "/v2/images/7iEIyVlLwodStIqiNAwuY1MinecraftSnapshot22w07a.jpeg",
            "title": "Minecraft Snapshot 22w07a"
        },
        "body": "<p>Another snapshot is now available</p>",
        "id": "2xdqMSspcov06H8r9v46rA"
    }"#;

    let detail: RawPatchNoteDetail =
        serde_json::from_str(raw_json).expect("should parse legacy patch note without date");
    assert_eq!(detail.id, "2xdqMSspcov06H8r9v46rA");
    assert_eq!(detail.version.as_deref(), Some("22w07a"));
    assert!(detail.date.is_none());
    assert_eq!(
        detail.body.as_deref(),
        Some("<p>Another snapshot is now available</p>")
    );
}

#[tokio::test]
async fn rejects_invalid_content_path_without_http_request() {
    let temp_dir = tempfile::tempdir().unwrap();
    let service = NewsService::new(reqwest::Client::new(), temp_dir.path().to_path_buf());

    let result = service.get_patch_note_details("../traversal.json").await;
    assert!(result.is_err());
}
