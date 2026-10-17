//! Application-owned dependencies and paths resolved at the native boundary.

pub(crate) mod accounts;
mod archive;
pub(crate) mod events;
pub(crate) mod games;
pub(crate) mod importer;
pub(crate) mod lifecycle;
mod paths;
pub(crate) mod platform;
pub(crate) mod screenshots;
mod state;
pub(crate) mod sync;

pub(crate) use archive::extract_zip_entries;
pub(crate) use paths::{safe_enclosed_filename, safe_enclosed_path, AppPaths};
pub(crate) use state::state;
pub(crate) use state::AppState;
