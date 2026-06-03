//! Scheduler probes via `tracepoint:sched/*`.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{SchedEvent, EVENT_TYPE_SCHED};
use crate::common::{make_header, submit_event};

// Kind constants
const KIND_SWITCH: u8 = 0;
const KIND_WAKEUP: u8 = 1;
const KIND_FORK: u8 = 2;
const KIND_EXIT: u8 = 3;

/// Scheduler context switch tracepoint.
/// Tracepoint: sched:sched_switch
/// Fields: prev_pid (offset 0), prev_state (offset 8), next_pid (offset 16), next_prio (offset 20), next_cpu (offset 24)
#[tracepoint(category = "sched", name = "sched_switch")]
pub fn handle_sched_switch(ctx: TracePointContext) -> i64 {
    let prev_pid = unsafe { ctx.read_at(0).unwrap_or(0) };
    let next_pid = unsafe { ctx.read_at(16).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
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

/// Scheduler wakeup tracepoint.
/// Tracepoint: sched:sched_wakeup
/// Fields: pid (offset 0), prio (offset 4), success (offset 8), target_cpu (offset 12)
#[tracepoint(category = "sched", name = "sched_wakeup")]
pub fn handle_sched_wakeup(ctx: TracePointContext) -> i64 {
    let pid = unsafe { ctx.read_at(0).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
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

/// Scheduler process fork tracepoint.
/// Tracepoint: sched:sched_process_fork
/// Fields: pid (offset 0), child_pid (offset 8), clone_flags (offset 16)
#[tracepoint(category = "sched", name = "sched_process_fork")]
pub fn handle_sched_process_fork(ctx: TracePointContext) -> i64 {
    let parent_pid = unsafe { ctx.read_at(0).unwrap_or(0) };
    let child_pid = unsafe { ctx.read_at(8).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
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

/// Scheduler process exit tracepoint.
/// Tracepoint: sched:sched_process_exit
/// Fields: pid (offset 0), exit_code (offset 4), exit_signal (offset 8)
#[tracepoint(category = "sched", name = "sched_process_exit")]
pub fn handle_sched_process_exit(ctx: TracePointContext) -> i64 {
    let pid = unsafe { ctx.read_at(0).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
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
