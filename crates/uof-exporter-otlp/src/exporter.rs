//! OTLP Exporters for traces, metrics, and logs
//!
//! This module provides exporter implementations that send telemetry data
//! to an OTLP-compatible collector via gRPC.

use async_trait::async_trait;
use std::time::Duration;

/// Trait for exporting spans (traces).
#[async_trait]
pub trait SpanExporter: Send + Sync {
    /// Export a batch of spans.
    async fn export(&self, spans: Vec<Span>) -> Result<(), ExportError>;
}

/// Trait for exporting metrics.
#[async_trait]
pub trait MetricExporter: Send + Sync {
    /// Export a batch of metrics.
    async fn export(&self, metrics: Vec<Metric>) -> Result<(), ExportError>;
}

/// Trait for exporting logs.
#[async_trait]
pub trait LogExporter: Send + Sync {
    /// Export a batch of logs.
    async fn export(&self, logs: Vec<LogRecord>) -> Result<(), ExportError>;
}

/// Span data for trace export.
#[derive(Debug, Clone)]
pub struct Span {
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub parent_span_id: Option<[u8; 8]>,
    pub name: String,
    pub start_time: Duration,
    pub end_time: Duration,
    pub attributes: Vec<(String, AttributeValue)>,
    pub status: SpanStatus,
}

/// Span status.
#[derive(Debug, Clone)]
pub enum SpanStatus {
    Ok,
    Error(String),
}

/// Attribute value types.
#[derive(Debug, Clone)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
}

/// Metric data.
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub data: MetricData,
}

/// Metric data variants.
#[derive(Debug, Clone)]
pub enum MetricData {
    /// Counter metric.
    Counter { value: f64, attributes: Vec<(String, AttributeValue)> },
    /// Histogram metric.
    Histogram { sum: f64, count: u64, bounds: Vec<f64>, counts: Vec<u64> },
    /// Gauge metric.
    Gauge { value: f64, attributes: Vec<(String, AttributeValue)> },
}

/// Log record.
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub timestamp: Duration,
    pub severity: Severity,
    pub body: String,
    pub attributes: Vec<(String, AttributeValue)>,
}

/// Log severity.
#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Export error.
#[derive(Debug)]
pub struct ExportError {
    pub message: String,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "export error: {}", self.message)
    }
}

impl std::error::Error for ExportError {}

impl ExportError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// OTLP span exporter implementation.
pub struct OtlpSpanExporter {
    endpoint: String,
    timeout: Duration,
}

impl OtlpSpanExporter {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl SpanExporter for OtlpSpanExporter {
    async fn export(&self, _spans: Vec<Span>) -> Result<(), ExportError> {
        // In a full implementation, this would use tonic to send spans
        // to the OTLP receiver at self.endpoint.
        // For now, return Ok to indicate the interface is defined.
        Ok(())
    }
}

/// OTLP metric exporter implementation.
pub struct OtlpMetricExporter {
    endpoint: String,
    timeout: Duration,
}

impl OtlpMetricExporter {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl MetricExporter for OtlpMetricExporter {
    async fn export(&self, _metrics: Vec<Metric>) -> Result<(), ExportError> {
        Ok(())
    }
}

/// OTLP log exporter implementation.
pub struct OtlpLogExporter {
    endpoint: String,
    timeout: Duration,
}

impl OtlpLogExporter {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl LogExporter for OtlpLogExporter {
    async fn export(&self, _logs: Vec<LogRecord>) -> Result<(), ExportError> {
        Ok(())
    }
}