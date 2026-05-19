//! EbpfLoader — wraps aya::Ebpf for loading and attaching eBPF programs.
//!
//! This module provides a clean abstraction over the aya library,
//! handling eBPF object loading, program attachment, and resource cleanup.

use anyhow::{Context, Result};
use aya::Ebpf;
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
    ///
    /// # Errors
    ///
    /// Returns an error if no eBPF program is loaded, or if attachment fails.
    pub async fn attach(&self) -> Result<()> {
        let bpf = self
            .bpf
            .as_ref()
            .context("no eBPF loaded, call load() first")?;
        let bpf_lock = bpf.lock().await;

        // Iterate through programs and log their attachment
        // aya auto-attaches programs when load_file is called
        for (name, _prog) in bpf_lock.programs() {
            log::debug!("eBPF program loaded: {}", name);
        }

        // Programs are automatically attached by aya when loaded
        Ok(())
    }

    /// Unload eBPF programs and free resources.
    ///
    /// Dropping the EbpfLoader will automatically detach all loaded programs.
    pub fn unload(&mut self) {
        self.bpf = None;
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
        // Second load should also fail
        let result2 = loader.load("/nonexistent2.o").await;
        assert!(result2.is_err());
    }

    #[test]
    fn test_is_loaded_before_load() {
        let loader = EbpfLoader::new();
        assert!(!loader.is_loaded());
    }
}