use super::*;
use crate::minecraft::launcher::models::LaunchError;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy)]
enum Failure {
    None,
    Java,
    Authentication,
}

struct FakePreparation {
    failure: Failure,
    java_calls: AtomicUsize,
    auth_calls: AtomicUsize,
}

impl FakePreparation {
    fn new(failure: Failure) -> Self {
        Self {
            failure,
            java_calls: AtomicUsize::new(0),
            auth_calls: AtomicUsize::new(0),
        }
    }

    fn success() -> Self {
        Self::new(Failure::None)
    }
}

impl PreparationEffects for FakePreparation {
    fn ensure_java<'a>(
        &'a self,
        _: &'a reqwest::Client,
        major_version: u32,
        _: Option<&'a str>,
        _: Option<crate::artifacts::java::JavaProgressFn>,
    ) -> PreparationFuture<'a, PathBuf> {
        Box::pin(async move {
            self.java_calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(major_version, 21);
            if matches!(self.failure, Failure::Java) {
                return Err(LaunchError::JavaRuntime(
                    crate::artifacts::java::JavaError::MissingBinary,
                ));
            }
            Ok(PathBuf::from("fixture-java"))
        })
    }

    fn resolve_auth(&self) -> PreparationFuture<'_, ResolvedAuth> {
        Box::pin(async move {
            self.auth_calls.fetch_add(1, Ordering::SeqCst);
            if matches!(self.failure, Failure::Authentication) {
                #[derive(Debug, thiserror::Error)]
                #[error("fixture account not found")]
                struct MockAccountNotFound;

                return Err(LaunchError::Account(
                    crate::minecraft::launcher::models::LaunchAccountError::new(
                        MockAccountNotFound,
                    ),
                ));
            }
            Ok(ResolvedAuth {
                player_name: "fixture-player".into(),
                uuid_str: "fixture-id".into(),
                access_token: "fixture-token".into(),
                user_type: "msa",
                xuid: None,
                is_ely: false,
            })
        })
    }
}

fn write_version_package(directory: &Path, id: &str, package: &serde_json::Value) {
    let package_directory = directory.join(format!("cache/versions/{id}"));
    fs::create_dir_all(&package_directory).unwrap();
    fs::write(
        package_directory.join(format!("{id}.json")),
        serde_json::to_vec(package).unwrap(),
    )
    .unwrap();
}

fn write_asset_index(directory: &Path, id: &str, index: &serde_json::Value) {
    let indexes = directory.join("assets/indexes");
    fs::create_dir_all(&indexes).unwrap();
    fs::write(
        indexes.join(format!("{id}.json")),
        serde_json::to_vec(index).unwrap(),
    )
    .unwrap();
}

fn write_asset_object(directory: &Path, hash: &str, content: &[u8]) {
    let prefix = &hash[..2];
    let obj_dest = directory.join("assets/objects").join(prefix).join(hash);
    fs::create_dir_all(obj_dest.parent().unwrap()).unwrap();
    fs::write(&obj_dest, content).unwrap();
}

fn write_library_jar(directory: &Path, rel_path: &str, content: &[u8]) {
    let lib_dest = directory.join("libraries").join(rel_path);
    fs::create_dir_all(lib_dest.parent().unwrap()).unwrap();
    fs::write(&lib_dest, content).unwrap();
}

fn write_loader_profile(directory: &Path, filename: &str, profile: &serde_json::Value) {
    let profiles_dir = directory.join("cache/loader_profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    fs::write(
        profiles_dir.join(filename),
        serde_json::to_vec(profile).unwrap(),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, InstanceConfig) {
    let directory = tempfile::tempdir().unwrap();
    let package = serde_json::json!({ "id": "fixture", "libraries": [], "mainClass": "fixture.Main", "javaVersion": { "component": "fixture", "majorVersion": 21 } });
    write_version_package(directory.path(), "fixture", &package);
    let instance = crate::minecraft::instance::prepare_instance(
        "fixture".into(),
        "fixture".into(),
        ModLoaderType::Vanilla,
        None,
    )
    .unwrap();
    (directory, instance)
}

async fn prepare(
    directory: &Path,
    instance: &InstanceConfig,
    effects: &FakePreparation,
) -> Result<LaunchPreparation, LaunchError> {
    let client = reqwest::Client::new();
    prepare_launch_environment(
        effects,
        &client,
        instance,
        &directory.join("cache"),
        &directory.join("assets"),
        &directory.join("libraries"),
        Arc::new(|_, _, _, _| {}),
    )
    .await
}

#[tokio::test]
async fn preparation_uses_injected_java_and_auth_without_native_runtime() {
    let (directory, instance) = fixture();
    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &instance, &effects)
        .await
        .unwrap();
    assert_eq!(result.java_bin, PathBuf::from("fixture-java"));
    assert_eq!(result.auth.player_name, "fixture-player");
    assert_eq!(result.auth.access_token, "fixture-token");
    assert_eq!(effects.java_calls.load(Ordering::SeqCst), 1);
    assert_eq!(effects.auth_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn java_failure_prevents_authentication_and_launch() {
    let (directory, instance) = fixture();
    let effects = FakePreparation::new(Failure::Java);
    assert!(matches!(
        prepare(directory.path(), &instance, &effects).await,
        Err(LaunchError::JavaRuntime(_))
    ));
    assert_eq!(effects.auth_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn authentication_failure_is_not_converted_to_offline_credentials() {
    let (directory, instance) = fixture();
    let effects = FakePreparation::new(Failure::Authentication);
    assert!(matches!(
        prepare(directory.path(), &instance, &effects).await,
        Err(LaunchError::Account(_))
    ));
}

#[test]
fn asset_task_rejects_invalid_hashes_before_constructing_paths_or_requests() {
    let directory = tempfile::tempdir().unwrap();
    for hash in ["", "a", "../unsafe", &"z".repeat(40), &"é".repeat(20)] {
        let object = crate::minecraft::version::AssetObject {
            hash: hash.into(),
            size: 1,
        };
        assert!(matches!(
            prepare_asset_task(directory.path(), object),
            Err(DownloadError::InvalidAssetHash)
        ));
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn asset_task_preserves_valid_hash_url_path_and_integrity_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let hash = "ab01234567890123456789012345678901234567";
    let object = crate::minecraft::version::AssetObject {
        hash: hash.into(),
        size: 12,
    };
    let task = prepare_asset_task(directory.path(), object).unwrap();
    assert_eq!(
        task.url,
        format!("https://resources.download.minecraft.net/ab/{hash}")
    );
    assert_eq!(task.dest, directory.path().join("objects/ab").join(hash));
    assert_eq!(task.sha1.as_deref(), Some(hash));
    assert_eq!(task.size, Some(12));
}

#[tokio::test]
async fn invalid_cached_asset_stops_preparation_without_network_or_authentication() {
    let (directory, instance) = fixture();
    let package_path = directory.path().join("cache/versions/fixture/fixture.json");
    let mut package: serde_json::Value =
        serde_json::from_slice(&fs::read(&package_path).unwrap()).unwrap();
    package["assetIndex"] = serde_json::json!({"id": "fixture", "sha1": "", "size": 0, "url": "invalid://not-requested"});
    fs::write(package_path, serde_json::to_vec(&package).unwrap()).unwrap();
    let indexes = directory.path().join("assets/indexes");
    fs::create_dir_all(&indexes).unwrap();
    let index = serde_json::json!({"objects": {"invalid": {"hash": "../unsafe", "size": 1}}});
    write_asset_index(directory.path(), "fixture", &index);
    let effects = FakePreparation::success();
    // Route any accidental external request to an observable loopback listener.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = reqwest::Proxy::all(format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    let result = prepare_launch_environment(
        &effects,
        &client,
        &instance,
        &directory.path().join("cache"),
        &directory.path().join("assets"),
        &directory.path().join("libraries"),
        Arc::new(|_, _, _, _| {}),
    )
    .await;
    assert!(matches!(
        result,
        Err(LaunchError::Download(DownloadError::InvalidAssetHash))
    ));
    assert_eq!(effects.auth_calls.load(Ordering::SeqCst), 0);
    assert!(!directory.path().join("assets/objects").exists());
    assert!(
        matches!(listener.into_std().unwrap().accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[tokio::test]
async fn preparation_deduplicates_duplicate_libraries() {
    let (directory, instance) = fixture();
    let lib_artifact = serde_json::json!({
        "path": "com/mojang/text2speech/1.10.3/text2speech-1.10.3.jar",
        "url": "https://libraries.minecraft.net/com/mojang/text2speech/1.10.3/text2speech-1.10.3.jar",
        "size": 17
    });
    // Duplicate library entries with identical artifact destination
    let package = serde_json::json!({
        "id": "fixture",
        "libraries": [
            {
                "name": "com.mojang:text2speech:1.10.3",
                "downloads": { "artifact": lib_artifact }
            },
            {
                "name": "com.mojang:text2speech:1.10.3",
                "downloads": { "artifact": lib_artifact }
            }
        ],
        "mainClass": "fixture.Main",
        "javaVersion": { "component": "fixture", "majorVersion": 21 }
    });
    write_version_package(directory.path(), "fixture", &package);
    write_library_jar(
        directory.path(),
        "com/mojang/text2speech/1.10.3/text2speech-1.10.3.jar",
        b"dummy jar content",
    );

    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &instance, &effects)
        .await
        .unwrap();

    let occurrences = result
        .classpath_entries
        .iter()
        .filter(|p| p.ends_with("text2speech-1.10.3.jar"))
        .count();
    assert_eq!(occurrences, 1);
}

#[tokio::test]
async fn preparation_deduplicates_duplicate_asset_hashes() {
    let (directory, instance) = fixture();
    let package_path = directory.path().join("cache/versions/fixture/fixture.json");
    let mut package: serde_json::Value =
        serde_json::from_slice(&fs::read(&package_path).unwrap()).unwrap();
    package["assetIndex"] = serde_json::json!({
        "id": "fixture",
        "sha1": "test_sha",
        "size": 100,
        "url": "http://127.0.0.1:9/unused"
    });
    write_version_package(directory.path(), "fixture", &package);

    // SHA-1 of b"test" is a94a8fe5ccb19ba61c4c0873d391e987982fbbd3 (size 4)
    let shared_hash = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";
    let index = serde_json::json!({
        "objects": {
            "minecraft/sounds/sound1.ogg": { "hash": shared_hash, "size": 4 },
            "minecraft/sounds/sound2.ogg": { "hash": shared_hash, "size": 4 }
        }
    });
    write_asset_index(directory.path(), "fixture", &index);
    write_asset_object(directory.path(), shared_hash, b"test");

    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &instance, &effects).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn preparation_missing_loader_version_unresolvable_fails_with_loader_version_not_selected() {
    let (directory, _) = fixture();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = reqwest::Proxy::all(format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_millis(50))
        .build()
        .unwrap();

    let instance = crate::minecraft::instance::prepare_instance(
        "forge-fixture".into(),
        "fixture".into(),
        ModLoaderType::Forge,
        None,
    )
    .unwrap();

    let effects = FakePreparation::success();

    let result = prepare_launch_environment(
        &effects,
        &client,
        &instance,
        &directory.path().join("cache"),
        &directory.path().join("assets"),
        &directory.path().join("libraries"),
        Arc::new(|_, _, _, _| {}),
    )
    .await;

    assert!(matches!(result, Err(LaunchError::LoaderVersionNotSelected)));
}

#[tokio::test]
async fn preparation_loader_libraries_take_precedence_over_vanilla_libraries() {
    let (directory, _) = fixture();
    let package = serde_json::json!({
        "id": "fixture",
        "libraries": [
            {
                "name": "com.google.guava:guava:15.0",
                "downloads": {
                    "artifact": {
                        "path": "com/google/guava/guava/15.0/guava-15.0.jar",
                        "url": "http://127.0.0.1:9/guava-15.0.jar",
                        "size": 10
                    }
                }
            }
        ],
        "mainClass": "fixture.Main",
        "javaVersion": { "component": "fixture", "majorVersion": 21 }
    });
    write_version_package(directory.path(), "fixture", &package);

    let profile = serde_json::json!({
        "id": "forge-profile",
        "main_class": "net.minecraft.launchwrapper.Launch",
        "libraries": [
            {
                "name": "com.google.guava:guava:17.0",
                "downloads": {
                    "artifact": {
                        "path": "com/google/guava/guava/17.0/guava-17.0.jar",
                        "url": "http://127.0.0.1:9/guava-17.0.jar",
                        "size": 10
                    }
                }
            }
        ],
        "jvm_args": [],
        "game_args": []
    });
    write_loader_profile(directory.path(), "v5_Forge_fixture_1.0.json", &profile);
    write_library_jar(
        directory.path(),
        "com/google/guava/guava/17.0/guava-17.0.jar",
        b"0123456789",
    );

    let forge_instance = crate::minecraft::instance::prepare_instance(
        "forge-fixture".into(),
        "fixture".into(),
        ModLoaderType::Forge,
        Some("1.0".into()),
    )
    .unwrap();

    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &forge_instance, &effects)
        .await
        .unwrap();

    assert!(result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("guava-17.0.jar")));
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("guava-15.0.jar")));
}

#[tokio::test]
async fn preparation_collects_natives_into_native_jars_and_not_classpath() {
    let (directory, instance) = fixture();
    let os_key = current_os_key();
    let package = serde_json::json!({
        "id": "fixture",
        "libraries": [
            {
                "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.4",
                "natives": {
                    os_key: "natives-windows"
                },
                "downloads": {
                    "classifiers": {
                        "natives-windows": {
                            "path": "org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows.jar",
                            "url": "http://127.0.0.1:9/natives.jar",
                            "size": 10
                        }
                    }
                }
            }
        ],
        "mainClass": "fixture.Main",
        "javaVersion": { "component": "fixture", "majorVersion": 21 }
    });
    write_version_package(directory.path(), "fixture", &package);
    write_library_jar(
        directory.path(),
        "org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows.jar",
        b"0123456789",
    );

    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &instance, &effects)
        .await
        .unwrap();

    assert_eq!(result.native_jars.len(), 1);
    assert!(result.native_jars[0].ends_with("lwjgl-platform-2.9.4-natives-windows.jar"));
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("lwjgl-platform-2.9.4-natives-windows.jar")));
}

#[tokio::test]
async fn preparation_downloads_maven_files_without_adding_to_classpath_or_shadowing_vanilla() {
    let (directory, _) = fixture();
    let package = serde_json::json!({
        "id": "fixture",
        "libraries": [
            {
                "name": "com.google.guava:guava:21.0",
                "downloads": {
                    "artifact": {
                        "path": "com/google/guava/guava/21.0/guava-21.0.jar",
                        "url": "http://127.0.0.1:9/guava-21.0.jar",
                        "size": 10
                    }
                }
            },
            {
                "name": "org.apache.logging.log4j:log4j-api:2.8.1",
                "downloads": {
                    "artifact": {
                        "path": "org/apache/logging/log4j/log4j-api/2.8.1/log4j-api-2.8.1.jar",
                        "url": "http://127.0.0.1:9/log4j-api-2.8.1.jar",
                        "size": 10
                    }
                }
            },
            {
                "name": "org.apache.logging.log4j:log4j-core:2.8.1",
                "downloads": {
                    "artifact": {
                        "path": "org/apache/logging/log4j/log4j-core/2.8.1/log4j-core-2.8.1.jar",
                        "url": "http://127.0.0.1:9/log4j-core-2.8.1.jar",
                        "size": 10
                    }
                }
            }
        ],
        "mainClass": "fixture.Main",
        "javaVersion": { "component": "fixture", "majorVersion": 21 }
    });
    write_version_package(directory.path(), "fixture", &package);

    let profile = serde_json::json!({
        "id": "forge-fixture-1.0",
        "main_class": "io.github.zekerzhayard.forgewrapper.installer.Main",
        "libraries": [
            {
                "name": "org.apache.logging.log4j:log4j-api:2.15.0",
                "downloads": {
                    "artifact": {
                        "path": "org/apache/logging/log4j/log4j-api/2.15.0/log4j-api-2.15.0.jar",
                        "url": "http://127.0.0.1:9/log4j-api-2.15.0.jar",
                        "size": 10
                    }
                }
            },
            {
                "name": "org.apache.logging.log4j:log4j-core:2.15.0",
                "downloads": {
                    "artifact": {
                        "path": "org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar",
                        "url": "http://127.0.0.1:9/log4j-core-2.15.0.jar",
                        "size": 10
                    }
                }
            },
            {
                "name": "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0",
                "downloads": {
                    "artifact": {
                        "path": "org/apache/logging/log4j/log4j-slf4j18-impl/2.15.0/log4j-slf4j18-impl-2.15.0.jar",
                        "url": "http://127.0.0.1:9/log4j-slf4j18-impl-2.15.0.jar",
                        "size": 10
                    }
                }
            }
        ],
        "maven_files": [
            {
                "name": "com.google.guava:guava:20.0",
                "downloads": {
                    "artifact": {
                        "path": "com/google/guava/guava/20.0/guava-20.0.jar",
                        "url": "http://127.0.0.1:9/guava-20.0.jar",
                        "size": 10
                    }
                }
            },
            {
                "name": "net.minecraftforge:forge:1.16.5-36.2.39:installer",
                "downloads": {
                    "artifact": {
                        "path": "net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-installer.jar",
                        "url": "http://127.0.0.1:9/installer.jar",
                        "size": 10
                    }
                }
            }
        ],
        "jvm_args": [],
        "game_args": []
    });
    write_loader_profile(directory.path(), "v5_Forge_fixture_1.0.json", &profile);

    for rel_path in [
        "com/google/guava/guava/21.0/guava-21.0.jar",
        "com/google/guava/guava/20.0/guava-20.0.jar",
        "org/apache/logging/log4j/log4j-api/2.15.0/log4j-api-2.15.0.jar",
        "org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar",
        "org/apache/logging/log4j/log4j-slf4j18-impl/2.15.0/log4j-slf4j18-impl-2.15.0.jar",
        "org/apache/logging/log4j/log4j-api/2.8.1/log4j-api-2.8.1.jar",
        "org/apache/logging/log4j/log4j-core/2.8.1/log4j-core-2.8.1.jar",
        "net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-installer.jar",
    ] {
        write_library_jar(directory.path(), rel_path, b"0123456789");
    }

    let forge_instance = crate::minecraft::instance::prepare_instance(
        "forge-fixture".into(),
        "fixture".into(),
        ModLoaderType::Forge,
        Some("1.0".into()),
    )
    .unwrap();

    let effects = FakePreparation::success();
    let result = prepare(directory.path(), &forge_instance, &effects)
        .await
        .unwrap();

    // Guava 21.0 from vanilla should be on the classpath
    assert!(result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("guava-21.0.jar")));

    // Maven files (guava 20.0 and installer) must NOT be on the classpath
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("guava-20.0.jar")));
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("forge-1.16.5-36.2.39-installer.jar")));

    // Log4j 2.15.0 override from loader profile must be on classpath
    assert!(result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("log4j-api-2.15.0.jar")));
    assert!(result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("log4j-core-2.15.0.jar")));
    assert!(result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("log4j-slf4j18-impl-2.15.0.jar")));

    // Vanilla log4j 2.8.1 must be overridden and not on classpath
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("log4j-api-2.8.1.jar")));
    assert!(!result
        .classpath_entries
        .iter()
        .any(|p| p.ends_with("log4j-core-2.8.1.jar")));
}
