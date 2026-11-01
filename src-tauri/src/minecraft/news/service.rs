use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::error::NewsError;
use super::models::{
    resolve_mojang_url, validate_content_path, NewsArticle, NewsFeedResponse, PatchNoteDetails,
    PatchNoteSummary, RawNewsFeed, RawPatchNoteDetail, RawPatchNotesFeed, MOJANG_CONTENT_BASE,
};

const FEED_CACHE_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct NewsService {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl NewsService {
    pub fn new(client: reqwest::Client, cache_dir: PathBuf) -> Self {
        Self { client, cache_dir }
    }

    pub async fn get_news_feed(&self, force_refresh: bool) -> Result<NewsFeedResponse, NewsError> {
        let feed_cache_path = self.cache_dir.join("feed.json");

        if !force_refresh {
            if let Some(cached) = Self::read_cached_feed(&feed_cache_path, false) {
                return Ok(cached);
            }
        }

        match self.fetch_remote_feed().await {
            Ok(feed) => {
                let _ = Self::write_cached_feed(&feed_cache_path, &feed);
                Ok(feed)
            }
            Err(err) => {
                if let Some(cached) = Self::read_cached_feed(&feed_cache_path, true) {
                    Ok(cached)
                } else {
                    Err(err)
                }
            }
        }
    }

    pub async fn get_patch_note_details(
        &self,
        content_path: &str,
    ) -> Result<PatchNoteDetails, NewsError> {
        if !validate_content_path(content_path) {
            return Err(NewsError::InvalidContentPath(content_path.to_string()));
        }

        let note_cache_dir = self.cache_dir.join("patch_notes");
        let safe_filename = content_path.replace(['/', '\\'], "_");
        let note_cache_path = note_cache_dir.join(safe_filename);

        if let Ok(content) = std::fs::read_to_string(&note_cache_path) {
            if let Ok(details) = serde_json::from_str::<PatchNoteDetails>(&content) {
                return Ok(details);
            }
        }

        let url = format!("{MOJANG_CONTENT_BASE}/v2/{content_path}");
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(NewsError::HttpStatus(response.status()));
        }

        let raw: RawPatchNoteDetail = response.json().await?;
        let image_url = raw
            .image
            .map(|img| resolve_mojang_url(&img.url))
            .filter(|u| !u.is_empty());

        let title = raw.title.unwrap_or_else(|| raw.id.clone());
        let version = raw.version.unwrap_or_else(|| title.clone());
        let patch_type = raw.patch_type.unwrap_or_else(|| "release".to_string());
        let date = raw.date.unwrap_or_default();
        let body_html = raw.body.unwrap_or_default();

        let details = PatchNoteDetails {
            id: raw.id,
            title,
            version,
            patch_type,
            date,
            image_url,
            body_html,
        };

        if let Ok(json) = serde_json::to_string(&details) {
            let _ = std::fs::create_dir_all(&note_cache_dir);
            let _ = std::fs::write(&note_cache_path, json);
        }

        Ok(details)
    }

    async fn fetch_remote_feed(&self) -> Result<NewsFeedResponse, NewsError> {
        let news_url = format!("{MOJANG_CONTENT_BASE}/v2/news.json");
        let patch_notes_url = format!("{MOJANG_CONTENT_BASE}/v2/javaPatchNotes.json");

        let (news_res, patch_res) = tokio::join!(
            self.client.get(&news_url).send(),
            self.client.get(&patch_notes_url).send()
        );

        let news_resp = news_res?;
        if !news_resp.status().is_success() {
            return Err(NewsError::HttpStatus(news_resp.status()));
        }
        let raw_news: RawNewsFeed = news_resp.json().await?;

        let patch_resp = patch_res?;
        if !patch_resp.status().is_success() {
            return Err(NewsError::HttpStatus(patch_resp.status()));
        }
        let raw_patch: RawPatchNotesFeed = patch_resp.json().await?;

        let articles = raw_news
            .entries
            .into_iter()
            .map(|entry| {
                let image_url = entry
                    .news_page_image
                    .or(entry.play_page_image)
                    .map(|img| resolve_mojang_url(&img.url));

                NewsArticle {
                    id: entry.id,
                    title: entry.title,
                    tag: entry.tag,
                    category: entry.category.unwrap_or_else(|| "Minecraft".to_string()),
                    date: entry.date,
                    text: entry.text.unwrap_or_default(),
                    image_url,
                    read_more_url: entry.read_more_link.unwrap_or_default(),
                    news_types: entry.news_type,
                }
            })
            .collect();

        let patch_notes = raw_patch
            .entries
            .into_iter()
            .map(|entry| {
                let image_url = entry.image.map(|img| resolve_mojang_url(&img.url));

                PatchNoteSummary {
                    id: entry.id,
                    title: entry.title,
                    version: entry.version,
                    patch_type: entry.patch_type,
                    date: entry.date,
                    short_text: entry.short_text.unwrap_or_default(),
                    image_url,
                    content_path: entry.content_path,
                }
            })
            .collect();

        Ok(NewsFeedResponse {
            articles,
            patch_notes,
        })
    }

    fn read_cached_feed(path: &Path, allow_stale: bool) -> Option<NewsFeedResponse> {
        let metadata = std::fs::metadata(path).ok()?;
        if !allow_stale {
            let modified = metadata.modified().ok()?;
            let age = SystemTime::now().duration_since(modified).ok()?;
            if age > FEED_CACHE_TTL {
                return None;
            }
        }
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn write_cached_feed(path: &Path, feed: &NewsFeedResponse) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(feed).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}
