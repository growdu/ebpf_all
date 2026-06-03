//! Lock contention tracepoint handlers.
//!
//! Tracepoints: lock:lock_acquire, lock:lock_release

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{LockEvent, EVENT_TYPE_LOCK};
use crate::common::{make_header, submit_event};

/// Lock acquire tracepoint.
/// Tracepoint: lock:lock_acquire
/// Fields: lock_addr (offset 0, u64), ret (offset 8, i32), contended (offset 12, i32)
#[tracepoint(target = "lock", name = "lock_acquire")]
pub fn handle_lock_acquire(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { *ctx.args().add(0) as *const u64 };
    let ret = unsafe { *ctx.args().add(8) as *const i32 };
    let contended = unsafe { *ctx.args().add(12) as *const i32 };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 0, // acquire
        lock_id: (lock_addr & 0xFFFFFFFF) as u32,
        wait_ns: if contended != 0 { ret as u32 } else { 0 },
        held_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Lock release tracepoint.
/// Tracepoint: lock:lock_release
/// Fields: lock_addr (offset 0, u64), wait_time (offset 8, u32), hold_time (offset 12, u32)
#[tracepoint(target = "lock", name = "lock_release")]
pub fn handle_lock_release(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { *ctx.args().add(0) as *const u64 };
    let wait_time = unsafe { *ctx.args().add(8) as *const u32 };
    let hold_time = unsafe { *ctx.args().add(12) as *const u32 };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 1, // release
        lock_id: (lock_addr & 0xFFFFFFFF) as u32,
        wait_ns: wait_time,
        held_ns: hold_time,
    };
    unsafe { submit_event(&evt) };
    0
}