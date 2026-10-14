//! Data transfer objects for Ely.by skin catalog and API requests.
use serde::Deserialize;

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElySkinItem {
    pub id: u64,
    pub skin_url: String,
    pub is_slim: bool,
    pub count_wearers: u64,
    pub count_cubes: u64,
    pub count_views: u64,
    pub tags: Vec<String>,
}

#[taurpc::ipc_type]
#[derive(Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElySkinsCatalogResponse {
    pub items: Vec<ElySkinItem>,
    pub total_items: u64,
    pub current_page: u32,
    pub last_page: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct ElyRawSkinItem {
    pub id: u64,
    pub skin_url: String,
    #[serde(default)]
    pub is_slim: bool,
    #[serde(default)]
    pub count_wearers: u64,
    #[serde(default)]
    pub count_cubes: u64,
    #[serde(default)]
    pub count_views_total: u64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ElyRawCatalogResponse {
    #[serde(default)]
    pub items: Vec<ElyRawSkinItem>,
    #[serde(default)]
    pub total_items: Option<u64>,
    #[serde(default)]
    pub current: Option<u32>,
    #[serde(default)]
    pub last: Option<u32>,
}

pub(crate) fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
