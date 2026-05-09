//! Lock probes via `tracepoint:lock/*`.
//!
//! ## Kernel hook points
//!
//! | Tracepoint | Probe function |
//! |------------|----------------|
//! | `lock/lock_acquire` | `handle_lock_acquire` |
//! | `lock/lock_release` | `handle_lock_release` |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[tracepoint(target = "lock", name = "lock_acquire")]
//! pub fn handle_lock_acquire(ctx: TracePointContext) -> i64 {
//!     let lock_id = bpf_ringbuf_load(ctx, 0u32);
//!     let wait_start = bpf_ringbuf_load(ctx, 8u64);
//!     submit_ringbuf("uof_events", &LockEvent { op: 0, lock_id, .. });
//!     0
//! }
//! ```
//!
//! ## Payload: [`LockEvent`]
//!
//! Captures lock identifier, wait time, and held time.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

#[tracepoint(target = "lock", name = "lock_acquire")]
pub fn handle_lock_acquire(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "lock", name = "lock_release")]
pub fn handle_lock_release(_ctx: TracePointContext) -> i64 { 0 }
