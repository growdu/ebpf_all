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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActivation {
    pub plugin_id: Uuid,
    pub version: String,
    pub action: PluginAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginAction {
    Install,
    Enable,
    Disable,
    Uninstall,
}

