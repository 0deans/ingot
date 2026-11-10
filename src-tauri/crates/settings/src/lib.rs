//! Launcher settings: validated values, persistence, and feature operations.

pub mod error;
pub mod models;
pub mod repository;
pub mod service;

pub use error::SettingsError;
pub use models::{
    LauncherBehavior, LauncherSettings, MemorySettings, SyncSettings, WindowSettings,
};
pub use repository::SettingsRepository;

#[cfg(test)]
mod tests;
