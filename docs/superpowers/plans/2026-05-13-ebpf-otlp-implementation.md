# eBPF + OTLP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement end-to-end observability data flow: eBPF probes → ring buffer → user-space runtime → EventPipeline → OTLP Exporter → OTEL Collector

**Architecture:** Separate eBPF probe programs (aya-ebpf, compiled to bpfel-unknown-none) from user-space runtime (aya loads .o file). OTLP uses opentelemetry-otlp with tonic gRPC.

**Tech Stack:** aya 0.13, aya-ebpf 0.2, opentelemetry-otlp 0.17, tonic 0.12, tokio

---

## Phase 1: eBPF Programs Crate

### Task 1: Create uof-ebpf-programs crate

**Files:**
- Create: `crates/uof-ebpf-programs/Cargo.toml`
- Modify: `Cargo.toml` (add workspace member)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "uof-ebpf-programs"
version.workspace = true
edition.workspace = true

[dependencies]
aya-ebpf = "0.2"
aya-ebpf-bindings = "0.2"
uof-ebpf = { path = "../uof-ebpf" }
```

- [ ] **Step 2: Add to workspace members in Cargo.toml**

Modify workspace `members` array to include `"crates/uof-ebpf-programs"`

- [ ] **Step 3: Verify build target available**

Run: `rustup target list | grep bpfel`
Expected: `bpfel-unknown-none (installed)`

- [ ] **Step 4: Verify workspace compiles**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none 2>&1 | head -20`
Expected: Should fail with "no manifest" until crate exists

- [ ] **Step 5: Commit**

```bash
git add crates/uof-ebpf-programs/Cargo.toml Cargo.toml
git commit -m "feat(uof-ebpf-programs): initial crate structure"
```

---

### Task 2: Create maps.rs with ringbuf definition

**Files:**
- Create: `crates/uof-ebpf-programs/src/maps.rs`

- [ ] **Step 1: Create maps.rs with ringbuf map**

```rust
use aya_ebpf::maps::RingBuf;

pub const RINGBUF_NAME: &[u8] = b"uof_events\0";

#[inline(always)]
pub fn ringbuf() -> RingBuf<uof_ebpf::event::EventHeader> {
    unsafe { RingBuf::from_raw(RINGBUF_NAME.as_ptr() as *mut _) }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none`
Expected: Compiles (no entry point yet)

- [ ] **Step 3: Commit**

```bash
git add crates/uof-ebpf-programs/src/maps.rs
git commit -m "feat(uof-ebpf-programs): add ringbuf map definition"
```

---

### Task 3: Create syscall.rs with kprobe probes

**Files:**
- Create: `crates/uof-ebpf-programs/src/syscall.rs`

- [ ] **Step 1: Create syscall.rs with kprobe handlers**

```rust
use aya_ebpf::{
    bindings::PT_REGS_PARM1,
    macros::{kprobe, kretprobe},
    programs::ProbeContext,
};
use uof_ebpf::event::{SyscallEvent, EVENT_TYPE_SYSCALL};

const SYSCALL_READ: u32 = 0;
const SYSCALL_WRITE: u32 = 1;
const SYSCALL_OPEN: u32 = 2;
const SYSCALL_CLOSE: u32 = 3;

fn entry_phase() -> u8 { 0 }
fn exit_phase() -> u8 { 1 }

#[kprobe]
pub fn handle_read_entry(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    // emit entry event
    0
}

#[kretprobe]
pub fn handle_read_exit(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    // emit exit event
    0
}

#[kprobe]
pub fn handle_write_entry(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}

#[kretprobe]
pub fn handle_write_exit(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}

#[kprobe]
pub fn handle_open_entry(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}

#[kretprobe]
pub fn handle_open_exit(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}

#[kprobe]
pub fn handle_close_entry(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}

#[kretprobe]
pub fn handle_close_exit(ctx: ProbeContext) -> u32 {
    let _ = ctx;
    0
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none 2>&1`
Expected: Compiles with warnings about unused functions

- [ ] **Step 3: Commit**

```bash
git add crates/uof-ebpf-programs/src/syscall.rs
git commit -m "feat(uof-ebpf-programs): add syscall kprobe handlers"
```

---

### Task 4: Create io.rs with block tracepoint probes

**Files:**
- Create: `crates/uof-ebpf-programs/src/io.rs`

- [ ] **Step 1: Create io.rs with tracepoint handlers**

```rust
use aya_ebpf::macros::tracepoint;

#[tracepoint]
pub fn handle_block_rq_insert(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_block_rq_complete(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none 2>&1`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add crates/uof-ebpf-programs/src/io.rs
git commit -m "feat(uof-ebpf-programs): add block I/O tracepoint handlers"
```

---

### Task 5: Create sched.rs with scheduler tracepoint probes

**Files:**
- Create: `crates/uof-ebpf-programs/src/sched.rs`

- [ ] **Step 1: Create sched.rs with tracepoint handlers**

```rust
use aya_ebpf::macros::tracepoint;

#[tracepoint]
pub fn handle_sched_switch(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_wakeup(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_process_fork(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}

#[tracepoint]
pub fn handle_sched_process_exit(ctx: TracepointContext) -> u32 {
    let _ = ctx;
    0
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none 2>&1`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add crates/uof-ebpf-programs/src/sched.rs
git commit -m "feat(uof-ebpf-programs): add scheduler tracepoint handlers"
```

---

### Task 6: Create lib.rs linking all probes

**Files:**
- Create: `crates/uof-ebpf-programs/src/lib.rs`

- [ ] **Step 1: Create lib.rs**

```rust
pub mod io;
pub mod maps;
pub mod sched;
pub mod syscall;

pub mod lock {
    use aya_ebpf::macros::tracepoint;

    #[tracepoint]
    pub fn handle_lock_acquire(_ctx: ()) -> u32 {
        0
    }

    #[tracepoint]
    pub fn handle_lock_release(_ctx: ()) -> u32 {
        0
    }
}

pub mod net {
    use aya_ebpf::macros::tracepoint;

    #[tracepoint]
    pub fn handle_sock_send(_ctx: ()) -> u32 {
        0
    }

    #[tracepoint]
    pub fn handle_sock_recv(_ctx: ()) -> u32 {
        0
    }
}

pub mod uprobe {
    use aya_ebpf::macros::{uprobe, uretprobe};

    #[uprobe]
    pub fn handle_uprobe(_ctx: ()) -> u32 {
        0
    }

    #[uretprobe]
    pub fn handle_uretprobe(_ctx: ()) -> u32 {
        0
    }
}
```

- [ ] **Step 2: Verify full compilation**

Run: `cargo build -p uof-ebpf-programs --target bpfel-unknown-none 2>&1`
Expected: Compiles successfully

- [ ] **Step 3: Verify .o file generated**

Run: `ls -la target/bpfel-unknown-none/debug/*.o 2>/dev/null || echo "No .o files"`
Expected: Should list uof-ebpf-programs.o

- [ ] **Step 4: Commit**

```bash
git add crates/uof-ebpf-programs/src/lib.rs
git commit -m "feat(uof-ebpf-programs): link all probe modules"
```

---

## Phase 2: User-Space Runtime

### Task 7: Restore aya dependencies

**Files:**
- Modify: `crates/uof-probe-runtime/Cargo.toml`

- [ ] **Step 1: Uncomment aya dependencies**

In `uof-probe-runtime/Cargo.toml`, uncomment:
```toml
aya = { version = "0.13", features = ["tokio"] }
aya-log = "0.2"
```

- [ ] **Step 2: Verify cargo build**

Run: `cargo build -p uof-probe-runtime 2>&1 | tail -20`
Expected: Compiles with aya

- [ ] **Step 3: Commit**

```bash
git add crates/uof-probe-runtime/Cargo.toml
git commit -m "feat(uof-probe-runtime): restore aya dependencies"
```

---

### Task 8: Implement real ringbuf reading

**Files:**
- Modify: `crates/uof-probe-runtime/src/ring_buffer_consumer.rs`

- [ ] **Step 1: Update start() to use aya RingBuf**

Modify `RingBufferConsumer::start()`:
```rust
use aya::Bpf;
use aya::maps::RingBuf;

pub async fn start<C: EventCallback + 'static>(&self, callback: Arc<C>, bpf: &Bpf) -> Result<()> {
    let ringbuf_name = CString::new(self.ringbuf_name.as_bytes())?;
    let mut ringbuf = RingBuf::map(bpf.map_fd(ringbuf_name.as_c_str())?)?;

    loop {
        if let Some(data) = ringbuf.next() {
            let event = self.decode(data);
            callback.on_event(event);
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-probe-runtime 2>&1`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add crates/uof-probe-runtime/src/ring_buffer_consumer.rs
git commit -m "feat(uof-probe-runtime): implement real ringbuf reading with aya"
```

---

## Phase 3: OTLP Exporter

### Task 9: Implement real OTLP export

**Files:**
- Modify: `crates/uof-exporter-otlp/src/exporter.rs`

- [ ] **Step 1: Update OtlpSpanExporter with real export**

Replace the no-op implementation:
```rust
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::{TracerProvider, SimpleSpanProcessor};
use tonic::transport::Channel;

pub struct OtlpSpanExporter {
    endpoint: String,
    tracer: opentelemetry_sdk::trace::Tracer,
}

impl OtlpSpanExporter {
    pub fn new(endpoint: String) -> Self {
        let exporter = SpanExporter::builder()
            .with_endpoint(&endpoint)
            .build_grpc()
            .expect("failed to create OTLP exporter");

        let provider = TracerProvider::builder()
            .add_span_processor(SimpleSpanProcessor::new(exporter))
            .build();

        let tracer = provider.tracer("uof");
        Self { endpoint, tracer }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl SpanExporter for OtlpSpanExporter {
    async fn export(&self, spans: Vec<Span>) -> Result<(), ExportError> {
        use opentelemetry_sdk::trace::Span;

        for span in spans {
            let mut builder = self.tracer.span_builder(&span.name);
            builder.start_with_context(&opentelemetry::Context::current(), &self.tracer);

            let mut otlp_span = builder.start(&self.tracer);
            otlp_span.set_attribute(opentelemetry::KeyValue::new("trace_id", format!("{:?}", span.trace_id)));
            otlp_span.set_attribute(opentelemetry::KeyValue::new("span_id", format!("{:?}", span.span_id)));

            for (key, value) in span.attributes {
                let kv = match value {
                    AttributeValue::String(s) => opentelemetry::Value::String(s.into()),
                    AttributeValue::Int(i) => opentelemetry::Value::I64(i),
                    AttributeValue::Double(f) => opentelemetry::Value::F64(f),
                    AttributeValue::Bool(b) => opentelemetry::Value::Bool(b),
                };
                otlp_span.set_attribute(opentelemetry::KeyValue::new(key, kv));
            }

            let status = match span.status {
                SpanStatus::Ok => opentelemetry::trace::Status::Ok,
                SpanStatus::Error(e) => opentelemetry::trace::Status::Error(e),
            };
            otlp_span.set_status(status);

            let start = opentelemetry::time::now() + std::time::Duration::from_nanos(span.start_time.as_nanos() as u64);
            let end = opentelemetry::time::now() + std::time::Duration::from_nanos(span.end_time.as_nanos() as u64);
            otlp_span.add_event("start", opentelemetry::trace::Event::new(start));
            otlp_span.end(end);
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p uof-exporter-otlp 2>&1`
Expected: Compiles

- [ ] **Step 3: Run tests**

Run: `cargo test -p uof-exporter-otlp 2>&1`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/uof-exporter-otlp/src/exporter.rs
git commit -m "feat(uof-exporter-otlp): implement real OTLP gRPC export"
```

---

## Phase 4: Integration

### Task 10: Build and verify end-to-end

- [ ] **Step 1: Build eBPF programs**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | tail -10`
Expected: Builds successfully

- [ ] **Step 2: Verify .o file exists**

Run: `ls -la target/bpfel-unknown-none/debug/uof-ebpf-programs.o`
Expected: File exists

- [ ] **Step 3: Build user-space**

Run: `cargo build -p uof-probe-runtime -p uof-agent 2>&1 | tail -10`
Expected: Builds successfully

- [ ] **Step 4: Run workspace tests**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: complete eBPF + OTLP end-to-end implementation"
```

---

## Summary

| Task | Description | Files Changed |
|------|-------------|---------------|
| 1 | Create uof-ebpf-programs crate | Create: 1, Modify: 1 |
| 2 | maps.rs ringbuf definition | Create: 1 |
| 3 | syscall.rs kprobe handlers | Create: 1 |
| 4 | io.rs block tracepoint | Create: 1 |
| 5 | sched.rs scheduler tracepoint | Create: 1 |
| 6 | lib.rs linking all probes | Create: 1 |
| 7 | Restore aya dependencies | Modify: 1 |
| 8 | Real ringbuf reading | Modify: 1 |
| 9 | Real OTLP export | Modify: 1 |
| 10 | Build and verify | - |

**Total: 10 tasks, 8 new files, 4 modified files**
