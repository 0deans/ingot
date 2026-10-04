//! Thin native account procedure adapters; string rejections remain wire-compatible.
use crate::account::AccountProfile;
use crate::app::accounts as account;
use tauri::Runtime;
#[taurpc::procedures(path = "accounts")]
pub(super) trait AccountsApi {
    async fn ely_login(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, String>;
    async fn microsoft_login_start(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<crate::auth::microsoft::MicrosoftDeviceCode, String>;
    async fn microsoft_login_finish(
        app_handle: tauri::AppHandle<impl Runtime>,
        code: crate::auth::microsoft::MicrosoftDeviceCode,
    ) -> Result<AccountProfile, String>;
    async fn microsoft_login_cancel(app_handle: tauri::AppHandle<impl Runtime>);
    async fn add_offline_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, String>;
    async fn get_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, String>;
    async fn set_active_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String>;
    async fn remove_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String>;
    async fn get_active_account_token(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String>;
    async fn reorder_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), String>;
}
#[derive(Clone)]
pub(super) struct AccountsApiImpl;
#[taurpc::resolvers]
impl AccountsApi for AccountsApiImpl {
    async fn ely_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, String> {
        account::ely_login(app_handle, username, password).await
    }

    async fn microsoft_login_start(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<crate::auth::microsoft::MicrosoftDeviceCode, String> {
        account::microsoft_login_start(app_handle).await
    }

    async fn microsoft_login_finish(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        code: crate::auth::microsoft::MicrosoftDeviceCode,
    ) -> Result<AccountProfile, String> {
        account::microsoft_login_finish(app_handle, code).await
    }

    async fn microsoft_login_cancel(self, app_handle: tauri::AppHandle<impl Runtime>) {
        account::microsoft_login_cancel(app_handle)
    }

    async fn add_offline_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, String> {
        account::add_offline_account(app_handle, username).await
    }

    async fn get_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, String> {
        account::get_accounts(app_handle)
    }

    async fn set_active_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String> {
        account::set_active_account(app_handle, account_id)
    }

    async fn remove_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), String> {
        account::remove_account(app_handle, account_id).await
    }

    async fn get_active_account_token(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, String> {
        account::get_active_account_token(app_handle).await
    }

    async fn reorder_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), String> {
        account::reorder_accounts(app_handle, account_ids)
    }
}
