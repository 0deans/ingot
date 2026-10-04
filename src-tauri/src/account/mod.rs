//! Account models and decisions independent of native UI, providers, and persistence.
pub(crate) mod login;
mod models;
pub(crate) mod policy;
pub use models::{AccountProfile, AccountSecrets};
