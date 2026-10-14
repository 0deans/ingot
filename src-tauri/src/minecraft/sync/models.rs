#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub options_synced: bool,
    pub servers_synced: bool,
    pub resource_packs_count: u32,
    pub command_history_synced: bool,
    pub creative_hotbars_synced: bool,
    pub message: String,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedSyncStatus {
    pub has_options: bool,
    pub has_servers: bool,
    pub resource_packs_count: u32,
    pub has_command_history: bool,
    pub has_creative_hotbars: bool,
    pub last_synced_timestamp: Option<u64>,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflictInfo {
    pub instance_id: String,
    pub instance_name: String,
    pub has_instance_options: bool,
    pub has_instance_servers: bool,
    pub has_shared_options: bool,
    pub has_shared_servers: bool,
    pub shared_modified: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncCategory {
    All,
    Options,
    Servers,
    CommandHistory,
    Hotbar,
    ResourcePacks,
}

impl SyncCategory {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "options" => Some(Self::Options),
            "servers" => Some(Self::Servers),
            "command_history" => Some(Self::CommandHistory),
            "hotbar" => Some(Self::Hotbar),
            "resourcepacks" => Some(Self::ResourcePacks),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Options => "options",
            Self::Servers => "servers",
            Self::CommandHistory => "command_history",
            Self::Hotbar => "hotbar",
            Self::ResourcePacks => "resourcepacks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    UseShared,
    UseInstance,
    DisableSync,
}

impl ConflictResolution {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "use_shared" => Some(Self::UseShared),
            "use_instance" => Some(Self::UseInstance),
            "disable_sync" => Some(Self::DisableSync),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Failed to get app data directory: {0}")]
    AppDataDir(String),

    #[error("Failed to create shared sync directory: {0}")]
    SharedDirCreation(#[source] std::io::Error),

    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Settings error: {0}")]
    Settings(String),

    #[error("Instances error: {0}")]
    Instances(String),

    #[error("Unknown sync category: {0}")]
    InvalidCategory(String),

    #[error("Unknown conflict resolution: {0}")]
    InvalidResolution(String),

    #[error("I/O error during sync: {0}")]
    Io(#[from] std::io::Error),
}

impl From<SyncError> for String {
    fn from(err: SyncError) -> Self {
        err.to_string()
    }
}
