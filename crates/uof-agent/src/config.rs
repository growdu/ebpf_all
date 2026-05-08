use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub service_name: String,
    pub admin_bind_addr: String,
    pub control_plane_endpoint: String,
    pub plugin_root: String,
    pub baseline_probes: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            service_name: "uof-agent".to_string(),
            admin_bind_addr: "127.0.0.1:18080".to_string(),
            control_plane_endpoint: "http://127.0.0.1:8080".to_string(),
            plugin_root: "/var/lib/uof/plugins".to_string(),
            baseline_probes: vec![
                "syscall".to_string(),
                "sched".to_string(),
                "io".to_string(),
            ],
        }
    }
}
