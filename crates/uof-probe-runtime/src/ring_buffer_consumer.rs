//! Ring buffer consumer - consumes events from eBPF ring buffer
//!
//! This module provides functionality to poll events from an eBPF ring buffer
//! map and dispatch them to registered callbacks.

use std::sync::Arc;
use anyhow::{Result, Context};
use aya::Ebpf;
use aya::maps::RingBuf;
use std::convert::TryFrom;
use tokio::sync::mpsc;

use uof_ebpf::event::{EventHeader, EVENT_TYPE_SYSCALL, EVENT_TYPE_SCHED, EVENT_TYPE_IO, EVENT_TYPE_LOCK, EVENT_TYPE_NET, EVENT_TYPE_UPROBE};

use crate::runtime::ProbeEvent;

/// Trait for handling probe events.
pub trait EventCallback: Send + Sync {
    /// Called when an event is received from the ring buffer.
    fn on_event(&self, event: ProbeEvent);
}

/// Consumer that polls events from the eBPF ring buffer.
///
/// The consumer runs an async loop that polls the ring buffer map
/// and dispatches decoded events to registered callbacks.
pub struct RingBufferConsumer {
    poll_interval_ms: u64,
    ringbuf_name: String,
}

impl RingBufferConsumer {
    /// Create a new consumer with default settings.
    pub fn new() -> Self {
        Self {
            poll_interval_ms: 100,
            ringbuf_name: "uof_events".to_string(),
        }
    }

    /// Set the polling interval in milliseconds.
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }

    /// Set the ring buffer map name.
    pub fn with_ringbuf_name(mut self, name: impl Into<String>) -> Self {
        self.ringbuf_name = name.into();
        self
    }

    /// Start the consumer with a callback handler.
    ///
    /// This spawns an async task that polls the ring buffer and
    /// calls the callback for each received event.
    pub async fn start<C: EventCallback + 'static>(&self, callback: Arc<C>, bpf: &mut Ebpf) -> Result<()> {
        let interval = tokio::time::Duration::from_millis(self.poll_interval_ms);

        // Open the ring buffer map
        let mut ringbuf = RingBuf::try_from(
            bpf.map_mut(&self.ringbuf_name)
                .context("failed to get ringbuf map")?
        ).context("failed to create ringbuf consumer")?;

        loop {
            tokio::time::sleep(interval).await;

            // Poll for events using aya's async interface
            while let Some(item) = ringbuf.next() {
                let data = &*item; // Deref RingBufItem to &[u8]
                let event = self.decode(data);
                callback.on_event(event);
            }
        }
    }

    /// Start the consumer with a channel sender.
    ///
    /// Events are sent to the channel instead of calling a callback.
    /// This is useful for connecting to an event pipeline.
    pub async fn start_with_channel(&self, tx: mpsc::Sender<ProbeEvent>, bpf: &mut Ebpf) -> Result<()> {
        let interval = tokio::time::Duration::from_millis(self.poll_interval_ms);

        let mut ringbuf = RingBuf::try_from(
            bpf.map_mut(&self.ringbuf_name)
                .context("failed to get ringbuf map")?
        ).context("failed to create ringbuf consumer")?;

        loop {
            tokio::time::sleep(interval).await;

            while let Some(item) = ringbuf.next() {
                let data = &*item;
                let event = self.decode(data);
                if tx.send(event).await.is_err() {
                    return Ok(());
                }
            }
        }
    }

    /// Decode raw bytes from the ring buffer into a ProbeEvent.
    #[allow(dead_code)]
    pub fn decode(&self, data: &[u8]) -> ProbeEvent {
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
            EVENT_TYPE_SYSCALL => Self::decode_syscall(data, hdr),
            EVENT_TYPE_SCHED => Self::decode_sched(data, hdr),
            EVENT_TYPE_IO => Self::decode_io(data, hdr),
            EVENT_TYPE_LOCK => ProbeEvent::Lock { op: 0, lock_id: 0, wait_ns: 0 },
            EVENT_TYPE_NET => ProbeEvent::Net { direction: 0, saddr: 0, daddr: 0, dport: 0, bytes: 0 },
            EVENT_TYPE_UPROBE => ProbeEvent::Unknown,
            _ => ProbeEvent::Unknown,
        }
    }

    fn decode_syscall(data: &[u8], hdr: EventHeader) -> ProbeEvent {
        if data.len() < 117 {
            return ProbeEvent::Unknown;
        }
        let syscall_id = u32::from_le_bytes([data[52], data[53], data[54], data[55]]);
        let phase = data[56];
        let entry = phase == 0;
        let ret = i64::from_le_bytes([data[109], data[110], data[111], data[112], data[113], data[114], data[115], data[116]]);
        ProbeEvent::Syscall(syscall_id as u64, hdr.pid, entry, ret)
    }

    fn decode_sched(data: &[u8], _hdr: EventHeader) -> ProbeEvent {
        if data.len() < 61 {
            return ProbeEvent::Unknown;
        }
        let kind = data[52];
        let prev_pid = u32::from_le_bytes([data[53], data[54], data[55], data[56]]);
        let next_pid = u32::from_le_bytes([data[57], data[58], data[59], data[60]]);
        ProbeEvent::Sched { kind, prev_pid, next_pid }
    }

    fn decode_io(data: &[u8], hdr: EventHeader) -> ProbeEvent {
        if data.len() < 78 {
            return ProbeEvent::Unknown;
        }
        let latency_ns = u32::from_le_bytes([data[66], data[67], data[68], data[69]]);
        ProbeEvent::Io { pid: hdr.pid as u64, latency_ns }
    }
}

impl Default for RingBufferConsumer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_syscall_event() {
        let consumer = RingBufferConsumer::new();
        let mut data = vec![0u8; 117];
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        data[8..10].copy_from_slice(&EVENT_TYPE_SYSCALL.to_le_bytes());
        data[10..12].copy_from_slice(&1u16.to_le_bytes());
        data[12..16].copy_from_slice(&0u32.to_le_bytes());
        data[16..20].copy_from_slice(&1234u32.to_le_bytes());
        data[20..24].copy_from_slice(&0u32.to_le_bytes());
        data[52..56].copy_from_slice(&0u32.to_le_bytes());
        data[56] = 1;
        data[109..117].copy_from_slice(&100i64.to_le_bytes());

        let event = consumer.decode(&data);
        match event {
            ProbeEvent::Syscall(id, pid, entry, ret) => {
                assert_eq!(id, 0);
                assert_eq!(pid, 1234);
                assert!(!entry);
                assert_eq!(ret, 100);
            }
            _ => panic!("Expected Syscall event"),
        }
    }

    #[test]
    fn test_decode_io_event() {
        let consumer = RingBufferConsumer::new();
        let mut data = vec![0u8; 78];
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        data[8..10].copy_from_slice(&EVENT_TYPE_IO.to_le_bytes());
        data[16..20].copy_from_slice(&5678u32.to_le_bytes());
        data[66..70].copy_from_slice(&5000u32.to_le_bytes());

        let event = consumer.decode(&data);
        match event {
            ProbeEvent::Io { pid, latency_ns } => {
                assert_eq!(pid, 5678);
                assert_eq!(latency_ns, 5000);
            }
            _ => panic!("Expected Io event"),
        }
    }
}