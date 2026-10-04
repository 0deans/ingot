//! Account models, decisions and persistence independent of native UI and providers.
mod error;
pub(crate) mod login;
mod models;
mod persistence;
pub(crate) mod policy;
pub(crate) mod repository;
pub(crate) use error::AccountError;
pub use models::{AccountProfile, AccountSecrets};
