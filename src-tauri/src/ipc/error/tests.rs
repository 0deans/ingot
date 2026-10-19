use super::*;
use crate::account::{
    credentials::CredentialError, pending_login::PendingGrantError, policy::AccountPolicyError,
    recovery::RecoveryError, session::SessionError, AccountError,
};
use crate::minecraft::launcher::LaunchError;
use crate::version_change::VersionChangeError;

#[test]
fn instance_adapters_preserve_causes_and_api_categories() {
    use crate::minecraft::{
        importer::models::ImportError, instance::InstanceError, metadata::MetadataError,
        screenshots::ScreenshotError,
    };
    use std::error::Error;

    fn failures() -> Vec<InstanceError> {
        vec![
            InstanceError::InvalidId,
            InstanceError::NotFound("missing-instance".into()),
            InstanceError::Io(std::io::ErrorKind::PermissionDenied.into()),
            InstanceError::Metadata(MetadataError::Conflict("busy instance".into())),
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
}

#[test]
fn importer_native_path_failure_preserves_its_source() {
    use crate::minecraft::importer::models::ImportError;
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
    use crate::version_change::sources::{Source, SourceError};

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
    use crate::minecraft::metadata::MetadataError;
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
        ApiError::from(LaunchError::IncompleteTransaction(
            crate::version_change::TransactionPhase::Swapping
        )),
        ApiError::IncompleteRecovery { .. }
    ));
    assert!(matches!(
        ApiError::from(LaunchError::JavaRuntime(
            crate::minecraft::java::JavaError::MissingBinary
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
            crate::account::operations::OperationError
        )),
        ApiError::Conflict {
            message: crate::account::operations::OperationError.to_string()
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
            source: crate::account::credentials::VaultFailure::injected(),
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
}

#[test]
fn test_process_error_boundary_mappings() {
    use crate::server::process::models::ProcessError;

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
        ApiError::from(ProcessError::Config(
            crate::server::config::ConfigError::Io(std::io::Error::other("bad config"))
        )),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn test_server_domain_error_boundary_mappings() {
    use crate::server::live::LiveError;
    use crate::server::map::MapError;
    use crate::server::slp::PingError;
    use crate::server::transfer::TransferError;
    use crate::server::tunnel::PlayitError;

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
}

#[test]
fn server_launch_preserves_recovery_and_java_error_categories() {
    use crate::minecraft::java::JavaError;
    use crate::minecraft::metadata::MetadataError;
    use crate::server::process::ProcessError;
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
        crate::server::config::ConfigError::Port(crate::server::config::PortError::PortInUse {
            port: 25565,
            owner: "another server".into(),
            suggested: 25566,
        });
    assert!(matches!(ApiError::from(error), ApiError::Conflict { .. }));
}

#[test]
fn native_boundary_failures_keep_actionable_api_categories() {
    use crate::server::{sandbox::SandboxError, transfer::TransferError};
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
            crate::server::process::models::ProcessError::AlreadyStarting
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
    use crate::account::{
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
        let launch = LaunchError::Account(session.into());
        let account = launch.source().unwrap();
        let cause = account.source().unwrap().source().unwrap();
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
    let recovery = LaunchError::Account(RecoveryError::Profile(AccountError::LockPoisoned).into());
    assert!(recovery
        .source()
        .unwrap()
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
fn launcher_settings_failures_preserve_causes_and_categories() {
    use crate::settings::SettingsError;
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
    use crate::minecraft::screenshots::ScreenshotError;
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
    use crate::server::plugins::{tracking::TrackingError, PluginError};
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
    let inspection =
        LaunchError::TransactionInspection(Box::new(VersionChangeError::JournalDecode(
            serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
        )));
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
    use crate::server::process::ProcessError;
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
    use crate::minecraft::downloader::{error::ContentRangeError, DownloadError};
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
fn live_queries_delegate_process_failure_categories() {
    use crate::server::{live::LiveError, process::ProcessError};
    let stopped = LiveError::Query(Box::new(ProcessError::NotRunning("srv1".into())));
    assert!(matches!(ApiError::from(stopped), ApiError::NotFound { .. }));
    let timeout = LiveError::Query(Box::new(ProcessError::NoReply("list".into())));
    assert!(matches!(
        ApiError::from(timeout),
        ApiError::ProviderUnavailable { .. }
    ));
    let storage = LiveError::Query(Box::new(ProcessError::StdinWrite(
        std::io::ErrorKind::BrokenPipe.into(),
    )));
    assert!(matches!(
        ApiError::from(storage),
        ApiError::StorageFailure { .. }
    ));
}

#[test]
fn sync_settings_delegate_validation_and_state_categories() {
    use crate::{minecraft::sync::SyncError, settings::SettingsError};
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
    use crate::{
        app::ArchiveError,
        server::{
            bedrock::BedrockServerError, bedrock_packs::BedrockPackError, files::ServerFileError,
            slp::PingError,
        },
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
