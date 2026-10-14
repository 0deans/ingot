//! Ely.by skins catalog, web authentication, and skin upload/wear service.
pub(crate) mod catalog;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod session;
pub(crate) mod skins;
pub(crate) mod web_session;

#[cfg(test)]
mod tests;

pub use error::ElyError;
pub use models::{ElySkinItem, ElySkinsCatalogResponse};

pub struct ElyAuthService {
    client: reqwest::Client,
    ely_base: String,
    account_base: String,
}

impl Default for ElyAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl ElyAuthService {
    pub const DEFAULT_ELY_BASE: &'static str = "https://ely.by";
    pub const DEFAULT_ACCOUNT_BASE: &'static str = "https://account.ely.by";

    pub fn new() -> Self {
        Self::with_endpoints(
            reqwest::Client::builder()
                .user_agent(crate::USER_AGENT)
                .build()
                .unwrap_or_default(),
            Self::DEFAULT_ELY_BASE,
            Self::DEFAULT_ACCOUNT_BASE,
        )
    }

    pub fn with_endpoints(
        client: reqwest::Client,
        ely_base: impl Into<String>,
        account_base: impl Into<String>,
    ) -> Self {
        Self {
            client,
            ely_base: ely_base.into(),
            account_base: account_base.into(),
        }
    }

    pub async fn fetch_catalog(
        &self,
        page: u32,
        query: Option<String>,
        sort: Option<String>,
        model: Option<String>,
        uploader: Option<String>,
    ) -> Result<ElySkinsCatalogResponse, ElyError> {
        catalog::fetch_catalog(
            &self.client,
            &self.ely_base,
            page,
            query,
            sort,
            model,
            uploader,
        )
        .await
    }

    pub async fn wear_skin(
        &self,
        login: &str,
        password: &str,
        skin_id: u64,
    ) -> Result<(), ElyError> {
        let client = web_session::create_authenticated_web_client(
            login,
            password,
            &self.ely_base,
            &self.account_base,
        )
        .await?;
        skins::wear_skin(&client, &self.ely_base, skin_id).await
    }

    pub async fn upload_skin(
        &self,
        login: &str,
        password: &str,
        png_bytes: Vec<u8>,
    ) -> Result<(), ElyError> {
        let client = web_session::create_authenticated_web_client(
            login,
            password,
            &self.ely_base,
            &self.account_base,
        )
        .await?;
        skins::upload_skin(&client, &self.ely_base, png_bytes).await
    }
}
