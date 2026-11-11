use super::ApiError;

impl From<ingot_artifacts::java::JavaError> for ApiError {
    fn from(error: ingot_artifacts::java::JavaError) -> Self {
        use ingot_artifacts::java::JavaError;
        match error {
            JavaError::ProbeCancelled => Self::cancelled(error.to_string()),
            JavaError::Io(_) => Self::storage_failure(error.to_string()),
            JavaError::Http(_) => Self::provider_unavailable(error.to_string()),
            JavaError::MissingBinary => Self::not_found(error.to_string()),
            JavaError::InvalidLocation
            | JavaError::InvalidVersion
            | JavaError::VersionMismatch { .. }
            | JavaError::UnsafeArchive => Self::invalid_input(error.to_string()),
            JavaError::Zip(_)
            | JavaError::ProbeFailed
            | JavaError::ProbeTimeout(_)
            | JavaError::ProbeSpawn(_)
            | JavaError::ProbeCleanup { .. }
            | JavaError::Task(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::config::service::ConfigError> for ApiError {
    fn from(error: ingot_server::config::service::ConfigError) -> Self {
        use ingot_server::config::service::ConfigError;
        match error {
            ConfigError::Metadata(error) => Self::from(error),
            ConfigError::File(error) => Self::from(error),
            ConfigError::Port(ingot_server::config::PortError::PortInUse { .. }) => {
                Self::conflict(error.to_string())
            }
            ConfigError::Port(ingot_server::config::PortError::InvalidPortRange)
            | ConfigError::EmptyName
            | ConfigError::InvalidMemory { .. }
            | ConfigError::InvalidId
            | ConfigError::InvalidPort => Self::invalid_input(error.to_string()),
            ConfigError::Directory(_) | ConfigError::Io(_) => {
                Self::storage_failure(error.to_string())
            }
        }
    }
}

impl From<ingot_server::files::ServerFileError> for ApiError {
    fn from(error: ingot_server::files::ServerFileError) -> Self {
        use ingot_server::files::ServerFileError;
        match error {
            ServerFileError::InvalidProperty(_)
            | ServerFileError::InvalidPath(_)
            | ServerFileError::InvalidIconEncoding(_) => Self::invalid_input(error.to_string()),
            ServerFileError::MissingConfig(_) => Self::not_found(error.to_string()),
            ServerFileError::Io(_)
            | ServerFileError::Json(_)
            | ServerFileError::InvalidAccessEntry(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<ingot_server::plugins::tracking::TrackingError> for ApiError {
    fn from(error: ingot_server::plugins::tracking::TrackingError) -> Self {
        match error {
            ingot_server::plugins::tracking::TrackingError::Recovery(error) => Self::from(*error),
            error => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<ingot_server::process::models::ProcessError> for ApiError {
    fn from(error: ingot_server::process::models::ProcessError) -> Self {
        use ingot_server::process::models::ProcessError;
        match error {
            ProcessError::NotRunning(_)
            | ProcessError::ServerNotFound(_)
            | ProcessError::MissingJavaConsoleBinary { .. } => Self::not_found(error.to_string()),
            ProcessError::AlreadyRunning
            | ProcessError::AlreadyStarting
            | ProcessError::StillStarting
            | ProcessError::ServerStopped
            | ProcessError::MustBeRunningToSleep
            | ProcessError::PortInUse { .. }
            | ProcessError::StartAdmissionPaused => Self::conflict(error.to_string()),
            ProcessError::NoReply(_) | ProcessError::ShutdownTimeout { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            ProcessError::LaunchCancelled => Self::cancelled(error.to_string()),
            ProcessError::ProxyStartupCleanup { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            ProcessError::Termination(source) => tree_termination_error(source.cause()),
            ProcessError::ShutdownWorker(_) => Self::internal(error.to_string()),
            ProcessError::ShutdownFailed { .. } => Self::incomplete_recovery(error.to_string()),
            ProcessError::Config(error) => Self::from(error),
            ProcessError::Metadata(error) => Self::from(error),
            ProcessError::File(error) => Self::from(error),
            ProcessError::Java(error) => Self::from(error),
            ProcessError::StdinWrite(_) | ProcessError::StdinFlush(_) => {
                Self::storage_failure(error.to_string())
            }
            ProcessError::Core(err) => Self::from(err),
            ProcessError::Installer(err) => Self::from(err),
            ProcessError::Pumpkin(err) => Self::from(err),
            ProcessError::Bedrock(err) => Self::from(err),
            ProcessError::Sandbox(err) => Self::from(err),
            ProcessError::Io(_) => Self::storage_failure(error.to_string()),
            _ => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::plugins::PluginError> for ApiError {
    fn from(error: ingot_server::plugins::PluginError) -> Self {
        use ingot_server::plugins::PluginError;
        match error {
            PluginError::UnsupportedCore
            | PluginError::InvalidJarName(_)
            | PluginError::NoCompatibleVersion { .. }
            | PluginError::ManualDownloadRequired(_) => Self::invalid_input(error.to_string()),
            PluginError::Tracking(error) => Self::from(error),
            PluginError::Io(error) => Self::storage_failure(error.to_string()),
            PluginError::PublicationRollbackFailed { .. }
            | PluginError::Cleanup { .. }
            | PluginError::RecoveryBlocked(_) => Self::incomplete_recovery(error.to_string()),
            PluginError::PublicationFailed { primary } => Self::from(*primary),
            PluginError::HttpClient(_)
            | PluginError::RequestFailed(_)
            | PluginError::HttpStatus(_)
            | PluginError::InvalidResponse(_)
            | PluginError::InvalidPayload(_)
            | PluginError::EmptyDownload => Self::provider_unavailable(error.to_string()),
            PluginError::Json(_) | PluginError::Task(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::pumpkin::PumpkinError> for ApiError {
    fn from(error: ingot_server::pumpkin::PumpkinError) -> Self {
        use ingot_server::pumpkin::PumpkinError;
        match error {
            PumpkinError::Network(_) | PumpkinError::HttpStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            PumpkinError::AssetNotFound(_) => Self::not_found(error.to_string()),
            PumpkinError::InvalidBinaryPath => Self::invalid_input(error.to_string()),
            PumpkinError::Io(_) | PumpkinError::TomlParse(_) => {
                Self::storage_failure(error.to_string())
            }
            PumpkinError::Sandbox(err) => Self::from(err),
            PumpkinError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::bedrock::BedrockServerError> for ApiError {
    fn from(error: ingot_server::bedrock::BedrockServerError) -> Self {
        use ingot_server::bedrock::BedrockServerError;
        match error {
            BedrockServerError::Network(_) | BedrockServerError::HttpStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            BedrockServerError::BinaryNotFound(_) => Self::not_found(error.to_string()),
            BedrockServerError::Io(_) => Self::storage_failure(error.to_string()),
            BedrockServerError::Archive(source) => Self::from(source),
            BedrockServerError::InvalidBinaryPath(_)
            | BedrockServerError::InvalidArchivePath(_)
            | BedrockServerError::UnsafeDestination(_)
            | BedrockServerError::InvalidStagingRoot(_)
            | BedrockServerError::UnsupportedEntry(_) => Self::invalid_input(error.to_string()),
            BedrockServerError::Zip(_)
            | BedrockServerError::Task(_)
            | BedrockServerError::RelativePath { .. } => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::ServerCoreError> for ApiError {
    fn from(error: ingot_server::ServerCoreError) -> Self {
        use ingot_server::ServerCoreError;
        match error {
            ServerCoreError::Network(_) => Self::provider_unavailable(error.to_string()),
            ServerCoreError::Download(err) => Self::from(err),
            ServerCoreError::NotFound(_) => Self::not_found(error.to_string()),
            ServerCoreError::Installer(err) => Self::from(err),
            ServerCoreError::Pumpkin(err) => Self::from(err),
            ServerCoreError::Bedrock(err) => Self::from(err),
            ServerCoreError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::installer::InstallerError> for ApiError {
    fn from(error: ingot_server::installer::InstallerError) -> Self {
        use ingot_server::installer::InstallerError;
        match error {
            InstallerError::Cleanup { .. } => Self::incomplete_recovery(error.to_string()),
            InstallerError::ExecutionTimeout { .. } => Self::internal(error.to_string()),
            InstallerError::Network(_) => Self::provider_unavailable(error.to_string()),
            InstallerError::Download(err) => Self::from(err),
            InstallerError::VersionUnavailable { .. }
            | InstallerError::NoQuiltInstaller
            | InstallerError::NotConfigured(_) => Self::not_found(error.to_string()),
            InstallerError::UnsupportedCore(_) | InstallerError::ForgeVersionUnsupported(_) => {
                Self::invalid_input(error.to_string())
            }
            InstallerError::Io(_) => Self::storage_failure(error.to_string()),
            InstallerError::Process { .. }
            | InstallerError::InstallationFailed { .. }
            | InstallerError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::companion::CompanionError> for ApiError {
    fn from(error: ingot_server::companion::CompanionError) -> Self {
        use ingot_server::companion::CompanionError;
        match error {
            CompanionError::UnsupportedPlatform => Self::invalid_input(error.to_string()),
            CompanionError::Plugin(error) => Self::from(error),
            CompanionError::Io(error) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<ingot_server::bedrock_packs::BedrockPackError> for ApiError {
    fn from(error: ingot_server::bedrock_packs::BedrockPackError) -> Self {
        use ingot_server::bedrock_packs::BedrockPackError;
        match error {
            BedrockPackError::SystemPack
            | BedrockPackError::InvalidFolderName
            | BedrockPackError::MissingManifest
            | BedrockPackError::InvalidManifest
            | BedrockPackError::ManifestJson(_) => Self::invalid_input(error.to_string()),
            BedrockPackError::NotFound(_) => Self::not_found(error.to_string()),
            BedrockPackError::AlreadyExists(_) => Self::conflict(error.to_string()),
            BedrockPackError::HttpStatus(_) | BedrockPackError::Network(_) => {
                Self::provider_unavailable(error.to_string())
            }
            BedrockPackError::Io(_) | BedrockPackError::Persist(_) => {
                Self::storage_failure(error.to_string())
            }
            BedrockPackError::CleanupFailed { .. } => Self::incomplete_recovery(error.to_string()),
            BedrockPackError::Archive(err) => Self::from(err),
            BedrockPackError::Json(_) | BedrockPackError::Task(_) | BedrockPackError::Zip(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<ingot_core::ArchiveError> for ApiError {
    fn from(error: ingot_core::ArchiveError) -> Self {
        use ingot_core::ArchiveError;
        match error {
            ArchiveError::UnsafeEntry => Self::invalid_input(error.to_string()),
            ArchiveError::Zip(_) => Self::internal(error.to_string()),
            ArchiveError::Io(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<ingot_server_map::map::MapError> for ApiError {
    fn from(error: ingot_server_map::map::MapError) -> Self {
        use ingot_server_map::map::MapError;
        match error {
            MapError::RegionRead { .. } | MapError::TileRead { .. } => {
                Self::storage_failure(error.to_string())
            }
            MapError::BedrockOpen { .. }
            | MapError::BedrockQuery { .. }
            | MapError::PngEncoding(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server_map::live::LiveError> for ApiError {
    fn from(error: ingot_server_map::live::LiveError) -> Self {
        use ingot_server_map::live::LiveError;
        match error {
            LiveError::InvalidPlayerName(_) => Self::invalid_input(error.to_string()),
            LiveError::PlayerNotOnline(_) => Self::conflict(error.to_string()),
            LiveError::NoSavedData(_) => Self::not_found(error.to_string()),
            LiveError::Io(_) => Self::storage_failure(error.to_string()),
            LiveError::Snbt(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::live_query::LiveQueryError> for ApiError {
    fn from(error: ingot_server::live_query::LiveQueryError) -> Self {
        use ingot_server::live_query::LiveQueryError;
        match error {
            LiveQueryError::InvalidPlayerName(_) => Self::invalid_input(error.to_string()),
            LiveQueryError::PlayerNotOnline(_) => Self::conflict(error.to_string()),
            LiveQueryError::Query(source) => Self::from(*source),
            LiveQueryError::Snbt(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_server::transfer::TransferError> for ApiError {
    fn from(error: ingot_server::transfer::TransferError) -> Self {
        use ingot_server::transfer::TransferError;
        match error {
            TransferError::InvalidZip(_)
            | TransferError::NotAnIngotExport
            | TransferError::InvalidManifest(_)
            | TransferError::UnsupportedManifestFormat(_)
            | TransferError::InvalidUploadId
            | TransferError::UnsupportedEntry
            | TransferError::InvalidPathEncoding => Self::invalid_input(error.to_string()),
            TransferError::Process(source) => Self::from(source),
            TransferError::Task(_) => Self::internal(error.to_string()),
            TransferError::UploadFinalized => Self::conflict(error.to_string()),
            TransferError::UploadCleanup { .. } | TransferError::StagingCleanup { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
            TransferError::Config(err) => Self::from(err),
            TransferError::File(err) => Self::from(err),
            TransferError::Installer(err) => Self::from(err),
            TransferError::Archive(err) => Self::from(err),
            TransferError::Io(_) | TransferError::CopyFailed { .. } => {
                Self::storage_failure(error.to_string())
            }
            TransferError::Zip(_)
            | TransferError::Json(_)
            | TransferError::MissingParent
            | TransferError::StripPrefix(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_tunnel::PlayitError> for ApiError {
    fn from(error: ingot_tunnel::PlayitError) -> Self {
        use ingot_tunnel::PlayitError;
        match error {
            PlayitError::Io(_)
            | PlayitError::Directory(_)
            | PlayitError::CredentialRead(_)
            | PlayitError::CredentialWrite(_)
            | PlayitError::CredentialDelete(_) => Self::storage_failure(error.to_string()),
            PlayitError::Cancelled => Self::cancelled(error.to_string()),
            PlayitError::TerminationFailed(_) => Self::internal(error.to_string()),
            PlayitError::Network(_)
            | PlayitError::HttpStatus(_)
            | PlayitError::InvalidResponse
            | PlayitError::Rejected(_)
            | PlayitError::ClaimRejected
            | PlayitError::ClaimExpired
            | PlayitError::ClaimTimeout
            | PlayitError::InvalidCredential
            | PlayitError::AgentExited(_) => Self::provider_unavailable(error.to_string()),
            PlayitError::Sandbox(err) => Self::from(err),
        }
    }
}

impl From<ingot_server::slp::PingError> for ApiError {
    fn from(error: ingot_server::slp::PingError) -> Self {
        use ingot_server::slp::PingError;
        match error {
            PingError::Timeout => Self::provider_unavailable("Server ping timed out"),
            PingError::Io(_) => Self::provider_unavailable(error.to_string()),
            PingError::InvalidResponse(_)
            | PingError::Utf8(_)
            | PingError::InvalidVarInt(_)
            | PingError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<ingot_sandbox::SandboxError> for ApiError {
    fn from(error: ingot_sandbox::SandboxError) -> Self {
        use ingot_sandbox::SandboxError;
        match error {
            SandboxError::Io(_) => Self::storage_failure(error.to_string()),
            SandboxError::PublicationPending(_)
            | SandboxError::RecoveryBlocked(_)
            | SandboxError::Receipt(_)
            | SandboxError::Cleanup { .. }
            | SandboxError::ProcessCleanup { .. } => Self::incomplete_recovery(error.to_string()),
            SandboxError::InvalidRootfs => Self::provider_unavailable(error.to_string()),
            SandboxError::Network(_) | SandboxError::DownloadStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            SandboxError::ProotNotFound
            | SandboxError::LoaderNotFound(_)
            | SandboxError::RootfsRequired
            | SandboxError::JavaBinaryNotFound { .. } => Self::not_found(error.to_string()),
            SandboxError::ExecutionTimeout
            | SandboxError::WorkerStopped
            | SandboxError::Task(_)
            | SandboxError::JoinPaths(_)
            | SandboxError::JavaInstallFailed { .. } => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::app::lifecycle::LifecycleError> for ApiError {
    fn from(error: crate::app::lifecycle::LifecycleError) -> Self {
        match error {
            crate::app::lifecycle::LifecycleError::LeftoverServerMissing => {
                Self::not_found(error.to_string())
            }
            crate::app::lifecycle::LifecycleError::Servers(_) => {
                Self::incomplete_recovery(error.to_string())
            }
            crate::app::lifecycle::LifecycleError::Termination(source) => {
                tree_termination_error(&source)
            }
            crate::app::lifecycle::LifecycleError::ShutdownWorker(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}

fn tree_termination_error(error: &ingot_core::process_tree::TerminationError) -> ApiError {
    use ingot_core::process_tree::TerminationError;
    match error {
        TerminationError::Cleanup { .. }
        | TerminationError::Failures { .. }
        | TerminationError::ExitUnconfirmed(_) => ApiError::incomplete_recovery(error.to_string()),
        _ => ApiError::internal(error.to_string()),
    }
}
