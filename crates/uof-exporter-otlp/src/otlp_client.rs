//! OTLP Client — gRPC client for OTLP exporter communication.
//!
//! This module provides a shared client for connecting to OTLP-compatible
//! collectors via tonic gRPC.

use anyhow::{Context, Result};
use tonic::transport::Channel;
use std::time::Duration;

/// Shared OTLP gRPC client for traces, metrics, and logs.
#[derive(Clone)]
pub struct OtlpClient {
    channel: Channel,
    timeout: Duration,
}

impl OtlpClient {
    /// Create a new OTLP client connecting to the given endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - OTLP collector endpoint, e.g., "http://localhost:4317"
    ///
    /// # Errors
    ///
    /// Returns an error if connection to the endpoint fails.
    pub async fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint_str = endpoint.into();
        let channel = Channel::from_shared(endpoint_str)
            .map_err(|e| anyhow::anyhow!("invalid endpoint: {e}"))?
            .timeout(Duration::from_secs(5))
            .connect()
            .await
            .context("failed to connect to OTLP endpoint")?;

        Ok(Self {
            channel,
            timeout: Duration::from_secs(5),
        })
    }

    /// Returns the channel for creating protocol clients.
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Returns the configured timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_endpoint() {
        let result = OtlpClient::new("http://invalid:99999").await;
        assert!(result.is_err());
    }
}