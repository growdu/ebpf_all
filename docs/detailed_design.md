# 通用软件可观测性框架详细设计文档（v1.0）

## 1. 文档目标

本文档在 [overview_design.md](F:\ai\ebpf_all\overview_design.md) 基础上，进一步展开 UOF 的详细设计，目标是为研发实施提供直接输入。

本文档重点覆盖：

* Rust workspace 与 crate 划分
* eBPF 探针运行时设计
* Agent 内部模块设计
* 控制面服务设计
* 插件包格式与生命周期
* 统一观测模型详细结构
* 核心接口、关键流程与状态机
* MVP 研发拆分建议

---

## 2. 设计范围

本详细设计覆盖 MVP 范围内的核心模块：

* 基础 eBPF probe runtime
* Rust Agent
* OTLP 输出链路
* 控制面最小能力
* PostgreSQL / Nginx 模板示例
* 插件与模板加载机制

暂不展开以下内容：

* 自研复杂 Web 前端
* 多租户隔离的完整实现
* 高级自动根因分析引擎
* 商业化计费和组织管理能力

---

## 3. 总体实现形态

UOF 采用 Rust workspace 多 crate 组织方式，按“共享模型、内核探针、节点 Agent、控制面、工具链”拆分。

建议仓库结构如下：

```text
uof/
├─ Cargo.toml
├─ Cargo.lock
├─ crates/
│  ├─ uof-common/
│  ├─ uof-model/
│  ├─ uof-ebpf/
│  ├─ uof-probe-runtime/
│  ├─ uof-agent/
│  ├─ uof-exporter-otlp/
│  ├─ uof-plugin-sdk/
│  ├─ uof-control-plane/
│  ├─ uof-control-api/
│  ├─ uof-registry/
│  ├─ uof-cli/
│  └─ uof-testkit/
├─ plugins/
│  ├─ postgres/
│  └─ nginx/
├─ deploy/
│  ├─ helm/
│  └─ systemd/
└─ docs/
```

---

## 4. Workspace 与 crate 设计

## 4.1 `uof-common`

职责：

* 通用错误类型
* 公共配置结构
* 时间、ID、资源标签等基础工具
* 统一日志初始化

建议内容：

* `error.rs`
* `config.rs`
* `id.rs`
* `time.rs`
* `resource.rs`

## 4.2 `uof-model`

职责：

* 定义统一观测模型
* 定义 Metrics / Trace / Event / Log 的内部表达
* 定义实体模型与标签规范

建议内容：

* `entity.rs`
* `metric.rs`
* `trace.rs`
* `event.rs`
* `log.rs`
* `severity.rs`
* `schema.rs`

## 4.3 `uof-ebpf`

职责：

* 基于 `aya-ebpf` 编写内核态探针程序
* 定义 map、ring buffer、probe entrypoint
* 输出统一的原始事件结构

说明：

* 该 crate 编译目标为 `bpfel-unknown-none`
* 需要与用户态共享最小化、可安全序列化的结构定义

建议目录：

```text
uof-ebpf/
├─ src/
│  ├─ main.rs
│  ├─ probes/
│  │  ├─ syscall.rs
│  │  ├─ sched.rs
│  │  ├─ io.rs
│  │  ├─ net.rs
│  │  ├─ lock.rs
│  │  └─ uprobe.rs
│  ├─ maps.rs
│  └─ event.rs
```

## 4.4 `uof-probe-runtime`

职责：

* 在用户态加载 eBPF 对象
* 管理 probe 生命周期
* 绑定配置与 probe 参数
* 暴露标准化的事件消费接口

建议内容：

* `loader.rs`
* `attach.rs`
* `ringbuf.rs`
* `registry.rs`
* `compat.rs`

## 4.5 `uof-agent`

职责：

* 作为节点级常驻进程运行
* 管理 probe runtime、插件、事件流水线和数据导出
* 提供本地健康检查与调试接口

这是 MVP 最核心的 crate。

## 4.6 `uof-exporter-otlp`

职责：

* 将内部统一模型转换为 OTLP 数据
* 负责批量发送、重试、背压和限流

## 4.7 `uof-plugin-sdk`

职责：

* 定义插件 manifest、模板 schema、兼容矩阵
* 提供插件打包与校验逻辑
* 提供模板开发辅助能力

## 4.8 `uof-control-plane`

职责：

* 编排控制面核心逻辑
* 处理 Agent 注册、插件发布、模板启停、配置下发

## 4.9 `uof-control-api`

职责：

* 基于 `axum` 暴露 HTTP API
* 作为控制面对外入口

## 4.10 `uof-registry`

职责：

* 与 OCI registry 交互
* 下载、上传、校验插件包

## 4.11 `uof-cli`

职责：

* 提供本地运维和开发命令
* 支持插件打包、部署、调试、状态查看

## 4.12 `uof-testkit`

职责：

* 提供测试夹具
* 集成测试辅助
* 回归测试数据生成

---

## 5. eBPF 详细设计

## 5.1 探针分类

MVP 探针按以下类别组织：

* `syscall probe`
* `scheduler probe`
* `io probe`
* `network probe`
* `lock probe`
* `uprobe`

MVP 优先级建议：

1. syscall
2. sched
3. io
4. network
5. lock
6. uprobe

## 5.2 原始事件结构

内核态输出统一头部，用户态再做二次归一化。

```rust
#[repr(C)]
pub struct RawEventHeader {
    pub ts_ns: u64,
    pub event_type: u16,
    pub version: u16,
    pub cpu_id: u32,
    pub pid: u32,
    pub tid: u32,
    pub uid: u32,
    pub gid: u32,
    pub cgroup_id: u64,
    pub mount_ns: u64,
    pub payload_len: u32,
}
```

payload 建议按事件类型拆分：

* `SyscallPayload`
* `SchedPayload`
* `IoPayload`
* `NetPayload`
* `LockPayload`
* `UprobePayload`

## 5.3 map 设计

MVP 建议使用以下 map：

* `RINGBUF events`
* `HASH config_map`
* `LRU_HASH pid_context`
* `HASH sampling_policy`
* `PERCPU_ARRAY scratch`

设计原则：

* 事件主通道统一走 ring buffer
* 高速临时计算使用 per-cpu scratch
* 上下文缓存使用 LRU，避免无限增长

## 5.4 probe 生命周期

probe 状态机：

```text
Registered -> Loaded -> Attached -> Running -> Draining -> Detached -> Unloaded
```

状态说明：

* `Registered`：探针已登记但未加载
* `Loaded`：eBPF object 已加载进内核
* `Attached`：已附着到 hook 点
* `Running`：持续产生事件
* `Draining`：停止接收新请求，等待缓冲区清空
* `Detached`：已从 hook 点解绑
* `Unloaded`：资源已释放

## 5.5 内核兼容策略

首版按以下方式处理兼容：

1. 启动时收集内核版本与能力信息
2. 根据能力矩阵决定启用哪些 probe
3. 不支持的 probe 标记为 `degraded`
4. Agent 上报兼容性结果给控制面

兼容结果结构建议：

```rust
pub struct ProbeCapabilityResult {
    pub probe_name: String,
    pub supported: bool,
    pub reason: Option<String>,
    pub fallback_used: bool,
}
```

---

## 6. Agent 详细设计

## 6.1 进程内模块结构

```text
uof-agent
├─ bootstrap
├─ config-manager
├─ probe-manager
├─ plugin-manager
├─ context-resolver
├─ event-pipeline
├─ aggregator
├─ exporter
├─ health
└─ admin-api
```

## 6.2 启动流程

```text
load config
  -> init logger / metrics
  -> collect host capability
  -> register to control plane
  -> load baseline plugins
  -> init probe runtime
  -> attach baseline probes
  -> start event pipeline
  -> start otlp exporter
  -> expose admin api
```

## 6.3 核心子模块

### 6.3.1 `bootstrap`

职责：

* 加载配置
* 初始化 tracing
* 初始化资源限制
* 启动各核心模块

### 6.3.2 `config-manager`

职责：

* 合并本地配置和控制面下发配置
* 支持动态刷新部分策略

配置来源优先级：

1. CLI 参数
2. 本地文件
3. 环境变量
4. 控制面下发的运行策略

### 6.3.3 `probe-manager`

职责：

* 管理 probe runtime
* 将模板配置映射到具体探针实例
* 动态启停 probe

建议接口：

```rust
#[async_trait]
pub trait ProbeManager {
    async fn load_plugin_probes(&self, plugin_id: &str) -> Result<()>;
    async fn enable_probe(&self, probe_id: &str) -> Result<()>;
    async fn disable_probe(&self, probe_id: &str) -> Result<()>;
    async fn list_status(&self) -> Result<Vec<ProbeStatus>>;
}
```

### 6.3.4 `plugin-manager`

职责：

* 从本地缓存或 registry 加载插件包
* 完成签名、版本、兼容性校验
* 注册模板和映射规则

缓存目录建议：

```text
/var/lib/uof/plugins/
├─ cache/
├─ active/
└─ staging/
```

### 6.3.5 `context-resolver`

职责：

* 将 pid/tid/cgroup 等原始信息解析为统一实体上下文
* 补充 host、container、process、service、db_instance 等字段

数据来源：

* `/proc`
* cgroup
* 容器 runtime 元数据
* 本地服务发现缓存
* 模板定义的映射规则

### 6.3.6 `event-pipeline`

职责：

* 消费 ring buffer
* 反序列化原始事件
* 应用采样、过滤、脱敏和归一化
* 分发到 metric、trace、event、log 转换器

流水线阶段建议：

```text
raw -> decode -> enrich -> filter -> sample -> classify -> aggregate/export
```

### 6.3.7 `aggregator`

职责：

* 本地预聚合高频事件
* 生成周期性指标
* 保留关键异常事件明细

建议聚合窗口：

* 默认 `5s`
* 可按插件模板覆盖为 `1s / 10s / 30s`

### 6.3.8 `exporter`

职责：

* OTLP 数据封装
* 批量发送
* 重试、背压、优先级丢弃

建议队列分层：

* `critical queue`：锁等待、异常事件、严重告警
* `normal queue`：指标、普通 trace
* `best effort queue`：调试日志、低优先级样本

### 6.3.9 `health`

职责：

* 采集 Agent 自身健康指标
* 执行插件资源预算检查
* 异常时触发降级

### 6.3.10 `admin-api`

职责：

* 暴露本地 `/healthz` `/readyz` `/metrics` `/debug/*`
* 支持本地调试和排障

---

## 7. 控制面详细设计

## 7.1 服务划分

MVP 阶段建议逻辑上拆分、物理上单体部署：

* `control-api`
* `agent-service`
* `plugin-service`
* `template-service`
* `policy-service`
* `audit-service`

## 7.2 数据表设计

建议核心表如下：

* `agents`
* `agent_heartbeats`
* `plugins`
* `plugin_versions`
* `templates`
* `template_bindings`
* `policies`
* `audit_logs`

### 7.2.1 `agents`

字段建议：

* `id`
* `hostname`
* `node_name`
* `ip`
* `kernel_version`
* `os_release`
* `arch`
* `status`
* `last_seen_at`
* `labels`
* `created_at`
* `updated_at`

### 7.2.2 `plugins`

字段建议：

* `id`
* `name`
* `kind`
* `publisher`
* `default_version`
* `status`
* `created_at`
* `updated_at`

### 7.2.3 `plugin_versions`

字段建议：

* `id`
* `plugin_id`
* `version`
* `digest`
* `oci_ref`
* `manifest`
* `compat_matrix`
* `signature_status`
* `created_at`

## 7.3 控制面 API

### 7.3.1 Agent API

* `POST /api/v1/agents/register`
* `POST /api/v1/agents/{id}/heartbeat`
* `GET /api/v1/agents/{id}/desired-state`
* `POST /api/v1/agents/{id}/ack`

注册请求示例：

```json
{
  "hostname": "db-node-01",
  "kernel_version": "5.15.0",
  "arch": "x86_64",
  "labels": {
    "env": "prod",
    "role": "database"
  },
  "capabilities": {
    "ebpf": true,
    "ringbuf": true,
    "kprobe": true,
    "uprobe": true
  }
}
```

### 7.3.2 Plugin API

* `POST /api/v1/plugins`
* `POST /api/v1/plugins/{plugin_id}/versions`
* `GET /api/v1/plugins`
* `GET /api/v1/plugins/{plugin_id}`
* `POST /api/v1/plugins/{plugin_id}/release`

### 7.3.3 Template API

* `POST /api/v1/templates`
* `GET /api/v1/templates`
* `POST /api/v1/template-bindings`
* `DELETE /api/v1/template-bindings/{id}`

## 7.4 期望状态下发模型

控制面不直接远程调用 Agent 执行动作，而是下发 `desired state`。

```rust
pub struct DesiredState {
    pub generation: u64,
    pub plugins: Vec<PluginActivation>,
    pub templates: Vec<TemplateBinding>,
    pub sampling: SamplingPolicySet,
    pub exporter: ExporterPolicy,
}
```

Agent 比较本地 `applied generation` 与远端 `generation`，按顺序执行并回执。

这种方式更适合网络抖动和断连重连场景。

---

## 8. 插件与模板详细设计

## 8.1 插件包目录结构

```text
plugin/
├─ manifest.yaml
├─ probes/
│  ├─ syscall.bpf.o
│  ├─ io.bpf.o
│  └─ lock.bpf.o
├─ mappings/
│  ├─ entities.yaml
│  └─ events.yaml
├─ dashboards/
│  └─ grafana.json
├─ alerts/
│  └─ rules.yaml
└─ templates/
   └─ scenario.yaml
```

## 8.2 manifest 设计

```yaml
apiVersion: uof.io/v1
kind: Plugin
metadata:
  name: postgres-observability
  version: 0.1.0
  publisher: uof
spec:
  type: template
  target:
    software: postgresql
    versions: ["14", "15", "16"]
  runtime:
    minKernel: "5.4.0"
    arch: ["x86_64", "aarch64"]
  resources:
    maxCpuPercent: 3
    maxMemoryMB: 128
    maxEventsPerSec: 10000
  permissions:
    requires:
      - bpf
      - perfmon
  assets:
    probes:
      - probes/syscall.bpf.o
      - probes/io.bpf.o
    mappings:
      - mappings/entities.yaml
      - mappings/events.yaml
    dashboards:
      - dashboards/grafana.json
    alerts:
      - alerts/rules.yaml
```

## 8.3 插件生命周期

状态机：

```text
Uploaded -> Verified -> Published -> Downloaded -> Installed -> Enabled
Enabled -> Disabled -> Enabled
Installed -> Upgraded
Installed -> Uninstalled
Any -> Failed
```

设计规则：

* `Uploaded` 仅表示包已存在
* `Verified` 表示签名、格式、兼容性检查通过
* `Published` 表示可被 Agent 拉取
* `Enabled` 表示至少一个 Agent 已应用

## 8.4 模板绑定

模板不直接等于插件。模板绑定用于表达“哪个模板作用于哪些目标”。

绑定维度建议：

* 节点标签
* 进程特征
* 容器标签
* 软件类型
* 手动绑定

模板绑定示例：

```yaml
selector:
  labels:
    role: database
target:
  software: postgresql
policy:
  samplingProfile: default-db
  alertsEnabled: true
```

---

## 9. 统一观测模型详细设计

## 9.1 实体模型

### `HostEntity`

```rust
pub struct HostEntity {
    pub host_id: String,
    pub hostname: String,
    pub ip: Option<String>,
    pub kernel_version: String,
    pub labels: BTreeMap<String, String>,
}
```

### `ProcessEntity`

```rust
pub struct ProcessEntity {
    pub process_id: String,
    pub pid: u32,
    pub ppid: u32,
    pub exe: String,
    pub cmdline: Vec<String>,
    pub container_id: Option<String>,
    pub service_name: Option<String>,
}
```

### `DbSessionEntity`

```rust
pub struct DbSessionEntity {
    pub session_id: String,
    pub db_instance_id: String,
    pub backend_pid: Option<u32>,
    pub user_name: Option<String>,
    pub database_name: Option<String>,
    pub client_addr: Option<String>,
}
```

## 9.2 事件模型

```rust
pub struct EventRecord {
    pub event_id: String,
    pub ts_ns: u64,
    pub severity: Severity,
    pub category: EventCategory,
    pub entity_refs: EntityRefs,
    pub attrs: BTreeMap<String, Value>,
}
```

`EventCategory` 建议值：

* `syscall`
* `sched`
* `io`
* `network`
* `lock`
* `db`
* `anomaly`

## 9.3 指标模型

```rust
pub struct MetricRecord {
    pub name: String,
    pub kind: MetricKind,
    pub unit: String,
    pub resource: ResourceRef,
    pub attributes: BTreeMap<String, String>,
    pub point: MetricPoint,
}
```

`MetricKind`：

* `Counter`
* `Gauge`
* `Histogram`
* `Summary`

## 9.4 Trace 模型

```rust
pub struct TraceRecord {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
    pub attributes: BTreeMap<String, Value>,
}
```

## 9.5 高基数字段处理

规则：

1. SQL 原文默认不进入指标标签
2. SQL 仅保留 `query_template_id`
3. 调用栈默认采样上报，不做全量指标标签
4. 文件路径、socket 地址等字段进入 event/trace 属性，必要时脱敏

---

## 10. 数据流详细设计

## 10.1 节点内数据流

```text
probe -> ringbuf -> decoder -> enrich -> classify -> aggregate -> export
```

各阶段说明：

* `probe`：内核态产生原始事件
* `decoder`：按 event_type 解码 payload
* `enrich`：补实体和上下文
* `classify`：转换成 metric/trace/event/log
* `aggregate`：本地聚合高频数据
* `export`：OTLP 发往 Collector

## 10.2 背压策略

按优先级丢弃：

1. 先丢弃 `best effort queue`
2. 再降低 trace 样本率
3. 再降低高频非关键 metric 细粒度
4. 禁止直接阻塞 probe 消费线程

## 10.3 失败处理

* ring buffer 读取失败：计数并重试
* exporter 发送失败：进入重试队列
* 插件加载失败：回滚到上一个稳定版本
* 控制面不可达：进入离线模式，继续本地采集

---

## 11. 安全与权限详细设计

## 11.1 Agent 权限

建议能力：

* `CAP_BPF`
* `CAP_PERFMON`
* `CAP_NET_ADMIN` 仅在网络场景必要时启用

禁止默认申请过大权限。

## 11.2 插件校验

校验步骤：

1. digest 校验
2. manifest schema 校验
3. 签名校验
4. 兼容矩阵校验
5. 资源预算校验
6. 资产完整性校验

## 11.3 敏感数据治理

敏感信息分级：

* `L1`：普通运行指标
* `L2`：实例名、进程名、SQL 模板
* `L3`：SQL 原文、请求参数、文件路径、IP 等

策略：

* 默认仅采集 `L1/L2`
* `L3` 需显式开启
* `L3` 采集需审计记录

---

## 12. 可观测性与运维设计

## 12.1 Agent 自监控指标

* `uof_agent_events_total`
* `uof_agent_events_dropped_total`
* `uof_agent_ringbuf_read_errors_total`
* `uof_agent_plugin_load_failures_total`
* `uof_agent_export_retries_total`
* `uof_agent_probe_active_count`
* `uof_agent_pipeline_latency_ms`

## 12.2 控制面自监控指标

* `uof_control_agent_registered_total`
* `uof_control_plugin_publish_total`
* `uof_control_api_latency_ms`
* `uof_control_desired_state_generation`

## 12.3 调试接口

本地接口建议：

* `GET /healthz`
* `GET /readyz`
* `GET /metrics`
* `GET /debug/probes`
* `GET /debug/plugins`
* `GET /debug/pipeline`

---

## 13. PostgreSQL 模板详细设计

## 13.1 目标

提供 PostgreSQL 的基础性能诊断模板，覆盖：

* 慢查询
* 锁等待
* IO 抖动
* WAL 压力
* 活跃会话异常

## 13.2 探针组合

建议首版包含：

* syscall write/fsync 相关 probe
* 调度延迟 probe
* 文件 IO probe
* PostgreSQL backend 进程识别规则
* 可选 uprobe：后续按版本逐步支持

## 13.3 输出指标

* `db_query_latency_ms`
* `db_active_sessions`
* `db_lock_wait_seconds`
* `db_wal_write_bytes`
* `db_io_latency_ms`

## 13.4 输出事件

* `db.lock_wait_detected`
* `db.io_spike_detected`
* `db.slow_query_detected`

---

## 14. Nginx 模板详细设计

## 14.1 目标

提供 Nginx 的基础请求性能模板，覆盖：

* 请求延迟趋势
* Worker CPU 占用
* 网络与磁盘热点

## 14.2 输出指标

* `http_request_latency_ms`
* `http_active_connections`
* `http_worker_cpu_percent`
* `http_write_syscall_total`

---

## 15. 测试设计

## 15.1 单元测试

覆盖：

* manifest 解析
* 事件解码
* 统一模型转换
* 聚合窗口计算
* 兼容矩阵匹配

## 15.2 集成测试

覆盖：

* Agent 启动与 probe 加载
* OTLP 导出到测试 Collector
* 插件下载、校验、启用
* desired state 应用与回执

## 15.3 环境测试

建议最少覆盖：

* Ubuntu 22.04 / kernel 5.15
* Rocky Linux 9 / kernel 5.14
* Containerd + Kubernetes 环境

---

## 16. MVP 开发拆分建议

## 16.1 阶段一：基础框架

交付物：

* Rust workspace 初始化
* `uof-common`
* `uof-model`
* `uof-ebpf`
* `uof-probe-runtime`

## 16.2 阶段二：Agent 跑通

交付物：

* `uof-agent`
* ring buffer 消费链路
* OTLP exporter
* 本地 admin api

## 16.3 阶段三：控制面最小闭环

交付物：

* `uof-control-api`
* `uof-control-plane`
* `uof-registry`
* PostgreSQL 元数据表

## 16.4 阶段四：模板落地

交付物：

* PostgreSQL 模板
* Nginx 模板
* Grafana dashboard
* 基础告警规则

---

## 17. 待确认项

1. 首版 PostgreSQL 是否需要支持多个主版本差异化模板。
2. 首版是否支持 aarch64。
3. 是否要求裸机与 K8s 同时达到生产可用。
4. 是否需要在 MVP 即支持插件签名。
5. 是否需要在 MVP 即支持 SQL 原文脱敏规则配置。

---

## 18. 结论

本设计将 UOF 落成一套以 Rust 为主线的可实现方案：

* eBPF 探针基于 `Aya`
* 节点 Agent、控制面、CLI 全部基于 Rust workspace 管理
* 插件通过 OCI 分发，模板通过 manifest 和绑定策略驱动
* 数据通过统一模型归一化后，以 OTLP 进入 OTEL Collector

按该设计推进，可以较平滑地从 MVP 走向后续的插件生态和诊断能力扩展。
