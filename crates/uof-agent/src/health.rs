use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AgentHealthSnapshot {
    pub status: &'static str,
    pub loaded_plugins: usize,
    pub active_probes: usize,
}
