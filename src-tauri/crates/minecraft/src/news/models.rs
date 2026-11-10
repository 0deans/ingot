use serde::{Deserialize, Serialize};
use specta::Type;

pub(crate) const MOJANG_CONTENT_BASE: &str = "https://launchercontent.mojang.com";

pub(crate) fn resolve_mojang_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with('/') {
        format!("{MOJANG_CONTENT_BASE}{url}")
    } else {
        format!("{MOJANG_CONTENT_BASE}/{url}")
    }
}

pub(crate) fn validate_content_path(path: &str) -> bool {
    let prefix = "javaPatchNotes/";
    let suffix = ".json";
    if !path.starts_with(prefix) || !path.ends_with(suffix) {
        return false;
    }
    let filename = &path[prefix.len()..path.len() - suffix.len()];
    if filename.is_empty() {
        return false;
    }
    filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub(crate) fn is_safe_article_url(url: &str) -> bool {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return false;
        }
        if let Some(host) = parsed.host_str() {
            let host = host.to_ascii_lowercase();
            return host == "minecraft.net"
                || host.ends_with(".minecraft.net")
                || host == "mojang.com"
                || host.ends_with(".mojang.com");
        }
    }
    false
}

pub(crate) fn extract_article_content_html(raw_html: &str) -> Option<String> {
    let marker = "class=\"MC_articleGridA_container";
    let start_idx = raw_html.find(marker)?;
    let tag_open = raw_html[..start_idx].rfind('<')?;
    let section_end = raw_html[start_idx..].find("</section>")?;
    let content = &raw_html[tag_open..start_idx + section_end];

    let processed = content
        .replace(
            "src=\"/content/dam/",
            "src=\"https://www.minecraft.net/content/dam/",
        )
        .replace(
            "src='/content/dam/",
            "src='https://www.minecraft.net/content/dam/",
        )
        .replace(
            "href=\"/en-us/article/",
            "href=\"https://www.minecraft.net/article/",
        )
        .replace(
            "href=\"/article/",
            "href=\"https://www.minecraft.net/article/",
        );

    Some(processed)
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawNewsFeed {
    #[serde(default)]
    pub(crate) entries: Vec<RawNewsEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawNewsImage {
    pub(crate) url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawNewsEntry {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) tag: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) date: String,
    pub(crate) text: Option<String>,
    pub(crate) news_page_image: Option<RawNewsImage>,
    pub(crate) play_page_image: Option<RawNewsImage>,
    pub(crate) read_more_link: Option<String>,
    #[serde(default)]
    pub(crate) news_type: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawPatchNotesFeed {
    #[serde(default)]
    pub(crate) entries: Vec<RawPatchNoteEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPatchNoteImage {
    pub(crate) url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPatchNoteEntry {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) version: String,
    #[serde(rename = "type")]
    pub(crate) patch_type: String,
    pub(crate) date: String,
    pub(crate) short_text: Option<String>,
    pub(crate) image: Option<RawPatchNoteImage>,
    pub(crate) content_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPatchNoteDetail {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) version: Option<String>,
    #[serde(rename = "type", default)]
    pub(crate) patch_type: Option<String>,
    #[serde(default)]
    pub(crate) date: Option<String>,
    pub(crate) image: Option<RawPatchNoteImage>,
    #[serde(default)]
    pub(crate) body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewsArticle {
    pub id: String,
    pub title: String,
    pub tag: Option<String>,
    pub category: String,
    pub date: String,
    pub text: String,
    pub image_url: Option<String>,
    pub read_more_url: String,
    pub news_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewsArticleDetails {
    pub id: String,
    pub title: String,
    pub tag: Option<String>,
    pub category: String,
    pub date: String,
    pub text: String,
    pub image_url: Option<String>,
    pub read_more_url: String,
    pub body_html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatchNoteSummary {
    pub id: String,
    pub title: String,
    pub version: String,
    pub patch_type: String,
    pub date: String,
    pub short_text: String,
    pub image_url: Option<String>,
    pub content_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatchNoteDetails {
    pub id: String,
    pub title: String,
    pub version: String,
    pub patch_type: String,
    pub date: String,
    pub image_url: Option<String>,
    pub body_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewsFeedResponse {
    pub articles: Vec<NewsArticle>,
    pub patch_notes: Vec<PatchNoteSummary>,
}
