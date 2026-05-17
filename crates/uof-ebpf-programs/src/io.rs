use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

#[tracepoint]
pub fn handle_block_rq_insert(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_block_rq_complete(ctx: TracePointContext) -> u32 {
    let _ = ctx;
    0
}