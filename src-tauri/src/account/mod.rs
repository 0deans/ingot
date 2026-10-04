//! Account models, decisions and persistence independent of native UI and providers.
pub(crate) mod credentials;
mod error;
pub(crate) mod login;
mod models;
pub(crate) mod operations;
mod persistence;
pub(crate) mod policy;
pub(crate) mod repository;
pub(crate) use error::AccountError;
pub use models::{AccountProfile, AccountSecrets};
