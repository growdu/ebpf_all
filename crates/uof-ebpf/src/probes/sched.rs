//! Scheduler probes via `tracepoint:sched/*`.
//!
//! ## Kernel hook points
//!
//! | Tracepoint | Probe function |
//! |------------|----------------|
//! | `sched/sched_switch`          | `handle_sched_switch`          |
//! | `sched/sched_wakeup`          | `handle_sched_wakeup`          |
//! | `sched/sched_process_fork`    | `handle_sched_process_fork`    |
//! | `sched/sched_process_exit`     | `handle_sched_process_exit`     |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[tracepoint(target = "sched", name = "sched_switch")]
//! pub fn handle_sched_switch(ctx: TracePointContext) -> i64 {
//!     let prev_pid = bpf_ringbuf_load(ctx, 0u32);
//!     let next_pid = bpf_ringbuf_load(ctx, 8u32);
//!     submit_ringbuf("uof_events", &SchedEvent { prev_pid, next_pid, .. });
//!     0
//! }
//! ```
//!
//! ## Payload: [`SchedEvent`]
//!
//! Captures task switch source/target pids, wakeup latency, fork/exit events.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};
use aya_ebpf::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_ringbuf_output};

use crate::{event::{SchedEvent, EVENT_TYPE_SCHED}, EventHeader};

// Kind constants
const KIND_SWITCH: u8 = 0;
const KIND_WAKEUP: u8 = 1;
const KIND_FORK: u8 = 2;
const KIND_EXIT: u8 = 3;

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
        payload_len: core::mem::size_of::<SchedEvent>() as u32,
    }
}

/// Submit a SchedEvent to the ring buffer.
unsafe fn submit_event(event: &SchedEvent) {
    let size = core::mem::size_of::<SchedEvent>();
    bpf_ringbuf_output(
        core::ptr::null(),
        event as *const SchedEvent as *const u8,
        size,
        0,
    );
}

/// Handle sched_switch tracepoint.
///
/// Records task switch with prev_pid and next_pid.
#[tracepoint(target = "sched", name = "sched_switch")]
pub fn handle_sched_switch(ctx: TracePointContext) -> i64 {
    let prev_pid = unsafe { *ctx.args().add(0) as *const u32 };
    let next_pid = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED);
    let evt = SchedEvent {
        hdr,
        kind: KIND_SWITCH,
        prev_pid,
        next_pid,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle sched_wakeup tracepoint.
///
/// Records process wakeup with pid.
#[tracepoint(target = "sched", name = "sched_wakeup")]
pub fn handle_sched_wakeup(ctx: TracePointContext) -> i64 {
    let pid = unsafe { *ctx.args().add(0) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED);
    let evt = SchedEvent {
        hdr,
        kind: KIND_WAKEUP,
        prev_pid: pid,
        next_pid: 0,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle sched_process_fork tracepoint.
///
/// Records process fork with parent_pid and child_pid.
#[tracepoint(target = "sched", name = "sched_process_fork")]
pub fn handle_sched_process_fork(ctx: TracePointContext) -> i64 {
    let parent_pid = unsafe { *ctx.args().add(0) as *const u32 };
    let child_pid = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED);
    let evt = SchedEvent {
        hdr,
        kind: KIND_FORK,
        prev_pid: parent_pid,
        next_pid: child_pid,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle sched_process_exit tracepoint.
///
/// Records process exit with pid.
#[tracepoint(target = "sched", name = "sched_process_exit")]
pub fn handle_sched_process_exit(ctx: TracePointContext) -> i64 {
    let pid = unsafe { *ctx.args().add(0) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED);
    let evt = SchedEvent {
        hdr,
        kind: KIND_EXIT,
        prev_pid: pid,
        next_pid: 0,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}