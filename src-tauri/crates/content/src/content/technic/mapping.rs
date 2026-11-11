//! Pure Technic slug parsing and unified content mapping rules.
use super::models::{SolderPackResponse, TechnicPackDetail, TechnicSearchEntry};
use crate::content::{
    error::ContentError,
    models::{ContentScreenshot, UnifiedContentDetails, UnifiedContentItem, UnifiedContentVersion},
};

/// Extracts the modpack slug segment from a Technic Platform URL, if present.
pub(crate) fn extract_technic_slug_from_query(q: &str) -> Option<&str> {
    let lower = q.to_lowercase();
    let marker = "technicpack.net/modpack/";
    let idx = lower.find(marker)?;
    let after = &q[idx + marker.len()..];
    let slug = after.split(['/', '?', '#', '&']).next()?.trim();
    if slug.is_empty() {
        return None;
    }
    if let Some((base, suffix)) = slug.rsplit_once('.') {
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return Some(base);
        }
    }
    Some(slug)
}

pub(super) fn supports_project_type(project_type: &str) -> bool {
    matches!(
        project_type.trim().to_lowercase().as_str(),
        "all" | "modpack" | "modpacks" | ""
    )
}

pub(super) fn clean_pack_slug(project_id: &str) -> &str {
    extract_technic_slug_from_query(project_id).unwrap_or_else(|| {
        let trimmed = project_id.trim();
        trimmed.strip_prefix("technic:").unwrap_or(trimmed).trim()
    })
}

pub(super) fn resolve_entry_slug(entry: &TechnicSearchEntry) -> String {
    entry
        .slug
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&entry.name)
        .to_string()
}

pub(super) fn slice_page_entries(
    entries: &[TechnicSearchEntry],
    offset: u32,
    limit: u32,
) -> &[TechnicSearchEntry] {
    let start = (offset as usize).min(entries.len());
    let end = start.saturating_add(limit as usize).min(entries.len());
    &entries[start..end]
}

fn entry_icon_url(entry: &TechnicSearchEntry) -> Option<String> {
    entry
        .icon_url
        .clone()
        .or_else(|| entry.icon.as_ref().and_then(|i| i.to_url()))
        .or_else(|| entry.logo.as_ref().and_then(|l| l.to_url()))
}

pub(super) fn map_hydrated_entry_to_item(
    entry: TechnicSearchEntry,
    real_slug: String,
    detail_opt: Option<TechnicPackDetail>,
) -> UnifiedContentItem {
    if let Some(detail) = detail_opt {
        let icon_url = detail
            .icon
            .as_ref()
            .and_then(|i| i.to_url())
            .or_else(|| detail.logo.as_ref().and_then(|l| l.to_url()))
            .or_else(|| entry_icon_url(&entry));

        let title = detail.display_name.unwrap_or_else(|| detail.name.clone());
        let website_url = detail
            .platform_url
            .unwrap_or_else(|| format!("https://www.technicpack.net/modpack/{real_slug}"));

        UnifiedContentItem {
            id: format!("technic:{real_slug}"),
            source: "technic".to_string(),
            project_type: "modpack".to_string(),
            slug: real_slug,
            title,
            description: detail.description.unwrap_or_default(),
            author: detail.user.unwrap_or_else(|| "Technic".to_string()),
            icon_url,
            downloads: detail.downloads.unwrap_or(0),
            follows: detail.runs,
            categories: Vec::new(),
            latest_version: detail.version,
            game_versions: detail.minecraft.into_iter().collect(),
            loaders: vec!["Forge".to_string()],
            website_url: Some(website_url),
            date_modified: None,
        }
    } else {
        let icon_url = entry_icon_url(&entry);
        let title = entry.display_name.unwrap_or(entry.name);
        let website_url = format!("https://www.technicpack.net/modpack/{real_slug}");

        UnifiedContentItem {
            id: format!("technic:{real_slug}"),
            source: "technic".to_string(),
            project_type: "modpack".to_string(),
            slug: real_slug,
            title,
            description: entry.description.unwrap_or_default(),
            author: entry.user.unwrap_or_else(|| "Technic".to_string()),
            icon_url,
            downloads: entry.downloads.unwrap_or(0),
            follows: entry.runs,
            categories: Vec::new(),
            latest_version: None,
            game_versions: Vec::new(),
            loaders: vec!["Forge".to_string()],
            website_url: Some(website_url),
            date_modified: None,
        }
    }
}

pub(super) fn map_pack_details(
    pack: TechnicPackDetail,
    solder_data: Option<SolderPackResponse>,
) -> Result<UnifiedContentDetails, ContentError> {
    let mut versions = Vec::new();
    let display_title = pack.display_name.as_deref().unwrap_or(&pack.name);

    if let (Some(solder_url), Some(solder)) = (
        pack.solder
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        solder_data,
    ) {
        let solder_base = solder_url.trim_end_matches('/');
        for build in solder.builds {
            let is_recommended = solder.recommended.as_deref() == Some(&build);
            let vtype = if is_recommended { "release" } else { "beta" };
            let durl = format!("{solder_base}/modpack/{}/{build}?include=mods", pack.name);
            let fname = format!("{}-{build}-solder.json", pack.name);

            versions.push(UnifiedContentVersion {
                id: build.clone(),
                name: format!("{display_title} {build}"),
                version_number: build,
                game_versions: pack.minecraft.clone().into_iter().collect(),
                loaders: vec!["Forge".to_string()],
                version_type: vtype.to_string(),
                date_published: String::new(),
                downloads: 0,
                filename: fname,
                download_url: durl,
                size_bytes: 0,
            });
        }
    }

    if let Some(direct_url) = pack.url.as_deref().filter(|s| !s.trim().is_empty()) {
        let ver = pack.version.clone().unwrap_or_else(|| "1.0.0".to_string());
        let name = format!("{display_title} {ver}");
        let mc_part = pack
            .minecraft
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|m| format!("-mc{m}"))
            .unwrap_or_default();
        let fname = format!("{}-{ver}{mc_part}.zip", pack.name);
        versions.push(UnifiedContentVersion {
            id: ver.clone(),
            name,
            version_number: ver,
            game_versions: pack.minecraft.clone().into_iter().collect(),
            loaders: vec!["Forge".to_string()],
            version_type: "release".to_string(),
            date_published: String::new(),
            downloads: 0,
            filename: fname,
            download_url: direct_url.to_string(),
            size_bytes: 0,
        });
    }

    if versions.is_empty() {
        return Err(ContentError::MissingField(
            "modpack download URL or Solder build",
        ));
    }

    let icon_url = pack
        .icon
        .as_ref()
        .and_then(|i| i.to_url())
        .or_else(|| pack.logo.as_ref().and_then(|l| l.to_url()));

    let screenshots = pack
        .background
        .as_ref()
        .and_then(|b| b.to_url())
        .map(|url| vec![ContentScreenshot { url, title: None }])
        .unwrap_or_default();

    let desc = pack.description.unwrap_or_default();
    let body = pack.overview.unwrap_or_else(|| desc.clone());
    let title = pack.display_name.unwrap_or_else(|| pack.name.clone());

    Ok(UnifiedContentDetails {
        id: format!("technic:{}", pack.name),
        source: "technic".to_string(),
        project_type: "modpack".to_string(),
        slug: pack.name.clone(),
        title,
        description: desc,
        body,
        author: pack.user.unwrap_or_else(|| "Technic".to_string()),
        icon_url,
        downloads: pack.downloads.unwrap_or(0),
        follows: pack.runs,
        categories: Vec::new(),
        game_versions: pack.minecraft.into_iter().collect(),
        loaders: vec!["Forge".to_string()],
        website_url: Some(format!("https://www.technicpack.net/modpack/{}", pack.name)),
        issues_url: None,
        source_url: None,
        wiki_url: None,
        screenshots,
        versions,
    })
}

pub(super) fn details_to_item(details: UnifiedContentDetails) -> UnifiedContentItem {
    UnifiedContentItem {
        id: details.id,
        source: details.source,
        project_type: details.project_type,
        slug: details.slug,
        title: details.title,
        description: details.description,
        author: details.author,
        icon_url: details.icon_url,
        downloads: details.downloads,
        follows: details.follows,
        categories: details.categories,
        latest_version: details.versions.first().map(|v| v.version_number.clone()),
        game_versions: details.game_versions,
        loaders: details.loaders,
        website_url: details.website_url,
        date_modified: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::technic::models::{TechnicImage, TechnicSearchResponse};

    fn base_detail() -> TechnicPackDetail {
        TechnicPackDetail {
            name: "tekkit-main".to_string(),
            display_name: Some("Tekkit Main".to_string()),
            platform_url: None,
            user: Some("TechnicDev".to_string()),
            description: Some("Short desc".to_string()),
            overview: Some("Long overview".to_string()),
            url: None,
            solder: None,
            version: None,
            minecraft: Some("1.6.4".to_string()),
            icon: Some(TechnicImage::Str(
                "https://example.com/icon.png".to_string(),
            )),
            logo: None,
            background: Some(TechnicImage::Obj {
                url: Some("https://example.com/bg.png".to_string()),
            }),
            downloads: Some(1234),
            runs: Some(5678),
        }
    }

    #[test]
    fn extract_slug_and_clean_pack_slug_handle_urls_and_prefixes() {
        assert_eq!(
            extract_technic_slug_from_query(
                "https://www.technicpack.net/modpack/tekkit-main.552547?build=ingot"
            ),
            Some("tekkit-main")
        );
        assert_eq!(
            extract_technic_slug_from_query("https://www.technicpack.net/modpack/plain-slug/mods"),
            Some("plain-slug")
        );
        assert_eq!(
            extract_technic_slug_from_query("https://www.technicpack.net/modpack/"),
            None
        );
        assert_eq!(clean_pack_slug(" technic:my-pack "), "my-pack");
    }

    #[test]
    fn map_pack_details_supports_solder_recommended_vs_beta_and_rejects_missing_versions() {
        let mut solder_pack = base_detail();
        solder_pack.solder = Some("https://solder.example.com/api/".to_string());
        let solder = SolderPackResponse {
            recommended: Some("1.2.9f".to_string()),
            builds: vec!["1.2.9e".to_string(), "1.2.9f".to_string()],
        };
        let details = map_pack_details(solder_pack, Some(solder)).unwrap();
        assert_eq!(details.versions.len(), 2);
        assert_eq!(details.versions[0].version_type, "beta");
        assert_eq!(details.versions[1].version_type, "release");
        assert_eq!(
            details.versions[1].download_url,
            "https://solder.example.com/api/modpack/tekkit-main/1.2.9f?include=mods"
        );
        assert_eq!(details.screenshots.len(), 1);
        assert_eq!(details.screenshots[0].url, "https://example.com/bg.png");

        let empty_pack = base_detail();
        let err = map_pack_details(empty_pack, None).unwrap_err();
        assert!(matches!(
            err,
            ContentError::MissingField("modpack download URL or Solder build")
        ));
    }

    #[test]
    fn map_hydrated_entry_prefers_detail_fields_and_falls_back_to_entry_icon() {
        let entry = TechnicSearchEntry {
            name: "Entry Name".to_string(),
            slug: Some("entry-slug".to_string()),
            display_name: None,
            user: None,
            description: None,
            icon_url: Some("https://example.com/entry-icon.png".to_string()),
            icon: None,
            logo: None,
            downloads: Some(10),
            runs: None,
        };
        let mut detail = base_detail();
        detail.icon = None;
        detail.logo = None;
        let slug = resolve_entry_slug(&entry);
        assert_eq!(slug, "entry-slug");

        let item = map_hydrated_entry_to_item(entry, slug, Some(detail));
        assert_eq!(item.title, "Tekkit Main");
        assert_eq!(
            item.icon_url.as_deref(),
            Some("https://example.com/entry-icon.png")
        );
        assert_eq!(item.downloads, 1234);
    }

    #[test]
    fn rejects_malformed_search_and_solder_objects_missing_required_arrays() {
        assert!(serde_json::from_str::<TechnicSearchResponse>(r#"{"error":"bad"}"#).is_err());
        assert!(serde_json::from_str::<SolderPackResponse>(r#"{"error":"missing"}"#).is_err());
    }
}
