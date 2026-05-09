//! Template — a role/scenario-oriented packaging of a plugin.
//!
//! Templates bundle plugin + threshold defaults + Grafana dashboard
//! panels + alert rules, and are applied to target nodes via
//! template bindings.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// A template combines a plugin with role-specific defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: Uuid,

    /// ID of the plugin this template wraps.
    pub plugin_id: Uuid,

    /// Human-readable name (e.g. `postgres-dba-diagnostic`).
    pub name: String,

    /// Semver version string.
    pub version: String,

    /// Target software this template applies to.
    pub target_software: String,

    /// Optional scenario label (e.g. `production`, `dev`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,

    /// Default threshold values (metric name → threshold config).
    #[serde(default)]
    pub thresholds: BTreeMap<String, ThresholdConfig>,

    /// Dashboard panel definitions (Grafana JSON model).
    #[serde(default)]
    pub dashboard_panels: Vec<DashboardPanel>,

    /// Alert rule definitions.
    #[serde(default)]
    pub alert_rules: Vec<AlertRule>,

    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,

    /// Template manifest (can carry additional schema-specific data).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub manifest: serde_json::Value,
}

/// Threshold configuration for a metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdConfig {
    /// Comparison operator (`gt`, `lt`, `eq`, `ge`, `le`).
    pub op: String,
    /// Threshold value.
    pub value: f64,
    /// Unit (e.g. `ms`, `count`).
    #[serde(default)]
    pub unit: String,
    /// Evaluation window (seconds).
    #[serde(default = "default_window")]
    pub window_secs: u64,
}

fn default_window() -> u64 { 60 }

/// A Grafana panel definition (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPanel {
    pub id: String,
    pub title: String,
    pub targets: Vec<PanelTarget>,
    /// Grafana legend format string.
    #[serde(default)]
    pub legend: String,
    /// Panel width in grid units.
    #[serde(default = "default_width")]
    pub width: u32,
    /// Panel height in grid units.
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 { 12 }
fn default_height() -> u32 { 8 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelTarget {
    /// Prometheus / Tempo query expression.
    pub expr: String,
    pub legend: Option<String>,
}

/// An alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub name: String,
    pub condition: String,
    /// Evaluation interval in seconds.
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    /// Severity: `warning` or `critical`.
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Annotations attached to the alert (Grafana-friendly).
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
}

fn default_interval() -> u64 { 30 }
fn default_severity() -> String { "warning".into() }

/// Selector used to match target nodes for template binding.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TargetSelector {
    /// Match by host labels (AND).
    #[serde(default)]
    pub labels: BTreeMap<String, String>,

    /// Match by software name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub software: Option<String>,

    /// Match by container runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_runtime: Option<String>,

    /// Match by hostname glob pattern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname_glob: Option<String>,

    /// Custom selector expressions (operator-defined).
    #[serde(default)]
    pub expressions: Vec<String>,
}

impl TargetSelector {
    /// Returns true if this selector matches the given attributes.
    pub fn matches(
        &self,
        labels: &BTreeMap<String, String>,
        software: Option<&str>,
        hostname: &str,
    ) -> bool {
        // All label requirements must be satisfied
        for (k, v) in &self.labels {
            match labels.get(k) {
                Some(got) if got == v => {}
                _ => return false,
            }
        }

        if let Some(ref soft) = self.software {
            match software {
                Some(s) if s == soft => {}
                _ => return false,
            }
        }

        if let Some(ref glob) = self.hostname_glob {
            if !glob_match(glob, hostname) {
                return false;
            }
        }

        true
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    // Simple glob: supports * (any chars) and ? (single char)
    let mut pi = 0;
    let mut vi = 0;
    while pi < pattern.len() || vi < value.len() {
        match pattern.as_bytes().get(pi) {
            Some(&b'*') => {
                // Try matching at current position or skip one in value
                pi += 1;
                vi += 1;
            }
            Some(&b'?') if vi < value.len() => {
                pi += 1;
                vi += 1;
            }
            Some(c) if vi < value.len() && *c == value.as_bytes()[vi] => {
                pi += 1;
                vi += 1;
            }
            None | Some(_) => return false,
        }
    }
    true
}

/// A binding between a template and a set of target selectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBinding {
    pub id: Uuid,

    /// Template to apply.
    pub template_id: Uuid,

    /// Selector that determines which nodes receive this template.
    pub selector: TargetSelector,

    /// Optional per-binding override policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<serde_json::Value>,

    /// Whether this binding is active.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }
