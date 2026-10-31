use super::ApiError;

impl From<crate::minecraft::instance::InstanceError> for ApiError {
    fn from(error: crate::minecraft::instance::InstanceError) -> Self {
        use crate::minecraft::instance::InstanceError;
        match error {
            InstanceError::Metadata(error) => Self::from(error),
            InstanceError::Io(_) | InstanceError::Clock(_) => {
                Self::storage_failure(error.to_string())
            }
            InstanceError::InvalidId | InstanceError::EmptyName => {
                Self::invalid_input(error.to_string())
            }
            InstanceError::Task(_) => Self::internal(error.to_string()),
            InstanceError::NotFound(_) => Self::not_found(error.to_string()),
            InstanceError::Cleanup { .. } | InstanceError::FilesCleanupCommitted { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
        }
    }
}

impl From<crate::minecraft::importer::models::ImportError> for ApiError {
    fn from(error: crate::minecraft::importer::models::ImportError) -> Self {
        use crate::minecraft::importer::models::ImportError;
        match error {
            ImportError::OverlappingPaths | ImportError::UnknownLauncher(_) => {
                Self::invalid_input(error.to_string())
            }
            ImportError::SourceNotFound(_) => Self::not_found(error.to_string()),
            ImportError::Metadata(error) => Self::from(error),
            ImportError::Cleanup { .. } => Self::incomplete_recovery(error.to_string()),
            ImportError::Task(_) => Self::internal(error.to_string()),
            ImportError::InstanceCreate(source) | ImportError::InstanceDir(source) => {
                Self::from(source)
            }
            ImportError::Io(_) | ImportError::NativePath(_) => {
                Self::storage_failure(error.to_string())
            }
        }
    }
}

impl From<crate::minecraft::sync::SyncError> for ApiError {
    fn from(error: crate::minecraft::sync::SyncError) -> Self {
        use crate::minecraft::sync::SyncError;
        match error {
            SyncError::InstanceNotFound(_) => Self::not_found(error.to_string()),
            SyncError::InvalidCategory(_) | SyncError::InvalidResolution(_) => {
                Self::invalid_input(error.to_string())
            }
            SyncError::Task(_) => Self::internal(error.to_string()),
            SyncError::Instances(err) => Self::from(err),
            SyncError::Settings(source) => Self::from(source),
            _ => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::minecraft::screenshots::ScreenshotError> for ApiError {
    fn from(error: crate::minecraft::screenshots::ScreenshotError) -> Self {
        use crate::minecraft::screenshots::ScreenshotError;
        match error {
            ScreenshotError::NotFound(_) => Self::not_found(error.to_string()),
            ScreenshotError::InvalidFilename(_) => Self::invalid_input(error.to_string()),
            ScreenshotError::StaleMetadata | ScreenshotError::AmbiguousFilename(_) => {
                Self::conflict(error.to_string())
            }
            ScreenshotError::Task(_) => Self::internal(error.to_string()),
            ScreenshotError::InstanceDir(source) | ScreenshotError::LoadInstances(source) => {
                Self::from(source)
            }
            ScreenshotError::Io(_)
            | ScreenshotError::Image { .. }
            | ScreenshotError::CacheDir(_)
            | ScreenshotError::MissingCacheParent(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::minecraft::content::error::ContentError> for ApiError {
    fn from(error: crate::minecraft::content::error::ContentError) -> Self {
        use crate::minecraft::content::error::ContentError;
        match error {
            ContentError::Metadata(error) => Self::from(error),
            ContentError::InvalidPagination
            | ContentError::UnknownSource(_)
            | ContentError::MissingField(_)
            | ContentError::InvalidFilename(_)
            | ContentError::MissingManifest(_)
            | ContentError::UnsafePath(_)
            | ContentError::Integrity(_)
            | ContentError::Json(_)
            | ContentError::Zip(_) => Self::invalid_input(error.to_string()),
            ContentError::Io(error) => Self::storage_failure(error.to_string()),
            ContentError::Cleanup { .. } => Self::incomplete_recovery(error.to_string()),
            ContentError::ProvidersUnavailable { .. }
            | ContentError::Provider { .. }
            | ContentError::Transport(_)
            | ContentError::Response(_)
            | ContentError::HttpStatus(_) => Self::provider_unavailable(error.to_string()),
            ContentError::Instance(source) => Self::from(source),
            ContentError::Task(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::minecraft::metadata::MetadataError> for ApiError {
    fn from(error: crate::minecraft::metadata::MetadataError) -> Self {
        use crate::minecraft::metadata::MetadataError;
        match error {
            MetadataError::Conflict(message) => Self::conflict(message),
            MetadataError::Validation(_) => Self::internal(error.to_string()),
            MetadataError::Cleanup { .. }
            | MetadataError::Rollback { .. }
            | MetadataError::RecoveryBlocked(_)
            | MetadataError::RecoveryCleanup { .. } => Self::incomplete_recovery(error.to_string()),
            _ => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::version_change::service::ChangeServiceError> for ApiError {
    fn from(error: crate::version_change::service::ChangeServiceError) -> Self {
        use crate::version_change::service::ChangeServiceError;
        match error {
            ChangeServiceError::Metadata(error) => Self::from(error),
            ChangeServiceError::Transaction(error) => Self::from(error),
            ChangeServiceError::Tracking(error) => Self::from(error),
            ChangeServiceError::Busy | ChangeServiceError::StalePlan => {
                Self::conflict(error.to_string())
            }
            ChangeServiceError::NotFound => Self::not_found(error.to_string()),
        }
    }
}

impl From<crate::version_change::VersionChangeError> for ApiError {
    fn from(error: crate::version_change::VersionChangeError) -> Self {
        match error {
            crate::version_change::VersionChangeError::InvalidFileName(msg)
            | crate::version_change::VersionChangeError::InvalidPath(msg)
            | crate::version_change::VersionChangeError::PlanError(msg)
            | crate::version_change::VersionChangeError::UndoError(msg)
            | crate::version_change::VersionChangeError::InvalidTarget(msg) => {
                Self::invalid_input(msg)
            }
            crate::version_change::VersionChangeError::RollbackFailed {
                primary,
                restore,
                backup_path,
                checkpoint,
            } => Self::incomplete_recovery(format!(
                "{primary}; restore also failed: {restore}; checkpoint failure: {checkpoint:?}. Recovery backup preserved at {}",
                backup_path.display()
            )),
            crate::version_change::VersionChangeError::RollbackSucceeded(msg) => {
                Self::conflict(format!("Version change aborted and rolled back: {msg}"))
            }
            crate::version_change::VersionChangeError::Finalization { .. }
            | crate::version_change::VersionChangeError::MetadataAfterRestore { .. }
            | crate::version_change::VersionChangeError::CommitJournal { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            crate::version_change::VersionChangeError::FileIo { .. }
            | crate::version_change::VersionChangeError::JournalDecode(_) => Self::storage_failure(error.to_string()),
            crate::version_change::VersionChangeError::RestoreFailures(_)
            | crate::version_change::VersionChangeError::MissingRecoveryFile(_)
            | crate::version_change::VersionChangeError::AdditionalFailure { .. } => Self::incomplete_recovery(error.to_string()),
            crate::version_change::VersionChangeError::MetadataCommit(_) => Self::storage_failure(error.to_string()),
            crate::version_change::VersionChangeError::IncompleteTransaction(_)
            | crate::version_change::VersionChangeError::AlreadyDisabled(_) => Self::conflict(error.to_string()),
            crate::version_change::VersionChangeError::RecoveryUnavailable(_)
            | crate::version_change::VersionChangeError::NoVersionChange => Self::invalid_input(error.to_string()),
            crate::version_change::VersionChangeError::DownloadPrep { .. }
            | crate::version_change::VersionChangeError::JournalIo { .. }
            | crate::version_change::VersionChangeError::JournalEncode(_)
            | crate::version_change::VersionChangeError::ScanDirectory { .. }
            | crate::version_change::VersionChangeError::ScanFile { .. } => {
                Self::storage_failure(error.to_string())
            }
            crate::version_change::VersionChangeError::DownloadFailed { source, .. } => {
                Self::from(source)
            }
            crate::version_change::VersionChangeError::Task(_) => Self::internal(error.to_string()),
            crate::version_change::VersionChangeError::ChecksumMismatch(msg)
            | crate::version_change::VersionChangeError::UnknownHost(msg)
            | crate::version_change::VersionChangeError::NoChecksum(msg) => {
                Self::provider_unavailable(msg)
            }
            crate::version_change::VersionChangeError::Source(source) => match source {
                crate::version_change::sources::SourceError::InvalidProjectId { .. } => {
                    Self::invalid_input(source.to_string())
                }
                crate::version_change::sources::SourceError::Transport { .. }
                | crate::version_change::sources::SourceError::InvalidPayload { .. }
                | crate::version_change::sources::SourceError::HttpStatus { .. }
                | crate::version_change::sources::SourceError::Decode { .. } => {
                    Self::provider_unavailable(source.to_string())
                }
            },
        }
    }
}

impl From<crate::minecraft::launcher::LaunchError> for ApiError {
    fn from(error: crate::minecraft::launcher::LaunchError) -> Self {
        match error {
            crate::minecraft::launcher::LaunchError::Metadata(error) => Self::from(error),
            crate::minecraft::launcher::LaunchError::InstanceNotFound(id) => Self::not_found(format!("Instance not found: {id}")),
            crate::minecraft::launcher::LaunchError::AlreadyRunning => {
                Self::conflict("Instance is already running")
            }
            crate::minecraft::launcher::LaunchError::NotRunning(id) => {
                Self::not_found(format!("Instance {id} is not running"))
            }
            crate::minecraft::launcher::LaunchError::Maintenance => {
                Self::conflict("Instance maintenance is in progress")
            }
            crate::minecraft::launcher::LaunchError::LoaderVersionNotSelected => {
                Self::invalid_input("Loader version not selected")
            }
            crate::minecraft::launcher::LaunchError::InstanceDir(source) => Self::from(source),
            crate::minecraft::launcher::LaunchError::AppDataDir(source) => {
                Self::storage_failure(source.to_string())
            }
            crate::minecraft::launcher::LaunchError::IncompleteTransaction(msg) => {
                Self::incomplete_recovery(format!(
                    "Cannot launch instance: version change transaction is in incomplete state ({msg:?}). Please recover or revert it first."
                ))
            }
            crate::minecraft::launcher::LaunchError::VersionPackage(err) => {
                Self::from(err)
            }
            crate::minecraft::launcher::LaunchError::LoaderProfile(err) => {
                Self::from(err)
            }
            crate::minecraft::launcher::LaunchError::Download(err) => {
                Self::from(err)
            }
            crate::minecraft::launcher::LaunchError::JavaRuntime(err) => {
                Self::from(err)
            }
            crate::minecraft::launcher::LaunchError::Account(error) => {
                use crate::minecraft::launcher::models::LaunchAccountCause;
                match *error.source {
                    LaunchAccountCause::Recovery(source) => Self::from(source),
                    LaunchAccountCause::Session(source) => Self::from(source),
                }
            }
            crate::minecraft::launcher::LaunchError::Settings(source) => Self::from(source),
            crate::minecraft::launcher::LaunchError::ProcessSpawn(err) => {
                Self::internal(format!("Failed to spawn Minecraft process: {err}"))
            }
            crate::minecraft::launcher::LaunchError::Io(err) => {
                Self::storage_failure(err.to_string())
            }
            crate::minecraft::launcher::LaunchError::TransactionInspection(_) => {
                Self::incomplete_recovery(error.to_string())
            }
            crate::minecraft::launcher::LaunchError::ReservationCancelled => {
                Self::cancelled(error.to_string())
            }
            crate::minecraft::launcher::LaunchError::RegistryPoisoned
            | crate::minecraft::launcher::LaunchError::Termination(_)
            | crate::minecraft::launcher::LaunchError::Task(_) => Self::internal(error.to_string()),
            crate::minecraft::launcher::LaunchError::Horizon(source) => Self::from(source),
        }
    }
}

impl From<crate::minecraft::bedrock::BedrockError> for ApiError {
    fn from(error: crate::minecraft::bedrock::BedrockError) -> Self {
        use crate::minecraft::bedrock::BedrockError;
        match error {
            BedrockError::UnsupportedPlatform => Self::invalid_input(error.to_string()),
            BedrockError::FolderNotFound => Self::not_found(error.to_string()),
            BedrockError::ProcessSpawn(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::minecraft::version::VersionError> for ApiError {
    fn from(error: crate::minecraft::version::VersionError) -> Self {
        use crate::minecraft::version::VersionError;
        match error {
            VersionError::NotFound(_) => Self::not_found(error.to_string()),
            VersionError::Network(_) => Self::provider_unavailable(error.to_string()),
            VersionError::Json(_) | VersionError::Io(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::minecraft::loader::LoaderError> for ApiError {
    fn from(error: crate::minecraft::loader::LoaderError) -> Self {
        use crate::minecraft::loader::LoaderError;
        match error {
            LoaderError::Network(_) => Self::provider_unavailable(error.to_string()),
            LoaderError::VanillaNotSupported => Self::invalid_input(error.to_string()),
            LoaderError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::minecraft::downloader::DownloadError> for ApiError {
    fn from(error: crate::minecraft::downloader::DownloadError) -> Self {
        use crate::minecraft::downloader::DownloadError;
        match error {
            DownloadError::Duplicate(_) => Self::conflict(error.to_string()),
            DownloadError::Transport(_) | DownloadError::HttpStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            DownloadError::Io(_) => Self::storage_failure(error.to_string()),
            DownloadError::Integrity(_)
            | DownloadError::Sha1Mismatch(_)
            | DownloadError::ChunkLengthMismatch { .. }
            | DownloadError::ContentRange(_)
            | DownloadError::Task(_)
            | DownloadError::Semaphore(_)
            | DownloadError::InvalidAssetHash => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::app::screenshots::ScreenshotActionError> for ApiError {
    fn from(error: crate::app::screenshots::ScreenshotActionError) -> Self {
        match error {
            crate::app::screenshots::ScreenshotActionError::Screenshot(source) => {
                Self::from(source)
            }
            crate::app::screenshots::ScreenshotActionError::FileManager(source) => {
                Self::from(source)
            }
        }
    }
}

impl From<crate::minecraft::horizon::HorizonError> for ApiError {
    fn from(error: crate::minecraft::horizon::HorizonError) -> Self {
        use crate::minecraft::horizon::HorizonError;
        match error {
            HorizonError::Transport(_) => Self::provider_unavailable(error.to_string()),
            HorizonError::Io(_) => Self::storage_failure(error.to_string()),
            HorizonError::AssetNotFound(_) => Self::not_found(error.to_string()),
            HorizonError::Json(_)
            | HorizonError::Extraction(_)
            | HorizonError::ExecutableNotFound(_)
            | HorizonError::Session(_)
            | HorizonError::Preparation(_) => Self::internal(error.to_string()),
        }
    }
}
