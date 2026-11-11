//! Pure ATLauncher filtering, sorting, and unified content mapping rules.
use super::models::{AtLauncherPackDetail, AtLauncherPackSummary};
use crate::content::models::{
    ContentSearchQuery, ContentSearchResult, UnifiedContentDetails, UnifiedContentItem,
    UnifiedContentVersion,
};
use std::cmp::Ordering;

pub(crate) fn compute_atlauncher_safe_name(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

pub(super) fn supports_project_type(project_type: &str) -> bool {
    matches!(
        project_type.trim().to_lowercase().as_str(),
        "all" | "modpack" | "modpacks" | ""
    )
}

pub(super) fn clean_project_id(project_id: &str) -> &str {
    let trimmed = project_id.trim();
    trimmed
        .strip_prefix("atlauncher:")
        .or_else(|| trimmed.strip_prefix("atl:"))
        .unwrap_or(trimmed)
        .trim()
}

fn resolve_summary_safe_name(pack: &AtLauncherPackSummary) -> String {
    pack.safe_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| compute_atlauncher_safe_name(&pack.name))
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.contains(&value) {
        target.push(value);
    }
}

fn matches_search_query(pack: &AtLauncherPackSummary, query: &ContentSearchQuery<'_>) -> bool {
    if pack
        .pack_type
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("private"))
    {
        return false;
    }
    if pack.pack_type.is_some() && pack.versions.as_ref().is_some_and(|vl| vl.is_empty()) {
        return false;
    }

    if let Some(q) = query.query.map(str::trim).filter(|s| !s.is_empty()) {
        let q_lower = q.to_lowercase();
        let name_matches = pack.name.to_lowercase().contains(&q_lower);
        let desc_matches = pack
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&q_lower);
        if !name_matches && !desc_matches {
            return false;
        }
    }

    if let Some(gv) = query
        .game_version
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
    {
        let has_mc = pack.versions.as_ref().is_some_and(|vl| {
            vl.iter().any(|v| {
                v.minecraft
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(gv)
            })
        });
        if !has_mc {
            return false;
        }
    }

    if let Some(ldr) = query
        .loader
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
    {
        let ldr_lower = ldr.to_lowercase();
        let has_ldr = pack.categories.iter().any(|c| c.eq_ignore_ascii_case(ldr))
            || pack
                .system
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&ldr_lower));
        if !has_ldr && !ldr.eq_ignore_ascii_case("forge") {
            return false;
        }
    }

    true
}

fn compare_by_downloads_then_position(
    a: &AtLauncherPackSummary,
    b: &AtLauncherPackSummary,
) -> Ordering {
    match (a.downloads, b.downloads) {
        (Some(ad), Some(bd)) => bd.cmp(&ad),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => match (a.position, b.position) {
            (Some(ap), Some(bp)) => ap.cmp(&bp),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        },
    }
}

fn sort_packs(packs: &mut [AtLauncherPackSummary], sort: Option<&str>) {
    match sort.unwrap_or("downloads") {
        "updated" => {
            packs.sort_by_key(|p| {
                let latest_pub = p
                    .versions
                    .as_ref()
                    .and_then(|v| v.first())
                    .and_then(|v| v.published)
                    .unwrap_or(0);
                std::cmp::Reverse(latest_pub)
            });
        }
        "newest" => {
            packs.sort_by_key(|p| std::cmp::Reverse(p.id.unwrap_or(0)));
        }
        _ => packs.sort_by(compare_by_downloads_then_position),
    }
}

pub(super) fn search_packs(
    packs: Vec<AtLauncherPackSummary>,
    query: &ContentSearchQuery<'_>,
    base_url: &str,
) -> ContentSearchResult {
    let mut filtered: Vec<AtLauncherPackSummary> = packs
        .into_iter()
        .filter(|pack| matches_search_query(pack, query))
        .collect();

    sort_packs(&mut filtered, query.sort);

    let total_hits = filtered.len() as u64;
    let start = (query.offset as usize).min(filtered.len());
    let end = start
        .saturating_add(query.limit as usize)
        .min(filtered.len());
    let items = filtered[start..end]
        .iter()
        .map(|pack| map_pack_summary_to_item(pack, base_url))
        .collect();

    ContentSearchResult {
        items,
        total_hits,
        offset: query.offset,
        limit: query.limit,
    }
}

fn map_pack_summary_to_item(pack: &AtLauncherPackSummary, base_url: &str) -> UnifiedContentItem {
    let safe_name = resolve_summary_safe_name(pack);
    let mut game_versions = Vec::new();
    if let Some(vers) = &pack.versions {
        for v in vers {
            if let Some(mc) = &v.minecraft {
                push_unique(&mut game_versions, mc.clone());
            }
        }
    }

    let latest_version = pack
        .versions
        .as_ref()
        .and_then(|v| v.first())
        .map(|v| v.version.clone());

    let icon_url = Some(format!(
        "{}/launcher/images/{}.png",
        base_url.trim_end_matches('/'),
        safe_name.to_lowercase()
    ));

    let website_url = pack
        .website_url
        .clone()
        .unwrap_or_else(|| format!("https://atlauncher.com/pack/{safe_name}"));

    UnifiedContentItem {
        id: format!("atlauncher:{safe_name}"),
        source: "atlauncher".to_string(),
        project_type: "modpack".to_string(),
        slug: safe_name,
        title: pack.name.clone(),
        description: pack.description.clone().unwrap_or_default(),
        author: "ATLauncher".to_string(),
        icon_url,
        downloads: pack.downloads.unwrap_or(0),
        follows: None,
        categories: pack.categories.clone(),
        latest_version,
        game_versions,
        loaders: vec!["Forge".to_string()],
        website_url: Some(website_url),
        date_modified: None,
    }
}

pub(super) fn find_pack_by_id(
    packs: Vec<AtLauncherPackSummary>,
    clean_id: &str,
) -> Option<AtLauncherPackSummary> {
    packs.into_iter().find(|p| {
        let sname = resolve_summary_safe_name(p);
        sname.eq_ignore_ascii_case(clean_id)
            || p.name.eq_ignore_ascii_case(clean_id)
            || p.id.map(|id| id.to_string()).as_deref() == Some(clean_id)
    })
}

pub(super) fn map_pack_detail_response(
    pack: AtLauncherPackDetail,
    clean_id: &str,
    base_url: &str,
) -> UnifiedContentDetails {
    let safe_name = pack
        .safe_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(clean_id);

    let mut game_versions = Vec::new();
    let mut loaders = Vec::new();
    let base = base_url.trim_end_matches('/');

    let versions: Vec<UnifiedContentVersion> = pack
        .versions
        .unwrap_or_default()
        .into_iter()
        .map(|ver| {
            if let Some(mc) = &ver.minecraft {
                push_unique(&mut game_versions, mc.clone());
            }
            let ldr = ver.loader.unwrap_or_else(|| "Forge".to_string());
            push_unique(&mut loaders, ldr.clone());

            let durl = ver
                .download_url
                .unwrap_or_else(|| format!("{base}/pack/{clean_id}/version/{}", ver.version));
            let fname = format!("atlauncher-{clean_id}-{}.json", ver.version);

            UnifiedContentVersion {
                id: ver.version.clone(),
                name: format!("{} {}", pack.name, ver.version),
                version_number: ver.version,
                game_versions: ver.minecraft.into_iter().collect(),
                loaders: vec![ldr],
                version_type: ver.version_type.unwrap_or_else(|| "release".into()),
                date_published: ver.published.map(|p| p.to_string()).unwrap_or_default(),
                downloads: 0,
                filename: fname,
                download_url: durl,
                size_bytes: 0,
            }
        })
        .collect();

    if loaders.is_empty() {
        loaders.push("Forge".to_string());
    }

    let desc = pack.description.unwrap_or_default();
    let body = pack.body.unwrap_or_else(|| desc.clone());
    let icon_url = Some(format!("{base}/images/{safe_name}.png"));
    let website_url = format!("https://atlauncher.com/pack/{safe_name}");

    UnifiedContentDetails {
        id: format!("atlauncher:{safe_name}"),
        source: "atlauncher".to_string(),
        project_type: "modpack".to_string(),
        slug: safe_name.to_string(),
        title: pack.name,
        description: desc,
        body,
        author: "ATLauncher".to_string(),
        icon_url,
        downloads: pack.downloads.unwrap_or(0),
        follows: None,
        categories: pack.categories,
        game_versions,
        loaders,
        website_url: Some(website_url),
        issues_url: None,
        source_url: None,
        wiki_url: None,
        screenshots: Vec::new(),
        versions,
    }
}

pub(super) fn map_pack_summary_to_details(
    pack: AtLauncherPackSummary,
    base_url: &str,
) -> UnifiedContentDetails {
    let safe_name = resolve_summary_safe_name(&pack);
    let mut game_versions = Vec::new();
    let mut loaders = Vec::new();
    let base = base_url.trim_end_matches('/');

    let versions: Vec<UnifiedContentVersion> = pack
        .versions
        .unwrap_or_default()
        .into_iter()
        .map(|ver| {
            if let Some(mc) = &ver.minecraft {
                push_unique(&mut game_versions, mc.clone());
            }
            let ldr = if ver.has_loader.unwrap_or(true) {
                "Forge".to_string()
            } else {
                "Vanilla".to_string()
            };
            push_unique(&mut loaders, ldr.clone());

            let durl = ver.download_url.unwrap_or_else(|| {
                format!(
                    "{base}/packs/{safe_name}/versions/{}/Configs.json",
                    ver.version
                )
            });
            let fname = format!("atlauncher-{safe_name}-{}.json", ver.version);

            UnifiedContentVersion {
                id: ver.version.clone(),
                name: format!("{} {}", pack.name, ver.version),
                version_number: ver.version,
                game_versions: ver.minecraft.into_iter().collect(),
                loaders: vec![ldr],
                version_type: ver.version_type.unwrap_or_else(|| "release".into()),
                date_published: ver.published.map(|p| p.to_string()).unwrap_or_default(),
                downloads: 0,
                filename: fname,
                download_url: durl,
                size_bytes: 0,
            }
        })
        .collect();

    if loaders.is_empty() {
        loaders.push("Forge".to_string());
    }

    let desc = pack.description.unwrap_or_default();
    let body = desc.clone();
    let icon_url = Some(format!(
        "{base}/launcher/images/{}.png",
        safe_name.to_lowercase()
    ));
    let website_url = pack
        .website_url
        .unwrap_or_else(|| format!("https://atlauncher.com/pack/{safe_name}"));

    UnifiedContentDetails {
        id: format!("atlauncher:{safe_name}"),
        source: "atlauncher".to_string(),
        project_type: "modpack".to_string(),
        slug: safe_name,
        title: pack.name,
        description: desc,
        body,
        author: "ATLauncher".to_string(),
        icon_url,
        downloads: pack.downloads.unwrap_or(0),
        follows: None,
        categories: pack.categories,
        game_versions,
        loaders,
        website_url: Some(website_url),
        issues_url: pack.support_url,
        source_url: None,
        wiki_url: None,
        screenshots: Vec::new(),
        versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::atlauncher::models::{
        AtLauncherDetailVersion, AtLauncherPackListResponse, AtLauncherVersionRef,
    };

    fn sample_summary(id: u64, name: &str) -> AtLauncherPackSummary {
        AtLauncherPackSummary {
            id: Some(id),
            position: Some(id),
            name: name.to_string(),
            pack_type: Some("public".to_string()),
            safe_name: None,
            description: Some(format!("Description for {name}")),
            system: None,
            versions: Some(vec![AtLauncherVersionRef {
                version: "1.0.0".to_string(),
                minecraft: Some("1.20.1".to_string()),
                published: Some(1_700_000_000),
                version_type: None,
                has_loader: Some(true),
                download_url: None,
            }]),
            downloads: Some(100),
            support_url: None,
            website_url: None,
            categories: vec![],
        }
    }

    #[test]
    fn clean_project_id_and_safe_name_rules() {
        assert_eq!(clean_project_id(" atlauncher:Crucial2 "), "Crucial2");
        assert_eq!(clean_project_id("atl: AllTheForge10 "), "AllTheForge10");
        assert_eq!(clean_project_id("  VanillaPlus  "), "VanillaPlus");
        assert_eq!(
            compute_atlauncher_safe_name("All The Forge: 10!"),
            "AllTheForge10"
        );
        assert!(supports_project_type("modpack"));
        assert!(supports_project_type("ALL"));
        assert!(!supports_project_type("mod"));
    }

    #[test]
    fn search_packs_filters_private_empty_versions_and_sorts_by_downloads_or_position() {
        let mut private_pack = sample_summary(1, "Hidden Pack");
        private_pack.pack_type = Some("private".to_string());

        let mut empty_versions_pack = sample_summary(2, "Empty Versions");
        empty_versions_pack.versions = Some(vec![]);

        let mut low = sample_summary(3, "Low Pack");
        low.downloads = None;
        low.position = Some(5);

        let mut high = sample_summary(4, "High Pack");
        high.downloads = None;
        high.position = Some(1);

        let query = ContentSearchQuery {
            project_type: "modpack",
            query: Some("pack"),
            game_version: Some("1.20.1"),
            loader: Some("forge"),
            sort: Some("downloads"),
            offset: 0,
            limit: 10,
        };
        let result = search_packs(
            vec![private_pack, empty_versions_pack, low, high],
            &query,
            "https://cdn.example/",
        );
        assert_eq!(result.total_hits, 2);
        assert_eq!(result.items[0].title, "High Pack");
        assert_eq!(result.items[1].title, "Low Pack");
        assert_eq!(
            result.items[0].icon_url.as_deref(),
            Some("https://cdn.example/launcher/images/highpack.png")
        );
    }

    #[test]
    fn search_packs_respects_loader_filter_when_non_forge() {
        let mut fabric_pack = sample_summary(1, "Fabric Pack");
        fabric_pack.categories = vec!["Fabric".to_string()];

        let forge_only = sample_summary(2, "Forge Pack");

        let query = ContentSearchQuery {
            project_type: "modpack",
            query: None,
            game_version: None,
            loader: Some("fabric"),
            sort: None,
            offset: 0,
            limit: 10,
        };
        let result = search_packs(vec![fabric_pack, forge_only], &query, "https://cdn.example");
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].title, "Fabric Pack");
    }

    #[test]
    fn map_pack_detail_and_summary_preserve_loader_and_url_fallbacks() {
        let detail = AtLauncherPackDetail {
            name: "Detail Pack".to_string(),
            safe_name: Some("   ".to_string()),
            description: Some("Short".to_string()),
            body: None,
            versions: Some(vec![AtLauncherDetailVersion {
                version: "2.0.0".to_string(),
                minecraft: Some("1.21.1".to_string()),
                loader: Some("NeoForge".to_string()),
                published: Some(12345),
                version_type: Some("beta".to_string()),
                download_url: None,
            }]),
            downloads: None,
            categories: vec!["Tech".to_string()],
        };
        let mapped_detail = map_pack_detail_response(detail, "DetailPack", "https://cdn.example/");
        assert_eq!(mapped_detail.slug, "DetailPack");
        assert_eq!(mapped_detail.body, "Short");
        assert_eq!(mapped_detail.loaders, vec!["NeoForge"]);
        assert_eq!(
            mapped_detail.icon_url.as_deref(),
            Some("https://cdn.example/images/DetailPack.png")
        );
        assert_eq!(
            mapped_detail.versions[0].download_url,
            "https://cdn.example/pack/DetailPack/version/2.0.0"
        );

        let mut summary = sample_summary(597, "Vanilla Style");
        summary.versions = Some(vec![AtLauncherVersionRef {
            version: "1.2.3".to_string(),
            minecraft: Some("1.20.4".to_string()),
            published: None,
            version_type: None,
            has_loader: Some(false),
            download_url: None,
        }]);
        summary.support_url = Some("https://example.com/issues".to_string());
        let mapped_summary = map_pack_summary_to_details(summary, "https://cdn.example");
        assert_eq!(mapped_summary.loaders, vec!["Vanilla"]);
        assert_eq!(
            mapped_summary.issues_url.as_deref(),
            Some("https://example.com/issues")
        );
        assert_eq!(
            mapped_summary.versions[0].download_url,
            "https://cdn.example/packs/VanillaStyle/versions/1.2.3/Configs.json"
        );
    }

    #[test]
    fn find_pack_by_id_matches_safe_name_display_name_or_numeric_id() {
        let packs = vec![sample_summary(597, "All The Forge 10")];
        assert!(find_pack_by_id(packs.clone(), "alltheforge10").is_some());
        assert!(find_pack_by_id(packs.clone(), "All The Forge 10").is_some());
        assert!(find_pack_by_id(packs, "597").is_some());
    }

    #[test]
    fn rejects_malformed_pack_list_and_deserializes_string_id() {
        let valid: AtLauncherPackListResponse =
            serde_json::from_str(r#"{"data":[{"id":"42","name":"StringIdPack"}]}"#).unwrap();
        let list = valid.into_vec();
        assert_eq!(list[0].id, Some(42));

        let invalid = serde_json::from_str::<AtLauncherPackListResponse>(r#"{"unexpected":true}"#);
        assert!(invalid.is_err());
    }
}
