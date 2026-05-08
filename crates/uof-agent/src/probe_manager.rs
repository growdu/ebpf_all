use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use uof_common::Result;
use uof_model::desired_state::{DesiredState, PluginAction};
use uof_probe_runtime::{ProbeLifecycleState, ProbeRuntime, RegisteredProbe};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeStatus {
    pub probe_id: String,
    pub state: String,
    pub plugin_id: Option<String>,
}

#[async_trait]
pub trait ProbeManager {
    async fn load_plugin_probes(&self, plugin_id: &str) -> Result<()>;
    async fn enable_probe(&self, probe_id: &str) -> Result<()>;
    async fn disable_probe(&self, probe_id: &str) -> Result<()>;
    async fn list_status(&self) -> Result<Vec<ProbeStatus>>;
}

#[derive(Debug, Clone)]
pub struct InMemoryProbeManager {
    runtime: Arc<RwLock<ProbeRuntime>>,
    plugin_index: Arc<RwLock<BTreeMap<String, Vec<String>>>>,
}

impl InMemoryProbeManager {
    pub fn new(baseline_probes: &[String]) -> Self {
        let mut runtime = ProbeRuntime::default();
        for probe_id in baseline_probes {
            runtime.register(RegisteredProbe {
                probe_id: probe_id.clone(),
                plugin_id: None,
                state: ProbeLifecycleState::Registered,
            });
        }

        Self {
            runtime: Arc::new(RwLock::new(runtime)),
            plugin_index: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub async fn apply_desired_state(&self, desired_state: &DesiredState) -> Result<()> {
        for plugin in &desired_state.plugins {
            let plugin_key = plugin.plugin_id.to_string();

            match plugin.action {
                PluginAction::Install => {
                    self.load_plugin_probes(&plugin_key).await?;
                }
                PluginAction::Enable => {
                    self.load_plugin_probes(&plugin_key).await?;
                    let probe_ids = {
                        let index = self.plugin_index.read().await;
                        index.get(&plugin_key).cloned().unwrap_or_default()
                    };

                    for probe_id in probe_ids {
                        self.enable_probe(&probe_id).await?;
                    }
                }
                PluginAction::Disable => {
                    let probe_ids = {
                        let index = self.plugin_index.read().await;
                        index.get(&plugin_key).cloned().unwrap_or_default()
                    };

                    for probe_id in probe_ids {
                        self.disable_probe(&probe_id).await?;
                    }
                }
                PluginAction::Uninstall => {
                    let probe_ids = {
                        let index = self.plugin_index.read().await;
                        index.get(&plugin_key).cloned().unwrap_or_default()
                    };

                    for probe_id in probe_ids {
                        self.disable_probe(&probe_id).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ProbeManager for InMemoryProbeManager {
    async fn load_plugin_probes(&self, plugin_id: &str) -> Result<()> {
        let mut runtime = self.runtime.write().await;
        let mut index = self.plugin_index.write().await;

        let probes = vec![
            format!("{plugin_id}-syscall"),
            format!("{plugin_id}-io"),
        ];

        for probe_id in &probes {
            runtime.register(RegisteredProbe {
                probe_id: probe_id.clone(),
                plugin_id: Some(plugin_id.to_string()),
                state: ProbeLifecycleState::Loaded,
            });
        }

        index.insert(plugin_id.to_string(), probes);
        Ok(())
    }

    async fn enable_probe(&self, probe_id: &str) -> Result<()> {
        let mut runtime = self.runtime.write().await;
        runtime.transition(probe_id, ProbeLifecycleState::Running)
    }

    async fn disable_probe(&self, probe_id: &str) -> Result<()> {
        let mut runtime = self.runtime.write().await;
        runtime.transition(probe_id, ProbeLifecycleState::Detached)
    }

    async fn list_status(&self) -> Result<Vec<ProbeStatus>> {
        let runtime = self.runtime.read().await;
        Ok(runtime
            .list()
            .into_iter()
            .map(|probe| ProbeStatus {
                probe_id: probe.probe_id,
                state: probe.state.to_string(),
                plugin_id: probe.plugin_id,
            })
            .collect())
    }
}
