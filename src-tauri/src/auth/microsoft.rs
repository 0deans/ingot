//! Microsoft device-code/OAuth and Xbox/Minecraft session protocol.
mod claims;
mod client;
mod clock;
mod error;
mod exchange;
mod grant;
mod models;
mod oauth;
mod skins;
pub use claims::xuid_from_token;
pub(crate) use client::MicrosoftSession;
pub(crate) use error::MicrosoftError;
pub(crate) use grant::MicrosoftGrant;
pub use models::{MicrosoftDeviceCode, MinecraftSession};
pub use skins::{
    set_skin_from_url, upload_skin, MicrosoftSkinClient, MicrosoftSkinError,
    DEFAULT_MINECRAFT_SERVICES_BASE,
};

#[cfg(test)]
mod tests;
