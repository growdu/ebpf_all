//! Syscall probe handlers using kprobe/kretprobe.
//!
//! These handlers emit syscall entry/exit events to the ring buffer.

use aya_ebpf::{
    macros::{kprobe, kretprobe},
    programs::{ProbeContext, RetProbeContext},
};

use crate::event::{SyscallEvent, EVENT_TYPE_SYSCALL};

const SYSCALL_READ: u32 = 0;
const SYSCALL_WRITE: u32 = 1;
const SYSCALL_OPEN: u32 = 2;
const SYSCALL_CLOSE: u32 = 3;

fn entry_phase() -> u8 { 0 }
fn exit_phase() -> u8 { 1 }

/// Emit a syscall event to the ring buffer.
unsafe fn emit_syscall_event(
    syscall_id: u32,
    phase: u8,
    args: [u64; 6],
    ret: i64,
) {
    let mut event = SyscallEvent {
        hdr: core::default::Default::default(),
        syscall_id,
        phase,
        flags: 0,
        args,
        ret,
    };

    // Get current PID/TID via bpf_get_current_pid_tgid()
    let pid_tgid = aya_ebpf::helpers::bpf_get_current_pid_tgid();
    event.hdr.pid = (pid_tgid >> 32) as u32;
    event.hdr.tid = pid_tgid as u32;

    // Get current UID/GID via bpf_get_current_uid_gid()
    let uid_gid = aya_ebpf::helpers::bpf_get_current_uid_gid();
    event.hdr.uid = (uid_gid >> 32) as u32;
    event.hdr.gid = uid_gid as u32;

    // Get timestamp
    event.hdr.ts_ns = aya_ebpf::helpers::bpf_ktime_get_ns();

    // Set event header
    event.hdr.event_type = EVENT_TYPE_SYSCALL;
    event.hdr.version = 1;
    event.hdr.cpu_id = aya_ebpf::helpers::bpf_get_smp_processor_id();
    event.hdr.payload_len = (core::mem::size_of::<SyscallEvent>() - 52) as u32;

    // Output to ring buffer using the ringbuf output method
    let _ = crate::maps::ringbuf().output(&event, 0);
}

#[kprobe]
pub fn handle_read_entry(ctx: ProbeContext) -> u32 {
    unsafe {
        let args = [
            ctx.arg::<u64>(0).unwrap_or(0),
            ctx.arg::<u64>(1).unwrap_or(0),
            ctx.arg::<u64>(2).unwrap_or(0),
            0, 0, 0,
        ];
        emit_syscall_event(SYSCALL_READ, entry_phase(), args, 0);
    }
    0
}

#[kretprobe]
pub fn handle_read_exit(ctx: RetProbeContext) -> u32 {
    unsafe {
        let ret = ctx.ret::<i64>().unwrap_or(0);
        emit_syscall_event(SYSCALL_READ, exit_phase(), [0; 6], ret);
    }
    0
}

#[kprobe]
pub fn handle_write_entry(ctx: ProbeContext) -> u32 {
    unsafe {
        let args = [
            ctx.arg::<u64>(0).unwrap_or(0),
            ctx.arg::<u64>(1).unwrap_or(0),
            ctx.arg::<u64>(2).unwrap_or(0),
            0, 0, 0,
        ];
        emit_syscall_event(SYSCALL_WRITE, entry_phase(), args, 0);
    }
    0
}

#[kretprobe]
pub fn handle_write_exit(ctx: RetProbeContext) -> u32 {
    unsafe {
        let ret = ctx.ret::<i64>().unwrap_or(0);
        emit_syscall_event(SYSCALL_WRITE, exit_phase(), [0; 6], ret);
    }
    0
}

#[kprobe]
pub fn handle_open_entry(ctx: ProbeContext) -> u32 {
    unsafe {
        let args = [
            ctx.arg::<u64>(0).unwrap_or(0),
            ctx.arg::<u64>(1).unwrap_or(0),
            ctx.arg::<u64>(2).unwrap_or(0),
            0, 0, 0,
        ];
        emit_syscall_event(SYSCALL_OPEN, entry_phase(), args, 0);
    }
    0
}

#[kretprobe]
pub fn handle_open_exit(ctx: RetProbeContext) -> u32 {
    unsafe {
        let ret = ctx.ret::<i64>().unwrap_or(0);
        emit_syscall_event(SYSCALL_OPEN, exit_phase(), [0; 6], ret);
    }
    0
}

#[kprobe]
pub fn handle_close_entry(ctx: ProbeContext) -> u32 {
    unsafe {
        let args = [
            ctx.arg::<u64>(0).unwrap_or(0),
            0, 0, 0, 0, 0,
        ];
        emit_syscall_event(SYSCALL_CLOSE, entry_phase(), args, 0);
    }
    0
}

#[kretprobe]
pub fn handle_close_exit(ctx: RetProbeContext) -> u32 {
    unsafe {
        let ret = ctx.ret::<i64>().unwrap_or(0);
        emit_syscall_event(SYSCALL_CLOSE, exit_phase(), [0; 6], ret);
    }
    0
}