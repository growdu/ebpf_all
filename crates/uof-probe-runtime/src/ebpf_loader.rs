//! EbpfLoader — wraps aya::Ebpf for loading and attaching eBPF programs.
//!
//! This module provides a clean abstraction over the aya library,
//! handling eBPF object loading, program attachment, and resource cleanup.

use log::{info, debug};
use std::ffi::OsStr;
use anyhow::{Context, Result};
use aya::{Ebpf, programs::{Program}};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wraps a loaded eBPF program, providing high-level load/attach/detach operations.
#[derive(Debug, Clone)]
pub struct EbpfLoader {
    bpf: Option<Arc<Mutex<Ebpf>>>,
}

impl EbpfLoader {
    /// Create a new empty loader.
    pub fn new() -> Self {
        Self { bpf: None }
    }

    /// Load an eBPF object file into the kernel via aya.
    ///
    /// The path should point to a compiled eBPF object file (`.o`).
    /// On success, the loaded `Ebpf` instance is stored and can be accessed
    /// via [`EbpfLoader::bpf_mut()`].
    pub async fn load(&mut self, path: &str) -> Result<()> {
        let bpf = Ebpf::load_file(path)
            .with_context(|| format!("failed to load eBPF file: {path}"))?;
        self.bpf = Some(Arc::new(Mutex::new(bpf)));
        Ok(())
    }

    /// Returns a shared reference to the loaded Ebpf instance wrapped in Arc<Mutex>.
    ///
    /// This is useful for passing to the ring buffer consumer from within
    /// a spawned Tokio task.
    pub fn bpf_arc(&self) -> Option<Arc<Mutex<Ebpf>>> {
        self.bpf.clone()
    }

    /// Returns true if an eBPF program has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.bpf.is_some()
    }

    /// Attach all loaded eBPF programs to their respective kernel hook points.
    ///
    /// This iterates through all loaded programs and attaches them using
    /// the appropriate attach method based on program type.
    pub async fn attach(&self) -> Result<()> {
        let bpf = self.bpf.as_ref().context("no eBPF loaded, call load() first")?;
        let mut bpf_lock = bpf.lock().await;

        for (name, prog) in bpf_lock.programs_mut() {
            match prog {
                Program::KProbe(kp) => {
                    kp.load().map_err(|e| anyhow::anyhow!("kprobe load failed: {}", e))?;
                    let fn_name = Self::probe_fn_name(name);
                    kp.attach(OsStr::new(&fn_name), 0)
                        .map_err(|e| anyhow::anyhow!("kprobe attach failed for {}: {}", name, e))?;
                    info!("attached kprobe {} to {}", name, fn_name);
                }
                Program::TracePoint(tp) => {
                    tp.load().map_err(|e| anyhow::anyhow!("tracepoint load failed: {}", e))?;
                    let (category, tp_name) = Self::tracepoint_parts(name)?;
                    tp.attach(&category, &tp_name)
                        .map_err(|e| anyhow::anyhow!("tracepoint attach failed for {}: {}", name, e))?;
                    info!("attached tracepoint {} to {}/{}", name, category, tp_name);
                }
                Program::UProbe(up) => {
                    up.load().map_err(|e| anyhow::anyhow!("uprobe load failed: {}", e))?;
                    let (target, fn_name, offset) = Self::uprobe_parts(name)?;
                    up.attach(Some(&fn_name), offset, &target, None)
                        .map_err(|e| anyhow::anyhow!("uprobe attach failed for {}: {}", name, e))?;
                    info!("attached uprobe {} to {}:{}", name, target, fn_name);
                }
                _ => {
                    debug!("skipping program {} (type not handled for attachment)", name);
                }
            }
        }
        Ok(())
    }

    /// Unload eBPF programs and free resources.
    ///
    /// Dropping the EbpfLoader will automatically detach all loaded programs.
    pub fn unload(&mut self) {
        self.bpf = None;
    }

    fn probe_fn_name(name: &str) -> String {
        name.trim_start_matches("handle_")
            .replace("_entry", "")
            .replace("_exit", "")
            .to_lowercase()
    }

    fn tracepoint_parts(name: &str) -> Result<(String, String), anyhow::Error> {
        let remaining = name.trim_start_matches("handle_");
        if let Some(idx) = remaining.find('_') {
            let cat = &remaining[..idx];
            let tp_name = &remaining[idx+1..];
            Ok((cat.to_string(), tp_name.to_string()))
        } else {
            Ok(("scheduler".to_string(), remaining.to_string()))
        }
    }

    fn uprobe_parts(_name: &str) -> Result<(String, String, u64), anyhow::Error> {
        // Format: handle_<binary>_<symbol>_<offset>
        // Default to libc malloc for now
        Ok((
            "/usr/lib/x86_64-linux-gnu/libc.so.6".to_string(),
            "malloc".to_string(),
            0,
        ))
    }
}

impl Default for EbpfLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let mut loader = EbpfLoader::new();
        let result = loader.load("/nonexistent/path/to/bpf.o").await;
        assert!(result.is_err());
        assert!(!loader.is_loaded());
    }

    #[tokio::test]
    async fn test_load_twice() {
        let mut loader = EbpfLoader::new();
        let result = loader.load("/nonexistent.o").await;
        assert!(result.is_err());
        let result2 = loader.load("/nonexistent2.o").await;
        assert!(result2.is_err());
    }

    #[test]
    fn test_is_loaded_before_load() {
        let loader = EbpfLoader::new();
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_probe_fn_name() {
        assert_eq!(EbpfLoader::probe_fn_name("handle_read_entry"), "read");
        assert_eq!(EbpfLoader::probe_fn_name("handle_write_exit"), "write");
        assert_eq!(EbpfLoader::probe_fn_name("handle_open"), "open");
    }

    #[test]
    fn test_tracepoint_parts() {
        let (cat, name) = EbpfLoader::tracepoint_parts("handle_sched_switch").unwrap();
        assert_eq!(cat, "sched");
        assert_eq!(name, "switch");

        let (cat, name) = EbpfLoader::tracepoint_parts("handle_block_rq_insert").unwrap();
        assert_eq!(cat, "block");
        assert_eq!(name, "rq_insert");
    }
}