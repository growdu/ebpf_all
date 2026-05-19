//! Database row types for PostgreSQL persistence.
//!
//! These types represent rows in the UOF control plane database.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Agent row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentRow {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub desired_state_generation: i64,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub last_heartbeat_status: Option<String>,
    pub acked_generation: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plugin row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PluginRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub publisher: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plugin version row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PluginVersionRow {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub version: String,
    pub digest: Option<String>,
    pub oci_ref: Option<String>,
    pub signature_status: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
}

/// Template row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TemplateRow {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub name: String,
    pub version: String,
    pub target_software: Option<String>,
    pub scenario: Option<String>,
    pub status: String,
    pub spec: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Template binding row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TemplateBindingRow {
    pub id: Uuid,
    pub template_id: Uuid,
    pub selector: serde_json::Value,
    pub target: Option<String>,
    pub policy: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}