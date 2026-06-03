//! Map definitions for eBPF programs.
//!
//! Ring buffer is used by eBPF programs to output events.

use aya_ebpf::macros::map;
use aya_ebpf::maps::RingBuf;

/// eBPF ring buffer map for event output.
/// Named "uof_events" so user space can find it via map_mut("uof_events").
#[map(name = "uof_events")]
static mut RINGBUF: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// Get a reference to the ring buffer map for emitting events.
#[allow(unused)]
#[allow(static_mut_refs)]
pub fn ringbuf() -> &'static RingBuf {
    unsafe { &RINGBUF }
}