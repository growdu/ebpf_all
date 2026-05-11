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
mod telemetry;

pub use config::OtlpConfig;
pub use exporter::{MetricExporter, SpanExporter, LogExporter, Span, SpanStatus, AttributeValue, OtlpSpanExporter};
pub use telemetry::init_telemetry;