use crate::account::pending_login::PendingMicrosoftLogin;
use crate::account::AccountProfile;
use crate::app::accounts as account;
use crate::ipc::ApiError;
use tauri::Runtime;

#[taurpc::procedures(path = "accounts")]
pub(super) trait AccountsApi {
    async fn get_pending_microsoft_login(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Option<PendingMicrosoftLogin>, ApiError>;
    async fn resume_microsoft_login(
        app_handle: tauri::AppHandle<impl Runtime>,
        ticket: String,
    ) -> Result<AccountProfile, ApiError>;
    async fn discard_pending_microsoft_login(
        app_handle: tauri::AppHandle<impl Runtime>,
        ticket: String,
    ) -> Result<(), ApiError>;
    async fn ely_login(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, ApiError>;
    async fn microsoft_login_start(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<crate::auth::microsoft::MicrosoftDeviceCode, ApiError>;
    async fn microsoft_login_finish(
        app_handle: tauri::AppHandle<impl Runtime>,
        code: crate::auth::microsoft::MicrosoftDeviceCode,
    ) -> Result<AccountProfile, ApiError>;
    async fn microsoft_login_cancel(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<(), ApiError>;
    async fn add_offline_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, ApiError>;
    async fn get_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, ApiError>;
    async fn set_active_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), ApiError>;
    async fn remove_account(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), ApiError>;
    async fn get_active_account_token(
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, ApiError>;
    async fn reorder_accounts(
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), ApiError>;
}
#[derive(Clone)]
pub(super) struct AccountsApiImpl;
#[taurpc::resolvers]
impl AccountsApi for AccountsApiImpl {
    async fn get_pending_microsoft_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Option<PendingMicrosoftLogin>, ApiError> {
        account::get_pending_microsoft_login(app_handle).map_err(ApiError::from)
    }
    async fn resume_microsoft_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        ticket: String,
    ) -> Result<AccountProfile, ApiError> {
        account::resume_microsoft_login(app_handle, ticket)
            .await
            .map_err(ApiError::from)
    }
    async fn discard_pending_microsoft_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        ticket: String,
    ) -> Result<(), ApiError> {
        account::discard_pending_microsoft_login(app_handle, ticket).map_err(ApiError::from)
    }
    async fn ely_login(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
        password: String,
    ) -> Result<AccountProfile, ApiError> {
        account::ely_login(app_handle, username, password)
            .await
            .map_err(ApiError::from)
    }

    async fn microsoft_login_start(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<crate::auth::microsoft::MicrosoftDeviceCode, ApiError> {
        account::microsoft_login_start(app_handle)
            .await
            .map_err(ApiError::from)
    }

    async fn microsoft_login_finish(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        code: crate::auth::microsoft::MicrosoftDeviceCode,
    ) -> Result<AccountProfile, ApiError> {
        account::microsoft_login_finish(app_handle, code)
            .await
            .map_err(ApiError::from)
    }

    async fn microsoft_login_cancel(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<(), ApiError> {
        account::microsoft_login_cancel(app_handle).map_err(ApiError::from)
    }

    async fn add_offline_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        username: String,
    ) -> Result<AccountProfile, ApiError> {
        account::add_offline_account(app_handle, username)
            .await
            .map_err(ApiError::from)
    }

    async fn get_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<Vec<AccountProfile>, ApiError> {
        account::get_accounts(app_handle).map_err(ApiError::from)
    }

    async fn set_active_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), ApiError> {
        account::set_active_account(app_handle, account_id).map_err(ApiError::from)
    }

    async fn remove_account(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_id: String,
    ) -> Result<(), ApiError> {
        account::remove_account(app_handle, account_id)
            .await
            .map_err(ApiError::from)
    }

    async fn get_active_account_token(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
    ) -> Result<String, ApiError> {
        account::get_active_account_token(app_handle)
            .await
            .map_err(ApiError::from)
    }

    async fn reorder_accounts(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        account_ids: Vec<String>,
    ) -> Result<(), ApiError> {
        account::reorder_accounts(app_handle, account_ids).map_err(ApiError::from)
    }
}
