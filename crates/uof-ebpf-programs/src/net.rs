//! Network socket tracepoint handlers.
//!
//! Tracepoints: net:netif_receive_skb, net:netif_tx

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{NetEvent, EVENT_TYPE_NET};
use crate::common::{make_header, submit_event};

/// Network packet receive tracepoint.
/// Tracepoint: net:netif_receive_skb
/// Fields: len (offset 0, u32), protocol (offset 4, u16)
#[tracepoint(target = "net", name = "netif_receive_skb")]
pub fn handle_netif_receive_skb(ctx: TracePointContext) -> i64 {
    let len = unsafe { *ctx.args().add(0) as *const u32 };
    let protocol = unsafe { *ctx.args().add(4) as *const u16 };

    let hdr = make_header(EVENT_TYPE_NET, core::mem::size_of::<NetEvent>() as u32);
    let evt = NetEvent {
        hdr,
        direction: 0, // receive
        protocol,
        saddr: 0,
        daddr: 0,
        sport: 0,
        dport: 0,
        payload_len: len,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Network packet send tracepoint.
/// Tracepoint: net:netif_tx
/// Fields: len (offset 0, u32), protocol (offset 4, u16)
#[tracepoint(target = "net", name = "netif_tx")]
pub fn handle_netif_tx(ctx: TracePointContext) -> i64 {
    let len = unsafe { *ctx.args().add(0) as *const u32 };
    let protocol = unsafe { *ctx.args().add(4) as *const u16 };

    let hdr = make_header(EVENT_TYPE_NET, core::mem::size_of::<NetEvent>() as u32);
    let evt = NetEvent {
        hdr,
        direction: 1, // send
        protocol,
        saddr: 0,
        daddr: 0,
        sport: 0,
        dport: 0,
        payload_len: len,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}