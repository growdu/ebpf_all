use sqlx::Row;
use sqlx::PgPool;
use uuid::Uuid;

use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest, AgentRegisterResponse},
    desired_state::{AckRequest, AckStatus, DesiredState, PluginAction, PluginActivation},
    plugin::{
        CreatePluginRequest, CreatePluginVersionRequest, Plugin, PluginDetail, PluginVersion,
        ReleasePluginRequest,
    },
    template::{CreateTemplateBindingRequest, CreateTemplateRequest, Template, TemplateBinding},
};

use crate::models::AgentRow;

#[derive(Debug, Clone)]
pub struct AppState {
    pool: PgPool,
    base_url: String,
}

impl AppState {
    pub async fn new(pool: PgPool, base_url: String) -> Self {
        Self { pool, base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn register_agent(&self, request: AgentRegisterRequest) -> AgentRegisterResponse {
        let agent_id = Uuid::new_v4();

        // Get default plugin for initial desired state
        let default_plugin: Option<(Uuid, String)> = sqlx::query_as(
            r#"SELECT id, name FROM plugins ORDER BY created_at LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(|(id, name): (Uuid, String)| (id, name));

        let desired_state = DesiredState {
            generation: 1,
            plugins: default_plugin
                .map(|(plugin_id, _)| PluginActivation {
                    plugin_id,
                    version: "0.1.0".to_string(),
                    action: PluginAction::Enable,
                    artifact_url: None,
                    artifact_digest: None,
                })
                .into_iter()
                .collect(),
            templates: vec![],
            sampling: serde_json::json!({ "profile": "default" }),
            exporter: serde_json::json!({ "endpoint": "http://otel-collector:4317" }),
        };

        let status = format!("registered:{}", request.hostname);

        sqlx::query(
            r#"INSERT INTO agents (id, name, status, desired_state_generation, last_heartbeat_status, acked_generation)
               VALUES ($1, $2, $3, $4, $5, $6)"#
        )
        .bind(agent_id)
        .bind(&request.hostname)
        .bind("online")
        .bind(1i64)
        .bind(&status)
        .bind(0i64)
        .execute(&self.pool)
        .await
        .ok();

        tracing::info!(agent_id = %agent_id, "agent registered in database");

        AgentRegisterResponse {
            agent_id,
            status,
            poll_interval_seconds: 15,
        }
    }

    pub async fn heartbeat(&self, agent_id: Uuid, request: AgentHeartbeatRequest) -> bool {
        let result = sqlx::query(
            r#"UPDATE agents SET last_heartbeat_at = NOW(), last_heartbeat_status = $1
               WHERE id = $2"#
        )
        .bind(&request.status)
        .bind(agent_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::info!(agent_id = %agent_id, "heartbeat accepted");
                true
            }
            _ => {
                tracing::warn!(agent_id = %agent_id, "heartbeat rejected: agent not found");
                false
            }
        }
    }

    pub async fn desired_state(&self, agent_id: Uuid) -> Option<DesiredState> {
        let agent: Option<AgentRow> = sqlx::query_as(
            r#"SELECT id, name, status, desired_state_generation, last_heartbeat_at,
                      last_heartbeat_status, acked_generation, created_at, updated_at
               FROM agents WHERE id = $1"#
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        agent.and_then(|a| {
            if a.acked_generation >= a.desired_state_generation {
                None
            } else {
                // TODO: Fetch actual desired_state from separate table
                Some(DesiredState {
                    generation: a.desired_state_generation,
                    plugins: vec![],
                    templates: vec![],
                    sampling: serde_json::json!({}),
                    exporter: serde_json::json!({}),
                })
            }
        })
    }

    pub async fn ack_desired_state(&self, agent_id: Uuid, request: AckRequest) -> bool {
        let result = match request.status {
            AckStatus::Applied => {
                sqlx::query(
                    r#"UPDATE agents SET acked_generation = $1 WHERE id = $2"#
                )
                .bind(request.generation)
                .bind(agent_id)
                .execute(&self.pool)
                .await
            }
            AckStatus::Failed => {
                sqlx::query(
                    r#"UPDATE agents SET last_heartbeat_status = $1 WHERE id = $2"#
                )
                .bind(request.message.unwrap_or_else(|| "apply failed".to_string()))
                .bind(agent_id)
                .execute(&self.pool)
                .await
            }
        };

        result.map(|r| r.rows_affected() > 0).unwrap_or(false)
    }

    pub async fn list_plugins(&self) -> Vec<Plugin> {
        let rows = sqlx::query(
            r#"SELECT id, name, kind, publisher, status FROM plugins"#
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|row| Plugin {
                id: row.get("id"),
                name: row.get("name"),
                kind: row.get("kind"),
                publisher: row.get("publisher"),
                status: row.get("status"),
            })
            .collect()
    }

    pub async fn create_plugin(&self, request: CreatePluginRequest) -> Plugin {
        let plugin = Plugin {
            id: Uuid::new_v4(),
            name: request.name,
            kind: request.kind,
            publisher: request.publisher,
            status: "draft".to_string(),
        };

        sqlx::query(
            r#"INSERT INTO plugins (id, name, kind, publisher, status)
               VALUES ($1, $2, $3, $4, $5)"#
        )
        .bind(plugin.id)
        .bind(&plugin.name)
        .bind(&plugin.kind)
        .bind(&plugin.publisher)
        .bind(&plugin.status)
        .execute(&self.pool)
        .await
        .ok();

        plugin
    }

    pub async fn get_plugin(&self, plugin_id: Uuid) -> Option<PluginDetail> {
        let rows = match sqlx::query(
            r#"SELECT id, name, kind, publisher, status FROM plugins WHERE id = $1"#
        )
        .bind(plugin_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(row)) => row,
            Ok(None) => {
                tracing::warn!(plugin_id = %plugin_id, "plugin not found");
                return None;
            }
            Err(e) => {
                tracing::error!(plugin_id = %plugin_id, error = %e, "failed to fetch plugin");
                return None;
            }
        };

        let plugin = Plugin {
            id: rows.get("id"),
            name: rows.get("name"),
            kind: rows.get("kind"),
            publisher: rows.get("publisher"),
            status: rows.get("status"),
        };

        let version_rows = sqlx::query(
            r#"SELECT id, plugin_id, version, digest, oci_ref, signature_status, published, created_at
               FROM plugin_versions WHERE plugin_id = $1"#
        )
        .bind(plugin_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let versions: Vec<PluginVersion> = version_rows
            .iter()
            .map(|row| PluginVersion {
                id: row.get("id"),
                plugin_id: row.get("plugin_id"),
                version: row.get("version"),
                digest: row.get("digest"),
                oci_ref: row.get("oci_ref"),
                signature_status: row.get("signature_status"),
                published: row.get("published"),
            })
            .collect();

        Some(PluginDetail {
            plugin,
            default_version_id: None,
            versions,
        })
    }

    pub async fn create_plugin_version(
        &self,
        plugin_id: Uuid,
        request: CreatePluginVersionRequest,
    ) -> Option<PluginVersion> {
        let version = PluginVersion {
            id: Uuid::new_v4(),
            plugin_id,
            version: request.version,
            digest: request.digest,
            oci_ref: request.oci_ref,
            signature_status: "pending".to_string(),
            published: false,
        };

        sqlx::query(
            r#"INSERT INTO plugin_versions (id, plugin_id, version, digest, oci_ref)
               VALUES ($1, $2, $3, $4, $5)"#
        )
        .bind(version.id)
        .bind(version.plugin_id)
        .bind(&version.version)
        .bind(&version.digest)
        .bind(&version.oci_ref)
        .execute(&self.pool)
        .await
        .ok()?;

        Some(version)
    }

    pub async fn find_version_by_string(&self, plugin_id: Uuid, version: &str) -> Option<(Uuid, String)> {
        sqlx::query_as::<_, (Uuid, String)>(
            r#"SELECT id, version FROM plugin_versions WHERE plugin_id = $1 AND version = $2"#
        )
        .bind(plugin_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn release_plugin_version(
        &self,
        plugin_id: Uuid,
        request: ReleasePluginRequest,
    ) -> bool {
        let result = sqlx::query(
            r#"UPDATE plugin_versions SET published = true WHERE plugin_id = $1 AND version = $2"#
        )
        .bind(plugin_id)
        .bind(&request.version)
        .execute(&self.pool)
        .await;

        result.map(|r| r.rows_affected() > 0).unwrap_or(false)
    }

    pub async fn notify_agents_plugin_released(
        &self,
        plugin_id: Uuid,
        version_id: Uuid,
        version_string: String,
    ) {
        // Bump generation for all agents
        sqlx::query(
            r#"UPDATE agents SET desired_state_generation = desired_state_generation + 1"#
        )
        .execute(&self.pool)
        .await
        .ok();

        tracing::info!(plugin_id = %plugin_id, version_id = %version_id, "notified agents of plugin release");
    }

    pub async fn plugin_artifact_url(
        &self,
        plugin_id: Uuid,
        version_id: Uuid,
    ) -> Option<String> {
        let oci_ref: Option<String> = sqlx::query_scalar(
            r#"SELECT oci_ref FROM plugin_versions WHERE plugin_id = $1 AND id = $2"#
        )
        .bind(plugin_id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        oci_ref.map(|ref oci_ref| {
            if oci_ref.starts_with("http://") || oci_ref.starts_with("https://") {
                oci_ref.clone()
            } else {
                format!(
                    "{}/api/v1/plugins/{}/versions/{}/artifact",
                    self.base_url, plugin_id, version_id
                )
            }
        })
    }

    pub async fn get_version_oci_ref(&self, plugin_id: Uuid, version_id: Uuid) -> Option<String> {
        sqlx::query_scalar(
            r#"SELECT oci_ref FROM plugin_versions WHERE plugin_id = $1 AND id = $2"#
        )
        .bind(plugin_id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn list_templates(&self) -> Vec<Template> {
        let rows = sqlx::query(
            r#"SELECT id, plugin_id, name, version, target_software, scenario, status FROM templates"#
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|row| Template {
                id: row.get("id"),
                plugin_id: row.get("plugin_id"),
                name: row.get("name"),
                version: row.get("version"),
                target_software: row.get("target_software"),
                scenario: row.get("scenario"),
                status: row.get("status"),
            })
            .collect()
    }

    pub async fn create_template(&self, request: CreateTemplateRequest) -> Template {
        let template = Template {
            id: Uuid::new_v4(),
            plugin_id: request.plugin_id,
            name: request.name,
            version: request.version,
            target_software: request.target_software,
            scenario: request.scenario,
            status: "active".to_string(),
        };

        sqlx::query(
            r#"INSERT INTO templates (id, plugin_id, name, version, target_software, scenario, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(template.id)
        .bind(template.plugin_id)
        .bind(&template.name)
        .bind(&template.version)
        .bind(&template.target_software)
        .bind(&template.scenario)
        .bind(&template.status)
        .execute(&self.pool)
        .await
        .ok();

        template
    }

    pub async fn create_template_binding(&self, request: CreateTemplateBindingRequest) -> TemplateBinding {
        let binding = TemplateBinding {
            id: Uuid::new_v4(),
            template_id: request.template_id,
            selector: request.selector,
            target: request.target,
            policy: request.policy.unwrap_or_else(|| serde_json::json!({})),
            enabled: request.enabled.unwrap_or(true),
        };

        sqlx::query(
            r#"INSERT INTO template_bindings (id, template_id, agent_id, variables)
               VALUES ($1, $2, $3, $4)"#
        )
        .bind(binding.id)
        .bind(binding.template_id)
        .bind(Uuid::nil()) // agent_id will be set when bound to an agent
        .bind(&binding.policy)
        .execute(&self.pool)
        .await
        .ok();

        binding
    }

    pub async fn delete_template_binding(&self, binding_id: Uuid) -> bool {
        let result = sqlx::query(
            r#"DELETE FROM template_bindings WHERE id = $1"#
        )
        .bind(binding_id)
        .execute(&self.pool)
        .await;

        result.map(|r| r.rows_affected() > 0).unwrap_or(false)
    }
}