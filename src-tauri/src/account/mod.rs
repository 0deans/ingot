//! Account models, decisions and persistence independent of native UI and providers.
pub(crate) mod credentials;
mod error;
pub(crate) mod login;
mod models;
pub(crate) mod operations;
pub(crate) mod pending_login;
mod persistence;
pub(crate) mod policy;
pub(crate) mod provider;
pub(crate) mod recovery;
pub(crate) mod repository;
pub(crate) mod session;
pub(crate) use error::AccountError;
pub use models::{AccountProfile, AccountSecrets};
