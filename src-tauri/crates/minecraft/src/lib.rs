pub(crate) use self as minecraft;
pub(crate) use ingot_accounts as account;
#[allow(unused_imports)]
pub(crate) use ingot_accounts as accounts;
pub(crate) use ingot_artifacts as artifacts;
pub(crate) use ingot_core as core;
pub(crate) use ingot_settings as settings;

pub mod bedrock;
pub mod content;
pub mod horizon;
pub mod importer;
pub mod instance;
pub mod instance_content;
pub mod instance_export;
pub mod launcher;
pub mod loader;
pub mod logs;
pub mod news;
pub mod screenshots;
pub mod sync;
pub mod version;
pub mod worlds;

pub use artifacts::downloader;
pub use artifacts::java;
pub use core::metadata;
