//! Block I/O probes via `tracepoint:block/*`.

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{IoEvent, EVENT_TYPE_IO};
use crate::common::{make_header, submit_event};

/// Handle block request insert tracepoint.
/// Tracepoint: block:block_rq_insert
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32)
/// Emits: operation=0 (insert)
#[tracepoint(target = "block", name = "block_rq_insert")]
pub fn handle_block_rq_insert(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 0,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle block request complete tracepoint.
/// Tracepoint: block:block_rq_complete
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32), error (offset 12, i32)
/// Emits: operation=1 (complete)
#[tracepoint(target = "block", name = "block_rq_complete")]
pub fn handle_block_rq_complete(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };
    let error = unsafe { *ctx.args().add(12) as *const i32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 1,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: error as i64,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle block request issue tracepoint.
/// Tracepoint: block:block_rq_issue
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32)
/// Emits: operation=2 (issue)
#[tracepoint(target = "block", name = "block_rq_issue")]
pub fn handle_block_rq_issue(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 2,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}