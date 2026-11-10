pub mod account;
pub mod auth;
pub mod keyring_store;

pub use account::*;
pub use auth::*;
pub use keyring_store::*;

pub const USER_AGENT: &str = concat!(
    "Ingot-Launcher/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/0deans/ingot)"
);
