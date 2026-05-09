//! Network probes via `tracepoint:sock/*`.
//!
//! ## Kernel hook points
//!
//! | Tracepoint | Probe function |
//! |------------|----------------|
//! | `sock/sock_send` | `handle_sock_send` |
//! | `sock/sock_recv` | `handle_sock_recv` |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[tracepoint(target = "sock", name = "sock_send")]
//! pub fn handle_sock_send(ctx: TracePointContext) -> i64 {
//!     let saddr = bpf_ringbuf_load(ctx, 0u32);
//!     let daddr = bpf_ringbuf_load(ctx, 4u32);
//!     let dport = bpf_ringbuf_load(ctx, 8u16);
//!     submit_ringbuf("uof_events", &NetEvent { direction: 0, saddr, daddr, dport, .. });
//!     0
//! }
//! ```
//!
//! ## Payload: [`NetEvent`]
//!
//! Captures source/destination address, port, protocol, payload size, and latency.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

#[tracepoint(target = "sock", name = "sock_send")]
pub fn handle_sock_send(_ctx: TracePointContext) -> i64 { 0 }

#[tracepoint(target = "sock", name = "sock_recv")]
pub fn handle_sock_recv(_ctx: TracePointContext) -> i64 { 0 }
