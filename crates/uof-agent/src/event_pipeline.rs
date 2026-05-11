//! Event pipeline for processing probe events
//!
//! The pipeline connects ring buffer consumers to OTLP exporters,
//! providing async event processing with batching and backpressure.

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use uof_exporter_otlp::{OtlpConfig, SpanExporter, OtlpSpanExporter};
use uof_probe_runtime::ProbeEvent;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Pipeline event wrapper with metadata.
#[derive(Debug, Clone)]
pub struct PipelineEvent {
    pub event: ProbeEvent,
    pub timestamp_ns: u64,
}

impl PipelineEvent {
    /// Create a pipeline event from a probe event.
    pub fn from_probe_event(event: ProbeEvent) -> Self {
        Self {
            event,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }
}

/// Event pipeline that processes probe events and exports them via OTLP.
pub struct EventPipeline {
    /// Channel to send events for processing
    sender: mpsc::Sender<PipelineEvent>,
    /// OTLP span exporter
    exporter: Arc<dyn SpanExporter>,
    /// Background task handle
    task: Option<JoinHandle<()>>,
    /// Shutdown signal sender
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _receiver_phantom: std::marker::PhantomData<PipelineEvent>,
}

impl EventPipeline {
    /// Create a new event pipeline with an OTLP exporter.
    pub fn new(exporter: Arc<dyn SpanExporter>) -> Self {
        let (sender, _) = mpsc::channel(1000);
        Self {
            sender,
            exporter,
            task: None,
            shutdown_tx: None,
            _receiver_phantom: std::marker::PhantomData,
        }
    }

    /// Create a new pipeline with default OTLP exporter.
    pub fn with_config(config: &OtlpConfig) -> Result<Self, PipelineError> {
        let exporter = Arc::new(OtlpSpanExporter::new(config.endpoint.clone()));
        Ok(Self::new(exporter))
    }

    /// Start the pipeline processing loop.
    pub fn start(&mut self, receiver: mpsc::Receiver<PipelineEvent>) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let exporter = self.exporter.clone();

        let handle = tokio::spawn(async move {
            Self::process_loop(receiver, exporter, shutdown_rx).await;
        });
        self.task = Some(handle);
    }

    async fn process_loop(
        mut receiver: mpsc::Receiver<PipelineEvent>,
        exporter: Arc<dyn SpanExporter>,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut batch: Vec<PipelineEvent> = Vec::with_capacity(100);
        let batch_timeout = tokio::time::Duration::from_millis(100);
        let mut shutdown_rx = shutdown_rx;

        loop {
            tokio::select! {
                Some(event) = receiver.recv() => {
                    batch.push(event);
                    if batch.len() >= 100 {
                        Self::flush_batch(&exporter, &mut batch).await;
                    }
                }
                _ = tokio::time::sleep(batch_timeout) => {
                    if !batch.is_empty() {
                        Self::flush_batch(&exporter, &mut batch).await;
                    }
                }
                _ = &mut shutdown_rx => {
                    if !batch.is_empty() {
                        Self::flush_batch(&exporter, &mut batch).await;
                    }
                    break;
                }
            }
        }
    }

    async fn flush_batch(exporter: &Arc<dyn SpanExporter>, batch: &mut Vec<PipelineEvent>) {
        let spans: Vec<_> = batch.drain(..)
            .filter_map(|e| Self::event_to_span(e))
            .collect();

        if !spans.is_empty() {
            if let Err(e) = exporter.export(spans).await {
                tracing::error!(error = %e, "failed to export spans");
            }
        }
    }

    fn event_to_span(event: PipelineEvent) -> Option<uof_exporter_otlp::Span> {
        use uof_exporter_otlp::{Span, SpanStatus, AttributeValue};

        match event.event {
            ProbeEvent::Syscall(syscall_id, pid, entry, ret) => {
                let name = if entry { "syscall_entry" } else { "syscall_exit" };
                let trace_id = Self::new_trace_id();
                let span_id = Self::new_span_id();
                Some(Span {
                    trace_id,
                    span_id,
                    parent_span_id: None,
                    name: format!("{}_{}", name, syscall_id),
                    start_time: Duration::from_nanos(event.timestamp_ns),
                    end_time: Duration::from_nanos(event.timestamp_ns + 1000),
                    attributes: vec![
                        ("pid".to_string(), AttributeValue::Int(pid as i64)),
                        ("syscall_id".to_string(), AttributeValue::Int(syscall_id as i64)),
                        ("entry".to_string(), AttributeValue::Bool(entry)),
                        ("return".to_string(), AttributeValue::Int(ret)),
                    ],
                    status: SpanStatus::Ok,
                })
            }
            ProbeEvent::Io { pid, latency_ns } => {
                let trace_id = Self::new_trace_id();
                let span_id = Self::new_span_id();
                Some(Span {
                    trace_id,
                    span_id,
                    parent_span_id: None,
                    name: "io_event".to_string(),
                    start_time: Duration::from_nanos(event.timestamp_ns),
                    end_time: Duration::from_nanos(event.timestamp_ns + latency_ns as u64),
                    attributes: vec![
                        ("pid".to_string(), AttributeValue::Int(pid as i64)),
                        ("latency_ns".to_string(), AttributeValue::Int(latency_ns as i64)),
                    ],
                    status: SpanStatus::Ok,
                })
            }
            ProbeEvent::Sched { kind, prev_pid, next_pid } => {
                let trace_id = Self::new_trace_id();
                let span_id = Self::new_span_id();
                Some(Span {
                    trace_id,
                    span_id,
                    parent_span_id: None,
                    name: "sched_event".to_string(),
                    start_time: Duration::from_nanos(event.timestamp_ns),
                    end_time: Duration::from_nanos(event.timestamp_ns + 100),
                    attributes: vec![
                        ("kind".to_string(), AttributeValue::Int(kind as i64)),
                        ("prev_pid".to_string(), AttributeValue::Int(prev_pid as i64)),
                        ("next_pid".to_string(), AttributeValue::Int(next_pid as i64)),
                    ],
                    status: SpanStatus::Ok,
                })
            }
            _ => None,
        }
    }

    fn new_trace_id() -> [u8; 16] {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut id = [0u8; 16];
        for i in 0..16 {
            id[i] = ((counter >> (i * 8)) & 0xFF) as u8;
        }
        id
    }

    fn new_span_id() -> [u8; 8] {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut id = [0u8; 8];
        for i in 0..8 {
            id[i] = ((counter >> (i * 8)) & 0xFF) as u8;
        }
        id
    }

    /// Send an event to the pipeline.
    pub async fn send(&self, event: PipelineEvent) -> Result<(), PipelineError> {
        self.sender.send(event).await.map_err(|_| PipelineError::new("channel closed"))?;
        Ok(())
    }

    /// Stop the pipeline.
    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task.take() {
            let _ = handle.await;
        }
    }
}

/// Pipeline error types.
#[derive(Debug)]
pub struct PipelineError {
    message: String,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pipeline error: {}", self.message)
    }
}

impl std::error::Error for PipelineError {}

impl PipelineError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// Event handler that sends events to the pipeline.
pub struct PipelineHandler {
    sender: mpsc::Sender<PipelineEvent>,
}

impl PipelineHandler {
    pub fn new(sender: mpsc::Sender<PipelineEvent>) -> Self {
        Self { sender }
    }
}

impl uof_probe_runtime::EventCallback for PipelineHandler {
    fn on_event(&self, event: ProbeEvent) {
        let pipeline_event = PipelineEvent::from_probe_event(event);
        if let Err(e) = self.sender.try_send(pipeline_event) {
            tracing::warn!(error = %e, "failed to send event to pipeline");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_event_from_probe() {
        let event = ProbeEvent::Unknown;
        let pe = PipelineEvent::from_probe_event(event);
        assert!(pe.timestamp_ns > 0);
    }
}