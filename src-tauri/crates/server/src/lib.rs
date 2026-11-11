pub(crate) use self as server;
pub(crate) use ingot_artifacts as artifacts;
pub(crate) use ingot_core as core;

pub use ingot_core::USER_AGENT;

pub mod bedrock;
pub mod bedrock_packs;
pub mod companion;
pub mod config;
pub mod downloader;
pub mod files;
pub mod installer;
pub mod map {
    pub use crate::config::level_name;
}
pub mod live_query;
pub mod plugins;
pub mod process;
pub mod proxy;
pub mod pumpkin;
pub mod slp;
pub mod stats;
pub mod storage;
pub mod supervisor;
pub mod transfer;

pub use config::{
    add_to_server_whitelist, get_server_icon_base64, read_server_properties_from_dir,
    read_server_whitelist, remove_from_server_whitelist, save_server_icon,
    write_server_properties_to_dir, RunningServerSummary, ServerConfig, ServerCoreType,
    ServerLogEvent, ServerProperties, ServerStatus, ServerStatusEvent, WhitelistEntry,
};
pub use downloader::{fetch_core_versions, ServerCoreError};
pub use process::ServerProcessManager;
pub use slp::{
    ping_server, PingError, ServerPingResponse, ServerPlayerSample, ServerPlayersInfo,
    ServerVersionInfo,
};
pub use supervisor::ServerSupervisorManager;
pub use transfer::TransferError;
