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
