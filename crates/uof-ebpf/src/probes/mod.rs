//! eBPF probe implementations organized by category.
//!
//! Each sub-module documents the kernel hook points it covers.
//! The actual probe functions are compiled with `aya-ebpf` when the
//! kernel headers are available.  The stubs below document the API.
//!
//! This module is only available when the `probes` feature is enabled
//! AND when building for the `bpfel-unknown-none` target.

#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod io;
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod lock;
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod net;
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod sched;
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod syscall;
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub mod uprobe;

// ---------------------------------------------------------------------------
// Public re-exports — enables `probes::syscall::handle_read_entry` etc.
// ---------------------------------------------------------------------------

#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use io::{handle_block_rq_complete, handle_block_rq_insert};
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use lock::{handle_lock_acquire, handle_lock_release};
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use net::{handle_sock_recv, handle_sock_send};
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use sched::{
    handle_sched_process_exit, handle_sched_process_fork, handle_sched_switch,
    handle_sched_wakeup,
};
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use syscall::{
    handle_close_entry, handle_close_exit, handle_open_entry, handle_open_exit,
    handle_read_entry, handle_read_exit, handle_write_entry, handle_write_exit,
};
#[cfg(all(feature = "probes", target_arch = "bpf"))]
pub use uprobe::{handle_uprobe, handle_uretprobe};