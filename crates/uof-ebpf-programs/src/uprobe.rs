//! User-space probe handlers.
//!
//! These are dynamic probes that attach to user-space function entries
//! and returns. The actual function addresses are specified at load time.

use aya_ebpf::{macros::{uprobe, uretprobe}, programs::{ProbeContext, RetProbeContext}};

use crate::event::{UprobeEvent, EVENT_TYPE_UPROBE};
use crate::common::{make_header, submit_event};

/// User-space function entry probe.
///
/// This is a template that should be loaded with a specific function address.
/// The probe captures the function's arguments (up to 6).
#[uprobe]
pub fn handle_uprobe(ctx: ProbeContext) -> u32 {
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0),
        ctx.arg::<u64>(1).unwrap_or(0),
        ctx.arg::<u64>(2).unwrap_or(0),
        ctx.arg::<u64>(3).unwrap_or(0),
        ctx.arg::<u64>(4).unwrap_or(0),
        ctx.arg::<u64>(5).unwrap_or(0),
    ];

    let hdr = make_header(EVENT_TYPE_UPROBE, core::mem::size_of::<UprobeEvent>() as u32);
    let evt = UprobeEvent {
        hdr,
        func_addr: 0, // Set by user-space at load time
        ret_addr: 0,
        args,
    };
    unsafe { submit_event(&evt) };
    0
}

/// User-space function return probe.
///
/// This is a template that should be loaded with a specific function address.
/// The probe captures the return value.
#[uretprobe]
pub fn handle_uretprobe(ctx: RetProbeContext) -> u32 {
    let ret = ctx.ret::<u64>().unwrap_or(0);

    let hdr = make_header(EVENT_TYPE_UPROBE, core::mem::size_of::<UprobeEvent>() as u32);
    let evt = UprobeEvent {
        hdr,
        func_addr: 0, // Set by user-space at load time
        ret_addr: ret,
        args: [0; 6],
    };
    unsafe { submit_event(&evt) };
    0
}