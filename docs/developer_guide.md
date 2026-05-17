# UOF 开发者指南

## 目录

1. [概述](#概述)
2. [项目结构](#项目结构)
3. [开发环境](#开发环境)
4. [架构详解](#架构详解)
5. [eBPF 探针开发](#ebpf-探针开发)
6. [Agent 开发](#agent-开发)
7. [Control Plane 开发](#control-plane-开发)
8. [插件开发](#插件开发)
9. [OTLP 导出开发](#otlp-导出开发)
10. [测试](#测试)
11. [调试](#调试)
12. [代码规范](#代码规范)

---

## 概述

本文档面向 UOF 框架的 contributors 和 extension developers，指导如何进行各组件的二次开发或功能扩展。

### 前置知识

- Rust 编程经验（1.75+）
- Linux 系统编程基础
- eBPF 基本概念（kprobe, kretprobe, ring buffer）
- OpenTelemetry 数据模型

---

## 项目结构

```
uof/                              # Rust workspace 根目录
├── Cargo.toml                    # Workspace 配置
├── crates/
│   ├── uof-common/              # 公共错误类型、配置、日志
│   ├── uof-model/              # 统一数据模型 (Agent, Plugin, Template)
│   ├── uof-ebpf/               # eBPF 内核态头文件
│   ├── uof-ebpf-programs/      # eBPF 探针实现 (aya-ebpf)
│   ├── uof-probe-runtime/      # 用户态探针加载与生命周期管理
│   ├── uof-agent/              # 节点常驻进程
│   ├── uof-exporter-otlp/      # OTLP 导出器
│   ├── uof-plugin-sdk/          # 插件 manifest 解析、打包工具
│   ├── uof-control-plane/      # 控制面核心逻辑
│   ├── uof-control-api/        # HTTP API 层 (Axum)
│   ├── uof-registry/           # OCI Registry 客户端
│   └── uof-cli/                # CLI 工具
├── docs/                        # 设计文档
└── target/                      # 编译输出
```

---

## 开发环境

### 系统要求

- Linux 内核 4.18+ (支持 eBPF)
- Rust 1.75+
- clang + llvm (编译 eBPF)
- bpftool (内核提供)

### 依赖安装 (Ubuntu)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup target add bpfel-unknown-none

# eBPF 工具链
sudo apt-get install -y \
    clang \
    llvm \
    libelf-dev \
    libpcap-dev \
    gcc \
    make \
    pkg-config

# bpftool (从内核源码或发行版安装)
sudo apt-get install -y linux-tools-$(uname -r) linux-tools-generic
```

### IDE 配置

推荐使用 VS Code + rust-analyzer：

```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.check.targets": ["bpfel-unknown-none"],
  "rust-analyzer.procMacro.enable": true
}
```

### 验证环境

```bash
# 检查 Rust
rustc --version    # >= 1.75.0

# 检查 eBPF 支持
cat /proc/sys/kernel/bpf_stats_enabled
# 应该输出 1

# 检查 clang
clang --version

# 测试 eBPF 编译
cd /home/ubuntu/ebpf_all
cargo build -p uof-ebpf-programs
```

---

## 架构详解

### 整体数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                        Linux Kernel                              │
│                                                                  │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐   │
│   │  syscall    │      │    io       │      │   sched     │   │
│   │  kprobe     │      │  tracepoint │      │  tracepoint │   │
│   └──────┬──────┘      └──────┬──────┘      └──────┬──────┘   │
│          │                    │                    │           │
│          └────────────────────┼────────────────────┘           │
│                               │                                 │
│                    ┌──────────▼──────────┐                   │
│                    │     Ring Buffer      │                   │
│                    │    (事件传输通道)     │                   │
│                    └──────────┬──────────┘                   │
└───────────────────────────────┼─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│                     Userspace (Agent)                           │
│                                                                  │
│   ┌─────────────────────┐    ┌────────────────────────────┐    │
│   │  RingBufferConsumer │───►│  Event Pipeline           │    │
│   │  (Poll + Decode)    │    │  (Filter/Aggregate/Router)│    │
│   └─────────────────────┘    └────────────┬─────────────┘    │
│                                            │                  │
│                    ┌────────────────────────┼────────────┐    │
│                    │                        │            │    │
│                    ▼                        ▼            ▼    │
│            ┌──────────────┐        ┌────────────┐  ┌────────┐ │
│            │ OTLP Exporter│        │   Plugin   │  │Local   │ │
│            │   (gRPC)     │        │  Callbacks │  │Storage │ │
│            └──────────────┘        └────────────┘  └────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 核心模块关系

```
uof-ebpf-programs          # eBPF 内核态程序
        │
        │ 包含探针处理函数
        │ - syscall.rs: 系统调用 kprobe/kretprobe
        │ - sched.rs: 调度事件 tracepoint
        │ - io.rs: IO 事件 tracepoint
        │
        ▼
uof-probe-runtime         # 用户态探针运行时
        │
        │ RingBufferConsumer.poll() 轮询 ring buffer
        │ EventCallback trait 定义事件处理接口
        │
        ▼
uof-agent                 # Agent 主进程
        │
        │ 组合探针运行时、插件系统、OTLP 导出
        │
        ▼
uof-exporter-otlp         # OTLP 导出器
```

---

## eBPF 探针开发

### 探针类型

| 类型 | 前缀 | 说明 |
|------|------|------|
| kprobe | `handle_xxx_entry` | 内核函数入口探针 |
| kretprobe | `handle_xxx_exit` | 内核函数返回探针 |
| tracepoint | `handle_xxx` | 静态跟踪点 |
| uprobe | `handle_xxx_entry` | 用户态函数入口探针 |

### 探针结构

每个探针程序由两部分组成：

1. **Event Header** (`uof-ebpf/src/event.rs`)
```rust
pub struct EventHeader {
    pub ts_ns: u64,       // 时间戳
    pub event_type: u16,  // 事件类型
    pub version: u16,     // 版本号
    pub cpu_id: u32,      // CPU ID
    pub pid: u32,         // 进程 ID
    pub tid: u32,         // 线程 ID
    pub uid: u32,         // 用户 ID
    pub gid: u32,         // 组 ID
    pub cgroup_id: u64,   // cgroup ID
    pub mount_ns: u64,    // 挂载命名空间
    pub payload_len: u32, // 负载长度
}
```

2. **Event Payload** - 各类型自定义字段

### 开发示例：添加新的系统调用探针

**步骤 1: 定义事件结构** (`uof-ebpf/src/event.rs`)

```rust
pub const EVENT_TYPE_OPEN: u16 = 10;

#[repr(C)]
pub struct OpenEvent {
    pub hdr: EventHeader,
    pub syscall_id: u32,
    pub phase: u8,        // 0=entry, 1=exit
    pub flags: u8,
    pub args: [u64; 6],   // 系统调用参数
    pub ret: i64,         // 返回值
}
```

**步骤 2: 实现探针处理函数** (`uof-ebpf-programs/src/syscall.rs`)

```rust
use uof_ebpf::event::{OpenEvent, EVENT_TYPE_OPEN};

const SYSCALL_OPEN: u32 = 2;
fn entry_phase() -> u8 { 0 }
fn exit_phase() -> u8 { 1 }

#[kprobe]
pub fn handle_open_entry(ctx: ProbeContext) -> u32 {
    unsafe {
        let args = [
            ctx.arg::<u64>(0).unwrap_or(0),  // pathname
            ctx.arg::<u64>(1).unwrap_or(0),  // flags
            ctx.arg::<u64>(2).unwrap_or(0),  // mode
            0, 0, 0,
        ];
        emit_syscall_event(SYSCALL_OPEN, entry_phase(), args, 0);
    }
    0
}

#[kretprobe]
pub fn handle_open_exit(ctx: RetProbeContext) -> u32 {
    unsafe {
        let ret = ctx.ret::<i64>().unwrap_or(0);
        emit_syscall_event(SYSCALL_OPEN, exit_phase(), [0; 6], ret);
    }
    0
}
```

**步骤 3: 注册到 maps** (`uof-ebpf-programs/src/maps.rs`)

```rust
use uof_ebpf::maps::RINGBUF;

pub fn ringbuf() -> &'static RingBuf { &RINGBUF }
```

### 编译 eBPF 程序

```bash
# 调试模式
cargo build -p uof-ebpf-programs

# Release 模式 (优化级别 z = size)
cargo build -p uof-ebpf-programs --release

# 输出位置
# target/bpfel-unknown-none/debug/uof-ebpf-programs
# target/bpfel-unknown-none/release/uof-ebpf-programs
```

### Ring Buffer 通信

内核态通过 `ringbuf().output()` 发送事件：

```rust
let _ = crate::maps::ringbuf().output(&event, 0);
```

用户态通过 `RingBuf::try_from()` + `ringbuf.next()` 接收：

```rust
let mut ringbuf = RingBuf::try_from(
    bpf.map_mut("uof_events").context("failed to get ringbuf")?
)?;

while let Some(item) = ringbuf.next() {
    let data = &*item;
    let event = decode(data);
    callback.on_event(event);
}
```

---

## Agent 开发

### Agent 架构

```
uof-agent/
├── src/
│   ├── main.rs              # 入口点
│   ├── lib.rs               # 库入口
│   ├── config.rs            # 配置加载
│   ├── probe_manager.rs     # 探针管理
│   ├── plugin_manager.rs    # 插件管理
│   ├── state_machine.rs     # Agent 状态机
│   └── runtime.rs           # 运行时
```

### 添加新的探针类型

**步骤 1: 定义探针 trait** (`uof-probe-runtime/src/runtime.rs`)

```rust
pub trait Probe: Send + Sync {
    fn attach(&self) -> Result<()>;
    fn detach(&self) -> Result<()>;
    fn is_attached(&self) -> bool;
}
```

**步骤 2: 实现探针加载器**

```rust
pub struct ProbeLoader {
    // ...
}

impl ProbeLoader {
    pub fn load_uprobe(
        &mut self,
        symbol: &str,
        process_name: &str,
        probe_name: &str,
    ) -> Result<ProbeInfo> {
        // 1. 发现进程 PID
        let pids = ProcessDiscovery::new().find_pids(process_name)?;

        // 2. 解析符号地址
        let addr = SymbolResolver::new().resolve(symbol, pids[0])?;

        // 3. 加载 uprobe
        // ... 实际通过 aya 加载

        Ok(ProbeInfo { probe_id, address: addr })
    }
}
```

### 状态机

Agent 遵循以下状态转换：

```
Registering → Running ↔ Paused
                ↓
            Shutdown
```

### 配置文件格式

```toml
# agent.toml
[control_plane]
endpoint = "http://localhost:19999"
heartbeat_interval = "30s"

[agent]
hostname = "auto"
data_dir = "/var/lib/uof"

[probes]
enabled = ["syscall", "sched", "io", "net", "lock"]
sampling_rate = 1000

[ebpf]
ring_buffer_size = 8192

[otel]
endpoint = "http://localhost:4317"
protocol = "grpc"
```

---

## Control Plane 开发

### API 框架

使用 Axum 构建 REST API：

```rust
// uof-control-api/src/main.rs
use axum::{
    Router,
    routing::{get, post},
};

let app = Router::new()
    .route("/healthz", get(healthz))
    .route("/api/v1/agents", post(register_agent))
    .route("/api/v1/plugins", get(list_plugins));
```

### 添加新的 API 端点

**步骤 1: 定义 handler**

```rust
// uof-control-api/src/handlers/plugins.rs
pub async fn create_plugin(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreatePluginRequest>,
) -> Result<Json<Plugin>, AppError> {
    let plugin = state.control_plane.create_plugin(payload).await?;
    Ok(Json(plugin))
}
```

**步骤 2: 注册路由**

```rust
let app = Router::new()
    .route("/api/v1/plugins", post(create_plugin));
```

### 数据库集成

当前 MVP 使用内存存储，正式版本使用 PostgreSQL：

```rust
// 控制面状态
#[derive(Default)]
pub struct ControlPlaneState {
    pub agents: HashMap<AgentId, Agent>,
    pub plugins: HashMap<PluginId, Plugin>,
    pub templates: HashMap<TemplateId, Template>,
}
```

---

## 插件开发

### 插件 manifest.yaml

```yaml
schemaVersion: "1"
name: my-plugin
version: 0.1.0
publisher: myorg
kind: observability

probes:
  - id: my-probe
    type: uprobe
    hook: function/my_function
    program_name: my_ebpf_program
    default_sampling_rate: 1000
    enabled_by_default: true

targets:
  - name: mytarget
    version_constraint: ">=1.0"

resource_budget:
  max_memory_bytes: 52428800
  max_map_entries: 65536
  cpu_overhead_millicores: 50
```

### 插件打包

```bash
cargo run -p uof-cli -- plugin pack \
  --dir ./plugins/my-plugin/ \
  --output my-plugin.tar.gz
```

### 插件推送

```bash
cargo run -p uof-cli -- plugin push \
  --registry ghcr.io \
  --repo myorg/my-plugin \
  --tag 0.1.0 \
  --artifact my-plugin.tar.gz
```

### 插件生命周期

```
Packaged → Uploaded → Downloaded → Loaded → Attached → Running
                        ↓                      ↓
                     Failed               Detached → Unloaded
```

---

## OTLP 导出开发

### OTLP 架构

```
Event → OTLP Exporter → OpenTelemetry Collector → Backend
                 │
                 └─ gRPC/HTTP + Protobuf
```

### SpanExporter trait

```rust
// uof-exporter-otlp/src/exporter.rs
#[async_trait]
pub trait UofSpanExporter: Send + Sync {
    async fn export(&self, spans: Vec<Span>) -> Result<(), ExportError>;
}
```

### 实现 OTLP Exporter

```rust
pub struct OtlpSpanExporter {
    endpoint: String,
    timeout: Duration,
}

#[async_trait]
impl UofSpanExporter for OtlpSpanExporter {
    async fn export(&self, spans: Vec<Span>) -> Result<(), ExportError> {
        // 使用 opentelemetry-otlp 发送
        for span in spans {
            tracing::debug!(span_name = %span.name, ...);
        }
        Ok(())
    }
}
```

### 指标类型

```rust
pub enum MetricData {
    Counter { value: f64, attributes: Vec<(String, AttributeValue)> },
    Histogram { sum: f64, count: u64, bounds: Vec<f64>, counts: Vec<u64> },
    Gauge { value: f64, attributes: Vec<(String, AttributeValue)> },
}
```

---

## 测试

### 单元测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 测试
cargo test -p uof-probe-runtime

# 带日志输出
RUST_LOG=debug cargo test test_decode_syscall_event
```

### 集成测试

```bash
# eBPF 程序需要内核环境，单独测试
cargo test -p uof-ebpf-programs --lib
```

### 探针解码测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_syscall_event() {
        let consumer = RingBufferConsumer::new();
        let mut data = vec![0u8; 117];
        // 填充测试数据...
        let event = consumer.decode(&data);
        match event {
            ProbeEvent::Syscall(id, pid, entry, ret) => {
                assert_eq!(pid, 1234);
            }
            _ => panic!("Expected Syscall event"),
        }
    }
}
```

### 覆盖率

```bash
cargo install cargo-llvm-cov
cargo llvm-cov report --open
```

---

## 调试

### Agent 调试

```bash
# 启动 Agent (调试模式)
RUST_LOG=debug cargo run -p uof-agent

# 查看探针状态
curl http://localhost:8081/debug/probes

# 查看 Ring Buffer 统计
curl http://localhost:8081/debug/ringbuf-stats
```

### eBPF 调试

```bash
# 查看已加载的 BPF 程序
bpftool prog show

# 查看 BPF maps
bpftool map show

# 查看特定 map 内容
bpftool map dump name uof_events

# dmesg 查看 BPF 验证器输出
dmesg | grep -i bpf
```

### 日志配置

通过 `RUST_LOG` 环境变量：

```bash
# 全部调试日志
RUST_LOG=debug cargo run -p uof-agent

# 只看某个 crate
RUST_LOG=uof_probe_runtime=debug cargo run -p uof-agent

# JSON 格式输出
RUST_LOG=debug RUST_LOG_FORMAT=json cargo run -p uof-agent
```

### 火焰图

```bash
# 采样 CPU
perf record -F 99 -p $(pidof uof-agent) -g --call-graph dwarf

# 生成火焰图
perf script | FlameGraph/flamegraph.pl > agent.svg
```

---

## 代码规范

### Rust 代码风格

- 遵循 `rustfmt` 默认风格
- 使用 `clippy` 检查

```bash
# 格式化
cargo fmt

# 检查
cargo clippy -- -D warnings
```

### 命名规范

| 类型 | 风格 | 示例 |
|------|------|------|
| 模块 | snake_case | `ring_buffer_consumer` |
| 结构体 | PascalCase | `RingBufferConsumer` |
| 函数 | snake_case | `decode_event` |
| 变量 | snake_case | `poll_interval_ms` |
| 常量 | SCREAMING_SNAKE | `MAX_BUFFER_SIZE` |

### 错误处理

使用 `anyhow::Result<T>` 处理错误：

```rust
use anyhow::{Result, Context};

pub fn load_probe(&self, name: &str) -> Result<ProbeInfo> {
    let map = self.bpf.map_mut(name)
        .context("failed to get map")?;
    // ...
}
```

### 文档注释

```rust
/// Ring buffer consumer - consumes events from eBPF ring buffer
///
/// The consumer runs an async loop that polls the ring buffer map
/// and dispatches decoded events to registered callbacks.
pub struct RingBufferConsumer { ... }
```

### 测试注释

测试函数使用 `#[test]` 属性：

```rust
#[test]
fn test_decode_syscall_event() {
    // 测试逻辑
}
```

---

## 下一步

- 查看 [详细设计文档](./detailed_design.md) 了解架构细节
- 查看 [用户使用指南](./user_guide.md) 了解操作手册
- 查看 [运维部署指南](./operations.md) 了解生产环境配置