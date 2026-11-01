use crate::app::news;
use crate::ipc::ApiError;
use crate::minecraft::news::{NewsArticleDetails, NewsFeedResponse, PatchNoteDetails};
use tauri::Runtime;

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
        news::get_news_feed(&app_handle, force_refresh)
            .await
            .map_err(ApiError::from)
    }

    async fn get_patch_note_details(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        content_path: String,
    ) -> Result<PatchNoteDetails, ApiError> {
        news::get_patch_note_details(&app_handle, &content_path)
            .await
            .map_err(ApiError::from)
    }

    async fn get_article_details(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        article_id: String,
    ) -> Result<NewsArticleDetails, ApiError> {
        news::get_article_details(&app_handle, &article_id)
            .await
            .map_err(ApiError::from)
    }
}
