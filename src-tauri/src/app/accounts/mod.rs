//! Native account adapters. Provider/storage string errors remain a tracked legacy boundary.
mod downloads;
mod profiles;
mod session;
mod skins;
mod storage;
pub(crate) mod vault;

pub(crate) use downloads::{get_skin_data_url, save_skin_to_downloads};
pub(crate) use profiles::{
    add_offline_account, get_accounts, remove_account, reorder_accounts, set_active_account,
};
pub(crate) use session::{
    discard_pending_microsoft_login, ely_login, get_account_token, get_active_account_token,
    get_pending_microsoft_login, microsoft_login_cancel, microsoft_login_finish,
    microsoft_login_start, resume_microsoft_login,
};
pub(crate) use skins::{
    apply_ely_skin, apply_microsoft_skin, get_ely_skins_catalog, has_ely_web_credentials,
    upload_ely_skin, upload_microsoft_skin,
};
pub(crate) use storage::load_accounts_file;

pub(crate) use downloads::DownloadError;
pub(crate) use profiles::ProfileError;
pub(crate) use skins::SkinError;
