use std::{collections::BTreeMap, time::Duration};

use reqwest::{Client, StatusCode};
use uuid::Uuid;

use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest, AgentRegisterResponse, MetricPayload},
    desired_state::{AckRequest, AckStatus, DesiredState},
};

use crate::{config::AgentConfig, probe_manager::ProbeStatus};

#[derive(Clone)]
pub struct ControlPlaneClient {
    base_url: String,
    http: Client,
}

impl ControlPlaneClient {
    pub fn new(base_url: String) -> anyhow::Result<Self> {
        let http = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self { base_url, http })
    }

    pub async fn register_agent(&self, config: &AgentConfig) -> anyhow::Result<AgentRegisterResponse> {
        let request = AgentRegisterRequest {
            hostname: Self::hostname(),
            node_name: None,
            ip: None,
            kernel_version: std::env::consts::OS.to_string(),
            os_release: None,
            arch: std::env::consts::ARCH.to_string(),
            labels: BTreeMap::from([
                ("service".to_string(), config.service_name.clone()),
                ("role".to_string(), "agent".to_string()),
            ]),
            capabilities: serde_json::json!({
                "ebpf": true,
                "ringbuf": true,
                "kprobe": true,
                "uprobe": true
            }),
        };

        let response = self
            .http
            .post(format!("{}/api/v1/agents/register", self.base_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn heartbeat(
        &self,
        agent_id: Uuid,
        status: &str,
        probe_status: Vec<ProbeStatus>,
        metrics: Vec<MetricPayload>,
    ) -> anyhow::Result<()> {
        let request = AgentHeartbeatRequest {
            status: status.to_string(),
            health_summary: serde_json::json!({ "status": status }),
            probe_status: probe_status
                .into_iter()
                .map(|probe| serde_json::json!(probe))
                .collect(),
            plugin_status: vec![],
            metrics,
        };

        self.http
            .post(format!("{}/api/v1/agents/heartbeat?id={}", self.base_url, agent_id))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn fetch_desired_state(&self, agent_id: Uuid) -> anyhow::Result<Option<DesiredState>> {
        let response = self
            .http
            .get(format!("{}/api/v1/agents/desired-state?id={}", self.base_url, agent_id))
            .send()
            .await?;

        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        Ok(Some(response.error_for_status()?.json().await?))
    }

    pub async fn ack_desired_state(
        &self,
        agent_id: Uuid,
        generation: i64,
        status: AckStatus,
        message: Option<String>,
    ) -> anyhow::Result<()> {
        let request = AckRequest {
            generation,
            status,
            message,
        };

        self.http
            .post(format!("{}/api/v1/agents/ack?id={}", self.base_url, agent_id))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Download a plugin artifact from the control plane for the given plugin_id.
    /// Returns raw tar.gz bytes.
    pub async fn download_plugin_artifact(
        &self,
        agent_id: Uuid,
        plugin_id: Uuid,
        version: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let url = format!(
            "{}/api/v1/agents/{}/plugins/{}/artifact?version={}",
            self.base_url, agent_id, plugin_id, version
        );

        tracing::info!(url, "downloading plugin artifact from control plane");
        let resp = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?;

        let bytes = resp.bytes().await?.to_vec();
        tracing::info!(url, bytes = bytes.len(), "plugin artifact downloaded");
        Ok(bytes)
    }

    fn hostname() -> String {
        std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string())
    }
}
