//! OTLP Exporter configuration

use std::time::Duration;

/// Configuration for OTLP exporter connection.
#[derive(Debug, Clone)]
pub struct OtlpConfig {
    /// OTLP endpoint (gRPC, default: http://localhost:4317)
    pub endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// Whether to use gzip compression
    pub compression_gzip: bool,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            timeout: Duration::from_secs(5),
            compression_gzip: false,
        }
    }
}

impl OtlpConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the OTLP endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable gzip compression.
    pub fn with_compression(mut self) -> Self {
        self.compression_gzip = true;
        self
    }
}

/// Builder for OtlpConfig.
#[derive(Default)]
pub struct Builder {
    config: OtlpConfig,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.endpoint = endpoint.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn with_compression(mut self) -> Self {
        self.config.compression_gzip = true;
        self
    }

    pub fn build(self) -> OtlpConfig {
        self.config
    }
}