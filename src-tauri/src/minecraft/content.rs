//! Unified Minecraft content discovery, details retrieval, and installation coordinator.

pub(crate) mod atlauncher;
pub(crate) mod curseforge;
pub(crate) mod error;
pub(crate) mod ftb;
pub(crate) mod install;
pub(crate) mod models;
pub(crate) mod modrinth;
pub(crate) mod technic;
#[cfg(test)]
mod tests;

pub use atlauncher::AtLauncherClient;
pub use curseforge::{CurseForgeClient, CURSEFORGE_API_KEY};
pub use error::ContentError;
pub use ftb::FtbClient;
pub use models::{
    ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
    UnifiedContentItem, UnifiedContentVersion,
};
pub use modrinth::ModrinthClient;
pub use technic::TechnicClient;

pub(crate) mod service;

/// Extracts the content source prefix and clean project identifier.
pub(crate) fn parse_content_source_and_id<'a>(
    source: &'a str,
    project_id: &'a str,
) -> (&'a str, &'a str) {
    let trimmed_id = project_id.trim();
    if let Some(stripped) = trimmed_id.strip_prefix("mr:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("cf:") {
        ("curseforge", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("modrinth:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("curseforge:") {
        ("curseforge", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("ftb:") {
        ("ftb", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("technic:") {
        ("technic", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("atlauncher:") {
        ("atlauncher", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("atl:") {
        ("atlauncher", stripped)
    } else if !source.trim().is_empty() {
        (source.trim(), trimmed_id)
    } else {
        ("", trimmed_id)
    }
}
