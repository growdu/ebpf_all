//! Process discovery module - finds target processes by name

use std::collections::HashSet;
use std::path::Path;
use anyhow::{Result, Context};

pub struct ProcessDiscovery;

impl ProcessDiscovery {
    pub fn new() -> Self { Self }

    pub fn find_pids(&self, process_name: &str) -> Result<Vec<u32>> {
        let mut pids = HashSet::new();
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("invalid proc entry"))?;
            if !file_name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let comm_path = path.join("comm");
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                if comm.trim() == process_name {
                    let pid: u32 = file_name.parse()
                        .with_context(|| format!("invalid PID: {}", file_name))?;
                    pids.insert(pid);
                }
            }
        }
        Ok(pids.into_iter().collect())
    }

    pub fn process_exists(&self, pid: u32) -> bool {
        Path::new(&format!("/proc/{}", pid)).exists()
    }

    pub fn get_binary_path(&self, pid: u32) -> Result<String> {
        let exe_link = format!("/proc/{}/exe", pid);
        let target = std::fs::read_link(&exe_link)?;
        let target_str = target.to_string_lossy();
        let target_str = target_str.strip_suffix(" (deleted)")
            .unwrap_or(&target_str);
        Ok(target_str.to_string())
    }
}

impl Default for ProcessDiscovery {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_pids() {
        let discovery = ProcessDiscovery::new();
        let pids = discovery.find_pids("bash").unwrap();
        assert!(!pids.is_empty(), "Should find at least one bash process");
    }
}