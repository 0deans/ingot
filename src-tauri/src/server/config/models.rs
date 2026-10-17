use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServerCoreType {
    #[default]
    Paper,
    Purpur,
    Fabric,
    Vanilla,
    Folia,
    Pumpkin,
    NeoForge,
    Forge,
    Quilt,
    Bedrock,
}

impl std::fmt::Display for ServerCoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paper => write!(f, "Paper"),
            Self::Purpur => write!(f, "Purpur"),
            Self::Fabric => write!(f, "Fabric"),
            Self::Vanilla => write!(f, "Vanilla"),
            Self::Folia => write!(f, "Folia"),
            Self::Pumpkin => write!(f, "Pumpkin"),
            Self::NeoForge => write!(f, "NeoForge"),
            Self::Forge => write!(f, "Forge"),
            Self::Quilt => write!(f, "Quilt"),
            Self::Bedrock => write!(f, "Bedrock"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Sleeping,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlayitTunnelStatus {
    pub is_running: bool,
    pub status: String,
    pub claim_url: Option<String>,
    pub public_address: Option<String>,
    pub ping_ms: Option<u32>,
    /// Human-readable detail for "no_tunnel" / "error" states
    pub message: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub core: ServerCoreType,
    pub game_version: String,
    pub build_number: Option<String>,
    pub port: u16,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub icon: Option<String>,
    pub java_path: Option<String>,
    pub jvm_args: Option<Vec<String>>,
    pub auto_start: Option<bool>,
    pub sleep_enabled: Option<bool>,
    pub idle_timeout_seconds: Option<u64>,
    pub internal_port: Option<u16>,
    pub playit_enabled: Option<bool>,
    pub playit_secret_key: Option<String>,
    pub created_at: u64,
    pub last_run_at: Option<u64>,
}

impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerConfig")
            .field("id", &self.id)
            .field("core", &self.core)
            .field("game_version", &self.game_version)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerProperties {
    pub server_port: u16,
    pub motd: String,
    pub online_mode: bool,
    pub difficulty: String,
    pub gamemode: String,
    pub max_players: u32,
    pub pvp: bool,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub white_list: bool,
    pub allow_flight: bool,
    pub spawn_protection: u32,
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            server_port: 25565,
            motd: "A Minecraft Server hosted with Ingot".to_string(),
            online_mode: false, // Default to offline/hybrid mode so Ely.by/local players can connect easily
            difficulty: "normal".to_string(),
            gamemode: "survival".to_string(),
            max_players: 20,
            pvp: true,
            view_distance: 10,
            simulation_distance: 8,
            white_list: false,
            allow_flight: false,
            spawn_protection: 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunningServerSummary {
    pub server_id: String,
    pub pid: u32,
    pub port: u16,
    pub uptime_seconds: u64,
    pub status: ServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerLogEvent {
    pub server_id: String,
    pub line: String,
    pub level: String, // "info" | "warn" | "error"
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusEvent {
    pub server_id: String,
    pub status: ServerStatus,
    pub pid: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Port {port} is already used by \"{owner}\". Try {suggested}.")]
    PortInUse {
        port: u16,
        owner: String,
        suggested: u16,
    },
    #[error("Use a port between 1024 and 65534.")]
    InvalidPortRange,
}
