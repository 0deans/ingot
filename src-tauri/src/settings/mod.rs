//! Launcher settings: validated values, persistence, and feature operations.
//!
//! Tauri access remains in `system`; these modules can be tested without the app.

mod error;
mod models;
pub(crate) mod repository;
pub(crate) mod service;

pub use error::SettingsError;
pub use models::{
    LauncherBehavior, LauncherSettings, MemorySettings, SyncSettings, WindowSettings,
};

#[cfg(test)]
mod tests;
