//! Plugin SDK errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("incompatible kernel version: {0}")]
    IncompatibleKernel(String),

    #[error("semver parse error: {0}")]
    Semver(#[from] semver::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("digest mismatch: expected={expected} actual={actual}")]
    DigestMismatch { expected: String, actual: String },
}

pub type PluginResult<T> = Result<T, PluginError>;
