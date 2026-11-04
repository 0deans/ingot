pub mod dependencies;
pub mod error;
pub mod file_name;
pub mod identify;
pub mod listing;
pub mod metadata;
pub mod models;
pub mod mutation;
pub mod tracking;
pub mod updates;

pub use error::InstanceContentError;
pub use file_name::ContentFileName;
pub use identify::identify_content;
pub use models::*;
pub use updates::{apply_updates, check_updates};

#[cfg(test)]
mod tests;
