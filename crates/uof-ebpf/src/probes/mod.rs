//! eBPF probe implementations organized by category.
//!
//! Each sub-module documents the kernel hook points it covers.
//! The actual probe functions are compiled with `aya-ebpf` when the
//! kernel headers are available.  The stubs below document the API.

pub mod io;
pub mod lock;
pub mod net;
pub mod sched;
pub mod syscall;
pub mod uprobe;

// ---------------------------------------------------------------------------
// Public re-exports — enables `probes::syscall::handle_read_entry` etc.
// ---------------------------------------------------------------------------

pub use io::{handle_block_rq_complete, handle_block_rq_insert};
pub use lock::{handle_lock_acquire, handle_lock_release};
pub use net::{handle_sock_recv, handle_sock_send};
pub use sched::{
    handle_sched_process_exit, handle_sched_process_fork, handle_sched_switch,
    handle_sched_wakeup,
};
pub use syscall::{
    handle_close_entry, handle_close_exit, handle_open_entry, handle_open_exit,
    handle_read_entry, handle_read_exit, handle_write_entry, handle_write_exit,
};
pub use uprobe::{handle_uprobe, handle_uretprobe};
