use std::path::Path;

use reqwest::Client;

use super::super::error::InstanceContentError;
use super::super::file_name::ContentFileName;
use super::super::identify::category_scope;
use super::super::listing;
use super::super::models::{
    ContentCategory, ContentUpdateCandidate, ProviderSource, UpdateCheckResult,
};
use super::super::tracking::read_tracking;
use crate::artifacts::sources::{Channel, SourceClient};

pub async fn check_updates(
    instance_dir: &Path,
    category: ContentCategory,
    game_version: &str,
    loader_id: &str,
    http_client: &Client,
) -> Result<UpdateCheckResult, InstanceContentError> {
    let list_res = listing::list_content(instance_dir, category)?;
    let tracking = read_tracking(instance_dir).unwrap_or_default();
    let scope = category_scope(category, loader_id);
    let client = SourceClient::new(http_client);

    let mut updates = Vec::new();
    let mut warnings = Vec::new();

    for item in list_res.items {
        let parsed_name = match ContentFileName::parse(&item.file_name, category) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let origin = match tracking.get(parsed_name.base_name()) {
            Some(o) => o,
            None => continue,
        };

        let target_release = match origin.source {
            ProviderSource::Modrinth => {
                match client
                    .modrinth_target(&origin.project_id, game_version, &scope.modrinth)
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        warnings.push(format!(
                            "Modrinth target query failed for {}: {e}",
                            origin.title
                        ));
                        None
                    }
                }
            }
            ProviderSource::CurseForge => {
                if !scope.curseforge {
                    None
                } else {
                    match client
                        .cf_target(&origin.project_id, game_version, &scope)
                        .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            warnings.push(format!(
                                "CurseForge target query failed for {}: {e}",
                                origin.title
                            ));
                            None
                        }
                    }
                }
            }
        };

        let Some(release) = target_release else {
            continue;
        };

        let is_different_version = release.version_id != origin.version_id
            && release.version_number != origin.version_number;

        let Some(target_file) = release.file else {
            continue;
        };

        let is_different_file = target_file.file_name != parsed_name.base_name();

        if is_different_version || is_different_file {
            let channel_str = match release.channel {
                Channel::Release => "release".to_string(),
                Channel::Beta => "beta".to_string(),
                Channel::Alpha => "alpha".to_string(),
            };

            updates.push(ContentUpdateCandidate {
                file_name: item.file_name,
                current_version: Some(origin.version_number.clone()),
                latest_version: release.version_number,
                latest_version_id: release.version_id,
                title: origin.title.clone(),
                download_url: target_file.url,
                new_file_name: target_file.file_name,
                channel: channel_str,
                source: origin.source,
                sha1: target_file.sha1,
            });
        }
    }

    Ok(UpdateCheckResult { updates, warnings })
}
