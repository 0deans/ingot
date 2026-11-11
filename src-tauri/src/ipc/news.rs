use crate::ipc::ApiError;
use crate::minecraft::news::{NewsArticleDetails, NewsFeedResponse, NewsService, PatchNoteDetails};
use tauri::Runtime;

fn news_service<R: Runtime>(app: &tauri::AppHandle<R>) -> NewsService {
    let state = crate::app::state(app);
    NewsService::new(state.http.clone(), state.news_cache_dir.clone())
}

#[taurpc::procedures(path = "news")]
pub trait NewsApi {
    async fn get_news_feed(
        app_handle: tauri::AppHandle<impl Runtime>,
        force_refresh: bool,
    ) -> Result<NewsFeedResponse, ApiError>;

    async fn get_patch_note_details(
        app_handle: tauri::AppHandle<impl Runtime>,
        content_path: String,
    ) -> Result<PatchNoteDetails, ApiError>;

    async fn get_article_details(
        app_handle: tauri::AppHandle<impl Runtime>,
        article_id: String,
    ) -> Result<NewsArticleDetails, ApiError>;
}

#[derive(Clone)]
pub struct NewsApiImpl;

#[taurpc::resolvers]
impl NewsApi for NewsApiImpl {
    async fn get_news_feed(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        force_refresh: bool,
    ) -> Result<NewsFeedResponse, ApiError> {
        news_service(&app_handle)
            .get_news_feed(force_refresh)
            .await
            .map_err(ApiError::from)
    }

    async fn get_patch_note_details(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        content_path: String,
    ) -> Result<PatchNoteDetails, ApiError> {
        news_service(&app_handle)
            .get_patch_note_details(&content_path)
            .await
            .map_err(ApiError::from)
    }

    async fn get_article_details(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        article_id: String,
    ) -> Result<NewsArticleDetails, ApiError> {
        news_service(&app_handle)
            .get_article_details(&article_id)
            .await
            .map_err(ApiError::from)
    }
}
