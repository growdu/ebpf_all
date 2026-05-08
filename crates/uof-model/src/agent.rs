use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterRequest {
    pub hostname: String,
    pub node_name: Option<String>,
    pub ip: Option<String>,
    pub kernel_version: String,
    pub os_release: Option<String>,
    pub arch: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub capabilities: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterResponse {
    pub agent_id: Uuid,
    pub status: String,
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeatRequest {
    pub status: String,
    #[serde(default)]
    pub health_summary: serde_json::Value,
    #[serde(default)]
    pub probe_status: Vec<serde_json::Value>,
    #[serde(default)]
    pub plugin_status: Vec<serde_json::Value>,
}

