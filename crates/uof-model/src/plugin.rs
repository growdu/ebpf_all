use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub publisher: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub version: String,
    pub digest: String,
    pub oci_ref: String,
    pub signature_status: String,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDetail {
    #[serde(flatten)]
    pub plugin: Plugin,
    pub default_version_id: Option<Uuid>,
    #[serde(default)]
    pub versions: Vec<PluginVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePluginRequest {
    pub name: String,
    pub kind: String,
    pub publisher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePluginVersionRequest {
    pub version: String,
    pub digest: String,
    pub oci_ref: String,
    pub manifest: serde_json::Value,
    #[serde(default)]
    pub compat_matrix: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePluginRequest {
    pub version: String,
    pub make_default: Option<bool>,
}

