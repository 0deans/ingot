//! Minecraft external launcher discovery and instance import coordinator.
pub mod discovery;
pub mod execute;
pub(crate) mod formats;
pub mod models;
#[cfg(test)]
mod tests;
pub(crate) mod version_id;

pub use models::{
    DetectedLauncher, ImportError, ImportInstanceOptions, ImportReport, ImportableInstance,
};
