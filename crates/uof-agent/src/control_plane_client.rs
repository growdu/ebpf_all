use std::{collections::BTreeMap, time::Duration};

use reqwest::{Client, StatusCode};
use uuid::Uuid;

use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest, AgentRegisterResponse},
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
            hostname: hostname(),
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
    ) -> anyhow::Result<()> {
        let request = AgentHeartbeatRequest {
            status: status.to_string(),
            health_summary: serde_json::json!({ "status": status }),
            probe_status: probe_status
                .into_iter()
                .map(|probe| serde_json::json!(probe))
                .collect(),
            plugin_status: vec![],
        };

        self.http
            .post(format!("{}/api/v1/agents/{agent_id}/heartbeat", self.base_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn fetch_desired_state(&self, agent_id: Uuid) -> anyhow::Result<Option<DesiredState>> {
        let response = self
            .http
            .get(format!("{}/api/v1/agents/{agent_id}/desired-state", self.base_url))
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
            .post(format!("{}/api/v1/agents/{agent_id}/ack", self.base_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}
