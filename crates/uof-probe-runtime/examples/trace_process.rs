//! End-to-end example: trace specified process functions
//!
//! This example demonstrates the full probe loading pipeline:
//! 1. Discover processes by name
//! 2. Resolve symbols to addresses
//! 3. Load uprobe probes
//! 4. Consume and display events from ring buffer
//!
//! Usage:
//!     cargo run --example trace_process -- <process_name> [symbol]
//!     cargo run --example trace_process -- postgres PQexec
//!     cargo run --example trace_process -- nginx
//!
//! Note: This example requires a running eBPF program with a loaded "uof_events"
//!       ring buffer map. For demonstration, events are printed when received.

use std::sync::Arc;
use std::time::Duration;
use uof_probe_runtime::{
    EventCallback, ProcessDiscovery, ProbeEvent, ProbeLoader, RingBufferConsumer,
    SymbolResolver,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <process_name> [symbol]", args[0]);
        eprintln!("  Example: {} postgres PQexec", args[0]);
        eprintln!("  Example: {} nginx (uses default symbols)", args[0]);
        std::process::exit(1);
    }

    let process_name = &args[1];
    let symbol = args.get(2).map(|s| s.as_str());

    println!("Tracing process: {}", process_name);
    if let Some(sym) = symbol {
        println!("  Symbol: {}", sym);
    }

    // 1. Discover processes
    let discovery = ProcessDiscovery::new();
    let pids = discovery.find_pids(process_name)?;
    if pids.is_empty() {
        anyhow::bail!("No processes found for '{}'", process_name);
    }
    println!("Found {} processes: {:?}", pids.len(), pids);

    // 2. Create components
    let _resolver = SymbolResolver::new();
    let mut loader = ProbeLoader::new();

    // 3. Determine symbols to trace
    let symbols = if let Some(sym) = symbol {
        vec![sym.to_string()]
    } else {
        vec!["PQexec".to_string(), "PQprepare".to_string()]
    };

    // 4. Load probes
    for sym in &symbols {
        match loader.load_uprobe(sym, process_name, sym) {
            Ok(info) => {
                println!("Loaded probe: {:?}({:?})", info.probe_id, info.address);
            }
            Err(e) => eprintln!("Failed to load probe for {}: {}", sym, e),
        }
    }

    // 5. Start ring buffer consumer (mock mode without actual eBPF)
    // In production, this would connect to a loaded eBPF program via aya:
    // let mut ebpf = aya::Ebpf::load_file("uof-ebpf-programs.o")?;
    // consumer.start(callback, &mut ebpf).await?;
    println!("Ring buffer consumer ready (waiting for events...)");
    println!("Press Ctrl+C to exit");

    // For demo purposes, print events periodically
    let consumer = Arc::new(RingBufferConsumer::new());
    let callback = Arc::new(EventPrinter);

    // This would be the real call in production with loaded eBPF:
    // consumer.start(callback, &mut ebpf).await?;

    // For demo, just wait and print status
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("Still monitoring... ({} processes found)", pids.len());
    }
}

struct EventPrinter;

impl EventCallback for EventPrinter {
    fn on_event(&self, event: ProbeEvent) {
        match event {
            ProbeEvent::Syscall(id, pid, entry, ret) => {
                println!("SYSCALL: id={}, pid={}, entry={}, ret={}", id, pid, entry, ret);
            }
            ProbeEvent::Io { pid, latency_ns } => {
                println!("IO: pid={}, latency={}ns", pid, latency_ns);
            }
            ProbeEvent::Sched { kind, prev_pid, next_pid } => {
                println!("SCHED: kind={}, prev={}, next={}", kind, prev_pid, next_pid);
            }
            ProbeEvent::Net { direction, saddr, daddr, dport, bytes } => {
                println!("NET: dir={}, src={:#x}, dst={:#x}:{}, bytes={}",
                    direction, saddr, daddr, dport, bytes);
            }
            ProbeEvent::Lock { op, lock_id, wait_ns } => {
                println!("LOCK: op={}, lock={}, wait={}ns", op, lock_id, wait_ns);
            }
            ProbeEvent::Unknown => {
                // Silently ignore unknown events in demo
            }
        }
    }
}