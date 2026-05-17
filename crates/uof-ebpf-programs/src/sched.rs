use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

#[tracepoint]
pub fn handle_sched_switch(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_wakeup(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_process_fork(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_process_exit(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}