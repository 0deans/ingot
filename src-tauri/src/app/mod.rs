//! Application-owned dependencies and paths resolved at the native boundary.

pub(crate) mod accounts;
pub(crate) mod events;
pub(crate) mod lifecycle;
mod paths;
mod state;

pub(crate) use paths::AppPaths;
pub(crate) use state::state;
pub(crate) use state::AppState;
