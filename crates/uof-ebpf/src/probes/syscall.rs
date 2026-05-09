//! Syscall entry/exit probes.
//!
//! ## Kernel hook points
//!
//! | Syscall | Entry probe | Exit probe |
//! |---------|-------------|------------|
//! | read    | `handle_read_entry`   | `handle_read_exit`    |
//! | write   | `handle_write_entry`  | `handle_write_exit`   |
//! | open    | `handle_open_entry`   | `handle_open_exit`   |
//! | close   | `handle_close_entry`  | `handle_close_exit`  |
//!
//! ## Attachment (aya-ebpf)
//!
//! ```ignore
//! #[kprobe]
//! pub fn handle_read_entry(ctx: ProbeContext) -> i64 {
//!     let pid = bpf_get_current_pid_tgid() >> 32;
//!     let hdr = EventHeader { event_type: EVENT_TYPE_SYSCALL, .. };
//!     let evt = SyscallEvent { hdr, syscall_id: 0, phase: 0, .. };
//!     submit_ringbuf("uof_events", &evt);
//!     0
//! }
//! ```
//!
//! ## Payload: [`SyscallEvent`]
//!
//! Captures syscall number, argument values (up to 6), and return value.

use aya_ebpf::{macros::{kprobe, kretprobe}, programs::{ProbeContext, RetprobeContext}};
use crate::{event::{SyscallEvent, EVENT_TYPE_SYSCALL}, EventHeader};

#[kprobe]
pub fn handle_read_entry(_ctx: ProbeContext) -> i64 { 0 }

#[kretprobe]
pub fn handle_read_exit(_ctx: RetprobeContext) -> i64 { 0 }

#[kprobe]
pub fn handle_write_entry(_ctx: ProbeContext) -> i64 { 0 }

#[kretprobe]
pub fn handle_write_exit(_ctx: RetprobeContext) -> i64 { 0 }

#[kprobe]
pub fn handle_open_entry(_ctx: ProbeContext) -> i64 { 0 }

#[kretprobe]
pub fn handle_open_exit(_ctx: RetprobeContext) -> i64 { 0 }

#[kprobe]
pub fn handle_close_entry(_ctx: ProbeContext) -> i64 { 0 }

#[kretprobe]
pub fn handle_close_exit(_ctx: RetprobeContext) -> i64 { 0 }
