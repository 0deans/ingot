//! Runtime routing and headless exports use the same API registration list.
pub mod error;
pub use error::ApiError;

mod accounts;
mod application;
mod bedrock;
mod content;
pub(crate) mod events;
mod instances;
#[cfg(test)]
mod metadata;
mod servers;
mod settings;
mod skins;
mod versions;

use accounts::{AccountsApi, AccountsApiImpl};
use application::{ApplicationApi, ApplicationApiImpl};
use bedrock::{BedrockApi, BedrockApiImpl};
use content::{ContentApi, ContentApiImpl};
use events::{EventsApi, EventsApiImpl};
use instances::{
    imports::{InstanceImportsApi, InstanceImportsApiImpl},
    screenshots::{InstanceScreenshotsApi, InstanceScreenshotsApiImpl},
    sync::{InstanceSyncApi, InstanceSyncApiImpl},
    version_change::{InstanceVersionChangeApi, InstanceVersionChangeApiImpl},
    InstancesApi, InstancesApiImpl,
};
use servers::{
    config::{ServerConfigApi, ServerConfigApiImpl},
    map::{ServerMapApi, ServerMapApiImpl},
    metrics::{ServerMetricsApi, ServerMetricsApiImpl},
    packs::{ServerPacksApi, ServerPacksApiImpl},
    players::{ServerPlayersApi, ServerPlayersApiImpl},
    plugins::{ServerPluginsApi, ServerPluginsApiImpl},
    transfer::{ServerTransferApi, ServerTransferApiImpl},
    tunnel::{ServerTunnelApi, ServerTunnelApiImpl},
    version_change::{ServerVersionChangeApi, ServerVersionChangeApiImpl},
    ServersApi, ServersApiImpl,
};
use settings::{SettingsApi, SettingsApiImpl};
use skins::{SkinsApi, SkinsApiImpl};
use versions::{VersionsApi, VersionsApiImpl};

// Keep registrations in one place so exports cannot silently omit a namespace.
macro_rules! registered_apis {
    ($register:ident) => {
        $register!(ApplicationApiImpl);
        $register!(AccountsApiImpl);
        $register!(SkinsApiImpl);
        $register!(SettingsApiImpl);
        $register!(EventsApiImpl);
        $register!(InstancesApiImpl);
        $register!(InstanceImportsApiImpl);
        $register!(InstanceScreenshotsApiImpl);
        $register!(InstanceSyncApiImpl);
        $register!(InstanceVersionChangeApiImpl);
        $register!(ServersApiImpl);
        $register!(ServerConfigApiImpl);
        $register!(ServerPlayersApiImpl);
        $register!(ServerPluginsApiImpl);
        $register!(ServerPacksApiImpl);
        $register!(ServerMapApiImpl);
        $register!(ServerMetricsApiImpl);
        $register!(ServerTransferApiImpl);
        $register!(ServerTunnelApiImpl);
        $register!(ServerVersionChangeApiImpl);
        $register!(VersionsApiImpl);
        $register!(ContentApiImpl);
        $register!(BedrockApiImpl);
    };
}

pub fn router<R: tauri::Runtime>() -> taurpc::Router<R> {
    let mut router = taurpc::Router::<R>::new();
    macro_rules! merge {
        ($api:path) => {
            router = router.merge($api.into_handler());
        };
    }
    registered_apis!(merge);
    router
}

/// Collect the runtime APIs' metadata without spawning handlers or native UI.
#[cfg(test)]
pub(crate) fn contract() -> impl taurpc::Exportable {
    let mut metadata = metadata::ApiMetadata::default();
    macro_rules! register {
        ($api:path) => {
            metadata.register::<tauri::Wry, _>($api.into_handler());
        };
    }
    registered_apis!(register);
    metadata
}

#[cfg(test)]
mod tests;
