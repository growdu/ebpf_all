//! Map definitions for eBPF programs.
//!
//! Ring buffer is used by eBPF programs to output events.

use aya_ebpf::maps::RingBuf;

/// eBPF ring buffer map for event output.
static RINGBUF: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// Get a reference to the ring buffer map for emitting events.
pub fn ringbuf() -> &'static RingBuf {
    &RINGBUF
}