#![no_std]

pub mod event;
pub mod io;
pub mod maps;
pub mod sched;
pub mod syscall;
pub mod common;

pub mod lock;

pub mod net;

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