//! Lock contention tracepoint handlers.
//!
//! Tracepoints: lock:lock_acquire, lock:lock_release

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{LockEvent, EVENT_TYPE_LOCK};
use crate::common::{make_header, submit_event};

/// Lock acquire tracepoint.
/// Tracepoint: lock:lock_acquire
/// Fields: lock_addr (offset 0, u64), ret (offset 8, i32), contended (offset 12, i32)
#[tracepoint(category = "lock", name = "lock_acquire")]
pub fn handle_lock_acquire(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { ctx.read_at(0).unwrap_or(0) };
    let ret = unsafe { ctx.read_at::<i32>(8).unwrap_or(0) };
    let contended = unsafe { ctx.read_at::<i32>(12).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 0, // acquire
        lock_id: (lock_addr & 0xFFFFFFFF_u32) as u32,
        wait_ns: if contended != 0 { ret as u32 } else { 0 },
        held_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Lock release tracepoint.
/// Tracepoint: lock:lock_release
/// Fields: lock_addr (offset 0, u64), wait_time (offset 8, u32), hold_time (offset 12, u32)
#[tracepoint(category = "lock", name = "lock_release")]
pub fn handle_lock_release(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { ctx.read_at(0).unwrap_or(0) };
    let wait_time = unsafe { ctx.read_at(8).unwrap_or(0) };
    let hold_time = unsafe { ctx.read_at(12).unwrap_or(0) };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 1, // release
        lock_id: (lock_addr & 0xFFFFFFFF_u32) as u32,
        wait_ns: wait_time,
        held_ns: hold_time,
    };
    unsafe { submit_event(&evt) };
    0
}
