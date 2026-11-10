//! Durable discovery and roll-forward recovery for account publication.
mod error;
mod intent;
mod service;
pub use error::RecoveryError;
pub use intent::ProfileChange;
pub use service::AccountRecovery;
