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
        ApiError::from(LaunchError::IncompleteTransaction("test".into())),
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
        ApiError::from(LaunchError::AppDataDir("permission".into())),
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
            source: crate::account::credentials::VaultFailure("os keyring failed".into()),
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
            details: "busy".into()
        }),
        ApiError::Conflict { .. }
    ));
    assert!(matches!(
        ApiError::from(ProcessError::JarDownload("failed".into())),
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
        ApiError::from(MapError::RegionRead("corrupt".into())),
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
