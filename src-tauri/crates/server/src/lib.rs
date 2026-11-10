pub(crate) use self as server;
pub(crate) use ingot_artifacts as artifacts;
pub(crate) use ingot_core as core;

pub const USER_AGENT: &str = concat!(
    "Ingot-Launcher/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/0deans/ingot)"
);

pub mod bedrock;
pub mod bedrock_packs;
pub mod companion;
pub mod config;
pub mod downloader;
pub mod files;
pub mod installer;
pub mod live;
pub mod map;
pub mod plugins;
pub mod process;
pub mod proxy;
pub mod pumpkin;
pub mod sandbox;
pub mod slp;
pub mod stats;
pub mod storage;
pub mod supervisor;
pub mod transfer;
pub mod tunnel;

pub use config::{
    add_to_server_whitelist, get_server_icon_base64, read_server_properties_from_dir,
    read_server_whitelist, remove_from_server_whitelist, save_server_icon,
    write_server_properties_to_dir, PlayitTunnelStatus, RunningServerSummary, ServerConfig,
    ServerCoreType, ServerLogEvent, ServerProperties, ServerStatus, ServerStatusEvent,
    WhitelistEntry,
};
pub use downloader::{fetch_core_versions, ServerCoreError};
pub use process::ServerProcessManager;
pub use slp::{
    ping_server, PingError, ServerPingResponse, ServerPlayerSample, ServerPlayersInfo,
    ServerVersionInfo,
};
pub use supervisor::ServerSupervisorManager;
pub use transfer::TransferError;
pub use tunnel::{PlayitError, PlayitManager};
