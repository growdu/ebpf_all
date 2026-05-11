//! Ring buffer consumer - consumes events from eBPF ring buffer

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;

use uof_ebpf::event::{EventHeader, EVENT_TYPE_SYSCALL, EVENT_TYPE_SCHED, EVENT_TYPE_IO};

use crate::runtime::ProbeEvent;

pub trait EventCallback: Send + Sync {
    fn on_event(&self, event: ProbeEvent);
}

pub struct RingBufferConsumer {
    poll_interval_ms: u64,
}

impl RingBufferConsumer {
    pub fn new() -> Self {
        Self { poll_interval_ms: 100 }
    }

    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }

    pub async fn start<C: EventCallback + 'static>(&self, callback: Arc<C>) -> Result<()> {
        let interval = tokio::time::Duration::from_millis(self.poll_interval_ms);
        loop {
            tokio::time::sleep(interval).await;
            // In real implementation, poll ring buffer here
            let mock_event = ProbeEvent::Unknown;
            callback.on_event(mock_event);
        }
    }

    pub async fn start_with_channel(&self, tx: mpsc::Sender<ProbeEvent>) -> Result<()> {
        let interval = tokio::time::Duration::from_millis(self.poll_interval_ms);
        loop {
            tokio::time::sleep(interval).await;
            let event = ProbeEvent::Unknown;
            if tx.send(event).await.is_err() {
                break;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn decode(&self, data: &[u8]) -> ProbeEvent {
        if data.len() < 52 {
            return ProbeEvent::Unknown;
        }
        let hdr = EventHeader {
            ts_ns: u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
            event_type: u16::from_le_bytes([data[8], data[9]]),
            version: u16::from_le_bytes([data[10], data[11]]),
            cpu_id: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            pid: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            tid: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            uid: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            gid: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            cgroup_id: u64::from_le_bytes([data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39]]),
            mount_ns: u64::from_le_bytes([data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47]]),
            payload_len: u32::from_le_bytes([data[48], data[49], data[50], data[51]]),
        };
        match hdr.event_type {
            EVENT_TYPE_SYSCALL => ProbeEvent::Unknown,
            EVENT_TYPE_SCHED => ProbeEvent::Unknown,
            EVENT_TYPE_IO => ProbeEvent::Io { pid: hdr.pid as u64, latency_ns: 0 },
            _ => ProbeEvent::Unknown,
        }
    }
}

impl Default for RingBufferConsumer {
    fn default() -> Self { Self::new() }
}
