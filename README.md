# UOF — Universal Observability Framework

通用软件可观测性框架。基于 **eBPF + OpenTelemetry + Rust**，为 Linux 环境提供低开销、统一建模、可扩展的观测与诊断能力。

## 核心定位

UOF 不是某个数据库或中间件的专用监控工具，而是一个**可复用的观测底座**。通过模板和插件机制，将底层探针能力复用到数据库、Web、缓存、消息队列等多个场景。

## 架构总览

```
┌─────────────────────────────────────────────────────┐
│  Grafana / Alert / UI                               │
│  Dashboard · Alert · Diagnostic Views              │
└──────────────────────────▲──────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────┐
│  Template / Plugin Manager │  Control Plane API      │
│  Lifecycle · Compatibility · Versioning            │
└──────────────────────────▲──────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────┐
│  OTEL Collector                                   │
│  Aggregate · Sample · Route · Transform           │
└──────────────────────────▲──────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────┐
│  UOF Agent                                        │
│  eBPF Probe Manager · Event Pipeline · OTLP Exp   │
└──────────────────────────▲──────────────────────────┘
                           │
┌──────────────────────────┴──────────────────────────┐
│  Linux Target System                                 │
│  Database · Web · Middleware · Runtime              │
└─────────────────────────────────────────────────────┘
```

## 技术栈

| 组件 | 技术选型 |
|------|----------|
| eBPF 探针 | Rust + Aya |
| Agent / Control Plane | Rust + Tokio + Axum |
| 数据管道 | OpenTelemetry Collector |
| 指标存储 | Prometheus |
| Trace 存储 | Tempo |
| 日志存储 | Loki |
| 可视化 | Grafana |

## 核心概念

### 观测模型

统一将 **Metrics / Trace / Log / Event** 映射到同一套实体模型：

```
host → process → thread
service → endpoint → request
db_instance → db_session → query_template
```

### Probe 生命周期

```
Registered → Loaded → Attached → Running → Draining → Detached → Unloaded
```

### Plugin 系统

插件封装某一类软件的默认观测能力（探针组合、指标定义、Dashboard、告警规则）。

#### 分发流程（OCI Registry）

插件通过 **OCI 1.0 Artifacts** 规范分发，支持标准的 OCI Registry（如 Docker Hub、GCR、Harbor、GitHub GHCR）。

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Plugin Distribution Flow                        │
└──────────────────────────────────────────────────────────────────────┘

  开发者机器                        OCI Registry                Control Plane
       │                                 │                             │
       │  1. uof-cli plugin pack         │                             │
       │     --dir ./postgres/           │                             │
       │     --output postgres.tar.gz    │                             │
       │                                 │                             │
       │  2. uof-cli plugin push        │                             │
       │     --registry ghcr.io          │                             │
       │     --repo myorg/postgres       │                             │
       │     --tag 0.1.0                 │                             │
       │     --artifact postgres.tar.gz  │  3. push_blob()             │
       │     ─────────────────────────►  │  PUT /v2/.../blobs/sha256:* │
       │                                 │  PUT /v2/.../manifests/0.1.0│
       │                                 │                             │
       │                                 │  4. Control Plane pulls     │
       │                                 │  POST /api/v1/plugins/pull  │
       │                                 │  ◄──────────────────────────│
       │                                 │                             │
       │                                 │  5. Agent streams via       │
       │                                 │  desired_state → payload    │
       │                                 │  ◄──────────────────────────│
```

#### manifest.yaml 示例

```yaml
schemaVersion: "1"
name: postgres-observability
version: 0.1.0
publisher: myorg
kind: observability
probes:
  - id: pg-sql-hook
    type: uprobe
    hook: function/PQexec
    program_name: postgres_ebpf
    default_sampling_rate: 1000
    enabled_by_default: true
  - id: pg-read-syscall
    type: syscall
    hook: syscall/read
    enabled_by_default: false
targets:
  - name: postgresql
    version_constraint: ">=13.0"
resource_budget:
  max_memory_bytes: 52428800   # 50 MiB
  max_map_entries: 65536
  cpu_overhead_millicores: 50
```

### Template 系统

模板是面向角色或场景的交付单元，例如"DBA 诊断模板"。包含推荐探针组合、阈值规则和诊断路径。

## Crate 结构

```
uof/                          # Rust workspace
├── crates/
│   ├── uof-common/           # 公共错误、配置、遥测
│   ├── uof-model/             # 统一数据模型（Agent、Plugin、Template、DesiredState）
│   ├── uof-ebpf/             # eBPF 内核态探针程序（aya-ebpf）
│   ├── uof-probe-runtime/    # 用户态探针加载与生命周期管理
│   ├── uof-agent/             # 节点常驻进程（探针管理、事件流水线、OTLP导出）
│   ├── uof-exporter-otlp/     # OTLP 导出器（批量发送、重试、背压）
│   ├── uof-plugin-sdk/        # 插件 manifest、模板 schema、打包工具
│   ├── uof-control-plane/      # 控制面核心逻辑（状态管理、版本治理）
│   ├── uof-control-api/        # HTTP API 层（Axum）
│   ├── uof-registry/           # OCI registry 交互（插件下载/上传/校验）
│   └── uof-cli/               # CLI 工具链
├── plugins/                   # 内置插件示例
│   ├── postgres/
│   └── nginx/
├── deploy/                   # 部署配置
│   ├── helm/
│   └── systemd/
└── docs/                    # 设计文档
```

## Quick Start

### 前置要求

- Rust 1.75+
- Linux 内核 4.18+（支持 eBPF）
- PostgreSQL（控制面元数据存储，MVP 可选）

### 构建

```bash
# 构建所有 crate
cargo build

# 构建 release 版本
cargo build --release
```

### 运行 Control Plane

```bash
# 使用默认配置启动（监听 127.0.0.1:8080）
cargo run -p uof-control-api

# 或指定绑定地址
UOF_BIND_ADDR=0.0.0.0:8080 cargo run -p uof-control-api
```

### 运行 Agent

```bash
# 启动 agent（需要连接 control plane）
cargo run -p uof-agent

# 启动 agent（独立模式，不连接 control plane）
UOF_CONTROL_PLANE_ENDPOINT=http://127.0.0.1:8080 cargo run -p uof-agent
```

### 使用 CLI

```bash
# 查看版本
cargo run -p uof-cli -- version

# 查看帮助
cargo run -p uof-cli -- --help
```

## API 概览

Control Plane API 暴露以下端点：

### Agent 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/agents/register` | 注册 Agent |
| POST | `/api/v1/agents/{id}/heartbeat` | Agent 心跳 |
| GET | `/api/v1/agents/{id}/desired-state` | 拉取期望配置 |
| POST | `/api/v1/agents/{id}/ack` | 上报配置应用结果 |

### Plugin 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/plugins` | 列出所有插件 |
| POST | `/api/v1/plugins` | 创建插件 |
| GET | `/api/v1/plugins/{id}` | 获取插件详情 |
| POST | `/api/v1/plugins/{id}/versions` | 上传插件版本 |
| POST | `/api/v1/plugins/{id}/release` | 发布插件版本 |
| POST | `/api/v1/plugins/pull` | 从 OCI Registry 拉取插件字节流 |

### Template 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/templates` | 列出所有模板 |
| POST | `/api/v1/templates` | 创建模板 |
| POST | `/api/v1/template-bindings` | 创建模板绑定 |
| DELETE | `/api/v1/template-bindings/{id}` | 删除模板绑定 |

### 健康检查

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/healthz` | 控制面健康状态 |

Agent Admin API：

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/healthz` | Agent 健康状态 |
| GET | `/readyz` | Agent 就绪状态 |
| GET | `/debug/probes` | 探针列表与状态 |

## MVP 范围

当前版本为 **MVP（最小可行产品）**，重点验证"通用框架 + 场景模板"方法。

**已实现：**

- ✅ Rust workspace 项目骨架
- ✅ eBPF 探针运行时框架（结构定义）
- ✅ Agent 核心模块（探针管理、插件生命周期、Control Plane 通信）
- ✅ Control Plane API（Agent 注册/心跳、Plugin CRUD、Template 绑定）
- ✅ OCI Registry 客户端（`uof-registry`，支持 push/pull blobs + manifests + Bearer auth）
- ✅ Plugin SDK（`uof-plugin-sdk`，manifest.yaml 解析 + tar.gz 打包 + 模板定义）
- ✅ CLI 插件命令（`plugin pack/push/pull`）

**暂未实现（v1.0 计划中）：**

- ⏳ 真实 eBPF 探针程序（syscall/sched/io/net/lock）
- ⏳ OTLP 导出器实现
- ⏳ PostgreSQL 持久化（当前为内存状态）
- ⏳ Grafana Dashboard 模板

## 设计文档

- [product.md](./docs/product.md) — 产品需求文档
- [overview_design.md](./docs/overview_design.md) — 概要设计（技术选型、架构）
- [detailed_design.md](./docs/detailed_design.md) — 详细设计（模块划分、数据流、接口定义）

## License

MIT
