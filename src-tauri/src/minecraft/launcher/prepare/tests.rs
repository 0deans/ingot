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

impl PreparationEffects for FakePreparation {
    fn ensure_java<'a>(
        &'a self,
        _: &'a reqwest::Client,
        major_version: u32,
        _: Option<&'a str>,
        _: Option<crate::minecraft::java::JavaProgressFn>,
    ) -> PreparationFuture<'a, PathBuf> {
        Box::pin(async move {
            self.java_calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(major_version, 21);
            if matches!(self.failure, Failure::Java) {
                return Err(LaunchError::JavaRuntime(
                    crate::minecraft::java::JavaError::MissingBinary,
                ));
            }
            Ok(PathBuf::from("fixture-java"))
        })
    }

    fn resolve_auth(&self) -> PreparationFuture<'_, ResolvedAuth> {
        Box::pin(async move {
            self.auth_calls.fetch_add(1, Ordering::SeqCst);
            if matches!(self.failure, Failure::Authentication) {
                return Err(LaunchError::Account(
                    crate::account::session::SessionError::AccountNotFound.into(),
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

fn fixture() -> (tempfile::TempDir, InstanceConfig) {
    let directory = tempfile::tempdir().unwrap();
    let package_directory = directory.path().join("cache/versions/fixture");
    fs::create_dir_all(&package_directory).unwrap();
    let package = serde_json::json!({ "id": "fixture", "libraries": [], "mainClass": "fixture.Main", "javaVersion": { "component": "fixture", "majorVersion": 21 } });
    fs::write(
        package_directory.join("fixture.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
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
    let effects = FakePreparation {
        failure: Failure::None,
        java_calls: AtomicUsize::new(0),
        auth_calls: AtomicUsize::new(0),
    };
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
    let effects = FakePreparation {
        failure: Failure::Java,
        java_calls: AtomicUsize::new(0),
        auth_calls: AtomicUsize::new(0),
    };
    assert!(matches!(
        prepare(directory.path(), &instance, &effects).await,
        Err(LaunchError::JavaRuntime(_))
    ));
    assert_eq!(effects.auth_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn authentication_failure_is_not_converted_to_offline_credentials() {
    let (directory, instance) = fixture();
    let effects = FakePreparation {
        failure: Failure::Authentication,
        java_calls: AtomicUsize::new(0),
        auth_calls: AtomicUsize::new(0),
    };
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
    fs::write(
        indexes.join("fixture.json"),
        serde_json::to_vec(&index).unwrap(),
    )
    .unwrap();
    let effects = FakePreparation {
        failure: Failure::None,
        java_calls: AtomicUsize::new(0),
        auth_calls: AtomicUsize::new(0),
    };
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
