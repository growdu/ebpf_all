//! OTLP Exporters for traces, metrics, and logs
//!
//! This module provides exporter implementations that send telemetry data
//! to an OTLP-compatible collector via gRPC.

use async_trait::async_trait;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics;
use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::{TracerProvider, Span as OtelSpan, SpanBuilder};
use opentelemetry::logs::{LoggerProvider, Logger, LogRecord as OtelLogRecord};
use std::time::{SystemTime, Duration};

/// Trait for exporting spans (traces).
#[async_trait]
pub trait UofSpanExporter: Send + Sync {
    /// Export a batch of spans.
    async fn export(&self, spans: Vec<UofSpan>) -> Result<(), ExportError>;
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
    async fn export(&self, logs: Vec<UofLogRecord>) -> Result<(), ExportError>;
}

/// Span data for trace export.
#[derive(Debug, Clone)]
pub struct UofSpan {
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
    Counter { value: f64, attributes: Vec<(String, AttributeValue)> },
    Histogram { sum: f64, count: u64, bounds: Vec<f64>, counts: Vec<u64> },
    Gauge { value: f64, attributes: Vec<(String, AttributeValue)> },
}

/// Log record.
#[derive(Debug, Clone)]
pub struct UofLogRecord {
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
#[derive(Debug, Clone)]
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
impl UofSpanExporter for OtlpSpanExporter {
    async fn export(&self, spans: Vec<UofSpan>) -> Result<(), ExportError> {
        // Build OTLP exporter using opentelemetry-otlp pipeline
        let tracer_provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&self.endpoint)
                    .with_timeout(self.timeout)
            )
            .install_simple()
            .map_err(|e| ExportError::new(format!("failed to build trace exporter: {e}")))?;

        let tracer = tracer_provider.tracer("uof");

        for span in spans {
            // Convert our Span to OpenTelemetry span
            let start_time = SystemTime::UNIX_EPOCH + span.start_time;
            let end_time = SystemTime::UNIX_EPOCH + span.end_time;

            // Build span using SpanBuilder trait API
            let mut builder = SpanBuilder::from_name(span.name.clone())
                .with_start_time(start_time)
                .with_end_time(end_time);

            // Add trace_id and span_id
            let trace_id = opentelemetry::trace::TraceId::from_bytes(span.trace_id);
            let span_id = opentelemetry::trace::SpanId::from_bytes(span.span_id);
            builder = builder.with_trace_id(trace_id);
            builder = builder.with_span_id(span_id);

            // Convert attributes
            let attrs: Vec<_> = span.attributes.iter().map(|(k, v)| {
                match v {
                    AttributeValue::String(s) => opentelemetry::KeyValue::new(k.clone(), s.clone()),
                    AttributeValue::Int(i) => opentelemetry::KeyValue::new(k.clone(), *i as f64),
                    AttributeValue::Double(d) => opentelemetry::KeyValue::new(k.clone(), *d),
                    AttributeValue::Bool(b) => opentelemetry::KeyValue::new(k.clone(), if *b { 1.0 } else { 0.0 }),
                }
            }).collect();
            builder = builder.with_attributes(attrs);

            // Add status
            match span.status {
                SpanStatus::Ok => {
                    builder = builder.with_status(opentelemetry::trace::Status::Ok);
                }
                SpanStatus::Error(ref msg) => {
                    builder = builder.with_status(opentelemetry::trace::Status::error(msg.clone()));
                }
            }

            // Build and export the span
            let mut otel_span = builder.start(&tracer);
            let trace_id = otel_span.span_context().trace_id().to_bytes();
            otel_span.end();

            tracing::debug!(
                trace_id = format!("{:x?}", trace_id),
                "exported span via OTLP gRPC"
            );
        }

        // Flush the tracer provider
        tracer_provider.force_flush();

        Ok(())
    }
}

/// OTLP metric exporter implementation.
#[derive(Debug, Clone)]
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
    async fn export(&self, metrics: Vec<Metric>) -> Result<(), ExportError> {
        // Build OTLP metric exporter
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(&self.endpoint)
            .with_timeout(self.timeout);

        let aggregation_selector = Box::new(opentelemetry_sdk::metrics::reader::DefaultAggregationSelector::default());
        let temporality_selector = Box::new(opentelemetry_sdk::metrics::reader::DefaultTemporalitySelector::default());

        let metrics_exporter = exporter.build_metrics_exporter(aggregation_selector, temporality_selector)
            .map_err(|e| ExportError::new(format!("failed to build metric exporter: {e}")))?;

        let reader = metrics::PeriodicReader::builder(metrics_exporter, opentelemetry_sdk::runtime::Tokio)
            .build();

        let meter_provider = metrics::SdkMeterProvider::builder()
            .with_reader(reader)
            .build();

        let meter = meter_provider.meter("uof");

        for metric in metrics {
            match metric.data {
                MetricData::Counter { value, attributes } => {
                    let attrs: Vec<_> = attributes.iter().map(|(k, v)| {
                        match v {
                            AttributeValue::Int(i) => opentelemetry::KeyValue::new(k.clone(), *i as f64),
                            AttributeValue::Double(d) => opentelemetry::KeyValue::new(k.clone(), *d),
                            AttributeValue::String(s) => opentelemetry::KeyValue::new(k.clone(), s.clone()),
                            AttributeValue::Bool(b) => opentelemetry::KeyValue::new(k.clone(), if *b { 1.0 } else { 0.0 }),
                        }
                    }).collect();

                    let counter = meter.f64_counter(metric.name.clone())
                        .with_unit(metric.unit.clone())
                        .with_description(metric.description.clone())
                        .init();
                    counter.add(value, &attrs);
                }
                MetricData::Histogram { sum, count, bounds, counts: _ } => {
                    let histogram = meter.f64_histogram(metric.name.clone())
                        .with_unit(metric.unit.clone())
                        .with_description(metric.description.clone())
                        .init();
                    // Record histogram values based on bounds
                    for i in 0..count as usize {
                        let value = if i < bounds.len() {
                            bounds[i]
                        } else {
                            sum
                        };
                        histogram.record(value, &[]);
                    }
                }
                MetricData::Gauge { value, attributes } => {
                    let attrs: Vec<_> = attributes.iter().map(|(k, v)| {
                        match v {
                            AttributeValue::Int(i) => opentelemetry::KeyValue::new(k.clone(), *i as f64),
                            AttributeValue::Double(d) => opentelemetry::KeyValue::new(k.clone(), *d),
                            AttributeValue::String(s) => opentelemetry::KeyValue::new(k.clone(), s.clone()),
                            AttributeValue::Bool(b) => opentelemetry::KeyValue::new(k.clone(), if *b { 1.0 } else { 0.0 }),
                        }
                    }).collect();

                    let gauge = meter.f64_observable_gauge(metric.name.clone())
                        .with_unit(metric.unit.clone())
                        .with_description(metric.description.clone())
                        .init();
                    gauge.observe(value, &attrs);
                }
            }
        }

        let _ = meter_provider.force_flush();
        Ok(())
    }
}

/// OTLP log exporter implementation.
#[derive(Debug, Clone)]
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

fn our_severity_to_otel(sev: Severity) -> opentelemetry::logs::Severity {
    match sev {
        Severity::Trace => opentelemetry::logs::Severity::Trace,
        Severity::Debug => opentelemetry::logs::Severity::Debug,
        Severity::Info => opentelemetry::logs::Severity::Info,
        Severity::Warn => opentelemetry::logs::Severity::Warn,
        Severity::Error => opentelemetry::logs::Severity::Error,
        Severity::Fatal => opentelemetry::logs::Severity::Fatal,
    }
}

#[async_trait]
impl LogExporter for OtlpLogExporter {
    async fn export(&self, logs: Vec<UofLogRecord>) -> Result<(), ExportError> {
        // Build OTLP log exporter using pipeline
        let logger_provider = opentelemetry_otlp::new_pipeline()
            .logging()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&self.endpoint)
                    .with_timeout(self.timeout)
            )
            .install_simple()
            .map_err(|e| ExportError::new(format!("failed to build log exporter: {e}")))?;

        let logger = logger_provider.logger("uof");

        for log in logs {
            let mut record = logger.create_log_record();
            let timestamp = SystemTime::UNIX_EPOCH + log.timestamp;

            record.set_body(opentelemetry::Value::String(log.body.into()).into());
            record.set_timestamp(timestamp);
            record.set_severity_number(our_severity_to_otel(log.severity));

            // Add attributes
            let attrs: Vec<_> = log.attributes.iter().map(|(k, v)| {
                let key = opentelemetry::Key::from(k.clone());
                match v {
                    AttributeValue::String(s) => (key, opentelemetry::logs::AnyValue::from(s.clone())),
                    AttributeValue::Int(i) => (key, opentelemetry::logs::AnyValue::from(*i as f64)),
                    AttributeValue::Double(d) => (key, opentelemetry::logs::AnyValue::from(*d)),
                    AttributeValue::Bool(b) => (key, opentelemetry::logs::AnyValue::from(if *b { 1.0 } else { 0.0 })),
                }
            }).collect();
            record.add_attributes(attrs);

            logger.emit(record);
        }

        logger_provider.force_flush();
        Ok(())
    }
}