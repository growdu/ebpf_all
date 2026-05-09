//! Plugin bundling and packing utilities.

use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::digest::digest_bytes;
use crate::error::{PluginError, PluginResult};
use crate::manifest::PluginManifest;

#[derive(Debug)]
pub struct PluginPackager {
    root: PathBuf,
    manifest: Option<PluginManifest>,
    ebpf_objects: Vec<PathBuf>,
    meta_files: Vec<PathBuf>,
}

impl PluginPackager {
    pub fn new(plugin_dir: &Path) -> PluginResult<Self> {
        if !plugin_dir.is_dir() {
            return Err(PluginError::InvalidManifest(
                format!("not a directory: {}", plugin_dir.display()),
            ));
        }
        Ok(Self {
            root: plugin_dir.to_path_buf(),
            manifest: None,
            ebpf_objects: Vec::new(),
            meta_files: Vec::new(),
        })
    }

    pub fn set_manifest(&mut self, manifest: PluginManifest) -> PluginResult<()> {
        manifest.validate()?;
        self.manifest = Some(manifest);
        Ok(())
    }

    pub fn add_ebpf_object(&mut self, rel_path: &str) -> PluginResult<()> {
        let full = self.root.join(rel_path);
        if !full.is_file() {
            return Err(PluginError::InvalidManifest(
                format!("not found: {}", full.display()),
            ));
        }
        self.ebpf_objects.push(rel_path.into());
        Ok(())
    }

    pub fn add_meta_file(&mut self, rel_path: &str) -> PluginResult<()> {
        let full = self.root.join(rel_path);
        if !full.is_file() {
            return Err(PluginError::InvalidManifest(
                format!("not found: {}", full.display()),
            ));
        }
        self.meta_files.push(rel_path.into());
        Ok(())
    }

    pub fn pack(&self, out: &mut dyn Write) -> PluginResult<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let enc = GzEncoder::new(&mut buf, Compression::default());
            let mut tar = tar::Builder::new(enc);

            if let Some(ref manifest) = self.manifest {
                let yaml = serde_yaml_ng::to_string(manifest)
                    .map_err(|e| PluginError::InvalidManifest(e.to_string()))?;
                let mut header = tar::Header::new_gnu();
                header.set_path("manifest.yaml").map_err(PluginError::Io)?;
                header.set_size(yaml.len() as u64);
                header.set_mode(0o644);
                tar.append(&mut header, yaml.as_bytes())
                    .map_err(PluginError::Io)?;
            }

            for obj in &self.ebpf_objects {
                let full = self.root.join(obj);
                tar.append_path(full).map_err(PluginError::Io)?;
            }

            for meta in &self.meta_files {
                let full = self.root.join(meta);
                tar.append_path(full).map_err(PluginError::Io)?;
            }

            tar.into_inner().map_err(PluginError::Io)?
                .finish().map_err(PluginError::Io)?;
        }
        out.write_all(&buf).map_err(PluginError::Io)?;
        Ok(buf)
    }

    pub fn digest(packed: &[u8]) -> String {
        digest_bytes(packed)
    }
}
