use super::*;
use ingot_accounts::account::{
    credentials::CredentialError, pending_login::PendingGrantError, policy::AccountPolicyError,
    recovery::RecoveryError, session::SessionError, AccountError,
};
use ingot_minecraft::launcher::LaunchError;
use ingot_version_change::VersionChangeError;

#[test]
fn instance_adapters_preserve_causes_and_api_categories() {
    use ingot_content::content::error::ContentError;
    use ingot_content::importer::models::ImportError;
    use ingot_content::instance_export::InstanceExportError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::{
        instance::InstanceError, logs::LogError, screenshots::ScreenshotError, sync::SyncError,
        worlds::WorldError,
    };
    use std::error::Error;

    fn failures() -> Vec<InstanceError> {
        vec![
            InstanceError::InvalidId,
            InstanceError::NotFound("missing-instance".into()),
            InstanceError::Io(std::io::ErrorKind::PermissionDenied.into()),
            InstanceError::Metadata(MetadataError::Conflict("busy instance".into())),
            InstanceError::Metadata(MetadataError::Decode {
                path: "instances.json".into(),
                source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
            }),
            InstanceError::Metadata(MetadataError::RecoveryBlocked("blocked recovery")),
            InstanceError::FilesCleanupCommitted {
                path: "retained-instance".into(),
                source: std::io::ErrorKind::PermissionDenied.into(),
            },
            InstanceError::Cleanup {
                primary: Box::new(InstanceError::InvalidId),
                cleanup: std::io::ErrorKind::PermissionDenied.into(),
            },
        ]
    }

    fn check<E: Error + Into<ApiError>>(wrap: impl Fn(InstanceError) -> E) {
        for (original, expected) in failures().into_iter().zip(failures()) {
            let wrapped = wrap(original);
            let source = wrapped.source().unwrap();
            let instance = source.downcast_ref::<InstanceError>().unwrap();
            if let InstanceError::Io(io) = instance {
                assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
            }
            assert_eq!(ApiError::from(expected), wrapped.into());
        }
    }

    check(LaunchError::InstanceDir);
    check(ScreenshotError::InstanceDir);
    check(ScreenshotError::LoadInstances);
    check(ImportError::InstanceDir);
    check(ImportError::InstanceCreate);
    check(InstanceExportError::Instance);
    check(WorldError::InstanceDir);
    check(WorldError::LoadInstances);
    check(SyncError::Instances);
    check(ContentError::Instance);
    check(LogError::Instance);
}

#[test]
fn importer_native_path_failure_preserves_its_source() {
    use ingot_content::importer::models::ImportError;
    use std::error::Error;

    let native_error = tauri::Error::Io(std::io::ErrorKind::PermissionDenied.into());
    let error = ImportError::NativePath(std::io::Error::other(native_error));
    let io = error
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    let native = io
        .get_ref()
        .unwrap()
        .downcast_ref::<tauri::Error>()
        .unwrap();
    assert!(matches!(native, tauri::Error::Io(source)
        if source.kind() == std::io::ErrorKind::PermissionDenied));
    assert!(matches!(
        ApiError::from(error),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn provider_payload_failure_is_not_reported_as_invalid_user_input() {
    use ingot_artifacts::sources::{Source, SourceError};

    let malformed = VersionChangeError::Source(SourceError::InvalidPayload {
        provider: Source::Modrinth,
        field: "version identity",
    });
    assert!(matches!(
        ApiError::from(malformed),
        ApiError::ProviderUnavailable { .. }
    ));

    let invalid_id = VersionChangeError::Source(SourceError::InvalidProjectId {
        provider: Source::Curseforge,
    });
    assert!(matches!(
        ApiError::from(invalid_id),
        ApiError::InvalidInput { .. }
    ));
}

#[test]
fn pending_deletion_recovery_remains_distinct_from_ordinary_storage_failure() {
    use ingot_core::metadata::MetadataError;
    let errors = [
        MetadataError::RecoveryBlocked("Ambiguous deletion directories"),
        MetadataError::RecoveryCleanup {
            path: std::path::PathBuf::from("retained-backup"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        },
    ];
    for error in errors {
        assert!(matches!(
            ApiError::from(error),
            ApiError::IncompleteRecovery { .. }
        ));
    }
}

#[test]
fn test_launch_error_boundary_mappings() {
    assert!(matches!(
        ApiError::from(LaunchError::AlreadyRunning),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::NotRunning("inst-1".into())),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(crate::app::games::GameLaunchError::IncompleteTransaction(
            ingot_version_change::TransactionPhase::Swapping
        )),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::JavaRuntime(
            ingot_artifacts::java::JavaError::MissingBinary
        )),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::LoaderVersionNotSelected),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::AppDataDir(
            std::io::ErrorKind::PermissionDenied.into()
        )),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn test_session_error_boundary_mappings() {
    assert_eq!(
        ApiError::from(SessionError::Cancelled),
        ApiError::Cancelled {
            message: "Cancelled".into()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::AccountNotFound),
        ApiError::NotFound {
            message: "Account not found".into()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::NoActiveAccount),
        ApiError::NotFound {
            message: "No active account selected".into()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::InvalidLogin),
        ApiError::InvalidInput {
            message: "Username and password are required".into()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::RotationCheckpoint(RecoveryError::Invalid)),
        ApiError::IncompleteRecovery {
            message: RecoveryError::Invalid.to_string()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::ResumeCheckpoint(PendingGrantError::NotFound)),
        ApiError::IncompleteRecovery {
            message: PendingGrantError::NotFound.to_string()
        }
    );
    assert_eq!(
        ApiError::from(SessionError::Ownership(
            ingot_accounts::account::operations::OperationError
        )),
        ApiError::Conflict {
            message: ingot_accounts::account::operations::OperationError.to_string()
        }
    );
}

#[test]
fn test_credential_error_boundary_mappings() {
    assert_eq!(
        ApiError::from(CredentialError::Missing),
        ApiError::NotFound {
            message: "No credentials found".into()
        }
    );
    assert!(matches!(
        ApiError::from(CredentialError::Vault {
            operation: "read",
            source: ingot_accounts::account::credentials::VaultFailure::injected(),
        }),
        ApiError::ProviderUnavailable { .. }
    ));
}

#[test]
fn test_pending_grant_error_boundary_mappings() {
    assert!(matches!(
        ApiError::from(PendingGrantError::NotFound),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(PendingGrantError::AlreadyPending),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(PendingGrantError::Expired),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(PendingGrantError::Invalid),
        ApiError::InvalidInput { .. }
    ));
}

#[test]
fn test_account_error_boundary_mappings() {
    assert_eq!(
        ApiError::from(AccountError::Policy(AccountPolicyError::NotFound)),
        ApiError::NotFound {
            message: "Account not found".into()
        }
    );
    assert!(matches!(
        ApiError::from(AccountError::Policy(
            AccountPolicyError::DuplicateReorderId("id".into())
        )),
        ApiError::InvalidInput { .. }
    ));
}

#[test]
fn test_version_change_error_boundary_mappings() {
    assert!(matches!(
        ApiError::from(VersionChangeError::RollbackFailed {
            primary: Box::new(VersionChangeError::InvalidPath("failed".into())),
            restore: Box::new(VersionChangeError::InvalidPath("also failed".into())),
            checkpoint: None,
            backup_path: std::path::PathBuf::from("/tmp/backup"),
        }),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::RollbackSucceeded(Box::new(
            VersionChangeError::InvalidPath("reverted".into())
        ))),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::JournalIo {
            operation: "create directory for",
            path: std::path::PathBuf::from("/tmp/backup"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::ScanDirectory {
            path: std::path::PathBuf::from("/tmp/mods"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::ScanFile {
            path: std::path::PathBuf::from("/tmp/mods/broken.jar"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::InvalidFileName("../escaped.jar".into())),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::DownloadPrep {
            operation: "remove staging",
            path: std::path::PathBuf::from("/tmp/staging"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(VersionChangeError::DownloadFailed {
            title: "Example mod".into(),
            source: ingot_artifacts::downloader::DownloadError::HttpStatus {
                status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            },
        }),
        ApiError::ProviderUnavailable { .. }
    ));
}

#[test]
fn test_process_error_boundary_mappings() {
    use ingot_server::process::models::ProcessError;

    assert!(matches!(
        ApiError::from(ProcessError::NotRunning("srv1".into())),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::ServerNotFound("srv1".into())),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::AlreadyRunning),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::MustBeRunningToSleep),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::PortInUse {
            port: 25565,
            source: std::io::ErrorKind::AddrInUse.into()
        }),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::ShutdownTimeout {
            server_id: "srv1".into(),
            timeout: std::time::Duration::from_secs(30),
        }),
        ApiError::ProviderUnavailable { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::Config(ingot_server::config::ConfigError::Io(
            std::io::Error::other("bad config")
        ))),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn test_server_domain_error_boundary_mappings() {
    use ingot_server::slp::PingError;
    use ingot_server::transfer::TransferError;
    use ingot_server_map::live::LiveError;
    use ingot_server_map::map::MapError;
    use ingot_tunnel::PlayitError;

    assert!(matches!(
        ApiError::from(LiveError::InvalidPlayerName("bad!name".into())),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(MapError::RegionRead {
            path: "region/r.0.0.mca".into(),
            source: std::io::ErrorKind::PermissionDenied.into(),
        }),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(PingError::Timeout),
        ApiError::ProviderUnavailable { .. }
    ));
    assert!(matches!(
        ApiError::from(TransferError::InvalidZip(
            zip::result::ZipError::InvalidArchive("not a zip".into())
        )),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(PlayitError::HttpStatus(reqwest::StatusCode::BAD_GATEWAY)),
        ApiError::ProviderUnavailable { .. }
    ));
    assert!(matches!(
        ApiError::from(PlayitError::InvalidResponse),
        ApiError::ProviderUnavailable { .. }
    ));
    assert!(matches!(
        ApiError::from(PlayitError::ClaimRejected),
        ApiError::ProviderUnavailable { .. }
    ));
    assert!(matches!(
        ApiError::from(PlayitError::Directory(
            std::io::ErrorKind::PermissionDenied.into()
        )),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn server_launch_preserves_recovery_and_java_error_categories() {
    use ingot_artifacts::java::JavaError;
    use ingot_core::metadata::MetadataError;
    use ingot_server::process::ProcessError;
    let recovery = ProcessError::Metadata(MetadataError::RecoveryBlocked("ambiguous receipt"));
    assert!(std::error::Error::source(&recovery).is_some());
    assert!(matches!(
        ApiError::from(recovery),
        ApiError::IncompleteRecovery { .. }
    ));
    let storage = ProcessError::Java(JavaError::Io(std::io::Error::other("unreadable runtime")));
    assert!(matches!(
        ApiError::from(storage),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::Java(JavaError::MissingBinary)),
        ApiError::NotFound { .. }
    ));
}

#[test]
fn server_port_collision_is_a_conflict() {
    let error =
        ingot_server::config::ConfigError::Port(ingot_server::config::PortError::PortInUse {
            port: 25565,
            owner: "another server".into(),
            suggested: 25566,
        });
    assert!(matches!(ApiError::from(error), ApiError::Conflict { .. }));
}

#[test]
fn native_boundary_failures_keep_actionable_api_categories() {
    use ingot_sandbox::SandboxError;
    use ingot_server::transfer::TransferError;
    assert!(matches!(
        ApiError::from(TransferError::UnsupportedManifestFormat(99)),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(TransferError::InvalidUploadId),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(TransferError::UploadFinalized),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(TransferError::Process(
            ingot_server::process::models::ProcessError::AlreadyStarting
        )),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(SandboxError::RecoveryBlocked("unfinished publication")),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(SandboxError::InvalidRootfs),
        ApiError::ProviderUnavailable { .. }
    ));
}

#[test]
fn launcher_account_failures_preserve_causes_and_categories() {
    use ingot_accounts::account::{
        operations::OperationError,
        provider::{ProviderError, ProviderOperation},
    };
    use std::error::Error;

    let sessions = [
        SessionError::AccountNotFound,
        SessionError::Cancelled,
        SessionError::Ownership(OperationError),
        SessionError::Provider(ProviderError::Rejected {
            operation: ProviderOperation::ElyRefresh,
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        }),
        SessionError::RotationCheckpoint(RecoveryError::Invalid),
    ];
    for session in sessions {
        let launch = LaunchError::Account(
            ingot_minecraft::launcher::models::LaunchAccountError::new(session),
        );
        let account = launch.source().unwrap();
        let cause = account.source().unwrap();
        let session = cause.downcast_ref::<SessionError>().unwrap();
        let expected = match session {
            SessionError::AccountNotFound => ApiError::from(SessionError::AccountNotFound),
            SessionError::Cancelled => ApiError::from(SessionError::Cancelled),
            SessionError::Ownership(_) => ApiError::from(SessionError::Ownership(OperationError)),
            SessionError::Provider(_) => {
                ApiError::from(SessionError::Provider(ProviderError::Rejected {
                    operation: ProviderOperation::ElyRefresh,
                    status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                }))
            }
            SessionError::RotationCheckpoint(_) => {
                ApiError::from(SessionError::RotationCheckpoint(RecoveryError::Invalid))
            }
            other => panic!("Unexpected fixture: {other}"),
        };
        assert_eq!(ApiError::from(launch), expected);
    }
    let recovery =
        LaunchError::Account(ingot_minecraft::launcher::models::LaunchAccountError::new(
            RecoveryError::Profile(AccountError::LockPoisoned),
        ));
    assert!(recovery
        .source()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<RecoveryError>()
        .is_some());
    assert!(matches!(
        ApiError::from(recovery),
        ApiError::IncompleteRecovery { .. }
    ));
}

#[test]
fn account_and_auth_skin_failures_map_to_actionable_api_categories_and_redact_secrets() {
    use crate::app::accounts::SkinError;
    use ingot_accounts::auth::ely::ElyError;
    use ingot_accounts::auth::microsoft::MicrosoftSkinError;
    use reqwest::StatusCode;

    // Ely uploaded-not-applied maps to IncompleteRecovery
    let incomplete_err = ElyError::UploadedNotApplied {
        skin_id: 8888,
        source: Box::new(ElyError::HttpStatus(StatusCode::SERVICE_UNAVAILABLE)),
    };
    let ipc = ApiError::from(SkinError::Ely(incomplete_err));
    assert!(matches!(ipc, ApiError::IncompleteRecovery { .. }));

    // Ely OAuth and authentication failures map to ProviderUnavailable and wire serialization redacts secrets
    for ely_err in [
        ElyError::OAuthCompletion {
            status: StatusCode::BAD_REQUEST,
        },
        ElyError::MissingRedirectUri,
        ElyError::AuthenticationRejected,
    ] {
        let ipc = ApiError::from(SkinError::Ely(ely_err));
        assert!(matches!(ipc, ApiError::ProviderUnavailable { .. }));
        let wire = serde_json::to_string(&ipc).unwrap();
        assert!(!wire.contains("private-"));
    }

    // Microsoft skin HTTP status errors map to ProviderUnavailable and wire serialization redacts secrets
    let ms_http_err = MicrosoftSkinError::HttpStatus(StatusCode::BAD_REQUEST);
    let ipc = ApiError::from(SkinError::Microsoft(ms_http_err));
    assert!(matches!(ipc, ApiError::ProviderUnavailable { .. }));
    let wire = serde_json::to_string(&ipc).unwrap();
    assert!(!wire.contains("private-"));

    // Microsoft skin decode/response and transport errors map to ProviderUnavailable and wire serialization redacts secrets
    let reqwest_err = reqwest::Client::new()
        .get("::invalid url::")
        .build()
        .unwrap_err();
    let ms_response_err = MicrosoftSkinError::Response(reqwest_err);
    let ipc = ApiError::from(SkinError::Microsoft(ms_response_err));
    assert!(matches!(ipc, ApiError::ProviderUnavailable { .. }));
    let wire = serde_json::to_string(&ipc).unwrap();
    assert!(!wire.contains("private-token"));

    // Additional SkinError failure categories map to expected API errors
    let profile_err = crate::app::accounts::ProfileError::EmptyNickname;
    let meta_err = SkinError::MetadataAfterProvider(profile_err);
    assert!(matches!(
        ApiError::from(meta_err),
        ApiError::IncompleteRecovery { .. }
    ));

    let cred_err = CredentialError::Missing;
    let cred_after_err = SkinError::CredentialsAfterProvider(cred_err);
    assert!(matches!(
        ApiError::from(cred_after_err),
        ApiError::IncompleteRecovery { .. }
    ));

    assert!(matches!(
        ApiError::from(SkinError::AccountNotFound),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(SkinError::PasswordRequired),
        ApiError::Unauthorized { .. }
    ));
    assert!(matches!(
        ApiError::from(SkinError::NotEly),
        ApiError::InvalidInput { .. }
    ));
}

#[test]
fn launcher_settings_failures_preserve_causes_and_categories() {
    use ingot_settings::SettingsError;
    use std::error::Error;

    let failures = [
        SettingsError::InvalidMemory {
            min_mb: 4096,
            max_mb: 1024,
        },
        SettingsError::StateUnavailable,
        SettingsError::Io {
            operation: "read",
            path: "settings.json".into(),
            source: std::io::ErrorKind::PermissionDenied.into(),
        },
    ];
    for source in failures {
        let error = LaunchError::Settings(source);
        let source = error
            .source()
            .unwrap()
            .downcast_ref::<SettingsError>()
            .unwrap();
        let api = match source {
            SettingsError::InvalidMemory { .. } => ApiError::invalid_input(source.to_string()),
            SettingsError::StateUnavailable => ApiError::internal(source.to_string()),
            SettingsError::Io { .. } => ApiError::storage_failure(source.to_string()),
            other => panic!("Unexpected fixture: {other}"),
        };
        assert_eq!(ApiError::from(error), api);
    }
}

#[test]
fn launcher_and_screenshot_native_paths_retain_original_errors() {
    use ingot_minecraft::screenshots::ScreenshotError;
    use std::error::Error;

    fn native_failure() -> std::io::Error {
        std::io::Error::other(tauri::Error::Io(
            std::io::ErrorKind::PermissionDenied.into(),
        ))
    }
    fn check(error: &dyn Error) {
        let io = error
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        let native = io
            .get_ref()
            .unwrap()
            .downcast_ref::<tauri::Error>()
            .unwrap();
        assert!(matches!(native, tauri::Error::Io(source)
            if source.kind() == std::io::ErrorKind::PermissionDenied));
    }
    let launch = LaunchError::AppDataDir(native_failure());
    let screenshot = ScreenshotError::CacheDir(native_failure());
    check(&launch);
    check(&screenshot);
    assert!(matches!(
        ApiError::from(launch),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(screenshot),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn lifecycle_and_tracking_boundaries_preserve_missing_and_recovery_categories() {
    use ingot_server::plugins::{tracking::TrackingError, PluginError};
    let missing = crate::app::lifecycle::LifecycleError::LeftoverServerMissing;
    assert!(matches!(ApiError::from(missing), ApiError::NotFound { .. }));
    let recovery = TrackingError::Recovery(Box::new(PluginError::RecoveryBlocked(
        "pending publication",
    )));
    assert!(matches!(
        ApiError::from(recovery),
        ApiError::IncompleteRecovery { .. }
    ));
}

#[test]
fn launcher_recovery_inspection_and_cancellation_keep_their_categories() {
    let inspection = crate::app::games::GameLaunchError::TransactionInspection(Box::new(
        VersionChangeError::JournalDecode(
            serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
        ),
    ));
    let transaction = std::error::Error::source(&inspection)
        .unwrap()
        .downcast_ref::<Box<VersionChangeError>>()
        .unwrap();
    assert!(std::error::Error::source(transaction.as_ref())
        .unwrap()
        .downcast_ref::<serde_json::Error>()
        .is_some());
    assert!(matches!(
        ApiError::from(inspection),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::ReservationCancelled),
        ApiError::Cancelled { .. }
    ));
}

#[test]
fn server_start_failure_and_cleanup_keep_distinct_categories() {
    use ingot_server::process::ProcessError;
    let startup = ProcessError::ProxyStartup {
        port: 25565,
        source: std::io::ErrorKind::PermissionDenied.into(),
    };
    assert!(matches!(ApiError::from(startup), ApiError::Internal { .. }));
    let cleanup = ProcessError::ProxyStartupCleanup {
        primary: Box::new(ProcessError::PortInUse {
            port: 25565,
            source: std::io::ErrorKind::AddrInUse.into(),
        }),
        cleanup: Box::new(ProcessError::ShutdownTimeout {
            server_id: "srv1".into(),
            timeout: std::time::Duration::from_secs(30),
        }),
    };
    assert!(matches!(
        ApiError::from(cleanup),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::LaunchCancelled),
        ApiError::Cancelled { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::StartAdmissionPaused),
        ApiError::Conflict { .. }
    ));
}

#[tokio::test]
async fn download_validation_and_admission_errors_remain_internal() {
    use ingot_artifacts::downloader::{error::ContentRangeError, DownloadError};
    for error in [
        DownloadError::InvalidAssetHash,
        DownloadError::ContentRange(ContentRangeError::MissingHeader),
    ] {
        assert!(matches!(ApiError::from(error), ApiError::Internal { .. }));
    }
    let semaphore = tokio::sync::Semaphore::new(1);
    semaphore.close();
    let source = semaphore.acquire().await.err().unwrap();
    assert!(matches!(
        ApiError::from(DownloadError::Semaphore(source)),
        ApiError::Internal { .. }
    ));
}

#[test]
fn download_error_diagnostics_are_redacted_at_api_boundary() {
    use ingot_artifacts::downloader::DownloadError;
    let error = DownloadError::HttpStatus {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
    };
    let boundary = ApiError::from(error);
    assert!(matches!(boundary, ApiError::ProviderUnavailable { .. }));
    let payload = serde_json::to_string(&boundary).unwrap();
    assert!(!payload.contains("download-secret"));
    assert!(!payload.contains("private-artifact"));
}

#[test]
fn live_queries_delegate_process_failure_categories() {
    use ingot_server::{live_query::LiveQueryError, process::ProcessError};
    let stopped = LiveQueryError::Query(Box::new(ProcessError::NotRunning("srv1".into())));
    assert!(matches!(ApiError::from(stopped), ApiError::NotFound { .. }));
    let timeout = LiveQueryError::Query(Box::new(ProcessError::NoReply("list".into())));
    assert!(matches!(
        ApiError::from(timeout),
        ApiError::ProviderUnavailable { .. }
    ));
    let storage = LiveQueryError::Query(Box::new(ProcessError::StdinWrite(
        std::io::ErrorKind::BrokenPipe.into(),
    )));
    assert!(matches!(
        ApiError::from(storage),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn sync_settings_delegate_validation_and_state_categories() {
    use ingot_minecraft::sync::SyncError;
    use ingot_settings::SettingsError;
    let invalid = SyncError::Settings(SettingsError::InvalidMemory {
        min_mb: 4096,
        max_mb: 1024,
    });
    assert!(matches!(
        ApiError::from(invalid),
        ApiError::InvalidInput { .. }
    ));
    let unavailable = SyncError::Settings(SettingsError::StateUnavailable);
    assert!(matches!(
        ApiError::from(unavailable),
        ApiError::Internal { .. }
    ));
    let path = SyncError::AppDataDir(std::io::ErrorKind::PermissionDenied.into());
    assert!(matches!(
        ApiError::from(path),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn bedrock_archive_and_protocol_errors_keep_categories() {
    use ingot_core::ArchiveError;
    use ingot_server::{
        bedrock::BedrockServerError, bedrock_packs::BedrockPackError, files::ServerFileError,
        slp::PingError,
    };
    let unsafe_archive = BedrockServerError::Archive(ArchiveError::UnsafeEntry);
    assert!(matches!(
        ApiError::from(unsafe_archive),
        ApiError::InvalidInput { .. }
    ));
    let unreadable_archive = BedrockServerError::Archive(ArchiveError::Io(
        std::io::ErrorKind::PermissionDenied.into(),
    ));
    assert!(matches!(
        ApiError::from(unreadable_archive),
        ApiError::StorageFailure { .. }
    ));
    let malformed_manifest =
        BedrockPackError::ManifestJson(serde_json::from_str::<serde_json::Value>("{").unwrap_err());
    assert!(matches!(
        ApiError::from(malformed_manifest),
        ApiError::InvalidInput { .. }
    ));
    let icon = ServerFileError::InvalidIconEncoding(base64::DecodeError::InvalidByte(0, b'!'));
    assert!(matches!(
        ApiError::from(icon),
        ApiError::InvalidInput { .. }
    ));
    let utf8 = PingError::Utf8(String::from_utf8(vec![0xff]).unwrap_err());
    assert!(matches!(ApiError::from(utf8), ApiError::Internal { .. }));
    let varint = PingError::InvalidVarInt(std::io::ErrorKind::InvalidData.into());
    assert!(matches!(ApiError::from(varint), ApiError::Internal { .. }));
}

#[test]
fn process_tree_failures_keep_cleanup_distinct_from_command_failures() {
    use crate::app::lifecycle::LifecycleError;
    use ingot_core::process_tree::TerminationError;

    let command = LifecycleError::Termination(TerminationError::Spawn {
        pid: 10,
        source: std::io::ErrorKind::PermissionDenied.into(),
    });
    assert!(matches!(ApiError::from(command), ApiError::Internal { .. }));
    let cleanup = LifecycleError::Termination(TerminationError::Cleanup {
        primary: Box::new(TerminationError::CommandTimeout(10)),
        signal: Some(std::io::ErrorKind::PermissionDenied.into()),
        reap: Some(std::io::ErrorKind::TimedOut.into()),
    });
    assert!(matches!(
        ApiError::from(cleanup),
        ApiError::IncompleteRecovery { .. }
    ));
    let unconfirmed = LifecycleError::Termination(TerminationError::ExitUnconfirmed(10));
    assert!(matches!(
        ApiError::from(unconfirmed),
        ApiError::IncompleteRecovery { .. }
    ));
}

#[test]
fn server_installer_error_ipc_categories_remain_correct() {
    use ingot_server::config::ServerCoreType;
    use ingot_server::installer::InstallerError;
    use std::io;

    let process_err = InstallerError::Process {
        core: ServerCoreType::NeoForge,
        source: Box::new(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken")),
    };
    let api_err = ApiError::from(process_err);
    assert!(matches!(api_err, ApiError::Internal { .. }));

    let io_err = InstallerError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
    let api_err = ApiError::from(io_err);
    assert!(matches!(api_err, ApiError::StorageFailure { .. }));

    let not_configured = InstallerError::NotConfigured(ServerCoreType::Forge);
    let api_err = ApiError::from(not_configured);
    assert!(matches!(api_err, ApiError::NotFound { .. }));

    let version_unavailable = InstallerError::VersionUnavailable {
        core: ServerCoreType::Quilt,
        game_version: "1.99".to_string(),
    };
    let api_err = ApiError::from(version_unavailable);
    assert!(matches!(api_err, ApiError::NotFound { .. }));

    let unsupported = InstallerError::UnsupportedCore(ServerCoreType::Vanilla);
    let api_err = ApiError::from(unsupported);
    assert!(matches!(api_err, ApiError::InvalidInput { .. }));
}

#[test]
fn instance_content_action_errors_preserve_causes_and_api_categories() {
    use crate::app::instance_content::InstanceContentActionError;
    use crate::app::platform::OpenDirectoryError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::InstanceError;
    use ingot_minecraft::instance_content::InstanceContentError;
    use std::error::Error;
    use std::io;

    let make_instance_failures = || -> Vec<InstanceError> {
        vec![
            InstanceError::InvalidId,
            InstanceError::NotFound("missing-instance".into()),
            InstanceError::Io(io::ErrorKind::PermissionDenied.into()),
            InstanceError::Metadata(MetadataError::Conflict("busy instance".into())),
            InstanceError::Metadata(MetadataError::RecoveryBlocked("blocked recovery")),
            InstanceError::Metadata(MetadataError::Decode {
                path: "instances.json".into(),
                source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
            }),
            InstanceError::Metadata(MetadataError::Io {
                operation: "read",
                path: "instances.json".into(),
                source: io::ErrorKind::PermissionDenied.into(),
            }),
            InstanceError::FilesCleanupCommitted {
                path: "retained-instance".into(),
                source: io::ErrorKind::PermissionDenied.into(),
            },
            InstanceError::Cleanup {
                primary: Box::new(InstanceError::InvalidId),
                cleanup: io::ErrorKind::PermissionDenied.into(),
            },
        ]
    };

    for (original, expected) in make_instance_failures()
        .into_iter()
        .zip(make_instance_failures())
    {
        let wrapped = InstanceContentActionError::from(original);
        match &wrapped {
            InstanceContentActionError::Instance(InstanceError::Io(_))
            | InstanceContentActionError::Instance(InstanceError::Metadata(MetadataError::Io {
                ..
            }))
            | InstanceContentActionError::Instance(InstanceError::FilesCleanupCommitted {
                ..
            }) => {
                let io = wrapped
                    .source()
                    .unwrap()
                    .downcast_ref::<io::Error>()
                    .unwrap();
                assert_eq!(io.kind(), io::ErrorKind::PermissionDenied);
            }
            InstanceContentActionError::Instance(InstanceError::Metadata(
                MetadataError::Decode { .. },
            )) => {
                assert!(wrapped
                    .source()
                    .unwrap()
                    .downcast_ref::<serde_json::Error>()
                    .is_some());
            }
            InstanceContentActionError::Instance(InstanceError::Cleanup { .. }) => {
                assert_eq!(wrapped.to_string(), expected.to_string());
                let primary = wrapped
                    .source()
                    .unwrap()
                    .downcast_ref::<Box<InstanceError>>()
                    .unwrap();
                assert!(matches!(primary.as_ref(), InstanceError::InvalidId));
            }
            _ => {}
        }
        assert_eq!(ApiError::from(wrapped), ApiError::from(expected));
    }

    let io_content = InstanceContentActionError::from(InstanceContentError::io(
        "mods/example.jar",
        io::ErrorKind::PermissionDenied.into(),
    ));
    assert_eq!(
        io_content
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(matches!(
        ApiError::from(io_content),
        ApiError::StorageFailure { .. }
    ));

    for (content_error, expected_kind_check) in [
        (
            InstanceContentError::InstanceNotFound("inst-1".into()),
            ApiError::not_found("inst-1"),
        ),
        (
            InstanceContentError::NotFound("mod.jar".into()),
            ApiError::not_found("mod.jar"),
        ),
        (
            InstanceContentError::WorldNotFound("world".into()),
            ApiError::not_found("world"),
        ),
        (
            InstanceContentError::InstanceRunning("inst-1".into()),
            ApiError::conflict("Instance 'inst-1' is currently running"),
        ),
        (
            InstanceContentError::NameConflict("mod.jar".into()),
            ApiError::conflict("mod.jar"),
        ),
        (
            InstanceContentError::invalid_file_name("../bad.jar", "traversal"),
            ApiError::invalid_input("traversal"),
        ),
        (
            InstanceContentError::Serialization("bad json".into()),
            ApiError::internal("bad json"),
        ),
        (
            InstanceContentError::InvalidOperation("failed".into()),
            ApiError::internal("failed"),
        ),
    ] {
        assert_eq!(
            ApiError::from(InstanceContentActionError::from(content_error)),
            expected_kind_check
        );
    }

    let spawn_err = InstanceContentActionError::from(OpenDirectoryError::Spawn(
        io::ErrorKind::PermissionDenied.into(),
    ));
    assert_eq!(
        spawn_err
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(matches!(
        ApiError::from(spawn_err),
        ApiError::Internal { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceContentActionError::from(
            OpenDirectoryError::Target(io::ErrorKind::NotFound.into())
        )),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceContentActionError::from(
            OpenDirectoryError::Target(io::ErrorKind::PermissionDenied.into())
        )),
        ApiError::StorageFailure { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceContentActionError::from(
            OpenDirectoryError::Unsupported
        )),
        ApiError::Internal { .. }
    ));
}

#[tokio::test]
async fn instance_content_blocking_task_panic_maps_to_internal_api_error() {
    use crate::app::instance_content::InstanceContentActionError;

    let join_error = tokio::task::spawn_blocking(|| {
        panic!("simulated blocking worker failure");
    })
    .await
    .unwrap_err();
    let wrapped = InstanceContentActionError::from(join_error);
    assert!(matches!(wrapped, InstanceContentActionError::Task(_)));
    assert!(matches!(ApiError::from(wrapped), ApiError::Internal { .. }));
}

#[tokio::test]
async fn instance_export_errors_preserve_sources_and_api_categories() {
    use ingot_content::instance_export::InstanceExportError;
    use ingot_core::metadata::MetadataError;
    use ingot_minecraft::instance::{InstanceError, ModLoaderType};
    use std::error::Error;

    let decode_err = InstanceExportError::from(InstanceError::Metadata(MetadataError::Decode {
        path: "instances.json".into(),
        source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
    }));
    assert!(decode_err
        .source()
        .and_then(|err| err.downcast_ref::<InstanceError>())
        .is_some());
    assert!(matches!(
        ApiError::from(decode_err),
        ApiError::StorageFailure { .. }
    ));

    let recovery_err = InstanceExportError::from(InstanceError::Metadata(
        MetadataError::RecoveryBlocked("instances.deletion.json"),
    ));
    assert!(matches!(
        ApiError::from(recovery_err),
        ApiError::IncompleteRecovery { .. }
    ));

    assert!(matches!(
        ApiError::from(InstanceExportError::InstanceRunning),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceExportError::NotFound("missing".into())),
        ApiError::NotFound { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceExportError::UnsupportedLoader(
            ModLoaderType::Horizon
        )),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceExportError::EmptyName),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceExportError::UnsafePath("../bad".into())),
        ApiError::InvalidInput { .. }
    ));
    assert!(matches!(
        ApiError::from(InstanceExportError::Serialization("bad json".into())),
        ApiError::Internal { .. }
    ));

    let join_error = tokio::task::spawn_blocking(|| {
        panic!("simulated export task panic");
    })
    .await
    .unwrap_err();
    assert!(matches!(
        ApiError::from(InstanceExportError::from(join_error)),
        ApiError::Internal { .. }
    ));
}
