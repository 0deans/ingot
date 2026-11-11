use super::*;
use crate::version::{AssetObject, VersionPackage};
use std::collections::HashMap;

#[test]
fn test_preparer_binary_name() {
    let name = preparer_binary_name();
    #[cfg(target_os = "windows")]
    assert_eq!(name, "horizon-prepare.exe");
    #[cfg(not(target_os = "windows"))]
    assert_eq!(name, "horizon-prepare");
}

#[test]
fn test_is_horizon_prepared_false_for_empty_dir() {
    let temp = tempfile::tempdir().unwrap();
    assert!(!is_horizon_prepared(temp.path(), "26.3"));
}

#[test]
fn test_is_horizon_complete() {
    let temp = tempfile::tempdir().unwrap();
    let res_dir = temp.path();

    // 1. Empty dir -> false
    assert!(!is_horizon_complete(res_dir));

    // 2. Only horizon-resources.json -> false
    fs::write(res_dir.join("horizon-resources.json"), b"{}").unwrap();
    assert!(!is_horizon_complete(res_dir));

    // 3. Both horizon-resources.json and asset-index.json -> true
    fs::write(res_dir.join("asset-index.json"), b"{}").unwrap();
    assert!(is_horizon_complete(res_dir));
}

#[test]
fn test_collect_library_tasks_filters_disallowed_os_and_maps_paths() {
    let temp = tempfile::tempdir().unwrap();
    let libraries_dir = temp.path().join("libraries");
    let manifest_json = r#"{
        "id": "26.3",
        "mainClass": "net.minecraft.client.main.Main",
        "libraries": [
            {
                "name": "at.yawk.lz4:lz4-java:1.10.1",
                "downloads": {
                    "artifact": {
                        "path": "at/yawk/lz4/lz4-java/1.10.1/lz4-java-1.10.1.jar",
                        "sha1": "f541d7f910fe3d76f38f799c507c48cc81b12ecb",
                        "size": 910232,
                        "url": "https://libraries.minecraft.net/at/yawk/lz4/lz4-java/1.10.1/lz4-java-1.10.1.jar"
                    }
                }
            },
            {
                "name": "custom.maven:lib:2.0",
                "url": "https://custom.repo/"
            },
            {
                "name": "disallowed:os-lib:1.0",
                "downloads": {
                    "artifact": {
                        "path": "disallowed/os-lib/1.0/os-lib-1.0.jar",
                        "sha1": "abcdef123456",
                        "size": 1234,
                        "url": "https://example.com/disallowed.jar"
                    }
                },
                "rules": [
                    {
                        "action": "allow",
                        "os": { "name": "nonexistent_os" }
                    }
                ]
            }
        ]
    }"#;

    let pkg: VersionPackage = serde_json::from_str(manifest_json).unwrap();
    let tasks = collect_library_tasks(&pkg, &libraries_dir);

    assert_eq!(tasks.len(), 2);
    assert_eq!(
        tasks[0].dest,
        libraries_dir.join("at/yawk/lz4/lz4-java/1.10.1/lz4-java-1.10.1.jar")
    );
    assert_eq!(
        tasks[0].sha1.as_deref(),
        Some("f541d7f910fe3d76f38f799c507c48cc81b12ecb")
    );
    assert_eq!(tasks[0].size, Some(910232));
    assert_eq!(
        tasks[0].url,
        "https://libraries.minecraft.net/at/yawk/lz4/lz4-java/1.10.1/lz4-java-1.10.1.jar"
    );

    assert_eq!(
        tasks[1].dest,
        libraries_dir.join("custom/maven/lib/2.0/lib-2.0.jar")
    );
    assert_eq!(
        tasks[1].url,
        "https://custom.repo/custom/maven/lib/2.0/lib-2.0.jar"
    );
    assert_eq!(tasks[1].sha1, None);
}

#[test]
fn test_collect_asset_tasks_deduplicates_and_validates() {
    let temp = tempfile::tempdir().unwrap();
    let assets_dir = temp.path();

    let mut objects = HashMap::new();
    let valid_hash_1 = "e787e8afd149a4cefa89d341b91f5aed8f5d93a0".to_string();
    let valid_hash_2 = "abcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string();

    objects.insert(
        "minecraft/sounds/click.ogg".to_string(),
        AssetObject {
            hash: valid_hash_1.clone(),
            size: 1024,
        },
    );
    // Duplicate hash with different key:
    objects.insert(
        "minecraft/sounds/click_alias.ogg".to_string(),
        AssetObject {
            hash: valid_hash_1.clone(),
            size: 1024,
        },
    );
    // Second distinct valid object:
    objects.insert(
        "minecraft/textures/gui/title/background/panorama_0.png".to_string(),
        AssetObject {
            hash: valid_hash_2.clone(),
            size: 2048,
        },
    );
    // Invalid hash (short length):
    objects.insert(
        "invalid/short.png".to_string(),
        AssetObject {
            hash: "short".to_string(),
            size: 50,
        },
    );
    // Invalid hash (non-hex characters):
    objects.insert(
        "invalid/nonhex.png".to_string(),
        AssetObject {
            hash: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string(),
            size: 50,
        },
    );

    let tasks = collect_asset_tasks(assets_dir, &objects);
    assert_eq!(tasks.len(), 2);

    let task1 = tasks
        .iter()
        .find(|t| t.sha1.as_deref() == Some(&valid_hash_1))
        .unwrap();
    assert_eq!(
        task1.dest,
        assets_dir.join("objects").join("e7").join(&valid_hash_1)
    );
    assert_eq!(
        task1.url,
        format!("https://resources.download.minecraft.net/e7/{valid_hash_1}")
    );
    assert_eq!(task1.size, Some(1024));

    let task2 = tasks
        .iter()
        .find(|t| t.sha1.as_deref() == Some(&valid_hash_2))
        .unwrap();
    assert_eq!(
        task2.dest,
        assets_dir.join("objects").join("ab").join(&valid_hash_2)
    );
    assert_eq!(
        task2.url,
        format!("https://resources.download.minecraft.net/ab/{valid_hash_2}")
    );
    assert_eq!(task2.size, Some(2048));
}
