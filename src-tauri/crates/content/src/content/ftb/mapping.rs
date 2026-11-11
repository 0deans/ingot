//! Pure FTB target extraction, filtering, and unified content mapping rules.
use super::models::{
    FtbArtItem, FtbAuthorItem, FtbPackDetail, FtbTagItem, FtbTarget, FtbVersionSummary,
};
use crate::content::models::{
    ContentScreenshot, ContentSearchQuery, UnifiedContentDetails, UnifiedContentItem,
    UnifiedContentVersion,
};

pub(super) fn supports_project_type(project_type: &str) -> bool {
    matches!(
        project_type.trim().to_lowercase().as_str(),
        "all" | "modpack" | "modpacks" | ""
    )
}

pub(super) fn clean_project_id(project_id: &str) -> &str {
    let trimmed = project_id.trim();
    trimmed.strip_prefix("ftb:").unwrap_or(trimmed).trim()
}

pub(super) fn slice_page_ids(pack_ids: &[u64], offset: u32, limit: u32) -> &[u64] {
    let start = (offset as usize).min(pack_ids.len());
    let end = start.saturating_add(limit as usize).min(pack_ids.len());
    &pack_ids[start..end]
}

fn select_icon_url(art: &[FtbArtItem]) -> Option<String> {
    art.iter()
        .find(|a| a.r#type == "square")
        .or_else(|| art.first())
        .map(|a| a.url.clone())
}

fn primary_author(authors: &[FtbAuthorItem]) -> String {
    authors
        .first()
        .map(|a| a.name().to_string())
        .unwrap_or_else(|| "FTB Team".to_string())
}

fn collect_categories(tags: &[FtbTagItem]) -> Vec<String> {
    tags.iter().map(|t| t.name().to_string()).collect()
}

fn resolve_slug(slug: Option<String>, pack_id: u64) -> String {
    slug.filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| pack_id.to_string())
}

fn pack_website_url(pack_id: u64) -> String {
    format!("https://www.feed-the-beast.com/modpacks/{pack_id}")
}

pub(super) fn matches_search_filters(
    item: &UnifiedContentItem,
    query: &ContentSearchQuery<'_>,
) -> bool {
    if let Some(gv) = query
        .game_version
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
    {
        if !item
            .game_versions
            .iter()
            .any(|v| v.eq_ignore_ascii_case(gv))
        {
            return false;
        }
    }

    if let Some(ldr) = query
        .loader
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
    {
        if !item.loaders.iter().any(|l| l.eq_ignore_ascii_case(ldr)) {
            return false;
        }
    }

    true
}

pub(super) fn map_pack_item(pack: FtbPackDetail) -> UnifiedContentItem {
    let icon_url = select_icon_url(&pack.art);
    let author = primary_author(&pack.authors);
    let categories = collect_categories(&pack.tags);
    let (game_versions, loaders) = extract_targets(&pack.targets, &pack.versions);
    let latest_version = pack.versions.last().map(|v| v.name.clone());
    let slug = resolve_slug(pack.slug, pack.id);
    let website_url = pack_website_url(pack.id);

    UnifiedContentItem {
        id: format!("ftb:{}", pack.id),
        source: "ftb".to_string(),
        project_type: "modpack".to_string(),
        slug,
        title: pack.name,
        description: pack.synopsis.unwrap_or_default(),
        author,
        icon_url,
        downloads: pack.installs.unwrap_or(0),
        follows: pack.plays,
        categories,
        latest_version,
        game_versions,
        loaders,
        website_url: Some(website_url),
        date_modified: None,
    }
}

pub(super) fn map_pack_details(pack: FtbPackDetail, base_url: &str) -> UnifiedContentDetails {
    let (pack_game_versions, pack_loaders) = extract_targets(&pack.targets, &pack.versions);
    let icon_url = select_icon_url(&pack.art);
    let author = primary_author(&pack.authors);

    let screenshots: Vec<ContentScreenshot> = pack
        .art
        .iter()
        .filter(|a| a.r#type == "screenshot" || a.r#type == "gallery")
        .map(|a| ContentScreenshot {
            url: a.url.clone(),
            title: None,
        })
        .collect();

    let base = base_url.trim_end_matches('/');
    let pack_id = pack.id;
    let versions: Vec<UnifiedContentVersion> = pack
        .versions
        .into_iter()
        .rev()
        .map(|ver| {
            let (ver_games, ver_loaders) = extract_targets(&ver.targets, &[]);
            let game_versions = if ver_games.is_empty() {
                pack_game_versions.clone()
            } else {
                ver_games
            };
            let loaders = if ver_loaders.is_empty() {
                pack_loaders.clone()
            } else {
                ver_loaders
            };
            UnifiedContentVersion {
                id: ver.id.to_string(),
                name: ver.name.clone(),
                version_number: ver.name,
                game_versions,
                loaders,
                version_type: ver.r#type.unwrap_or_else(|| "release".into()),
                date_published: ver.updated.map(|u| u.to_string()).unwrap_or_default(),
                downloads: 0,
                filename: format!("ftb-{}-{}.json", pack_id, ver.id),
                download_url: format!("{base}/modpack/{pack_id}/{}", ver.id),
                size_bytes: 0,
            }
        })
        .collect();

    let slug = resolve_slug(pack.slug, pack.id);
    let synopsis = pack.synopsis.unwrap_or_default();
    let body = pack.description.unwrap_or_else(|| synopsis.clone());
    let categories = collect_categories(&pack.tags);
    let website_url = pack_website_url(pack.id);

    UnifiedContentDetails {
        id: format!("ftb:{}", pack.id),
        source: "ftb".to_string(),
        project_type: "modpack".to_string(),
        slug,
        title: pack.name,
        description: synopsis,
        body,
        author,
        icon_url,
        downloads: pack.installs.unwrap_or(0),
        follows: pack.plays,
        categories,
        game_versions: pack_game_versions,
        loaders: pack_loaders,
        website_url: Some(website_url),
        issues_url: None,
        source_url: None,
        wiki_url: None,
        screenshots,
        versions,
    }
}

pub(super) fn extract_targets(
    targets: &[FtbTarget],
    versions: &[FtbVersionSummary],
) -> (Vec<String>, Vec<String>) {
    let mut game_versions = Vec::new();
    let mut loaders = Vec::new();

    for t in targets {
        let t_type = t.r#type.to_lowercase();
        let t_name = t.name.to_lowercase();
        if t_type == "game" || t_name == "minecraft" {
            if let Some(v) = &t.version {
                if !game_versions.contains(v) {
                    game_versions.push(v.clone());
                }
            }
        } else if t_type == "modloader" {
            let ldr_name = match t_name.as_str() {
                "forge" => "Forge",
                "neoforge" => "NeoForge",
                "fabric" => "Fabric",
                "quilt" => "Quilt",
                _ => t.name.as_str(),
            }
            .to_string();
            if !loaders.contains(&ldr_name) {
                loaders.push(ldr_name);
            }
        }
    }

    if game_versions.is_empty() || loaders.is_empty() {
        for v in versions {
            let (vg, vl) = extract_targets(&v.targets, &[]);
            for g in vg {
                if !game_versions.contains(&g) {
                    game_versions.push(g);
                }
            }
            for l in vl {
                if !loaders.contains(&l) {
                    loaders.push(l);
                }
            }
        }
    }

    (game_versions, loaders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ftb::models::FtbPackListResponse;

    fn sample_pack() -> FtbPackDetail {
        FtbPackDetail {
            id: 101,
            name: "Stoneblock 3".to_string(),
            slug: Some(" ".to_string()),
            synopsis: Some("Short synopsis".to_string()),
            description: None,
            art: vec![
                FtbArtItem {
                    r#type: "gallery".to_string(),
                    url: "https://example.com/gal.png".to_string(),
                },
                FtbArtItem {
                    r#type: "square".to_string(),
                    url: "https://example.com/sq.png".to_string(),
                },
            ],
            authors: vec![FtbAuthorItem::Str("AuthorStr".to_string())],
            tags: vec![FtbTagItem::Str("Tech".to_string())],
            targets: vec![],
            versions: vec![
                FtbVersionSummary {
                    id: 1001,
                    name: "1.0.0".to_string(),
                    r#type: None,
                    updated: Some(1_660_000_000),
                    targets: vec![
                        FtbTarget {
                            name: "minecraft".to_string(),
                            r#type: "game".to_string(),
                            version: Some("1.18.2".to_string()),
                        },
                        FtbTarget {
                            name: "neoforge".to_string(),
                            r#type: "modloader".to_string(),
                            version: Some("20.4.0".to_string()),
                        },
                    ],
                },
                FtbVersionSummary {
                    id: 1002,
                    name: "1.1.0".to_string(),
                    r#type: Some("beta".to_string()),
                    updated: None,
                    targets: vec![],
                },
            ],
            installs: Some(500),
            plays: Some(900),
        }
    }

    #[test]
    fn clean_project_id_and_pagination_slice_work() {
        assert_eq!(clean_project_id(" ftb:101 "), "101");
        assert_eq!(clean_project_id("  101  "), "101");
        assert!(supports_project_type("modpack"));
        assert!(!supports_project_type("shader"));
        assert_eq!(slice_page_ids(&[1, 2, 3, 4], 1, 2), &[2, 3]);
        assert_eq!(slice_page_ids(&[1, 2], 5, 10), &[] as &[u64]);
    }

    #[test]
    fn map_pack_item_and_details_fall_back_to_version_targets_and_normalize_loaders() {
        let item = map_pack_item(sample_pack());
        assert_eq!(item.slug, "101");
        assert_eq!(item.author, "AuthorStr");
        assert_eq!(item.icon_url.as_deref(), Some("https://example.com/sq.png"));
        assert_eq!(item.latest_version.as_deref(), Some("1.1.0"));
        assert_eq!(item.game_versions, vec!["1.18.2"]);
        assert_eq!(item.loaders, vec!["NeoForge"]);

        let details = map_pack_details(sample_pack(), "https://api.example/");
        assert_eq!(details.body, "Short synopsis");
        assert_eq!(details.screenshots.len(), 1);
        assert_eq!(details.screenshots[0].url, "https://example.com/gal.png");
        assert_eq!(details.versions.len(), 2);
        // Versions are reversed (newest first) and inherit pack targets when empty.
        assert_eq!(details.versions[0].id, "1002");
        assert_eq!(details.versions[0].version_type, "beta");
        assert_eq!(details.versions[0].game_versions, vec!["1.18.2"]);
        assert_eq!(details.versions[0].loaders, vec!["NeoForge"]);
        assert_eq!(
            details.versions[0].download_url,
            "https://api.example/modpack/101/1002"
        );
    }

    #[test]
    fn matches_search_filters_handles_case_insensitivity_and_wildcards() {
        let item = map_pack_item(sample_pack());
        let matching = ContentSearchQuery {
            project_type: "modpack",
            query: None,
            game_version: Some("1.18.2"),
            loader: Some("neoforge"),
            sort: None,
            offset: 0,
            limit: 10,
        };
        assert!(matches_search_filters(&item, &matching));

        let mismatch_loader = ContentSearchQuery {
            loader: Some("fabric"),
            ..matching
        };
        assert!(!matches_search_filters(&item, &mismatch_loader));

        let wildcard = ContentSearchQuery {
            game_version: Some("all"),
            loader: Some(""),
            ..matching
        };
        assert!(matches_search_filters(&item, &wildcard));
    }

    #[test]
    fn rejects_malformed_pack_list_without_packs_field() {
        let valid_array: FtbPackListResponse = serde_json::from_str(r#"["10", 20]"#).unwrap();
        assert_eq!(valid_array.into_ids(), vec![10, 20]);

        let missing_packs =
            serde_json::from_str::<FtbPackListResponse>(r#"{"status":"error","message":"bad"}"#);
        assert!(missing_packs.is_err());

        let invalid_id = serde_json::from_str::<FtbPackListResponse>(r#"{"packs":["abc"]}"#);
        assert!(invalid_id.is_err());
    }
}
