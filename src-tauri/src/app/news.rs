use crate::minecraft::news::{
    self, NewsArticleDetails, NewsError, NewsFeedResponse, PatchNoteDetails,
};
use tauri::{AppHandle, Runtime};

pub(crate) fn service<R: Runtime>(app: &AppHandle<R>) -> news::NewsService {
    let state = super::state(app);
    news::NewsService::new(state.http.clone(), state.news_cache_dir.clone())
}

pub(crate) async fn get_news_feed<R: Runtime>(
    app: &AppHandle<R>,
    force_refresh: bool,
) -> Result<NewsFeedResponse, NewsError> {
    service(app).get_news_feed(force_refresh).await
}

pub(crate) async fn get_patch_note_details<R: Runtime>(
    app: &AppHandle<R>,
    content_path: &str,
) -> Result<PatchNoteDetails, NewsError> {
    service(app).get_patch_note_details(content_path).await
}

pub(crate) async fn get_article_details<R: Runtime>(
    app: &AppHandle<R>,
    article_id: &str,
) -> Result<NewsArticleDetails, NewsError> {
    service(app).get_article_details(article_id).await
}
