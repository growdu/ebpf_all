//! Registry operation errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry responded with {status}: {body}")]
    HttpError { status: u16, body: String },

    #[error("manifest not found: {0}")]
    ManifestNotFound(String),

    #[error("layer not found: {digest}")]
    LayerNotFound { digest: String },

    #[error("digest mismatch: expected={expected} actual={actual}")]
    DigestMismatch { expected: String, actual: String },

    #[error("invalid OCI reference: {0}")]
    InvalidRef(String),

    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(i64),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    Request(#[from] reqwest::Error),
}

pub type RegistryResult<T> = Result<T, RegistryError>;

impl RegistryError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::HttpError { status: 404, .. }
                 | Self::ManifestNotFound(_)
                 | Self::LayerNotFound { .. })
    }
}
