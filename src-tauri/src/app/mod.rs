//! Application-owned dependencies and paths resolved at the native boundary.

pub(crate) mod accounts;
pub(crate) mod cli;
pub(crate) mod cli_waiter;
pub(crate) mod content;
pub(crate) mod events;
pub(crate) mod games;
pub(crate) mod importer;
pub(crate) mod instance_content;
pub(crate) mod instance_export;
pub(crate) mod instances;
pub(crate) mod java;
pub(crate) mod lifecycle;
pub(crate) mod paths;
pub(crate) mod platform;
pub(crate) mod screenshots;
pub(crate) mod server_config;
pub(crate) mod server_lifecycle;
pub(crate) mod server_transfer;
mod state;
pub(crate) mod sync;
pub(crate) mod worlds;

pub(crate) use paths::AppPaths;
pub(crate) use server_lifecycle::TauriServerLifecycleHooks;
pub(crate) use state::state;
pub(crate) use state::AppState;
