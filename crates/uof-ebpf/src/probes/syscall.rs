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

use aya_ebpf::{
    macros::{kprobe, kretprobe},
    programs::{ProbeContext, RetprobeContext},
};
use aya_ebpf::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_ringbuf_output};
use crate::{event::{SyscallEvent, EVENT_TYPE_SYSCALL}, EventHeader};

// Syscall IDs
const SYSCALL_READ: u32 = 0;
const SYSCALL_WRITE: u32 = 1;
const SYSCALL_OPEN: u32 = 2;
const SYSCALL_CLOSE: u32 = 3;

// Phase constants
const PHASE_ENTRY: u8 = 0;
const PHASE_EXIT: u8 = 1;

/// Get the current process ID from bpf_get_current_pid_tgid.
fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() >> 32
}

/// Create an EventHeader with the given event type and pid.
fn make_header(event_type: u16, pid: u32) -> EventHeader {
    let ts = unsafe { bpf_ktime_get_ns() };
    EventHeader {
        ts_ns: ts,
        event_type,
        version: 1,
        cpu_id: 0,
        pid,
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<SyscallEvent>() as u32,
    }
}

/// Submit a SyscallEvent to the ring buffer.
unsafe fn submit_event(event: &SyscallEvent) {
    let size = core::mem::size_of::<SyscallEvent>();
    bpf_ringbuf_output(
        core::ptr::null(),
        event as *const SyscallEvent as *const u8,
        size,
        0,
    );
}

#[kprobe]
pub fn handle_read_entry(ctx: ProbeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0), // fd
        ctx.arg::<u64>(1).unwrap_or(0), // buf
        ctx.arg::<u64>(2).unwrap_or(0), // count
        0,
        0,
        0,
    ];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_READ,
        phase: PHASE_ENTRY,
        flags: 0,
        args,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kretprobe]
pub fn handle_read_exit(ctx: RetprobeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let ret = ctx.ret().unwrap_or(0) as i64;
    let args = [0, 0, 0, 0, 0, 0];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_READ,
        phase: PHASE_EXIT,
        flags: 0,
        args,
        ret,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kprobe]
pub fn handle_write_entry(ctx: ProbeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0), // fd
        ctx.arg::<u64>(1).unwrap_or(0), // buf
        ctx.arg::<u64>(2).unwrap_or(0), // count
        0,
        0,
        0,
    ];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_WRITE,
        phase: PHASE_ENTRY,
        flags: 0,
        args,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kretprobe]
pub fn handle_write_exit(ctx: RetprobeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let ret = ctx.ret().unwrap_or(0) as i64;
    let args = [0, 0, 0, 0, 0, 0];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_WRITE,
        phase: PHASE_EXIT,
        flags: 0,
        args,
        ret,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kprobe]
pub fn handle_open_entry(ctx: ProbeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0), // pathname
        ctx.arg::<u64>(1).unwrap_or(0), // flags
        ctx.arg::<u64>(2).unwrap_or(0), // mode
        0,
        0,
        0,
    ];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_OPEN,
        phase: PHASE_ENTRY,
        flags: 0,
        args,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kretprobe]
pub fn handle_open_exit(ctx: RetprobeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let ret = ctx.ret().unwrap_or(0) as i64;
    let args = [0, 0, 0, 0, 0, 0];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_OPEN,
        phase: PHASE_EXIT,
        flags: 0,
        args,
        ret,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kprobe]
pub fn handle_close_entry(ctx: ProbeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0), // fd
        0,
        0,
        0,
        0,
        0,
    ];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_CLOSE,
        phase: PHASE_ENTRY,
        flags: 0,
        args,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

#[kretprobe]
pub fn handle_close_exit(ctx: RetprobeContext) -> i64 {
    let pid = current_pid();
    let hdr = make_header(EVENT_TYPE_SYSCALL, pid);
    let ret = ctx.ret().unwrap_or(0) as i64;
    let args = [0, 0, 0, 0, 0, 0];
    let evt = SyscallEvent {
        hdr,
        syscall_id: SYSCALL_CLOSE,
        phase: PHASE_EXIT,
        flags: 0,
        args,
        ret,
    };
    unsafe { submit_event(&evt) };
    0
}
