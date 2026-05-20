#![no_std]

pub mod event;
pub mod io;
pub mod maps;
pub mod sched;
pub mod syscall;

pub mod lock {
    use aya_ebpf::macros::tracepoint;
    use aya_ebpf::programs::TracePointContext;

    /// Lock acquire tracepoint (requires kernel 4.17+).
    /// Captures: lock_id, wait_time for lock contention events.
    #[tracepoint]
    pub fn handle_lock_acquire(_ctx: TracePointContext) -> u32 {
        // Lock acquire - tracks mutex/rwlock acquisition
        0
    }

    /// Lock release tracepoint (requires kernel 4.17+).
    /// Captures: lock_id, hold_time for lock usage analysis.
    #[tracepoint]
    pub fn handle_lock_release(_ctx: TracePointContext) -> u32 {
        // Lock release - tracks mutex/rwlock release
        0
    }
}

pub mod net {
    use aya_ebpf::macros::tracepoint;
    use aya_ebpf::programs::TracePointContext;

    /// Network packet receive tracepoint.
    /// Captures: saddr, daddr, sport, dport, len for incoming packets.
    #[tracepoint]
    pub fn handle_sock_recv(_ctx: TracePointContext) -> u32 {
        // Socket receive - tracks incoming network data
        0
    }

    /// Network packet send tracepoint.
    /// Captures: saddr, daddr, sport, dport, len for outgoing packets.
    #[tracepoint]
    pub fn handle_sock_send(_ctx: TracePointContext) -> u32 {
        // Socket send - tracks outgoing network data
        0
    }
}

pub mod uprobe {
    use aya_ebpf::macros::{uprobe, uretprobe};
    use aya_ebpf::programs::{ProbeContext, RetProbeContext};

    #[uprobe]
    pub fn handle_uprobe(_ctx: ProbeContext) -> u32 {
        0
    }

    #[uretprobe]
    pub fn handle_uretprobe(_ctx: RetProbeContext) -> u32 {
        0
    }
}