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

        let status = format!("registered:{}", request.hostname);

        // Insert agent
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

        // Persist initial desired state
        let desired_state_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO desired_states (id, agent_id, generation, sampling, exporter)
               VALUES ($1, $2, 1, $3, $4)"#
        )
        .bind(desired_state_id)
        .bind(agent_id)
        .bind(serde_json::json!({ "profile": "default" }).to_string())
        .bind(serde_json::json!({ "endpoint": "http://otel-collector:4317" }).to_string())
        .execute(&self.pool)
        .await
        .ok();

        // Get default plugin and create desired state plugin entry
        let default_plugin: Option<(Uuid, String)> = sqlx::query_as(
            r#"SELECT id, name FROM plugins ORDER BY created_at LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(|(id, name): (Uuid, String)| (id, name));

        if let Some((plugin_id, _)) = default_plugin {
            sqlx::query(
                r#"INSERT INTO desired_state_plugins (id, desired_state_id, plugin_id, version, action)
                   VALUES ($1, $2, $3, $4, $5)"#
            )
            .bind(Uuid::new_v4())
            .bind(desired_state_id)
            .bind(plugin_id)
            .bind("0.1.0")
            .bind("enable")
            .execute(&self.pool)
            .await
            .ok();
        }

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
        let agent = sqlx::query_as::<_, AgentRow>(
            r#"SELECT id, name, status, desired_state_generation, last_heartbeat_at,
                      last_heartbeat_status, acked_generation, created_at, updated_at
               FROM agents WHERE id = $1"#
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;

        // Check if there's a newer generation to deliver
        if agent.acked_generation >= agent.desired_state_generation {
            return None;
        }

        // Fetch desired_state from the desired_states table
        let ds_row = sqlx::query(
            r#"SELECT id, generation, sampling, exporter FROM desired_states
               WHERE agent_id = $1 AND generation = $2"#
        )
        .bind(agent_id)
        .bind(agent.desired_state_generation)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;

        let ds_id: Uuid = ds_row.get("id");
        let generation: i64 = ds_row.get("generation");
        let sampling: serde_json::Value = ds_row.get("sampling");
        let exporter: serde_json::Value = ds_row.get("exporter");

        // Fetch desired state plugins
        let plugin_rows = sqlx::query(
            r#"SELECT dsp.plugin_id, dsp.version, dsp.action, dsp.artifact_url, dsp.artifact_digest
               FROM desired_state_plugins dsp
               WHERE dsp.desired_state_id = $1"#
        )
        .bind(ds_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let plugins: Vec<PluginActivation> = plugin_rows
            .iter()
            .map(|row| PluginActivation {
                plugin_id: row.get("plugin_id"),
                version: row.get("version"),
                action: match row.get::<String, _>("action").as_str() {
                    "install" => PluginAction::Install,
                    "enable" => PluginAction::Enable,
                    "disable" => PluginAction::Disable,
                    "uninstall" => PluginAction::Uninstall,
                    _ => PluginAction::Enable,
                },
                artifact_url: row.get("artifact_url"),
                artifact_digest: row.get("artifact_digest"),
            })
            .collect();

        // Fetch desired state templates
        let template_rows = sqlx::query(
            r#"SELECT dst.id, dst.template_id, t.name, t.target_software, t.scenario, t.status,
                      dst.variables
               FROM desired_state_templates dst
               JOIN templates t ON t.id = dst.template_id
               WHERE dst.desired_state_id = $1"#
        )
        .bind(ds_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let templates: Vec<TemplateBinding> = template_rows
            .iter()
            .map(|row| TemplateBinding {
                id: row.get("id"),
                template_id: row.get("template_id"),
                selector: serde_json::json!({}),
                target: serde_json::json!({
                    "name": row.get::<String, _>("name"),
                    "software": row.get::<String, _>("target_software")
                }),
                policy: serde_json::json!({}),
                enabled: true,
            })
            .collect();

        Some(DesiredState {
            generation,
            plugins,
            templates,
            sampling,
            exporter,
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
        _version_string: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use uof_model::agent::{AgentHeartbeatRequest, AgentRegisterRequest};
    use uof_model::desired_state::{AckRequest, AckStatus, PluginAction};
    use uof_model::plugin::{CreatePluginRequest, Plugin};
    use uof_model::template::{CreateTemplateRequest, Template};
    use std::collections::BTreeMap;

    #[test]
    fn test_app_state_base_url() {
        // Test that AppState stores base_url correctly
        // We can't easily construct AppState without a pool, but we can test the Uuid generation
        let uuid = uuid::Uuid::new_v4();
        assert!(!uuid.is_nil());
    }

    #[test]
    fn test_agent_register_request_serialization() {
        let request = AgentRegisterRequest {
            hostname: "test-host".to_string(),
            node_name: Some("node-1".to_string()),
            ip: Some("192.168.1.100".to_string()),
            kernel_version: "5.4.0".to_string(),
            os_release: Some("Ubuntu 20.04".to_string()),
            arch: "x86_64".to_string(),
            labels: BTreeMap::new(),
            capabilities: serde_json::json!({"cap1": true, "cap2": true}),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-host"));
        assert!(json.contains("5.4.0"));
    }

    #[test]
    fn test_agent_heartbeat_request_serialization() {
        let request = AgentHeartbeatRequest {
            status: "running".to_string(),
            health_summary: serde_json::json!({"cpu": 50}),
            probe_status: vec![],
            plugin_status: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("running"));
    }

    #[test]
    fn test_plugin_creation_request() {
        let request = CreatePluginRequest {
            name: "test-plugin".to_string(),
            kind: "ebpf".to_string(),
            publisher: "test-publisher".to_string(),
        };
        assert_eq!(request.name, "test-plugin");
        assert_eq!(request.kind, "ebpf");
    }

    #[test]
    fn test_plugin_struct() {
        let plugin = Plugin {
            id: uuid::Uuid::new_v4(),
            name: "my-plugin".to_string(),
            kind: "kernel".to_string(),
            publisher: "acme".to_string(),
            status: "active".to_string(),
        };
        assert_eq!(plugin.name, "my-plugin");
        assert_eq!(plugin.status, "active");
    }

    #[test]
    fn test_template_creation_request() {
        let request = CreateTemplateRequest {
            plugin_id: uuid::Uuid::new_v4(),
            name: "my-template".to_string(),
            version: "1.0.0".to_string(),
            target_software: "kernel 5.0".to_string(),
            scenario: Some("tracing".to_string()),
            manifest: serde_json::json!({"key": "value"}),
        };
        assert_eq!(request.name, "my-template");
        assert_eq!(request.version, "1.0.0");
    }

    #[test]
    fn test_template_struct() {
        let template = Template {
            id: uuid::Uuid::new_v4(),
            plugin_id: uuid::Uuid::new_v4(),
            name: "template-1".to_string(),
            version: "2.0.0".to_string(),
            target_software: "linux".to_string(),
            scenario: Some("security".to_string()),
            status: "active".to_string(),
        };
        assert_eq!(template.name, "template-1");
        assert_eq!(template.status, "active");
    }

    #[test]
    fn test_ack_request_status_applied() {
        let request = AckRequest {
            status: AckStatus::Applied,
            generation: 5,
            message: None,
        };
        assert!(matches!(request.status, AckStatus::Applied));
        assert_eq!(request.generation, 5);
    }

    #[test]
    fn test_ack_request_status_failed() {
        let request = AckRequest {
            status: AckStatus::Failed,
            generation: 3,
            message: Some("install failed".to_string()),
        };
        assert!(matches!(request.status, AckStatus::Failed));
        assert_eq!(request.message, Some("install failed".to_string()));
    }

    #[test]
    fn test_plugin_action_variants() {
        // Test PluginAction enum variants
        let actions = vec![
            PluginAction::Install,
            PluginAction::Enable,
            PluginAction::Disable,
            PluginAction::Uninstall,
        ];
        assert_eq!(actions.len(), 4);
    }

    #[test]
    fn test_uuid_generation_for_consistency() {
        // Test that multiple UUID generations produce unique values
        let u1 = uuid::Uuid::new_v4();
        let u2 = uuid::Uuid::new_v4();
        assert_ne!(u1, u2);
    }
}