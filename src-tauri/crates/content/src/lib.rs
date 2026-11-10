#[allow(unused_imports)]
pub(crate) use ingot_artifacts as artifacts;
pub(crate) use ingot_core as core;
#[allow(unused_imports)]
pub(crate) use ingot_settings as settings;

pub mod minecraft {
    pub use crate::content;
    pub use crate::importer;
    pub use crate::instance_export;
    pub use ingot_minecraft::*;
}

pub mod content;
pub mod importer;
pub mod instance_export;

pub use content::{
    AtLauncherClient, ContentError, ContentScreenshot, ContentSearchQuery, ContentSearchResult,
    CurseForgeClient, FtbClient, ModrinthClient, TechnicClient, UnifiedContentDetails,
    UnifiedContentItem, UnifiedContentVersion, CURSEFORGE_API_KEY,
};
pub use importer::{
    DetectedLauncher, ImportError, ImportInstanceOptions, ImportReport, ImportableInstance,
};
pub use instance_export::{
    export_instance, preview_export, ContentResolver, ExportOptions, ExportPreview, ExportReport,
    InstanceExportError, SourceClientResolver,
};
