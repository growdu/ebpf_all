//! Probe loader module - loads and manages eBPF probes

use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use crate::process_discovery::ProcessDiscovery;
use crate::symbol_resolver::SymbolResolver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedProbeInfo {
    pub probe_id: String,
    pub probe_type: ProbeType,
    pub target: String,
    pub address: Option<u64>,
    pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    KProbe,
    KRetProbe,
    UProbe,
    URetProbe,
    TracePoint,
}

pub struct ProbeLoader {
    process_discovery: ProcessDiscovery,
    symbol_resolver: SymbolResolver,
    loaded_probes: HashMap<String, LoadedProbeInfo>,
}

impl ProbeLoader {
    pub fn new() -> Self {
        Self {
            process_discovery: ProcessDiscovery::new(),
            symbol_resolver: SymbolResolver::new(),
            loaded_probes: HashMap::new(),
        }
    }

    pub fn load_uprobe(&mut self, probe_id: &str, process_name: &str, symbol: &str) -> Result<LoadedProbeInfo> {
        let pids = self.process_discovery.find_pids(process_name)
            .context("Failed to discover processes")?;
        if pids.is_empty() {
            anyhow::bail!("No processes found for '{}'", process_name);
        }
        let binary_path = self.process_discovery.get_binary_path(pids[0])?;
        let addr = self.symbol_resolver.resolve(pids[0], &binary_path, symbol)?;
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::UProbe,
            target: binary_path,
            address: Some(addr),
            pids: pids.clone(),
        };
        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    pub fn load_kprobe(&mut self, probe_id: &str, function: &str) -> Result<LoadedProbeInfo> {
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::KProbe,
            target: function.to_string(),
            address: None,
            pids: vec![],
        };
        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    pub fn load_tracepoint(&mut self, probe_id: &str, category: &str, name: &str) -> Result<LoadedProbeInfo> {
        let target = format!("{}/{}", category, name);
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::TracePoint,
            target,
            address: None,
            pids: vec![],
        };
        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    pub fn unload(&mut self, probe_id: &str) -> Result<()> {
        if self.loaded_probes.remove(probe_id).is_none() {
            anyhow::bail!("Probe '{}' not found", probe_id);
        }
        Ok(())
    }

    pub fn list_loaded(&self) -> Vec<LoadedProbeInfo> {
        self.loaded_probes.values().cloned().collect()
    }
}

impl Default for ProbeLoader {
    fn default() -> Self { Self::new() }
}