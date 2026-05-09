//! Block I/O probes via `tracepoint:block/*`.
//!
//! ## Kernel hook points
//!
//! | Tracepoint | Probe function |
//! |------------|----------------|
//! | `block/block_rq_insert`   | `handle_block_rq_insert` |
//! | `block/block_rq_complete`| `handle_block_rq_complete` |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[tracepoint(target = "block", name = "block_rq_complete")]
//! pub fn handle_block_rq_complete(ctx: TracePointContext) -> i64 {
//!     let sector = bpf_ringbuf_load(ctx, 0u64);
//!     let latency_ns = bpf_ktime_get_ns() - start_ts;
//!     submit_ringbuf("uof_events", &IoEvent { latency_ns, .. });
//!     0
//! }
//! ```
//!
//! ## Payload: [`IoEvent`]
//!
//! Captures sector address, I/O size, operation type, and latency.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};
use aya_ebpf::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_ringbuf_output};

use crate::{event::{IoEvent, EVENT_TYPE_IO}, EventHeader};

/// Get the current process ID from bpf_get_current_pid_tgid.
fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() >> 32
}

/// Create an EventHeader with the given event type.
fn make_header(event_type: u16) -> EventHeader {
    let ts = unsafe { bpf_ktime_get_ns() };
    EventHeader {
        ts_ns: ts,
        event_type,
        version: 1,
        cpu_id: 0,
        pid: current_pid(),
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<IoEvent>() as u32,
    }
}

/// Submit an IoEvent to the ring buffer.
unsafe fn submit_event(event: &IoEvent) {
    let size = core::mem::size_of::<IoEvent>();
    bpf_ringbuf_output(
        core::ptr::null(),
        event as *const IoEvent as *const u8,
        size,
        0,
    );
}

/// Handle block_rq_insert tracepoint.
///
/// Records I/O request insertion with sector and operation info.
#[tracepoint(target = "block", name = "block_rq_insert")]
pub fn handle_block_rq_insert(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };

    let operation: u8 = 0; // 0=read (default, will be updated based on opcode if available)

    let hdr = make_header(EVENT_TYPE_IO);
    let evt = IoEvent {
        hdr,
        operation,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle block_rq_complete tracepoint.
///
/// Records I/O completion with sector, num_sectors, and error info.
#[tracepoint(target = "block", name = "block_rq_complete")]
pub fn handle_block_rq_complete(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };
    let error = unsafe { *ctx.args().add(12) as *const i32 };

    let operation: u8 = 0; // 0=read, 1=write

    let hdr = make_header(EVENT_TYPE_IO);
    let evt = IoEvent {
        hdr,
        operation,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: error as i64,
    };
    unsafe { submit_event(&evt) };
    0
}
