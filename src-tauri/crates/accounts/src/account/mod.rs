//! Account models, decisions and persistence independent of native UI and providers.
pub mod credentials;
pub mod error;
pub mod login;
pub mod models;
pub mod operations;
pub mod pending_login;
mod persistence;
pub mod policy;
pub mod provider;
pub mod recovery;
pub mod repository;
pub mod session;
pub mod validation;

pub use error::AccountError;
pub use models::{AccountProfile, AccountSecrets};
