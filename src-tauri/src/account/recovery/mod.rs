//! Durable discovery and roll-forward recovery for account publication.
mod error;
mod intent;
mod service;
pub(crate) use error::RecoveryError;
pub(crate) use intent::ProfileChange;
pub(crate) use service::AccountRecovery;
