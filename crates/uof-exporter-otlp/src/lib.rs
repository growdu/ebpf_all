//! OTLP Exporter for UOF
//!
//! Provides OpenTelemetry Protocol (OTLP) export functionality for traces,
//! metrics, and logs via gRPC using tonic and opentelemetry-otlp.
//!
//! # Usage
//!
//! ```ignore
//! use uof_exporter_otlp::OtlpExporter;
//!
//! let exporter = OtlpExporter::builder()
//!     .with_endpoint("http://localhost:4317")
//!     .build()?;
//! ```

mod config;
mod exporter;
mod otlp_client;
mod telemetry;

pub use config::OtlpConfig;
pub use exporter::{
    MetricExporter, UofSpanExporter, LogExporter, UofSpan, UofLogRecord, SpanStatus,
    AttributeValue, OtlpSpanExporter, OtlpMetricExporter, OtlpLogExporter,
    UofSpan as Span,
};
pub use otlp_client::OtlpClient;
pub use telemetry::init_telemetry;