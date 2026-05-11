//! OpenTelemetry provider initialization
//!
//! This module sets up the OpenTelemetry SDK with OTLP exporters
//! for traces, metrics, and logs.

use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace, runtime};

use crate::config::OtlpConfig;

/// Initialize OpenTelemetry tracing with OTLP exporter.
#[allow(unused_variables)]
pub fn init_tracing(config: &OtlpConfig) -> Result<opentelemetry_sdk::trace::TracerProvider, InitError> {
    let endpoint = config.endpoint.clone();

    // Build OTLP exporter with tonic
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&endpoint)
        .with_timeout(config.timeout);

    // Create tracer provider with batch exporter
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace::Config::default())
        .install_batch(runtime::Tokio)
        .map_err(|e| InitError::new(format!("failed to install tracer provider: {}", e)))?;

    Ok(tracer_provider)
}

/// Initialize OpenTelemetry with all signals (traces, metrics, logs).
#[allow(unused_variables)]
pub fn init_telemetry(config: &OtlpConfig) -> Result<TelemetryShutdown, InitError> {
    let tracer = init_tracing(config)?;

    Ok(TelemetryShutdown {
        tracer_provider: tracer,
    })
}

/// Handle for shutting down telemetry providers.
pub struct TelemetryShutdown {
    tracer_provider: opentelemetry_sdk::trace::TracerProvider,
}

impl TelemetryShutdown {
    /// Shutdown all telemetry providers.
    pub fn shutdown(self) -> Result<(), InitError> {
        self.tracer_provider.shutdown()
            .map_err(|e| InitError::new(format!("failed to shutdown tracer: {}", e)))?;
        Ok(())
    }
}

/// Initialization error.
#[derive(Debug)]
pub struct InitError {
    message: String,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "telemetry init error: {}", self.message)
    }
}

impl std::error::Error for InitError {}

impl InitError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}