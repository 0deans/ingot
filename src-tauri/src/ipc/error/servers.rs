use super::ApiError;

impl From<crate::minecraft::java::JavaError> for ApiError {
    fn from(error: crate::minecraft::java::JavaError) -> Self {
        use crate::minecraft::java::JavaError;
        match error {
            JavaError::Io(_) => Self::storage_failure(error.to_string()),
            JavaError::Http(_) => Self::provider_unavailable(error.to_string()),
            JavaError::MissingBinary => Self::not_found(error.to_string()),
            JavaError::InvalidLocation
            | JavaError::InvalidVersion
            | JavaError::VersionMismatch { .. }
            | JavaError::UnsafeArchive => Self::invalid_input(error.to_string()),
            JavaError::Zip(_) | JavaError::ProbeFailed | JavaError::Task(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<crate::server::config::service::ConfigError> for ApiError {
    fn from(error: crate::server::config::service::ConfigError) -> Self {
        use crate::server::config::service::ConfigError;
        match error {
            ConfigError::Metadata(error) => Self::from(error),
            ConfigError::File(error) => Self::from(error),
            ConfigError::Port(crate::server::config::PortError::PortInUse { .. }) => {
                Self::conflict(error.to_string())
            }
            ConfigError::Port(crate::server::config::PortError::InvalidPortRange)
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

impl From<crate::server::files::ServerFileError> for ApiError {
    fn from(error: crate::server::files::ServerFileError) -> Self {
        use crate::server::files::ServerFileError;
        match error {
            ServerFileError::InvalidProperty(_) | ServerFileError::InvalidPath(_) => {
                Self::invalid_input(error.to_string())
            }
            ServerFileError::MissingConfig(_) => Self::not_found(error.to_string()),
            ServerFileError::Io(_)
            | ServerFileError::Json(_)
            | ServerFileError::InvalidAccessEntry(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::server::plugins::tracking::TrackingError> for ApiError {
    fn from(error: crate::server::plugins::tracking::TrackingError) -> Self {
        Self::storage_failure(error.to_string())
    }
}

impl From<crate::server::process::models::ProcessError> for ApiError {
    fn from(error: crate::server::process::models::ProcessError) -> Self {
        use crate::server::process::models::ProcessError;
        match error {
            ProcessError::NotRunning(_) | ProcessError::ServerNotFound(_) => {
                Self::not_found(error.to_string())
            }
            ProcessError::AlreadyRunning
            | ProcessError::AlreadyStarting
            | ProcessError::StillStarting
            | ProcessError::ServerStopped
            | ProcessError::MustBeRunningToSleep
            | ProcessError::PortInUse { .. } => Self::conflict(error.to_string()),
            ProcessError::JarDownload(_) | ProcessError::NoReply(_) => {
                Self::provider_unavailable(error.to_string())
            }
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

impl From<crate::server::plugins::PluginError> for ApiError {
    fn from(error: crate::server::plugins::PluginError) -> Self {
        use crate::server::plugins::PluginError;
        match error {
            PluginError::UnsupportedCore
            | PluginError::InvalidJarName(_)
            | PluginError::NoCompatibleVersion { .. }
            | PluginError::ManualDownloadRequired(_) => Self::invalid_input(error.to_string()),
            PluginError::Tracking(error) => Self::from(error),
            PluginError::Io(error) => Self::storage_failure(error.to_string()),
            PluginError::PublicationRollbackFailed { .. } | PluginError::Cleanup { .. } => {
                Self::incomplete_recovery(error.to_string())
            }
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

impl From<crate::server::pumpkin::PumpkinError> for ApiError {
    fn from(error: crate::server::pumpkin::PumpkinError) -> Self {
        use crate::server::pumpkin::PumpkinError;
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

impl From<crate::server::bedrock::BedrockServerError> for ApiError {
    fn from(error: crate::server::bedrock::BedrockServerError) -> Self {
        use crate::server::bedrock::BedrockServerError;
        match error {
            BedrockServerError::Network(_) | BedrockServerError::HttpStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            BedrockServerError::BinaryNotFound(_) => Self::not_found(error.to_string()),
            BedrockServerError::Io(_) => Self::storage_failure(error.to_string()),
            BedrockServerError::Zip(_)
            | BedrockServerError::Task(_)
            | BedrockServerError::Extraction(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::server::ServerCoreError> for ApiError {
    fn from(error: crate::server::ServerCoreError) -> Self {
        use crate::server::ServerCoreError;
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

impl From<crate::server::installer::InstallerError> for ApiError {
    fn from(error: crate::server::installer::InstallerError) -> Self {
        use crate::server::installer::InstallerError;
        match error {
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

impl From<crate::server::companion::CompanionError> for ApiError {
    fn from(error: crate::server::companion::CompanionError) -> Self {
        use crate::server::companion::CompanionError;
        match error {
            CompanionError::UnsupportedPlatform => Self::invalid_input(error.to_string()),
            CompanionError::Plugin(error) => Self::from(error),
            CompanionError::Io(error) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::server::bedrock_packs::BedrockPackError> for ApiError {
    fn from(error: crate::server::bedrock_packs::BedrockPackError) -> Self {
        use crate::server::bedrock_packs::BedrockPackError;
        match error {
            BedrockPackError::SystemPack
            | BedrockPackError::InvalidFolderName
            | BedrockPackError::MissingManifest
            | BedrockPackError::InvalidManifest => Self::invalid_input(error.to_string()),
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

impl From<crate::app::ArchiveError> for ApiError {
    fn from(error: crate::app::ArchiveError) -> Self {
        use crate::app::ArchiveError;
        match error {
            ArchiveError::UnsafeEntry => Self::invalid_input(error.to_string()),
            ArchiveError::Zip(_) => Self::internal(error.to_string()),
            ArchiveError::Io(_) => Self::storage_failure(error.to_string()),
        }
    }
}

impl From<crate::server::map::MapError> for ApiError {
    fn from(error: crate::server::map::MapError) -> Self {
        use crate::server::map::MapError;
        match error {
            MapError::RegionRead(_) | MapError::TileRead(_) => {
                Self::storage_failure(error.to_string())
            }
            MapError::Bedrock(_) | MapError::PngEncoding(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::server::live::LiveError> for ApiError {
    fn from(error: crate::server::live::LiveError) -> Self {
        use crate::server::live::LiveError;
        match error {
            LiveError::InvalidPlayerName(_) => Self::invalid_input(error.to_string()),
            LiveError::PlayerNotOnline(_) => Self::conflict(error.to_string()),
            LiveError::NoSavedData(_) => Self::not_found(error.to_string()),
            LiveError::Io(_) => Self::storage_failure(error.to_string()),
            LiveError::Query(_) => Self::provider_unavailable(error.to_string()),
            LiveError::Snbt(_) | LiveError::Other(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::server::transfer::TransferError> for ApiError {
    fn from(error: crate::server::transfer::TransferError) -> Self {
        use crate::server::transfer::TransferError;
        match error {
            TransferError::InvalidZip
            | TransferError::NotAnIngotExport
            | TransferError::InvalidManifest(_)
            | TransferError::UnsupportedEntry
            | TransferError::InvalidPathEncoding => Self::invalid_input(error.to_string()),
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

impl From<crate::server::tunnel::PlayitError> for ApiError {
    fn from(error: crate::server::tunnel::PlayitError) -> Self {
        use crate::server::tunnel::PlayitError;
        match error {
            PlayitError::Io(_) | PlayitError::AppData(_) => {
                Self::storage_failure(error.to_string())
            }
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

impl From<crate::server::slp::PingError> for ApiError {
    fn from(error: crate::server::slp::PingError) -> Self {
        use crate::server::slp::PingError;
        match error {
            PingError::Timeout => Self::provider_unavailable("Server ping timed out"),
            PingError::Io(_) => Self::provider_unavailable(error.to_string()),
            PingError::InvalidResponse(_) | PingError::Json(_) => Self::internal(error.to_string()),
        }
    }
}

impl From<crate::server::sandbox::SandboxError> for ApiError {
    fn from(error: crate::server::sandbox::SandboxError) -> Self {
        use crate::server::sandbox::SandboxError;
        match error {
            SandboxError::Io(_) | SandboxError::AppDataDir(_) => {
                Self::storage_failure(error.to_string())
            }
            SandboxError::Network(_) | SandboxError::DownloadStatus { .. } => {
                Self::provider_unavailable(error.to_string())
            }
            SandboxError::ProotNotFound
            | SandboxError::LoaderNotFound(_)
            | SandboxError::RootfsRequired
            | SandboxError::JavaBinaryNotFound { .. } => Self::not_found(error.to_string()),
            SandboxError::Task(_)
            | SandboxError::JoinPaths(_)
            | SandboxError::JavaInstallFailed { .. }
            | SandboxError::Other(_) => Self::internal(error.to_string()),
        }
    }
}
