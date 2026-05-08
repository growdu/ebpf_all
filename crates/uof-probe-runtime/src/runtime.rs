use serde::{Deserialize, Serialize};

use uof_common::{Result, UofError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeLifecycleState {
    Registered,
    Loaded,
    Attached,
    Running,
    Draining,
    Detached,
    Unloaded,
}

impl std::fmt::Display for ProbeLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Registered => "registered",
            Self::Loaded => "loaded",
            Self::Attached => "attached",
            Self::Running => "running",
            Self::Draining => "draining",
            Self::Detached => "detached",
            Self::Unloaded => "unloaded",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredProbe {
    pub probe_id: String,
    pub plugin_id: Option<String>,
    pub state: ProbeLifecycleState,
}

#[derive(Debug, Clone, Default)]
pub struct ProbeRuntime {
    probes: Vec<RegisteredProbe>,
}

impl ProbeRuntime {
    pub fn register(&mut self, probe: RegisteredProbe) {
        self.probes.retain(|existing| existing.probe_id != probe.probe_id);
        self.probes.push(probe);
    }

    pub fn transition(&mut self, probe_id: &str, next: ProbeLifecycleState) -> Result<()> {
        let probe = self
            .probes
            .iter_mut()
            .find(|probe| probe.probe_id == probe_id)
            .ok_or_else(|| UofError::Internal(format!("probe not found: {probe_id}")))?;
        probe.state = next;
        Ok(())
    }

    pub fn list(&self) -> Vec<RegisteredProbe> {
        self.probes.clone()
    }
}
