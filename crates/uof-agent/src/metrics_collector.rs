//! Metrics collector for aggregating probe events into counters and histograms.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use uof_model::agent::MetricPayload;
use uof_probe_runtime::ProbeEvent;

/// Latency histogram buckets (in microseconds)
const LATENCY_BUCKETS_US: &[u64] = &[
    100, 500, 1_000, 5_000, 10_000, 50_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000,
    u64::MAX,
];

/// Metrics collector for aggregating probe events
#[derive(Default)]
pub struct MetricsCollector {
    // Counters
    syscall_count: AtomicU64,
    io_count: AtomicU64,
    sched_count: AtomicU64,
    net_count: AtomicU64,
    lock_count: AtomicU64,

    // Latency accumulators (sum and count for computing avg)
    syscall_latency_sum: AtomicU64,
    syscall_latency_count: AtomicU64,
    io_latency_sum: AtomicU64,
    io_latency_count: AtomicU64,

    // Histogram buckets for latency distribution
    syscall_latency_buckets: [AtomicU64; 12],
    io_latency_buckets: [AtomicU64; 12],

    // Network bytes total
    net_bytes_total: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Aggregate a probe event into metrics
    pub fn aggregate(&self, event: &ProbeEvent) {
        match event {
            ProbeEvent::Syscall(_id, _pid, entry, ret) => {
                self.syscall_count.fetch_add(1, Ordering::Relaxed);
                if !entry {
                    let latency_ns = ret.unsigned_abs();
                    self.record_latency(
                        latency_ns,
                        &self.syscall_latency_sum,
                        &self.syscall_latency_count,
                        &self.syscall_latency_buckets,
                    );
                }
            }
            ProbeEvent::Io { latency_ns, .. } => {
                self.io_count.fetch_add(1, Ordering::Relaxed);
                self.record_latency(
                    *latency_ns as u64,
                    &self.io_latency_sum,
                    &self.io_latency_count,
                    &self.io_latency_buckets,
                );
            }
            ProbeEvent::Sched { .. } => {
                self.sched_count.fetch_add(1, Ordering::Relaxed);
            }
            ProbeEvent::Net { bytes, .. } => {
                self.net_count.fetch_add(1, Ordering::Relaxed);
                self.net_bytes_total.fetch_add(*bytes as u64, Ordering::Relaxed);
            }
            ProbeEvent::Lock { .. } => {
                self.lock_count.fetch_add(1, Ordering::Relaxed);
            }
            ProbeEvent::Unknown => {}
        }
    }

    /// Export current metrics as a vector of MetricPayload
    pub fn export_metrics(&self) -> Vec<MetricPayload> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut metrics = Vec::new();

        // Counters
        metrics.push(MetricPayload {
            name: "probe_events_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.syscall_count.load(Ordering::Relaxed) as f64,
            labels: [("event_type".to_string(), "syscall".to_string())].into(),
            collected_at_ms: now_ms,
        });
        metrics.push(MetricPayload {
            name: "probe_events_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.io_count.load(Ordering::Relaxed) as f64,
            labels: [("event_type".to_string(), "io".to_string())].into(),
            collected_at_ms: now_ms,
        });
        metrics.push(MetricPayload {
            name: "probe_events_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.sched_count.load(Ordering::Relaxed) as f64,
            labels: [("event_type".to_string(), "sched".to_string())].into(),
            collected_at_ms: now_ms,
        });
        metrics.push(MetricPayload {
            name: "probe_events_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.net_count.load(Ordering::Relaxed) as f64,
            labels: [("event_type".to_string(), "net".to_string())].into(),
            collected_at_ms: now_ms,
        });
        metrics.push(MetricPayload {
            name: "probe_events_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.lock_count.load(Ordering::Relaxed) as f64,
            labels: [("event_type".to_string(), "lock".to_string())].into(),
            collected_at_ms: now_ms,
        });

        // Latency stats
        if let Some(avg) = self.compute_avg(&self.syscall_latency_sum, &self.syscall_latency_count) {
            metrics.push(MetricPayload {
                name: "probe_latency_avg_us".to_string(),
                metric_type: "gauge".to_string(),
                value: avg,
                labels: [("event_type".to_string(), "syscall".to_string())].into(),
                collected_at_ms: now_ms,
            });
        }

        if let Some(avg) = self.compute_avg(&self.io_latency_sum, &self.io_latency_count) {
            metrics.push(MetricPayload {
                name: "probe_latency_avg_us".to_string(),
                metric_type: "gauge".to_string(),
                value: avg,
                labels: [("event_type".to_string(), "io".to_string())].into(),
                collected_at_ms: now_ms,
            });
        }

        // Network bytes total
        metrics.push(MetricPayload {
            name: "probe_network_bytes_total".to_string(),
            metric_type: "counter".to_string(),
            value: self.net_bytes_total.load(Ordering::Relaxed) as f64,
            labels: BTreeMap::new(),
            collected_at_ms: now_ms,
        });

        metrics
    }

    /// Reset counters after exporting
    pub fn reset(&self) {
        self.syscall_count.store(0, Ordering::Relaxed);
        self.io_count.store(0, Ordering::Relaxed);
        self.sched_count.store(0, Ordering::Relaxed);
        self.net_count.store(0, Ordering::Relaxed);
        self.lock_count.store(0, Ordering::Relaxed);
        self.syscall_latency_sum.store(0, Ordering::Relaxed);
        self.syscall_latency_count.store(0, Ordering::Relaxed);
        self.io_latency_sum.store(0, Ordering::Relaxed);
        self.io_latency_count.store(0, Ordering::Relaxed);
        self.net_bytes_total.store(0, Ordering::Relaxed);
        for bucket in &self.syscall_latency_buckets {
            bucket.store(0, Ordering::Relaxed);
        }
        for bucket in &self.io_latency_buckets {
            bucket.store(0, Ordering::Relaxed);
        }
    }

    /// Get summary for metrics API
    pub fn get_summary(&self) -> MetricsSummary {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        MetricsSummary {
            total_events_per_sec: self.compute_total_events(),
            syscall_per_sec: self.syscall_count.load(Ordering::Relaxed) as f64,
            io_per_sec: self.io_count.load(Ordering::Relaxed) as f64,
            sched_per_sec: self.sched_count.load(Ordering::Relaxed) as f64,
            net_per_sec: self.net_count.load(Ordering::Relaxed) as f64,
            syscall_latency_avg_us: self
                .compute_avg(&self.syscall_latency_sum, &self.syscall_latency_count)
                .unwrap_or(0.0),
            io_latency_avg_us: self
                .compute_avg(&self.io_latency_sum, &self.io_latency_count)
                .unwrap_or(0.0),
            timestamp_ms: now_ms,
        }
    }

    fn record_latency(&self, latency_ns: u64, sum: &AtomicU64, count: &AtomicU64, buckets: &[AtomicU64; 12]) {
        let latency_us = latency_ns / 1000;
        sum.fetch_add(latency_us, Ordering::Relaxed);
        count.fetch_add(1, Ordering::Relaxed);
        let bucket_idx = LATENCY_BUCKETS_US.iter().position(|&b| latency_us < b).unwrap_or(11);
        buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
    }

    fn compute_avg(&self, sum: &AtomicU64, count: &AtomicU64) -> Option<f64> {
        let s = sum.load(Ordering::Relaxed);
        let c = count.load(Ordering::Relaxed);
        if c > 0 {
            Some(s as f64 / c as f64)
        } else {
            None
        }
    }

    fn compute_total_events(&self) -> f64 {
        self.syscall_count.load(Ordering::Relaxed) as f64
            + self.io_count.load(Ordering::Relaxed) as f64
            + self.sched_count.load(Ordering::Relaxed) as f64
            + self.net_count.load(Ordering::Relaxed) as f64
    }
}

/// Summary metrics for API response
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSummary {
    pub total_events_per_sec: f64,
    pub syscall_per_sec: f64,
    pub io_per_sec: f64,
    pub sched_per_sec: f64,
    pub net_per_sec: f64,
    pub syscall_latency_avg_us: f64,
    pub io_latency_avg_us: f64,
    pub timestamp_ms: u64,
}
