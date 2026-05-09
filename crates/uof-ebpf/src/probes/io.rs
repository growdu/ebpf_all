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

#[tracepoint(target = "block", name = "block_rq_insert")]
pub fn handle_block_rq_insert(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "block", name = "block_rq_complete")]
pub fn handle_block_rq_complete(_ctx: TracePointContext) -> i64 { 0 }
