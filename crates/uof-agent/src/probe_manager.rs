use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use flate2::read::GzDecoder;
use tar::Archive;
use tokio::sync::RwLock;

use uof_common::{Result, UofError};
use uof_model::desired_state::{DesiredState, PluginAction};
use uof_probe_runtime::{ProbeLifecycleState, ProbeRuntime, RegisteredProbe};

use crate::event_pipeline::EventPipeline;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeStatus {
    pub probe_id: String,
    pub state: String,
    pub plugin_id: Option<String>,
}

#[async_trait]
pub trait ProbeManager: Send + Sync {
    async fn load_plugin_probes(&self, plugin_id: &str) -> Result<()>;
    async fn load_plugin_artifact(
        &self,
        plugin_id: &str,
        artifact_bytes: Vec<u8>,
        expected_digest: Option<&str>,
    ) -> Result<PathBuf>;
    async fn enable_probe(&self, probe_id: &str) -> Result<()>;
    async fn disable_probe(&self, probe_id: &str) -> Result<()>;
    async fn list_status(&self) -> Result<Vec<ProbeStatus>>;
}

#[derive(Clone)]
pub struct InMemoryProbeManager {
    runtime: Arc<RwLock<ProbeRuntime>>,
    plugin_index: Arc<RwLock<BTreeMap<String, Vec<String>>>>,
    artifact_dirs: Arc<RwLock<BTreeMap<String, PathBuf>>>,
    pipeline_initialized: Arc<RwLock<bool>>,
}

impl InMemoryProbeManager {
    pub fn new(baseline_probes: &[String]) -> Self {
        let mut runtime = ProbeRuntime::default();
        for probe_id in baseline_probes {
            runtime.register(RegisteredProbe {
                probe_id: probe_id.clone(),
                plugin_id: None,
                state: ProbeLifecycleState::Registered,
                degrade_reason: None,
            });
        }

        Self {
            runtime: Arc::new(RwLock::new(runtime)),
            plugin_index: Arc::new(RwLock::new(BTreeMap::new())),
            artifact_dirs: Arc::new(RwLock::new(BTreeMap::new())),
            pipeline_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the event pipeline with OTLP export.
    pub async fn init_event_pipeline(&self, otlp_endpoint: &str) -> Result<()> {
        use uof_exporter_otlp::OtlpConfig;
        use tokio::sync::mpsc;

        let config = OtlpConfig::new()
            .with_endpoint(otlp_endpoint.to_string());

        let (pipeline, sender, receiver) = EventPipeline::with_config(&config)
            .map_err(|e| anyhow::anyhow!("failed to create event pipeline: {}", e))?;

        // Create a channel to connect handlers to the pipeline
        let (handler_tx, mut handler_rx) = mpsc::channel::<uof_probe_runtime::ProbeEvent>(100);
        let sender_clone = sender.clone();

        // Spawn a task that forwards events from handlers to the pipeline
        tokio::spawn(async move {
            while let Some(event) = handler_rx.recv().await {
                let pipeline_event = crate::event_pipeline::PipelineEvent::from_probe_event(event);
                if sender_clone.send(pipeline_event).await.is_err() {
                    tracing::info!("handler event channel closed");
                    break;
                }
            }
        });

        // Create a simple handler that sends to our channel
        let handler = TestEventHandler { sender: handler_tx };
        {
            let mut runtime = self.runtime.write().await;
            runtime.set_event_handler(handler);
            // Don't call spawn_event_loop since state is Idle - just keep the handler
        }

        let mut started_lock = self.pipeline_initialized.write().await;
        *started_lock = true;

        // Start the pipeline processing loop (pipeline is consumed here)
        pipeline.start(receiver);

        // Start test event generator to simulate probe events
        // (real BPF programs will replace this in production)
        crate::event_pipeline::generate_test_events(sender);

        Ok(())
    }

    /// Full lifecycle: verify digest → unpack tar.gz → parse manifest → register probes.
    ///
    /// Returns the temp directory path where the plugin was extracted.
    pub async fn load_plugin_artifact(
        &self,
        plugin_id: &str,
        artifact_bytes: Vec<u8>,
        expected_digest: Option<&str>,
    ) -> Result<PathBuf> {
        // 1. Verify digest if provided
        if let Some(expected) = expected_digest {
            let actual = uof_plugin_sdk::digest_bytes(&artifact_bytes);
            if !actual.starts_with(expected.trim_start_matches("sha256:")) {
                return Err(UofError::Internal(format!(
                    "artifact digest mismatch: expected={expected} actual=sha256:{actual}"
                )).into());
            }
            tracing::info!(plugin_id, bytes = artifact_bytes.len(), "artifact digest verified");
        }

        // 2. Unpack tar.gz into a temp directory
        let temp_dir = tempfile::tempdir()
            .map_err(|e| UofError::Internal(format!("failed to create temp dir: {e}")))?;

        {
            let decoder = GzDecoder::new(&artifact_bytes[..]);
            let mut archive = Archive::new(decoder);
            archive.unpack(temp_dir.path())
                .map_err(|e| UofError::Internal(format!("failed to unpack artifact: {e}")))?;
        }

        tracing::info!(plugin_id, path = %temp_dir.path().display(), "plugin artifact unpacked");

        // 3. Attempt to load eBPF programs (stub runtime — no-op in this environment)
        let ebpf_path = temp_dir.path().join("artifacts").join("ebpf");
        let runtime_path = ebpf_path.to_string_lossy().to_string();
        {
            let mut runtime = self.runtime.write().await;
            if let Err(e) = runtime.load(&runtime_path).await {
                tracing::warn!(plugin_id, error = %e, "runtime.load() error (expected in stub mode)");
            }
        }

        // 4. Parse manifest.yaml and register probes
        let manifest_path = temp_dir.path().join("manifest.yaml");
        if manifest_path.is_file() {
            let yaml = tokio::fs::read_to_string(&manifest_path).await
                .map_err(|e| UofError::Internal(format!("failed to read manifest: {e}")))?;

            match uof_plugin_sdk::PluginManifest::from_yaml(&yaml) {
                Ok(manifest) => {
                    self.register_manifest_probes(plugin_id, &manifest).await?;
                }
                Err(e) => {
                    tracing::warn!(plugin_id, error = %e, "failed to parse manifest.yaml");
                    self.register_default_probes(plugin_id).await?;
                }
            }
        } else {
            tracing::info!(plugin_id, "no manifest.yaml found, using default probe names");
            self.register_default_probes(plugin_id).await?;
        }

        // 5. Store temp dir path so it persists until uninstall
        {
            let mut dirs = self.artifact_dirs.write().await;
            dirs.insert(plugin_id.to_string(), temp_dir.path().to_path_buf());
        }

        Ok(temp_dir.path().to_path_buf())
    }

    async fn register_manifest_probes(
        &self,
        plugin_id: &str,
        manifest: &uof_plugin_sdk::PluginManifest,
    ) -> Result<()> {
        let mut probe_ids = Vec::new();
        let mut runtime = self.runtime.write().await;

        for probe in &manifest.probes {
            let probe_id = probe.id.clone();
            runtime.register(RegisteredProbe {
                probe_id: probe_id.clone(),
                plugin_id: Some(plugin_id.to_string()),
                state: ProbeLifecycleState::Loaded,
                degrade_reason: None,
            });
            probe_ids.push(probe_id);
        }

        let mut index = self.plugin_index.write().await;
        index.insert(plugin_id.to_string(), probe_ids);
        tracing::info!(plugin_id, probes = manifest.probes.len(), "probes registered from manifest");
        Ok(())
    }

    async fn register_default_probes(&self, plugin_id: &str) -> Result<()> {
        let probes = vec![
            format!("{plugin_id}-syscall"),
            format!("{plugin_id}-io"),
        ];
        let mut runtime = self.runtime.write().await;
        for probe_id in &probes {
            runtime.register(RegisteredProbe {
                probe_id: probe_id.clone(),
                plugin_id: Some(plugin_id.to_string()),
                state: ProbeLifecycleState::Loaded,
                degrade_reason: None,
            });
        }
        let mut index = self.plugin_index.write().await;
        index.insert(plugin_id.to_string(), probes);
        Ok(())
    }

    pub async fn apply_desired_state(&self, desired_state: &DesiredState) -> Result<()> {
        for plugin in &desired_state.plugins {
            let plugin_key = plugin.plugin_id.to_string();

            match plugin.action {
                PluginAction::Install | PluginAction::Enable => {
                    if let Some(ref url) = plugin.artifact_url {
                        let bytes = download_artifact(url).await?;
                        let digest = plugin.artifact_digest.as_deref();
                        self.load_plugin_artifact(&plugin_key, bytes, digest).await?;
                    }
                    // Transition registered probes to Running
                    let probe_ids = {
                        let index = self.plugin_index.read().await;
                        index.get(&plugin_key).cloned().unwrap_or_default()
                    };
                    for probe_id in probe_ids {
                        let mut runtime = self.runtime.write().await;
                        let _ = runtime.transition(&probe_id, ProbeLifecycleState::Running);
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
                    {
                        let mut dirs = self.artifact_dirs.write().await;
                        dirs.remove(&plugin_key);
                    }
                    {
                        let mut index = self.plugin_index.write().await;
                        index.remove(&plugin_key);
                    }
                }
            }
        }

        Ok(())
    }
}

async fn download_artifact(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| UofError::Internal(format!("failed to build HTTP client: {e}")))?;

    tracing::info!(url, "downloading plugin artifact");
    let resp = client.get(url).send().await
        .map_err(|e| UofError::Internal(format!("download failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(UofError::Internal(format!(
            "artifact download failed: HTTP {}", resp.status()
        )).into());
    }

    let bytes = resp.bytes().await
        .map_err(|e| UofError::Internal(format!("failed to read artifact bytes: {e}")))?
        .to_vec();

    tracing::info!(url, bytes = bytes.len(), "artifact downloaded");
    Ok(bytes)
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
                degrade_reason: None,
            });
        }

        index.insert(plugin_id.to_string(), probes);
        Ok(())
    }

    async fn load_plugin_artifact(
        &self,
        plugin_id: &str,
        artifact_bytes: Vec<u8>,
        expected_digest: Option<&str>,
    ) -> Result<PathBuf> {
        self.load_plugin_artifact(plugin_id, artifact_bytes, expected_digest).await
    }

    async fn enable_probe(&self, probe_id: &str) -> Result<()> {
        let mut runtime = self.runtime.write().await;
        let probes = runtime.list();
        let probe_state = probes.iter().find(|p| p.probe_id == probe_id).map(|p| p.state);
        match probe_state {
            Some(ProbeLifecycleState::Registered) => {
                runtime.transition(probe_id, ProbeLifecycleState::Loaded)?;
                runtime.transition(probe_id, ProbeLifecycleState::Attached)?;
                runtime.transition(probe_id, ProbeLifecycleState::Running)?;
            }
            Some(ProbeLifecycleState::Loaded) => {
                runtime.transition(probe_id, ProbeLifecycleState::Attached)?;
                runtime.transition(probe_id, ProbeLifecycleState::Running)?;
            }
            Some(ProbeLifecycleState::Attached) => {
                runtime.transition(probe_id, ProbeLifecycleState::Running)?;
            }
            Some(ProbeLifecycleState::Running) => {
                // already running, no-op
            }
            _ => {}
        }
        Ok(())
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

/// A simple event handler that forwards events to a channel.
struct TestEventHandler {
    sender: tokio::sync::mpsc::Sender<uof_probe_runtime::ProbeEvent>,
}

impl uof_probe_runtime::EventHandler for TestEventHandler {
    fn on_event(&self, event: uof_probe_runtime::ProbeEvent) {
        if self.sender.try_send(event).is_err() {
            tracing::debug!("event handler channel full, dropping event");
        }
    }
}
