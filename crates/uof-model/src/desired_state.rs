use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::template::TemplateBinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesiredState {
    pub generation: i64,
    pub plugins: Vec<PluginActivation>,
    pub templates: Vec<TemplateBinding>,
    pub sampling: serde_json::Value,
    pub exporter: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckRequest {
    pub generation: i64,
    pub status: AckStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
    Applied,
    Failed,
}

/// Request for a plugin artifact — sent by an agent to the control plane
/// to fetch the raw plugin tarball bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginArtifactRequest {
    pub plugin_id: Uuid,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActivation {
    pub plugin_id: Uuid,
    pub version: String,
    pub action: PluginAction,
    /// Full URL where the agent can download the plugin tarball artifact.
    /// Present when action is Install or Enable; absent for Disable/Uninstall.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_url: Option<String>,
    /// Expected SHA-256 digest of the artifact (hex-encoded).
    /// The agent verifies this after download.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginAction {
    Install,
    Enable,
    Disable,
    Uninstall,
}
