//! Minecraft external launcher discovery and instance import coordinator.
pub(crate) mod discovery;
pub(crate) mod execute;
pub(crate) mod formats;
pub(crate) mod models;
#[cfg(test)]
mod tests;
pub(crate) mod version_id;

pub(crate) use models::ImportError;
pub use models::{DetectedLauncher, ImportInstanceOptions, ImportReport, ImportableInstance};
