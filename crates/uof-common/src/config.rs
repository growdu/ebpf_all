use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub service_name: String,
    pub bind_addr: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            service_name: "uof-control-api".to_string(),
            bind_addr: "127.0.0.1:8080".to_string(),
        }
    }
}

