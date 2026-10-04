//! Thin native account procedure adapters; string rejections remain wire-compatible.
use crate::app::accounts as account;
use tauri::Runtime;
#[taurpc::procedures(path = "skins")]
pub(super) trait SkinsApi {
    async fn get_skin_data_url(
        app_handle: tauri::AppHandle<impl Runtime>,
        skin_url: String,
    ) -> Result<String, String>;
    async fn save_skin_to_downloads(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        skin_url: String,
    ) -> Result<String, String>;
    async fn get_ely_skins(
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
        uploader: Option<String>,
    ) -> Result<crate::auth::ely::ElySkinsCatalogResponse, String>;
    async fn apply_ely_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_id: u64,
        password: Option<String>,
    ) -> Result<(), String>;
    async fn upload_ely_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        password: Option<String>,
    ) -> Result<(), String>;
    async fn has_ely_web_credentials(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<bool, String>;
    async fn apply_microsoft_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_url: String,
        is_slim: bool,
    ) -> Result<(), String>;
    async fn upload_microsoft_skin(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        is_slim: bool,
    ) -> Result<(), String>;
}
#[derive(Clone)]
pub(super) struct SkinsApiImpl;
#[taurpc::resolvers]
impl SkinsApi for SkinsApiImpl {
    async fn get_skin_data_url(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        skin_url: String,
    ) -> Result<String, String> {
        account::get_skin_data_url(app_handle, skin_url).await
    }

    async fn save_skin_to_downloads(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        skin_url: String,
    ) -> Result<String, String> {
        account::save_skin_to_downloads(app_handle, username, skin_url).await
    }

    async fn get_ely_skins(
        self,
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
        uploader: Option<String>,
    ) -> Result<crate::auth::ely::ElySkinsCatalogResponse, String> {
        account::get_ely_skins_catalog(page, query, sort, model, uploader).await
    }

    async fn apply_ely_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_id: u64,
        password: Option<String>,
    ) -> Result<(), String> {
        account::apply_ely_skin(&app_handle, &account_id, skin_id, password).await
    }

    async fn upload_ely_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        password: Option<String>,
    ) -> Result<(), String> {
        account::upload_ely_skin(&app_handle, &account_id, &image_base64, password).await
    }

    async fn apply_microsoft_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        skin_url: String,
        is_slim: bool,
    ) -> Result<(), String> {
        account::apply_microsoft_skin(&app_handle, &account_id, &skin_url, is_slim).await
    }

    async fn upload_microsoft_skin(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
        image_base64: String,
        is_slim: bool,
    ) -> Result<(), String> {
        account::upload_microsoft_skin(&app_handle, &account_id, &image_base64, is_slim).await
    }

    async fn has_ely_web_credentials(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<bool, String> {
        account::has_ely_web_credentials(&app_handle, &account_id)
    }
}
