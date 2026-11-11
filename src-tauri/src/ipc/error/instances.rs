use super::ApiError;

impl From<ingot_minecraft::instance::InstanceError> for ApiError {
    fn from(error: ingot_minecraft::instance::InstanceError) -> Self {
        use ingot_minecraft::instance::{InstanceError, ShortcutError};
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
            InstanceError::Shortcut(ref source) => match source {
                ShortcutError::SteamNotFound => Self::not_found(source.to_string()),
                ShortcutError::DirectoryNotFound(_)
                | ShortcutError::Executable(_)
                | ShortcutError::Io(_)
                | ShortcutError::SteamCorruptVdf(_)
                | ShortcutError::Image(_) => Self::storage_failure(source.to_string()),
                ShortcutError::UnsupportedPlatform(_) => Self::invalid_input(source.to_string()),
            },
        }
    }
}

impl From<ingot_content::importer::models::ImportError> for ApiError {
    fn from(error: ingot_content::importer::models::ImportError) -> Self {
        use ingot_content::importer::models::ImportError;
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

impl From<ingot_minecraft::sync::SyncError> for ApiError {
    fn from(error: ingot_minecraft::sync::SyncError) -> Self {
        use ingot_minecraft::sync::SyncError;
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

impl From<ingot_minecraft::screenshots::ScreenshotError> for ApiError {
    fn from(error: ingot_minecraft::screenshots::ScreenshotError) -> Self {
        use ingot_minecraft::screenshots::ScreenshotError;
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

impl From<ingot_content::content::error::ContentError> for ApiError {
    fn from(error: ingot_content::content::error::ContentError) -> Self {
        use ingot_content::content::error::ContentError;
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
            | ContentError::UnsupportedLoader { .. }
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

impl From<ingot_core::metadata::MetadataError> for ApiError {
    fn from(error: ingot_core::metadata::MetadataError) -> Self {
        use ingot_core::metadata::MetadataError;
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

impl From<ingot_version_change::service::ChangeServiceError> for ApiError {
    fn from(error: ingot_version_change::service::ChangeServiceError) -> Self {
        use ingot_version_change::service::ChangeServiceError;
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

impl From<ingot_version_change::VersionChangeError> for ApiError {
    fn from(error: ingot_version_change::VersionChangeError) -> Self {
        use ingot_artifacts::sources::SourceError;
        use ingot_version_change::VersionChangeError;
        match error {
            VersionChangeError::InvalidFileName(msg)
            | VersionChangeError::InvalidPath(msg)
            | VersionChangeError::PlanError(msg)
            | VersionChangeError::UndoError(msg)
            | VersionChangeError::InvalidTarget(msg) => Self::invalid_input(msg),
            VersionChangeError::RollbackFailed {
                primary,
                restore,
                backup_path,
                checkpoint,
            } => Self::incomplete_recovery(format!(
                "{primary}; restore also failed: {restore}; checkpoint failure: {checkpoint:?}. Recovery backup preserved at {}",
                backup_path.display()
            )),
            VersionChangeError::RollbackSucceeded(msg) => {
                Self::conflict(format!("Version change aborted and rolled back: {msg}"))
            }
            VersionChangeError::Finalization { .. }
            | VersionChangeError::MetadataAfterRestore { .. }
            | VersionChangeError::CommitJournal { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            VersionChangeError::FileIo { .. } | VersionChangeError::JournalDecode(_) => {
                Self::storage_failure(error.to_string())
            }
            VersionChangeError::RestoreFailures(_)
            | VersionChangeError::MissingRecoveryFile(_)
            | VersionChangeError::AdditionalFailure { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            VersionChangeError::MetadataCommit(_) => Self::storage_failure(error.to_string()),
            VersionChangeError::ActiveTransaction { .. }
            | VersionChangeError::IncompleteTransaction(_)
            | VersionChangeError::AlreadyDisabled(_) => Self::conflict(error.to_string()),
            VersionChangeError::RecoveryUnavailable(_) | VersionChangeError::NoVersionChange => {
                Self::invalid_input(error.to_string())
            }
            VersionChangeError::DownloadPrep { .. }
            | VersionChangeError::JournalIo { .. }
            | VersionChangeError::JournalEncode(_)
            | VersionChangeError::ScanDirectory { .. }
            | VersionChangeError::ScanFile { .. } => Self::storage_failure(error.to_string()),
            VersionChangeError::DownloadFailed { source, .. } => Self::from(source),
            VersionChangeError::Task(_) => Self::internal(error.to_string()),
            VersionChangeError::ChecksumMismatch(msg)
            | VersionChangeError::UnknownHost(msg)
            | VersionChangeError::NoChecksum(msg) => Self::provider_unavailable(msg),
            VersionChangeError::Source(source) => match source {
                SourceError::InvalidProjectId { .. } => Self::invalid_input(source.to_string()),
                SourceError::Transport { .. }
                | SourceError::InvalidPayload { .. }
                | SourceError::HttpStatus { .. }
                | SourceError::Decode { .. } => Self::provider_unavailable(source.to_string()),
            },
        }
    }
}

impl From<ingot_artifacts::crash::CrashError> for ApiError {
    fn from(error: ingot_artifacts::crash::CrashError) -> Self {
        use ingot_artifacts::crash::CrashError;
        match error {
            CrashError::InvalidFileName(msg) | CrashError::InvalidPath(msg) => {
                Self::invalid_input(msg)
            }
            CrashError::FileIo { .. }
            | CrashError::JournalDecode(_)
            | CrashError::JournalIo { .. } => Self::storage_failure(error.to_string()),
            CrashError::IncompleteTransaction(_) | CrashError::AlreadyDisabled(_) => {
                Self::conflict(error.to_string())
            }
            CrashError::NoVersionChange => Self::invalid_input(error.to_string()),
        }
    }
}

impl From<crate::app::games::GameLaunchError> for ApiError {
    fn from(error: crate::app::games::GameLaunchError) -> Self {
        match error {
            crate::app::games::GameLaunchError::Launcher(err) => Self::from(err),
            crate::app::games::GameLaunchError::IncompleteTransaction(msg) => {
                Self::incomplete_recovery(format!(
                    "Cannot launch instance: version change transaction is in incomplete state ({msg:?}). Please recover or revert it first."
                ))
            }
            crate::app::games::GameLaunchError::TransactionInspection(_) => {
                Self::incomplete_recovery(error.to_string())
            }
        }
    }
}

impl From<ingot_minecraft::launcher::LaunchError> for ApiError {
    fn from(error: ingot_minecraft::launcher::LaunchError) -> Self {
        use ingot_accounts::account::{recovery::RecoveryError, session::SessionError};
        use ingot_minecraft::launcher::LaunchError;
        match error {
            LaunchError::Metadata(error) => Self::from(error),
            LaunchError::InstanceNotFound(id) => {
                Self::not_found(format!("Instance not found: {id}"))
            }
            LaunchError::AlreadyRunning => Self::conflict("Instance is already running"),
            LaunchError::NotRunning(id) => Self::not_found(format!("Instance {id} is not running")),
            LaunchError::Maintenance => Self::conflict("Instance maintenance is in progress"),
            LaunchError::LoaderVersionNotSelected => {
                Self::invalid_input("Loader version not selected")
            }
            LaunchError::InstanceDir(source) => Self::from(source),
            LaunchError::AppDataDir(source) => Self::storage_failure(source.to_string()),
            LaunchError::VersionPackage(err) => Self::from(err),
            LaunchError::LoaderProfile(err) => Self::from(err),
            LaunchError::Download(err) => Self::from(err),
            LaunchError::JavaRuntime(err) => Self::from(err),
            LaunchError::Account(error) => match error.downcast::<RecoveryError>() {
                Ok(source) => Self::from(*source),
                Err(error) => match error.downcast::<SessionError>() {
                    Ok(source) => Self::from(*source),
                    Err(error) => Self::internal(error.to_string()),
                },
            },
            LaunchError::Settings(source) => Self::from(source),
            LaunchError::ProcessSpawn(err) => {
                Self::internal(format!("Failed to spawn Minecraft process: {err}"))
            }
            LaunchError::Io(err) => Self::storage_failure(err.to_string()),
            LaunchError::ReservationCancelled => Self::cancelled(error.to_string()),
            LaunchError::RegistryPoisoned | LaunchError::Termination(_) | LaunchError::Task(_) => {
                Self::internal(error.to_string())
            }
            LaunchError::Horizon(source) => Self::from(source),
        }
    }
}

impl From<ingot_minecraft::bedrock::BedrockError> for ApiError {
    fn from(error: ingot_minecraft::bedrock::BedrockError) -> Self {
        use ingot_minecraft::bedrock::BedrockError;
        match error {
            BedrockError::UnsupportedPlatform => Self::invalid_input(error.to_string()),
            BedrockError::FolderNotFound => Self::not_found(error.to_string()),
            BedrockError::ProcessSpawn(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_minecraft::version::VersionError> for ApiError {
    fn from(error: ingot_minecraft::version::VersionError) -> Self {
        use ingot_minecraft::version::VersionError;
        match error {
            VersionError::NotFound(_) => Self::not_found(error.to_string()),
            VersionError::Network(_) => Self::provider_unavailable(error.to_string()),
            VersionError::Json(_) | VersionError::Io(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_minecraft::loader::LoaderError> for ApiError {
    fn from(error: ingot_minecraft::loader::LoaderError) -> Self {
        use ingot_minecraft::loader::LoaderError;
        match error {
            LoaderError::Network(_) => Self::provider_unavailable(error.to_string()),
            LoaderError::VanillaNotSupported => Self::invalid_input(error.to_string()),
            LoaderError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_artifacts::downloader::DownloadError> for ApiError {
    fn from(error: ingot_artifacts::downloader::DownloadError) -> Self {
        use ingot_artifacts::downloader::DownloadError;
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

impl From<ingot_minecraft::horizon::HorizonError> for ApiError {
    fn from(error: ingot_minecraft::horizon::HorizonError) -> Self {
        use ingot_minecraft::horizon::HorizonError;
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

impl From<ingot_minecraft::news::NewsError> for ApiError {
    fn from(error: ingot_minecraft::news::NewsError) -> Self {
        use ingot_minecraft::news::NewsError;
        match error {
            NewsError::Network(_) | NewsError::HttpStatus(_) => {
                Self::provider_unavailable(error.to_string())
            }
            NewsError::InvalidContentPath(_) => Self::invalid_input(error.to_string()),
            NewsError::ArticleNotFound(_) => Self::not_found(error.to_string()),
            NewsError::Io(_) => Self::storage_failure(error.to_string()),
            NewsError::Parse(_) | NewsError::Task(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_minecraft::logs::LogError> for ApiError {
    fn from(error: ingot_minecraft::logs::LogError) -> Self {
        use ingot_minecraft::logs::LogError;
        match error {
            LogError::InvalidFileName(_)
            | LogError::EmptyContent
            | LogError::FileTooLarge(_)
            | LogError::ProviderRejected(_) => Self::invalid_input(error.to_string()),
            LogError::NotFound(_) => Self::not_found(error.to_string()),
            LogError::Io(_) => Self::storage_failure(error.to_string()),
            LogError::Instance(source) => Self::from(source),
            LogError::Task(_) => Self::internal(error.to_string()),
            LogError::Transport(_) | LogError::HttpStatus { .. } | LogError::InvalidResponse(_) => {
                Self::provider_unavailable(error.to_string())
            }
        }
    }
}

impl From<ingot_minecraft::worlds::WorldError> for ApiError {
    fn from(error: ingot_minecraft::worlds::WorldError) -> Self {
        use ingot_minecraft::worlds::WorldError;
        match error {
            WorldError::InstanceNotFound(msg) | WorldError::NotFound(msg) => Self::not_found(msg),
            WorldError::DimensionNotFound(dim) => {
                Self::not_found(format!("Dimension not found: {dim:?}"))
            }
            WorldError::InstanceRunning => {
                Self::conflict("Instance is currently running or in maintenance")
            }
            WorldError::InvalidFolderName(msg)
            | WorldError::Archive(msg)
            | WorldError::Nbt(msg)
            | WorldError::Gz(msg) => Self::invalid_input(msg),
            WorldError::Io(_) | WorldError::Zip(_) => Self::storage_failure(error.to_string()),
            WorldError::TaskJoin(_) => Self::internal(error.to_string()),
            WorldError::LoadInstances(source) | WorldError::InstanceDir(source) => {
                Self::from(source)
            }
        }
    }
}

impl From<ingot_minecraft::instance_content::InstanceContentError> for ApiError {
    fn from(error: ingot_minecraft::instance_content::InstanceContentError) -> Self {
        use ingot_minecraft::instance_content::InstanceContentError;
        match error {
            InstanceContentError::InstanceNotFound(msg)
            | InstanceContentError::NotFound(msg)
            | InstanceContentError::WorldNotFound(msg) => Self::not_found(msg),
            InstanceContentError::InstanceRunning(inst) => {
                Self::conflict(format!("Instance '{inst}' is currently running"))
            }
            InstanceContentError::NameConflict(msg) => Self::conflict(msg),
            InstanceContentError::InvalidFileName { reason, .. } => Self::invalid_input(reason),
            InstanceContentError::Io { .. } => Self::storage_failure(error.to_string()),
            InstanceContentError::Serialization(msg)
            | InstanceContentError::Metadata(msg)
            | InstanceContentError::InvalidOperation(msg) => Self::internal(msg),
        }
    }
}

impl From<crate::app::instance_content::InstanceContentActionError> for ApiError {
    fn from(error: crate::app::instance_content::InstanceContentActionError) -> Self {
        use crate::app::instance_content::InstanceContentActionError;
        match error {
            InstanceContentActionError::Content(source) => Self::from(source),
            InstanceContentActionError::Instance(source) => Self::from(source),
            InstanceContentActionError::Task(_) => Self::internal(error.to_string()),
            InstanceContentActionError::FileManager(source) => Self::from(source),
        }
    }
}

impl From<ingot_content::instance_export::InstanceExportError> for ApiError {
    fn from(error: ingot_content::instance_export::InstanceExportError) -> Self {
        use ingot_content::instance_export::InstanceExportError;
        match error {
            InstanceExportError::UnsupportedLoader(_)
            | InstanceExportError::UnsafePath(_)
            | InstanceExportError::EmptyName => Self::invalid_input(error.to_string()),
            InstanceExportError::NotFound(id) => Self::not_found(id),
            InstanceExportError::InstanceRunning => {
                Self::conflict("Instance is currently running".to_string())
            }
            InstanceExportError::Io { .. } | InstanceExportError::Zip(_) => {
                Self::storage_failure(error.to_string())
            }
            InstanceExportError::Provider(_) => Self::provider_unavailable(error.to_string()),
            InstanceExportError::Content(e) => Self::from(e),
            InstanceExportError::Instance(e) => Self::from(e),
            InstanceExportError::Task(_) | InstanceExportError::Serialization(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}
