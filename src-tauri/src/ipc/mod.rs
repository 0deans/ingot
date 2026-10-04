//! Runtime routing and headless exports use the same API registration list.
mod application;
pub(crate) mod events;
mod legacy;
#[cfg(test)]
mod metadata;
mod settings;

use application::{ApplicationApi, ApplicationApiImpl};
use events::{EventsApi, EventsApiImpl};
use legacy::{AppApi, AppApiImpl};
use settings::{SettingsApi, SettingsApiImpl};

// Keep registrations in one place so exports cannot silently omit a namespace.
macro_rules! registered_apis {
    ($register:ident) => {
        $register!(AppApiImpl);
        $register!(ApplicationApiImpl);
        $register!(SettingsApiImpl);
        $register!(EventsApiImpl);
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
