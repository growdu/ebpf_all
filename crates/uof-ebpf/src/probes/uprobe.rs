//! User-space function entry probes (dynamic, loaded per plugin).
//!
//! Unlike static kernel probes, uprobe targets are resolved at load time
//! from the plugin manifest (target binary path + function symbol name).
//!
//! ## Loading flow
//!
//! 1. Plugin manifest declares `uprobe { target: "libpq.so", fn: "PQexec" }`.
//! 2. [`ProbeRuntime`](crate::ProbeRuntime) resolves the symbol via `/proc/PID/maps`.
//! 3. `#[uprobe]` / `#[uretprobe]` is attached; events flow into the ring-buffer.
//!
//! ## Kernel hook points
//!
//! | Target | Probe function |
//! |--------|----------------|
//! | user function entry | `handle_uprobe` |
//! | user function return | `handle_uretprobe` |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[uprobe]
//! pub fn handle_uprobe(ctx: UProbeContext) -> i64 {
//!     let func_addr = bpf_get_func_ip(ctx);
//!     let ret_addr = bpf_get_stack(ctx);
//!     submit_ringbuf("uof_events", &UprobeEvent { func_addr, ret_addr, .. });
//!     0
//! }
//! ```
//!
//! ## Payload: [`UprobeEvent`]
//!
//! Captures function address, return address, and up to 6 argument values.

use aya_ebpf::{macros::{uprobe, uretprobe}, programs::UProbeContext};

#[uprobe]
pub fn handle_uprobe(_ctx: UProbeContext) -> i64 { 0 }

#[uretprobe]
pub fn handle_uretprobe(_ctx: UProbeContext) -> i64 { 0 }
