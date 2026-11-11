use std::collections::HashMap;
use std::fs;
use std::path::Path;

use reqwest::Client;

use super::error::InstanceContentError;
use super::file_name::ContentFileName;
use super::listing;
use super::models::{
    ContentCategory, ContentOrigin, IdentifyResult, ProviderSource, TrackedOrigin,
};
use super::tracking::{read_tracking, write_tracking};
use ingot_artifacts::models::sha1_hex;
use ingot_artifacts::sources::{cf_fingerprint, Scope, SourceClient};
use ingot_artifacts::targets::mod_scope;

pub fn category_scope(category: ContentCategory, loader_id: &str) -> Scope {
    match category {
        ContentCategory::Mod => mod_scope(loader_id),
        ContentCategory::ResourcePack => Scope {
            modrinth: vec!["minecraft"],
            curseforge: true,
            ..Default::default()
        },
        ContentCategory::ShaderPack => Scope {
            modrinth: vec!["iris", "optifine", "canvas", "vanilla"],
            curseforge: true,
            ..Default::default()
        },
        ContentCategory::DataPack => Scope {
            modrinth: vec!["datapack", "minecraft"],
            curseforge: true,
            ..Default::default()
        },
    }
}

pub async fn identify_content(
    instance_dir: &Path,
    category: ContentCategory,
    game_version: &str,
    loader_id: &str,
    http_client: &Client,
) -> Result<IdentifyResult, InstanceContentError> {
    let list_res = listing::list_content(instance_dir, category)?;
    let total_count = list_res.items.len() as u32;

    let folder = instance_dir.join(category.default_subfolder());
    let scope = category_scope(category, loader_id);
    let client = SourceClient::new(http_client);

    let mut current_tracking = read_tracking(instance_dir).unwrap_or_default();
    let mut warnings = Vec::new();

    // Collect files that need identification
    let mut unknown_files = Vec::new();
    for item in list_res.items {
        if matches!(item.origin, ContentOrigin::Unknown) {
            let path = folder.join(&item.file_name);
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let sha1 = sha1_hex(&bytes);
            let fingerprint = if scope.curseforge {
                Some(cf_fingerprint(&bytes))
            } else {
                None
            };
            let parsed_name = match ContentFileName::parse(&item.file_name, category) {
                Ok(n) => n,
                Err(_) => continue,
            };
            unknown_files.push((parsed_name, sha1, fingerprint));
        }
    }

    if unknown_files.is_empty() {
        return Ok(IdentifyResult {
            identified_count: 0,
            total_count,
            warnings,
        });
    }

    let mut identified_count = 0;
    let mut unmatched_files = Vec::new();

    // 1. Modrinth SHA-1 identification
    let sha1_list: Vec<String> = unknown_files.iter().map(|(_, s, _)| s.clone()).collect();
    let modrinth_matches = match client
        .modrinth_identify(&sha1_list, game_version, &scope.modrinth)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warnings.push(format!("Modrinth hash identification error: {e}"));
            HashMap::new()
        }
    };

    let mut mr_project_ids = Vec::new();
    for (_, sha1, _) in &unknown_files {
        if let Some((release, _)) = modrinth_matches.get(sha1) {
            mr_project_ids.push(release.project.clone());
        }
    }

    let mr_projects = if !mr_project_ids.is_empty() {
        client
            .modrinth_projects(&mr_project_ids)
            .await
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    for (file_name, sha1, fp) in unknown_files {
        if let Some((release, _)) = modrinth_matches.get(&sha1) {
            let project_info = mr_projects.get(&release.project);
            let title = project_info
                .map(|p| p.title.clone())
                .unwrap_or_else(|| release.project.clone());
            let icon_url = project_info.and_then(|p| p.icon_url.clone());

            current_tracking.insert(
                file_name.base_name().to_string(),
                TrackedOrigin {
                    source: ProviderSource::Modrinth,
                    project_id: release.project.clone(),
                    project_slug: None,
                    version_id: release.version_id.clone(),
                    version_number: release.version_number.clone(),
                    title,
                    icon_url,
                },
            );
            identified_count += 1;
        } else {
            unmatched_files.push((file_name, fp));
        }
    }

    // 2. CurseForge fingerprint identification for remaining files
    if scope.curseforge && !unmatched_files.is_empty() {
        let fp_list: Vec<u32> = unmatched_files.iter().filter_map(|(_, fp)| *fp).collect();
        if !fp_list.is_empty() {
            let cf_matches = match client.cf_identify(&fp_list, game_version, &scope).await {
                Ok(m) => m,
                Err(e) => {
                    warnings.push(format!("CurseForge fingerprint identification error: {e}"));
                    HashMap::new()
                }
            };

            let mut cf_project_ids = Vec::new();
            for (_, fp) in &unmatched_files {
                if let Some(fp_val) = fp {
                    if let Some((release, _)) = cf_matches.get(fp_val) {
                        cf_project_ids.push(release.project.clone());
                    }
                }
            }

            let cf_projects = if !cf_project_ids.is_empty() {
                client
                    .cf_projects(&cf_project_ids)
                    .await
                    .unwrap_or_default()
            } else {
                HashMap::new()
            };

            for (file_name, fp) in unmatched_files {
                if let Some(fp_val) = fp {
                    if let Some((release, _)) = cf_matches.get(&fp_val) {
                        let project_info = cf_projects.get(&release.project);
                        let title = project_info
                            .map(|p| p.title.clone())
                            .unwrap_or_else(|| release.project.clone());
                        let icon_url = project_info.and_then(|p| p.icon_url.clone());

                        current_tracking.insert(
                            file_name.base_name().to_string(),
                            TrackedOrigin {
                                source: ProviderSource::CurseForge,
                                project_id: release.project.clone(),
                                project_slug: None,
                                version_id: release.version_id.clone(),
                                version_number: release.version_number.clone(),
                                title,
                                icon_url,
                            },
                        );
                        identified_count += 1;
                    }
                }
            }
        }
    }

    if identified_count > 0 {
        write_tracking(instance_dir, &current_tracking)?;
    }

    Ok(IdentifyResult {
        identified_count,
        total_count,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_scopes() {
        let m_scope = category_scope(ContentCategory::Mod, "fabric");
        assert!(m_scope.modrinth.contains(&"fabric"));

        let rp_scope = category_scope(ContentCategory::ResourcePack, "fabric");
        assert!(rp_scope.curseforge);
        assert!(rp_scope.modrinth.contains(&"minecraft"));

        let shader_scope = category_scope(ContentCategory::ShaderPack, "fabric");
        assert!(shader_scope.curseforge);
        assert!(shader_scope.modrinth.contains(&"iris"));
    }
}
