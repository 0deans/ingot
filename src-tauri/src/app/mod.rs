//! Application-owned dependencies and paths resolved at the native boundary.

mod paths;
mod state;

pub(crate) use paths::AppPaths;
pub(crate) use state::AppState;
