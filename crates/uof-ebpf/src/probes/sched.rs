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

#[tracepoint(target = "sched", name = "sched_switch")]
pub fn handle_sched_switch(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "sched", name = "sched_wakeup")]
pub fn handle_sched_wakeup(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "sched", name = "sched_process_fork")]
pub fn handle_sched_process_fork(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "sched", name = "sched_process_exit")]
pub fn handle_sched_process_exit(_ctx: TracePointContext) -> i64 { 0 }
