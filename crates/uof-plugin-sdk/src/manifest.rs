//! Plugin manifest schema (manifest.yaml).
//!
//! The manifest is the authoritative description of a plugin's contents,
//! probes, and resource requirements.  It is stored as the OCI config blob
//! when the plugin is published to a registry.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginManifest {
    /// Schema version — currently `1`.
    pub schema_version: String,

    /// Unique plugin name (e.g. `postgres-observability`).
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// Semantic version of this plugin release.
    pub version: String,

    /// Plugin author or organization.
    pub publisher: String,

    /// Plugin kind.
    #[serde(default)]
    pub kind: PluginKind,

    /// Probe definitions bundled in this plugin.
    #[serde(default)]
    pub probes: Vec<ProbeEntry>,

    /// Target software this plugin instruments.
    #[serde(default)]
    pub targets: Vec<TargetSoftware>,

    /// Kernel compatibility constraints.
    #[serde(default)]
    pub kernel_constraints: Vec<KernelConstraint>,

    /// Resource budget for this plugin.
    #[serde(default)]
    pub resource_budget: ResourceBudget,

    /// Policy defaults (sampling rates, alert thresholds).
    #[serde(default)]
    pub policy: BTreeMap<String, serde_json::Value>,

    /// Artifact layers referenced by this plugin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginKind {
    Observability,
    Security,
    Network,
    #[serde(other)]
    Other,
}

impl Default for PluginKind {
    fn default() -> Self {
        Self::Observability
    }
}

/// A probe definition included in the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeEntry {
    /// Unique probe identifier within the plugin.
    pub id: String,

    /// Probe category.
    #[serde(rename = "type")]
    pub probe_type: ProbeType,

    /// Kernel hook point (e.g. `syscall/read`, `tracepoint/block/*`).
    pub hook: String,

    /// eBPF program object name in the ELF binary.
    #[serde(default)]
    pub program_name: Option<String>,

    /// Default sampling rate for this probe (events per second).
    #[serde(default)]
    pub default_sampling_rate: Option<u32>,

    /// Whether this probe is enabled by default.
    #[serde(default = "default_true")]
    pub enabled_by_default: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeType {
    Syscall,
    Io,
    Sched,
    Net,
    Lock,
    Uprobe,
    Custom,
}

/// A software target this plugin supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSoftware {
    /// Software name (e.g. `postgresql`, `nginx`).
    pub name: String,

    /// Version constraint (semver range, e.g. `>=13.0`).
    pub version_constraint: String,
}

/// A kernel version compatibility constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConstraint {
    /// Constraint operator (`ge` = greater or equal, `eq`, `lt`, …).
    pub op: String,
    /// Kernel version to compare against (e.g. `5.8.0`).
    pub version: String,
    /// Optional: specific probe types affected by this constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe_types: Option<Vec<ProbeType>>,
}

/// Resource budget for a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBudget {
    #[serde(default)]
    pub max_memory_bytes: Option<u64>,
    #[serde(default)]
    pub max_map_entries: Option<u32>,
    #[serde(default)]
    pub max_uprobes: Option<u32>,
    #[serde(default)]
    pub cpu_overhead_millicores: Option<u32>,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(50 * 1024 * 1024),
            max_map_entries: Some(65536),
            max_uprobes: Some(256),
            cpu_overhead_millicores: Some(50),
        }
    }
}

/// Reference to an artifact layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
    pub path: String,
}

impl PluginManifest {
    /// Load a manifest from a YAML string.
    pub fn from_yaml(yaml: &str) -> crate::PluginResult<Self> {
        let manifest: PluginManifest = serde_yaml_ng::from_str(yaml)
            .map_err(|e| crate::PluginError::InvalidManifest(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest for internal consistency.
    pub fn validate(&self) -> crate::PluginResult<()> {
        if self.name.is_empty() {
            return Err(crate::PluginError::MissingField("name".into()));
        }
        if self.version.is_empty() {
            return Err(crate::PluginError::MissingField("version".into()));
        }
        semver::Version::parse(&self.version)?;
        let mut seen = std::collections::HashSet::new();
        for probe in &self.probes {
            if !seen.insert(&probe.id) {
                return Err(crate::PluginError::InvalidManifest(
                    format!("duplicate probe id: {}", probe.id),
                ));
            }
        }
        Ok(())
    }
}
