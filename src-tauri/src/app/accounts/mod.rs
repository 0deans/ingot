//! Native account adapters. Provider/storage string errors remain a tracked legacy boundary.
mod downloads;
mod profiles;
mod session;
mod skins;
mod storage;

pub(crate) use downloads::{get_skin_data_url, save_skin_to_downloads};
pub(crate) use profiles::{
    add_offline_account, get_accounts, remove_account, reorder_accounts, set_active_account,
};
pub(crate) use session::{
    ely_login, get_active_account_token, microsoft_login_cancel, microsoft_login_finish,
    microsoft_login_start,
};
pub(crate) use skins::{
    apply_ely_skin, apply_microsoft_skin, get_ely_skins_catalog, has_ely_web_credentials,
    upload_ely_skin, upload_microsoft_skin,
};
pub(crate) use storage::load_accounts_file;
