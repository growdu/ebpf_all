use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub name: String,
    pub version: String,
    pub target_software: String,
    pub scenario: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub plugin_id: Uuid,
    pub name: String,
    pub version: String,
    pub target_software: String,
    pub scenario: Option<String>,
    pub manifest: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBinding {
    pub id: Uuid,
    pub template_id: Uuid,
    pub selector: serde_json::Value,
    pub target: serde_json::Value,
    pub policy: serde_json::Value,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateBindingRequest {
    pub template_id: Uuid,
    pub selector: serde_json::Value,
    pub target: serde_json::Value,
    pub policy: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

