# eBPF + OTLP 完整实现设计

## 概述

实现端到端的可观测性数据流：
```
eBPF 探针 → ring buffer → 用户态运行时 → EventPipeline → OTLP Exporter → OTEL Collector
```

## 架构变更

### 新增 Crate: uof-ebpf-programs

独立 crate 用于编译 eBPF 探针程序，与数据结构（uof-ebpf）分离。

```
crates/uof-ebpf-programs/
├── Cargo.toml
├── src/
│   ├── main.rs         # 链接所有 probe 模块
│   ├── syscall.rs      # kprobe/kretprobe: read, write, open, close
│   ├── io.rs           # block tracepoint: block_rq_insert, block_rq_complete
│   ├── sched.rs        # sched tracepoint: sched_switch, sched_wakeup, sched_process_fork/exit
│   ├── lock.rs         # lock tracepoint (stub)
│   ├── net.rs          # sock tracepoint (stub)
│   ├── uprobe.rs       # uprobe/uretprobe (dynamic)
│   └── maps.rs         # ringbuf/perf map 定义
```

### 依赖变更

**uof-probe-runtime/Cargo.toml** 恢复:

```toml
aya = { version = "0.13", features = ["tokio"] }
aya-log = "0.2"
```

**uof-ebpf-programs/Cargo.toml**:

```toml
[package]
name = "uof-ebpf-programs"
edition = "2021"

[dependencies]
aya-ebpf = "0.2"
aya-ebpf-bindings = "0.2"
uof-ebpf = { path = "../uof-ebpf" }

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = "z"
lto = true
```

## 模块设计

### 1. uof-ebpf (保持不变)

- 纯数据结构: `EventHeader`, `SyscallEvent`, `IoEvent`, `SchedEvent`, etc.
- 常量定义: `EVENT_TYPE_*`, map 名称
- 无 aya 依赖

### 2. uof-ebpf-programs (新增)

使用 `aya-ebpf` 宏编写探针:

```rust
use aya_ebpf::bindings::TC_ACT_SHOT;
use aya_ebpf::maps::RingBuf;
use aya_ebpfpf_macros::{kprobe, kretprobe, tracepoint};
use uof_ebpf::event::*;

#[tracepoint]
pub fn handle_block_rq_complete(ctx: BlockRqCompleteCtx) -> u32 {
    // 构造 IoEvent，写入 ringbuf
}

#[kprobe]
pub fn handle_read_entry(ctx: PT_REGS_PARAMS(read)) -> u32 {
    // 构造 SyscallEvent (entry)
}

#[kretprobe]
pub fn handle_read_exit(ctx: PT_REGS_PARAMS(read)) -> u32 {
    // 构造 SyscallEvent (exit)
}
```

### 3. uof-probe-runtime (恢复 aya)

`RingBufferConsumer::start()` 真实实现:

```rust
use aya::Bpf;
use aya::maps::RingBuf;

pub async fn start<C: EventCallback>(&self, callback: Arc<C>) -> Result<()> {
    let bpf = Bpf::load_file("/path/to/uof-ebpf-programs.o")?;
    let mut ringbuf = RingBuf::map(&bpf, bpf::RINGBUF_NAME)?;
    let decoder = RingBufferConsumer::new();

    loop {
        if let Some(data) = ringbuf.next() {
            let event = decoder.decode(data);
            callback.on_event(event);
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
}
```

### 4. uof-exporter-otlp (使用 opentelemetry-otlp)

```rust
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::{TracerProvider, SimpleSpanProcessor};
use tonic::transport::Channel;

pub struct OtlpSpanExporter {
    endpoint: String,
    tracer: Tracer,
}

impl OtlpSpanExporter {
    pub fn new(endpoint: String) -> Self {
        let exporter = SpanExporter::builder()
            .with_endpoint(&endpoint)
            .build_grpc()
            .unwrap();

        let provider = TracerProvider::builder()
            .add_span_processor(SimpleSpanProcessor::new(exporter))
            .build();

        let tracer = provider.tracer("uof");
        Self { endpoint, tracer }
    }
}

#[async_trait]
impl SpanExporter for OtlpSpanExporter {
    async fn export(&self, spans: Vec<Span>) -> Result<(), ExportError> {
        for span in spans {
            let mut builder = self.tracer.span_builder(&span.name);
            builder.start_with_context(&Context::current(), &self.tracer);
            // 设置属性、时间、状态
        }
        Ok(())
    }
}
```

## 构建流程

### 1. 编译 eBPF 程序

```bash
cargo build --target bpfel-unknown-none -p uof-ebpf-programs
# 输出: target/bpfel-unknown-none/debug/uof-ebpf-programs.o
```

### 2. 编译用户态

```bash
cargo build -p uof-probe-runtime -p uof-agent
```

### 3. 运行

```bash
./target/debug/uof-agent \
    --ebpf-programs target/bpfel-unknown-none/debug/uof-ebpf-programs.o \
    --otlp-endpoint http://localhost:4317
```

## 工作范围

### 阶段 1: eBPF 探针程序
- [ ] 创建 `uof-ebpf-programs` crate
- [ ] 实现 `syscall.rs`: kprobe/kretprobe for read, write, open, close
- [ ] 实现 `io.rs`: block tracepoint
- [ ] 实现 `sched.rs`: sched tracepoint
- [ ] 实现 `maps.rs`: ringbuf map 定义
- [ ] 验证 `cargo build --target bpfel-unknown-none`

### 阶段 2: 用户态运行时
- [ ] 恢复 `aya` 依赖
- [ ] 实现 `RingBufferConsumer::start()` 真实读取
- [ ] 集成探针加载到 `ProbeRuntime`
- [ ] 验证 ringbuf 数据流

### 阶段 3: OTLP 导出器
- [ ] 实现 `OtlpSpanExporter::export()` 真实 gRPC 发送
- [ ] 实现 `OtlpMetricExporter`, `OtlpLogExporter`
- [ ] 验证数据到达 OTEL Collector

## 文件变更清单

### 新增
- `crates/uof-ebpf-programs/Cargo.toml`
- `crates/uof-ebpf-programs/src/lib.rs`
- `crates/uof-ebpf-programs/src/maps.rs`
- `crates/uof-ebpf-programs/src/syscall.rs`
- `crates/uof-ebpf-programs/src/io.rs`
- `crates/uof-ebpf-programs/src/sched.rs`
- `crates/uof-ebpf-programs/src/lock.rs` (stub)
- `crates/uof-ebpf-programs/src/net.rs` (stub)
- `crates/uof-ebpf-programs/src/uprobe.rs` (stub)

### 修改
- `crates/uof-probe-runtime/Cargo.toml` (取消注释 aya)
- `crates/uof-probe-runtime/src/ring_buffer_consumer.rs` (真实 ringbuf 读取)
- `crates/uof-exporter-otlp/src/exporter.rs` (真实 OTLP 导出)
- `Cargo.toml` (添加 workspace member)
