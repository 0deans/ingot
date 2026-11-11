use super::InstallerError;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeMap;

const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const QUILT_META: &str = "https://meta.quiltmc.org/v3/versions";

#[derive(Debug, Clone)]
pub(super) struct InstallerEndpoints {
    neoforge_maven: Cow<'static, str>,
    forge_maven: Cow<'static, str>,
    forge_promotions: Cow<'static, str>,
    quilt_meta: Cow<'static, str>,
}

impl InstallerEndpoints {
    pub(super) fn production() -> Self {
        Self {
            neoforge_maven: Cow::Borrowed(NEOFORGE_MAVEN),
            forge_maven: Cow::Borrowed(FORGE_MAVEN),
            forge_promotions: Cow::Borrowed(FORGE_PROMOTIONS),
            quilt_meta: Cow::Borrowed(QUILT_META),
        }
    }

    #[cfg(test)]
    pub(super) fn from_base(base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            neoforge_maven: Cow::Owned(format!("{base}/neoforge")),
            forge_maven: Cow::Owned(format!("{base}/forge")),
            forge_promotions: Cow::Owned(format!("{base}/forge/promotions_slim.json")),
            quilt_meta: Cow::Owned(format!("{base}/quilt/v3/versions")),
        }
    }

    pub(super) fn neoforge_metadata_url(&self) -> String {
        format!(
            "{}/maven-metadata.xml",
            self.neoforge_maven.trim_end_matches('/')
        )
    }

    pub(super) fn neoforge_installer_url(&self, version: &str) -> String {
        format!(
            "{}/{version}/neoforge-{version}-installer.jar",
            self.neoforge_maven.trim_end_matches('/')
        )
    }

    pub(super) fn forge_installer_url(&self, version: &str) -> String {
        format!(
            "{}/{version}/forge-{version}-installer.jar",
            self.forge_maven.trim_end_matches('/')
        )
    }

    pub(super) fn quilt_game_versions_url(&self) -> String {
        format!("{}/game", self.quilt_meta.trim_end_matches('/'))
    }

    pub(super) fn quilt_installer_url(&self) -> String {
        format!("{}/installer", self.quilt_meta.trim_end_matches('/'))
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct QuiltGameVersion {
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub(super) version: String,
    pub(super) stable: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct QuiltInstallerVersion {
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub(super) url: String,
}

#[derive(Debug, Deserialize)]
struct ForgePromotionsPayload {
    #[serde(deserialize_with = "deserialize_promotions_map")]
    promos: BTreeMap<String, String>,
}

fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("expected a non-empty string"));
    }
    Ok(value)
}

fn deserialize_promotions_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let promos = BTreeMap::<String, String>::deserialize(deserializer)?;
    if promos
        .iter()
        .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
    {
        return Err(serde::de::Error::custom(
            "expected non-empty Forge promotion keys and build strings",
        ));
    }
    Ok(promos)
}

pub(super) fn parse_neoforge_versions(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|fragment| fragment.split_once("</version>").map(|(v, _)| v))
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(super) fn parse_forge_promotions(
    bytes: &[u8],
) -> Result<BTreeMap<String, String>, InstallerError> {
    serde_json::from_slice::<ForgePromotionsPayload>(bytes)
        .map(|payload| payload.promos)
        .map_err(InstallerError::Json)
}

pub(super) fn parse_quilt_game_versions(
    bytes: &[u8],
) -> Result<Vec<QuiltGameVersion>, InstallerError> {
    serde_json::from_slice(bytes).map_err(InstallerError::Json)
}

pub(super) fn parse_quilt_installers(
    bytes: &[u8],
) -> Result<Vec<QuiltInstallerVersion>, InstallerError> {
    serde_json::from_slice(bytes).map_err(InstallerError::Json)
}

async fn get(client: &reqwest::Client, url: &str) -> Result<reqwest::Response, InstallerError> {
    client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(InstallerError::Network)
}

pub(super) async fn fetch_neoforge_versions(
    client: &reqwest::Client,
    endpoints: &InstallerEndpoints,
) -> Result<Vec<String>, InstallerError> {
    let xml = get(client, &endpoints.neoforge_metadata_url())
        .await?
        .text()
        .await
        .map_err(InstallerError::Network)?;
    Ok(parse_neoforge_versions(&xml))
}

pub(super) async fn fetch_forge_promotions(
    client: &reqwest::Client,
    endpoints: &InstallerEndpoints,
) -> Result<BTreeMap<String, String>, InstallerError> {
    let bytes = get(client, &endpoints.forge_promotions)
        .await?
        .bytes()
        .await
        .map_err(InstallerError::Network)?;
    parse_forge_promotions(&bytes)
}

pub(super) async fn fetch_quilt_game_versions(
    client: &reqwest::Client,
    endpoints: &InstallerEndpoints,
) -> Result<Vec<QuiltGameVersion>, InstallerError> {
    let bytes = get(client, &endpoints.quilt_game_versions_url())
        .await?
        .bytes()
        .await
        .map_err(InstallerError::Network)?;
    parse_quilt_game_versions(&bytes)
}

pub(super) async fn fetch_quilt_installer_url(
    client: &reqwest::Client,
    endpoints: &InstallerEndpoints,
) -> Result<String, InstallerError> {
    let bytes = get(client, &endpoints.quilt_installer_url())
        .await?
        .bytes()
        .await
        .map_err(InstallerError::Network)?;
    parse_quilt_installers(&bytes)?
        .into_iter()
        .next()
        .map(|entry| entry.url)
        .ok_or(InstallerError::NoQuiltInstaller)
}
