//! UOF eBPF probe library.
//!
//! Provides the canonical event type definitions used by both the kernel-space
//! eBPF programs and the user-space [`ProbeRuntime`](crate::ProbeRuntime).
//!
//! The kernel-space probe programs themselves (kprobe, uprobe, tracepoint
//! handlers) are compiled separately with `aya-ebpf` + `bpfel-unknown-none`
//! target when the kernel headers are available.
//!
//! # Probe categories (planned)
//!
//! - **syscall** – entry/exit for selected syscalls (read, write, open, etc.)
//! - **io**       – block I/O request start/complete
//! - **sched**    – task switch, wakeup, fork, exit
//! - **net**      – socket send/recv events
//! - **lock**     – spinlock acquire/release (via tracepoints)
//! - **uprobe**   – user-space function entry (dynamic, loaded per plugin)

pub mod event;
pub mod maps;
#[cfg(feature = "probes")]
pub mod probes;

pub use event::{
    EventHeader, IoEvent, LockEvent, NetEvent, SchedEvent, SyscallEvent,
    UprobeEvent, EVENT_TYPE_IO, EVENT_TYPE_LOCK, EVENT_TYPE_NET, EVENT_TYPE_SCHED,
    EVENT_TYPE_SYSCALL, EVENT_TYPE_UPROBE,
};

#[cfg(feature = "probes")]
pub use probes::{
    handle_block_rq_complete, handle_block_rq_insert, handle_lock_acquire,
    handle_lock_release, handle_sock_recv, handle_sock_send, handle_sched_process_exit,
    handle_sched_process_fork, handle_sched_switch, handle_sched_wakeup,
    handle_close_entry, handle_close_exit, handle_open_entry, handle_open_exit,
    handle_read_entry, handle_read_exit, handle_write_entry, handle_write_exit,
    handle_uprobe, handle_uretprobe,
};
