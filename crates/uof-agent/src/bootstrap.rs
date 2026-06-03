use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;
use uof_common::telemetry::init_tracing;
use uuid::Uuid;

use crate::{
    admin_api::{self, AdminState},
    config::AgentConfig,
    control_plane_client::ControlPlaneClient,
    probe_manager::{InMemoryProbeManager, ProbeManager},
};

pub struct AgentApplication {
    config: AgentConfig,
    probe_manager: Arc<InMemoryProbeManager>,
    control_plane_client: ControlPlaneClient,
}

impl AgentApplication {
    pub fn new(config: AgentConfig) -> anyhow::Result<Self> {
        let probe_manager = Arc::new(InMemoryProbeManager::new(&config.baseline_probes));
        let control_plane_client = ControlPlaneClient::new(config.control_plane_endpoint.clone())?;
        Ok(Self {
            config,
            probe_manager,
            control_plane_client,
        })
    }

    async fn bootstrap(&self) -> anyhow::Result<AgentRuntime> {
        init_tracing();

        tracing::info!(
            service = %self.config.service_name,
            endpoint = %self.config.control_plane_endpoint,
            "bootstrapping agent"
        );

        // Initialize event pipeline before enabling probes
        let otlp_endpoint = "http://127.0.0.1:4317";
        if let Err(e) = self.probe_manager.init_event_pipeline(otlp_endpoint).await {
            tracing::warn!(error = %e, "failed to init event pipeline, continuing without OTLP export");
        }

        for probe in &self.config.baseline_probes {
            self.probe_manager.enable_probe(probe).await?;
        }

        let registration = self.control_plane_client.register_agent(&self.config).await?;
        tracing::info!(agent_id = %registration.agent_id, "agent registered");

        Ok(AgentRuntime {
            agent_id: registration.agent_id,
            poll_interval_seconds: registration.poll_interval_seconds.max(5),
            applied_generation: 0,
        })
    }

    pub fn router(&self) -> Router {
        admin_api::router(AdminState {
            probe_manager: self.probe_manager.clone(),
        })
        .layer(TraceLayer::new_for_http())
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let runtime = self.bootstrap().await?;

        let listener = tokio::net::TcpListener::bind(&self.config.admin_bind_addr).await?;
        tracing::info!(
            service = %self.config.service_name,
            addr = %self.config.admin_bind_addr,
            "agent admin api listening"
        );
        let control_plane_loop = self.run_control_plane_loop(runtime);

        tokio::select! {
            result = axum::serve(listener, self.router()) => {
                result?;
            }
            result = control_plane_loop => {
                result?;
            }
        }
        Ok(())
    }

    async fn run_control_plane_loop(&self, mut runtime: AgentRuntime) -> anyhow::Result<()> {
        let interval = std::time::Duration::from_secs(runtime.poll_interval_seconds);

        loop {
            let probe_status = self.probe_manager.list_status().await?;
            self.control_plane_client
                .heartbeat(runtime.agent_id, "running", probe_status)
                .await?;

            if let Some(desired_state) = self
                .control_plane_client
                .fetch_desired_state(runtime.agent_id)
                .await?
            {
                if desired_state.generation > runtime.applied_generation {
                    let apply_result = self.probe_manager.apply_desired_state(&desired_state).await;

                    match apply_result {
                        Ok(()) => {
                            runtime.applied_generation = desired_state.generation;
                            self.control_plane_client
                                .ack_desired_state(
                                    runtime.agent_id,
                                    desired_state.generation,
                                    uof_model::desired_state::AckStatus::Applied,
                                    None,
                                )
                                .await?;
                        }
                        Err(error) => {
                            self.control_plane_client
                                .ack_desired_state(
                                    runtime.agent_id,
                                    desired_state.generation,
                                    uof_model::desired_state::AckStatus::Failed,
                                    Some(error.to_string()),
                                )
                                .await?;
                        }
                    }
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}

struct AgentRuntime {
    agent_id: Uuid,
    poll_interval_seconds: u64,
    applied_generation: i64,
}
