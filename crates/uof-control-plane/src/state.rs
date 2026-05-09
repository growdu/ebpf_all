use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::RwLock;
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

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<RwLock<StateStore>>,
    /// Base URL of this control plane (used to construct artifact download URLs).
    base_url: String,
}

#[derive(Debug, Default)]
struct StateStore {
    agents: BTreeMap<Uuid, AgentRecord>,
    plugins: BTreeMap<Uuid, PluginRecord>,
    templates: BTreeMap<Uuid, Template>,
    template_bindings: BTreeMap<Uuid, TemplateBinding>,
}

#[derive(Debug, Clone)]
struct AgentRecord {
    desired_state: DesiredState,
    last_heartbeat_status: Option<String>,
    acked_generation: i64,
}

#[derive(Debug, Clone)]
struct PluginRecord {
    plugin: Plugin,
    default_version_id: Option<Uuid>,
    versions: Vec<PluginVersion>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new("http://127.0.0.1:8080".to_string())
    }
}

impl AppState {
    pub fn new(base_url: String) -> Self {
        let plugin_id = Uuid::new_v4();
        let default_plugin = Plugin {
            id: plugin_id,
            name: "postgres-observability".to_string(),
            kind: "template".to_string(),
            publisher: "uof".to_string(),
            status: "draft".to_string(),
        };

        let store = StateStore {
            agents: BTreeMap::new(),
            plugins: BTreeMap::from([(
                plugin_id,
                PluginRecord {
                    plugin: default_plugin,
                    default_version_id: None,
                    versions: vec![],
                },
            )]),
            templates: BTreeMap::new(),
            template_bindings: BTreeMap::new(),
        };

        Self {
            inner: Arc::new(RwLock::new(store)),
            base_url,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn register_agent(&self, request: AgentRegisterRequest) -> AgentRegisterResponse {
        let mut state = self.inner.write().await;
        let agent_id = Uuid::new_v4();
        let default_plugin = state.plugins.values().next().map(|record| record.plugin.id);

        let desired_state = DesiredState {
            generation: 1,
            plugins: default_plugin
                .map(|plugin_id| PluginActivation {
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

        state.agents.insert(
            agent_id,
            AgentRecord {
                desired_state,
                last_heartbeat_status: Some(format!("registered:{}", request.hostname)),
                acked_generation: 0,
            },
        );
        tracing::info!(agent_id = %agent_id, total_agents = state.agents.len(), "agent registered in state");

        AgentRegisterResponse {
            agent_id,
            status: format!("registered:{}", request.hostname),
            poll_interval_seconds: 15,
        }
    }

    pub async fn heartbeat(&self, agent_id: Uuid, request: AgentHeartbeatRequest) -> bool {
        let mut state = self.inner.write().await;
        tracing::info!(agent_id = %agent_id, known_agents = ?state.agents.keys().collect::<Vec<_>>(), "heartbeat received");
        if let Some(agent) = state.agents.get_mut(&agent_id) {
            agent.last_heartbeat_status = Some(request.status);
            tracing::info!(agent_id = %agent_id, "heartbeat accepted");
            true
        } else {
            tracing::warn!(agent_id = %agent_id, "heartbeat rejected: agent not found");
            false
        }
    }

    pub async fn desired_state(&self, agent_id: Uuid) -> Option<DesiredState> {
        let state = self.inner.read().await;
        let agent = state.agents.get(&agent_id)?;
        if agent.acked_generation >= agent.desired_state.generation {
            None
        } else {
            Some(agent.desired_state.clone())
        }
    }

    pub async fn ack_desired_state(&self, agent_id: Uuid, request: AckRequest) -> bool {
        let mut state = self.inner.write().await;
        if let Some(agent) = state.agents.get_mut(&agent_id) {
            match request.status {
                AckStatus::Applied => {
                    agent.acked_generation = request.generation;
                }
                AckStatus::Failed => {
                    agent.last_heartbeat_status = request.message.or_else(|| Some("apply failed".to_string()));
                }
            }
            true
        } else {
            false
        }
    }

    pub async fn list_plugins(&self) -> Vec<Plugin> {
        let state = self.inner.read().await;
        state
            .plugins
            .values()
            .map(|record| record.plugin.clone())
            .collect()
    }

    pub async fn create_plugin(&self, request: CreatePluginRequest) -> Plugin {
        let mut state = self.inner.write().await;
        let plugin = Plugin {
            id: Uuid::new_v4(),
            name: request.name,
            kind: request.kind,
            publisher: request.publisher,
            status: "draft".to_string(),
        };

        state.plugins.insert(
            plugin.id,
            PluginRecord {
                plugin: plugin.clone(),
                default_version_id: None,
                versions: vec![],
            },
        );

        plugin
    }

    pub async fn get_plugin(&self, plugin_id: Uuid) -> Option<PluginDetail> {
        let state = self.inner.read().await;
        state.plugins.get(&plugin_id).map(|record| PluginDetail {
            plugin: record.plugin.clone(),
            default_version_id: record.default_version_id,
            versions: record.versions.clone(),
        })
    }

    pub async fn create_plugin_version(
        &self,
        plugin_id: Uuid,
        request: CreatePluginVersionRequest,
    ) -> Option<PluginVersion> {
        let mut state = self.inner.write().await;
        let record = state.plugins.get_mut(&plugin_id)?;

        let version = PluginVersion {
            id: Uuid::new_v4(),
            plugin_id,
            version: request.version,
            digest: request.digest,
            oci_ref: request.oci_ref,
            signature_status: "pending".to_string(),
            published: false,
        };

        record.versions.push(version.clone());
        Some(version)
    }


    /// Look up a plugin version by its version string.
    pub async fn find_version_by_string(&self, plugin_id: Uuid, version: &str) -> Option<(Uuid, String)> {
        let state = self.inner.read().await;
        let record = state.plugins.get(&plugin_id)?;
        record.versions.iter()
            .find(|v| v.version == version)
            .map(|v| (v.id, v.version.clone()))
    }

    pub async fn release_plugin_version(
        &self,
        plugin_id: Uuid,
        request: ReleasePluginRequest,
    ) -> bool {
        let mut state = self.inner.write().await;
        let Some(record) = state.plugins.get_mut(&plugin_id) else {
            return false;
        };

        let mut released_version_id = None;
        for version in &mut record.versions {
            if version.version == request.version {
                version.published = true;
                released_version_id = Some(version.id);
            }
        }

        if request.make_default.unwrap_or(false) {
            record.default_version_id = released_version_id;
        }

        released_version_id.is_some()
    }

    /// Update desired_state for all agents to include the artifact_url for the newly released version.
    pub async fn notify_agents_plugin_released(
        &self,
        plugin_id: Uuid,
        version_id: Uuid,
        version_string: String,
    ) {
        let artifact_url = self.plugin_artifact_url(plugin_id, version_id).await;
        let mut state = self.inner.write().await;
        for agent in state.agents.values_mut() {
            // Find and update the matching plugin in desired_state
            for plugin in &mut agent.desired_state.plugins {
                if plugin.plugin_id == plugin_id {
                    plugin.artifact_url = artifact_url.clone();
                    plugin.version = version_string.clone();
                    agent.desired_state.generation += 1;
                    tracing::info!(plugin_id = %plugin_id, generation = agent.desired_state.generation, "bumped desired_state generation");
                }
            }
        }
    }

    /// Build the artifact download URL for a specific plugin version.
    pub async fn plugin_artifact_url(
        &self,
        plugin_id: Uuid,
        version_id: Uuid,
    ) -> Option<String> {
        let state = self.inner.read().await;
        let record = state.plugins.get(&plugin_id)?;
        let version = record.versions.iter().find(|v| v.id == version_id)?;
        // If oci_ref is already an HTTP URL, return it directly;
        // otherwise construct the control-plane proxy URL.
        if version.oci_ref.starts_with("http://") || version.oci_ref.starts_with("https://") {
            Some(version.oci_ref.clone())
        } else {
            // Control plane will proxy the pull request to the OCI registry
            Some(format!(
                "{}/api/v1/plugins/{}/versions/{}/artifact",
                self.base_url, plugin_id, version_id
            ))
        }
    }

    /// Get the oci_ref stored for a specific plugin version.
    pub async fn get_version_oci_ref(&self, plugin_id: Uuid, version_id: Uuid) -> Option<String> {
        let state = self.inner.read().await;
        let record = state.plugins.get(&plugin_id)?;
        let version = record.versions.iter().find(|v| v.id == version_id)?;
        Some(version.oci_ref.clone())
    }

    pub async fn list_templates(&self) -> Vec<Template> {
        let state = self.inner.read().await;
        state.templates.values().cloned().collect()
    }

    pub async fn create_template(&self, request: CreateTemplateRequest) -> Template {
        let mut state = self.inner.write().await;
        let template = Template {
            id: Uuid::new_v4(),
            plugin_id: request.plugin_id,
            name: request.name,
            version: request.version,
            target_software: request.target_software,
            scenario: request.scenario,
            status: "active".to_string(),
        };

        state.templates.insert(template.id, template.clone());
        template
    }

    pub async fn create_template_binding(&self, request: CreateTemplateBindingRequest) -> TemplateBinding {
        let mut state = self.inner.write().await;
        let binding = TemplateBinding {
            id: Uuid::new_v4(),
            template_id: request.template_id,
            selector: request.selector,
            target: request.target,
            policy: request.policy.unwrap_or_else(|| serde_json::json!({})),
            enabled: request.enabled.unwrap_or(true),
        };

        state.template_bindings.insert(binding.id, binding.clone());
        binding
    }

    pub async fn delete_template_binding(&self, binding_id: Uuid) -> bool {
        let mut state = self.inner.write().await;
        state.template_bindings.remove(&binding_id).is_some()
    }
}
