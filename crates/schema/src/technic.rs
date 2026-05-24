use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub const TECHNIC_SEARCH_URL: &str = "https://launcher.technicpack.net/api/search";
pub const TECHNIC_MODPACK_URL: &str = "https://launcher.technicpack.net/api/modpack";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TechnicSearchRequest {
    #[serde(rename = "q", skip_serializing_if = "Option::is_none")]
    pub query: Option<Arc<str>>,
    #[serde(skip_serializing_if = "is_zero")]
    pub offset: usize,
    pub limit: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TechnicModpackRequest {
    pub name: Arc<str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicSearchResult {
    pub results: Arc<[TechnicSearchHit]>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicSearchHit {
    pub name: Arc<str>,
    pub display_name: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub author: Option<Arc<str>>,
    pub icon: Option<Arc<str>>,
    pub logo: Option<Arc<str>>,
    pub url: Option<Arc<str>>,
    pub recommended: Option<Arc<str>>,
    pub latest: Option<Arc<str>>,
    pub downloads: Option<u64>,
    #[serde(default)]
    pub featured: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicModpackInfo {
    pub name: Arc<str>,
    pub display_name: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub author: Option<Arc<str>>,
    pub icon: Option<Arc<str>>,
    pub logo: Option<Arc<str>>,
    pub url: Option<Arc<str>>,
    pub recommended: Option<Arc<str>>,
    pub latest: Option<Arc<str>>,
    pub downloads: Option<u64>,
    pub versions: Option<Arc<[TechnicVersion]>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicVersion {
    pub version: Arc<str>,
    #[serde(rename = "minecraft")]
    pub minecraft_version: Option<Arc<str>>,
    #[serde(rename = "java")]
    pub java_version: Option<Arc<str>>,
    pub build: Option<u32>,
    pub released: Option<Arc<str>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicPackManifestJson {
    pub name: Option<Arc<str>>,
    pub version: Arc<str>,
    pub minecraft: Option<Arc<str>>,
    pub java: Option<Arc<str>>,
    #[serde(default)]
    pub mods: Arc<[Arc<str>]>,
    #[serde(default)]
    pub java_args: Arc<[Arc<str>]>,
}

#[derive(Clone, Debug)]
pub struct CachedTechnicFileInfo {
    pub hash: [u8; 20],
    pub filename: Arc<str>,
}
