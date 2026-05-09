//! OCI manifest structure for UOF plugin artifacts.
//!
//! Implements the OCI Image Manifest Specification (v1.1.0).

use serde::{Deserialize, Serialize};

/// OCI manifest schema version supported by this implementation.
pub const SUPPORTED_SCHEMA_VERSION: i64 = 2;

/// Top-level OCI image manifest.
/// Ref: <https://github.com/opencontainers/image-spec/blob/main/manifest.md>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    /// Manifest schema version.  Must be `2`.
    pub schema_version: i64,

    /// Optional mediatype — `application/vnd.oci.image.manifest.v1+json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// Artifact type, e.g. `application/vnd.uof.plugin.v1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,

    /// List of filesystem layers in the order they should be applied.
    #[serde(default)]
    pub layers: Vec<OciManifestLayer>,

    /// Configuration object (plugin manifest stored here).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<OciConfigRef>,
}

/// A reference to a content-addressable configuration blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciConfigRef {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

/// A reference to a content-addressable layer blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,

    /// Optional URL for remote layer (OCI distribution-spec "url").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,

    /// Optional platform constraint (empty = all platforms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<OciPlatform>,

    /// Annotations on this layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

/// Platform constraint for a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciPlatform {
    pub architecture: String,
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

impl OciManifest {
    /// Build an empty manifest for a plugin artifact.
    pub fn new() -> Self {
        Self {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            media_type: Some("application/vnd.oci.image.manifest.v1+json".into()),
            artifact_type: Some("application/vnd.uof.plugin.v1".into()),
            layers: Vec::new(),
            config: None,
        }
    }

    /// Add a layer reference.
    pub fn add_layer(&mut self, layer: OciManifestLayer) {
        self.layers.push(layer);
    }

    /// Verify schema version is supported.
    pub fn validate(&self) -> Result<(), super::RegistryError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(super::RegistryError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        Ok(())
    }
}

impl Default for OciManifest {
    fn default() -> Self {
        Self::new()
    }
}
