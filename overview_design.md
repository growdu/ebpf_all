# 通用软件可观测性框架概要设计文档（v1.0）

## 1. 文档目标

本文档用于在产品文档基础上，给出通用软件可观测性框架 UOF 的技术选型与概要设计，作为后续详细设计、PoC 和研发拆分的输入。

本文档重点回答以下问题：

* 首版采用什么技术栈
* 为什么这样选
* 各模块边界如何划分
* 数据如何流转
* 插件与模板如何落地
* MVP 应该先做什么

---

## 2. 设计目标与约束

## 2.1 设计目标

1. 支持 Linux 上多类软件的低开销观测。
2. 以统一方式采集 Metrics、Trace、Log、Event。
3. 支持模板化场景交付，数据库 DBA 作为首发示例。
4. 支持插件扩展，避免每接入一个场景都重做一套系统。
5. 优先满足可落地、可迭代、可运维，而不是一次性追求功能最全。

## 2.2 关键约束

1. 目标环境以 Linux 为主，内核建议 `>= 4.18`。
2. 需要兼顾裸机、虚机和 Kubernetes 场景。
3. 采集链路必须低开销，不能明显干扰业务。
4. 插件能力需要受控，不能让用户代码破坏目标主机。
5. 首版优先完成通用底座和一个完整示例场景。

---

## 3. 总体技术路线

UOF 采用“内核 eBPF 探针 + 用户态 Agent + OTEL 数据管道 + 模板/插件系统 + 可视化/告警”的总体路线。

分层如下：

1. **采集层**  
   负责从内核和进程侧采集底层运行时数据。

2. **归一化层**  
   负责将原始事件映射为统一观测模型。

3. **管道层**  
   负责数据接收、聚合、采样、路由和导出。

4. **交付层**  
   负责模板、Dashboard、告警、插件和诊断视图。

5. **控制面层**  
   负责插件管理、模板管理、配置下发、版本治理和权限控制。

---

## 4. 技术选型

## 4.1 选型总览

| 模块 | 选型 | 结论 |
| --- | --- | --- |
| eBPF 程序开发 | `Rust + Aya` | 首选 |
| Agent 用户态开发 | `Rust` | 首选 |
| 控制面服务 | `Rust` | 首选 |
| 数据接入协议 | `OTLP/gRPC` | 首选 |
| 数据管道 | `OpenTelemetry Collector` | 首选 |
| 指标存储 | `Prometheus` | MVP 首选 |
| Trace 存储 | `Tempo` | MVP 首选 |
| 日志/事件存储 | `Loki` | MVP 首选 |
| Dashboard | `Grafana` | 首选 |
| 插件包格式 | `OCI Artifact + manifest` | 推荐 |
| 插件元数据存储 | `PostgreSQL` | 首选 |
| 缓存/任务协同 | `Redis` | 可选，不进 MVP 必选项 |
| 部署方式 | `Kubernetes + Helm`，兼容 systemd/裸机 | 首选 |

## 4.2 eBPF 方案选型

### 候选方案

* `Rust + Aya`
* `C + libbpf + CO-RE`
* `BCC`

### 结论

首版建议选择 **`Rust + Aya`**。

### 原因

1. 可以将 eBPF、Agent、控制面统一到 Rust 技术栈，降低语言切换成本。
2. Aya 提供纯 Rust 的 eBPF 开发与加载能力，不依赖 libbpf 和 clang 运行时。
3. 更符合当前项目“全栈 Rust”的技术路线，便于共享类型定义、工具链和工程规范。
4. 在保障性能的同时，提升内存安全和代码一致性。
5. 对长期维护、代码审查和团队知识复用更友好。

### 放弃原因

* `C + libbpf + CO-RE` 在生产成熟度上依然非常强，但会引入双语言栈，增加开发与维护心智负担。
* `BCC` 开发体验较好，但运行时依赖较重，跨环境交付与长期维护成本更高。

### 选型注意事项

1. Aya 路线能够提升语言统一性，但首版需要额外验证内核兼容性和复杂 probe 场景。
2. 对极复杂或特殊内核特性的探针，后续仍应保留少量回退到 `C + libbpf` 的预案。
3. MVP 阶段应优先覆盖 syscall、调度、网络、IO、锁等通用探针，再逐步扩展复杂场景。

## 4.3 Agent / Control Plane 开发语言选型

### 候选方案

* `Rust`
* `Go`
* `C++`

### 结论

首版建议选择 **`Rust`** 作为用户态 Agent 和控制面主语言。

### 原因

1. 更适合构建对性能、内存安全和资源控制要求较高的基础设施组件。
2. 与 eBPF / Linux 系统编程生态契合度更高，便于统一语言栈。
3. 在高并发、长生命周期 Agent 场景中，能更细粒度控制内存分配和运行时开销。
4. 适合后续扩展到高性能解码、聚合、规则处理和本地分析逻辑。
5. 社区已有较成熟的异步、gRPC、CLI、序列化与可观测性生态可用。

### 说明

eBPF、用户态 Agent 与控制面统一使用 Rust 生态，形成“全栈 Rust”组合。其中 eBPF 探针建议基于 Aya 开发。

### 推荐 Rust 技术栈

* eBPF framework：`aya` `aya-ebpf`
* 异步运行时：`tokio`
* gRPC / OTLP 客户端：`tonic`
* HTTP API：`axum`
* 序列化：`serde`
* CLI：`clap`
* 数据库访问：`sqlx`
* tracing：`tracing`

## 4.4 数据管道选型

### 结论

首版建议以 **OpenTelemetry Collector** 作为统一数据管道。

### 原因

1. 可以避免自建一整套接收、处理、转发系统。
2. 原生支持 OTLP，利于后续与外部生态集成。
3. 支持 processor/exporter 扩展，适合做统一观测模型转换。
4. 便于逐步演进出多级 Collector、边缘聚合和分流策略。

### 设计建议

* Agent 输出优先采用 `OTLP/gRPC`
* Collector 中增加自定义 processor，用于 UOF 统一语义补全
* 不建议首版直接引入 Kafka 作为强依赖

## 4.5 存储与可视化选型

### MVP 推荐组合

* 指标：`Prometheus`
* Trace：`Tempo`
* 日志与事件：`Loki`
* 可视化：`Grafana`

### 原因

1. 与 OTEL 和云原生生态契合度高。
2. Grafana 能直接承载模板化交付。
3. 首版可以快速搭建端到端闭环。
4. 降低自研 UI 和存储系统复杂度。

### 后续演进

* 若事件查询、聚合分析需求增强，可再引入 Elasticsearch / ClickHouse。
* 若指标规模增长明显，可再演进到分层 TSDB 架构。

## 4.6 控制面元数据存储选型

### 结论

首版建议使用 **PostgreSQL** 存储控制面元数据。

### 存储内容

* 插件元数据
* 模板版本
* 兼容矩阵
* Agent 注册信息
* 策略配置
* 审计记录

### 原因

1. 关系模型清晰，适合版本、状态、兼容性和权限管理。
2. 事务能力强，适合控制面数据一致性要求。
3. 运维成熟，便于后续扩展。

## 4.7 插件打包与分发选型

### 结论

首版建议插件采用 **`OCI Artifact + manifest`** 形式分发。

### 包内容建议

* 插件元数据 `manifest.yaml`
* eBPF 对象文件
* 用户态转换逻辑配置
* 统一模型映射规则
* Grafana dashboard 模板
* 告警规则模板
* 兼容性声明

### 原因

1. 便于复用现有镜像仓库和制品管理能力。
2. 支持版本化、签名和回滚。
3. 有利于后续构建模板市场或企业内部插件仓库。

---

## 5. 总体架构设计

## 5.1 架构图

```text
+--------------------------------------------------------------+
|                         Control Plane                        |
|  Plugin API | Template API | Policy API | Metadata Store    |
+--------------------------------------------------------------+
                 |                    ^
                 v                    |
+-----------------------------+       |
|    Plugin / Template Repo   |-------+
|    OCI Registry / Store     |
+-----------------------------+

                 |
                 v
+--------------------------------------------------------------+
|                         Target Node                          |
|  +-------------------+        +---------------------------+  |
|  | UOF Agent         |        | Target Process           |  |
|  | - Probe Manager   |<------>| DB / Web / Middleware    |  |
|  | - Event Decoder   |        +---------------------------+  |
|  | - OTLP Exporter   |                                      |
|  +-------------------+                                      |
|          ^                                                   |
|          |                                                   |
|  +-------------------+                                      |
|  | eBPF Programs     |                                      |
|  | kprobe/uprobe/... |                                      |
|  +-------------------+                                      |
+--------------------------------------------------------------+

                 |
                 v
+--------------------------------------------------------------+
|                     OTEL Collector Layer                     |
| receiver -> processor -> enricher -> exporter               |
+--------------------------------------------------------------+
                 |
                 v
+--------------------------------------------------------------+
|        Prometheus / Tempo / Loki / Grafana / Alerting        |
+--------------------------------------------------------------+
```

## 5.2 模块划分

### 1. eBPF Probe Runtime

职责：

* 管理内核态探针生命周期
* 采集 syscall、调度、锁、IO、网络、函数调用等事件
* 将原始事件写入 ring buffer / map

### 2. Agent

职责：

* 加载和卸载 eBPF 探针
* 消费 ring buffer 数据
* 进行事件解码、采样、聚合、字段补全
* 按统一模型转成 OTEL 数据
* 上报运行状态和采集健康度

### 3. Collector Pipeline

职责：

* 统一接收 Agent 上报数据
* 做批处理、限流、资源控制
* 做跨节点统一补充和导出

### 4. Plugin / Template Manager

职责：

* 管理插件安装、升级、启停、回滚
* 校验插件兼容性和签名
* 下发模板和配置

### 5. Control Plane

职责：

* 提供管理 API
* 管理 Agent、插件、模板、策略和审计
* 为后续 Web UI 提供服务端能力

### 6. Visualization / Alert

职责：

* 提供 Dashboard
* 提供告警规则
* 提供按场景组织的诊断视图

---

## 6. 核心模块概要设计

## 6.1 Agent 设计

### 内部子模块

* `probe-manager`
* `plugin-loader`
* `event-decoder`
* `event-normalizer`
* `sampler`
* `aggregator`
* `otel-exporter`
* `health-reporter`

### 关键职责

1. 根据配置装载默认探针与模板探针。
2. 将原始 eBPF 事件解码为结构化事件。
3. 填充主机、容器、进程、线程、服务等上下文字段。
4. 执行本地采样、聚合与速率限制。
5. 输出 OTLP 数据到 Collector。

### 关键设计决策

* 优先使用 `ring buffer`，性能和可维护性优于传统 perf buffer 方案。
* 高基数事件尽量在 Agent 本地预聚合，避免把原始明细全量打到中心。
* Agent 必须可降级，中心不可用时允许丢弃低优先级数据而不是阻塞业务线程。

## 6.2 插件系统设计

### 插件类型

* **Probe 插件**：新增探针和事件定义
* **Mapping 插件**：定义统一语义映射规则
* **Template 插件**：提供 dashboard、告警与场景说明

### 首版建议

首版不要开放“任意用户代码执行型插件”，而是采用“受约束的插件包”模式：

* 限定插件结构
* 限定可加载对象
* 限定配置项
* 限定输出 schema

这样能先控制安全和复杂度。

### 后续演进

如需更强扩展能力，可在后续版本引入 Wasm 作为用户态轻量扩展沙箱，用于字段加工、规则处理和轻量转换逻辑。

## 6.3 统一观测模型设计

### 统一实体

* infrastructure：`host` `node` `container`
* runtime：`process` `thread`
* service：`service` `endpoint`
* database：`db_instance` `db_session` `query_template`

### 统一事件分类

* `resource_event`
* `syscall_event`
* `io_event`
* `network_event`
* `lock_event`
* `scheduler_event`
* `app_function_event`
* `anomaly_event`

### 统一标签原则

1. 必须区分稳定标签和高基数标签。
2. 默认存储稳定标签，高基数字段以事件或 trace 属性形式保存。
3. SQL 文本、调用栈、文件路径等高基数数据必须做脱敏、截断或模板归并。

## 6.4 控制面设计

### API 能力

* Agent 注册与心跳
* 插件上传、签名校验、版本发布
* 模板启用、禁用、配置下发
* 兼容矩阵管理
* 审计日志查询

### 服务拆分建议

MVP 阶段可先单体实现：

* `uof-control-api`
* `uof-scheduler`
* `uof-registry-adapter`

后续再按规模拆分。

---

## 7. 数据链路设计

## 7.1 采集链路

```text
eBPF Probe
  -> Ring Buffer
  -> Agent Decoder
  -> Local Normalize / Sample / Aggregate
  -> OTLP Exporter
  -> OTEL Collector
  -> Storage
  -> Grafana / Alert / UI
```

## 7.2 数据格式策略

### Metrics

适合：

* QPS
* 延迟分布
* CPU / 内存 / IO 用量
* 锁等待总量

### Trace

适合：

* 请求分阶段耗时
* 函数调用链
* 数据库操作路径

### Event

适合：

* 异常触发
* 状态变化
* 阻塞链发现
* 高开销热点出现

### Log

适合：

* 补充错误上下文
* 记录诊断辅助信息

## 7.3 采样与聚合策略

首版建议：

* 高频事件默认采样
* 明细 trace 仅保留热点样本
* 计数和摘要尽量本地聚合
* 对锁、IO、慢 SQL 等关键事件保留更高优先级

---

## 8. 部署设计

## 8.1 Kubernetes 部署

### 组件形态

* `uof-agent`：DaemonSet
* `uof-collector`：Deployment
* `uof-control-plane`：Deployment
* `postgresql`：StatefulSet 或外部托管
* `grafana/prometheus/tempo/loki`：复用现有监控栈或独立部署

### 原因

* Agent 天然适合按节点部署
* Collector 和控制面可水平扩缩容
* 便于模板、配置、版本统一治理

## 8.2 裸机部署

建议提供：

* systemd 服务
* 本地配置文件
* CLI 管理工具

这样兼容非 K8s 客户环境。

---

## 9. 安全设计

## 9.1 权限最小化

优先使用更细粒度的权限能力，不默认依赖 `CAP_SYS_ADMIN`。在内核和发行版允许的情况下，优先考虑：

* `CAP_BPF`
* `CAP_PERFMON`
* `CAP_NET_ADMIN`（仅在必要场景）

## 9.2 插件安全

* 插件包必须带版本、来源和兼容性声明
* 插件安装前执行签名校验和静态检查
* 插件启用后持续采集资源开销
* 异常插件支持自动禁用和回滚

## 9.3 数据安全

* SQL、参数、文件路径、请求参数等敏感字段需脱敏
* 支持字段级开关，控制是否采集高敏感信息
* 控制面所有配置变更应有审计记录

---

## 10. 可运维性设计

UOF 自身也必须可观测。

### 关键自监控指标

* Agent CPU / 内存占用
* ring buffer 丢包率
* 事件解码失败数
* OTLP 发送失败率
* 插件加载成功率
* Collector 背压情况

### 关键运维能力

* 动态调整采样率
* 动态关闭高开销探针
* 快速回滚插件版本
* 探针健康检查与自愈

---

## 11. MVP 设计建议

## 11.1 MVP 范围

首版建议聚焦以下内容：

1. 基础 eBPF probe runtime
2. Rust Agent
3. OTLP 输出
4. OTEL Collector 接入
5. Prometheus + Tempo + Loki + Grafana 闭环
6. PostgreSQL 模板示例
7. Nginx 模板示例
8. 控制面最小能力：插件包管理、模板启用、Agent 注册

## 11.2 MVP 不做

* 复杂多租户
* 完整插件市场
* 自研前端诊断控制台
* 自动化根因推理引擎
* 大规模消息总线强依赖

## 11.3 里程碑建议

### M1：底座跑通

* 单机 Agent 加载基础探针
* 输出 CPU / IO / syscall 等基础数据
* 接入 Collector 和 Grafana

### M2：模板跑通

* 完成 PostgreSQL 模板
* 支持慢查询、锁等待、IO 相关观测
* 输出默认 Dashboard 和告警

### M3：平台最小闭环

* 引入插件包管理
* 支持模板启停与版本管理
* 完成 K8s 部署

---

## 12. 风险与待决策项

## 12.1 主要风险

1. 不同内核版本兼容成本高。
2. 高基数数据易导致存储和查询成本失控。
3. 数据库等复杂场景可能需要额外结合应用内部视图，单靠 eBPF 不足。
4. 插件体系若过早开放过强能力，会带来明显安全风险。

## 12.2 待决策项

1. 首版是否同时支持 PostgreSQL 和 MySQL。
2. 首版控制面是否直接提供 Web UI。
3. 是否在 v1.0 就引入 Wasm 扩展机制。
4. 非 K8s 环境是否作为首版重点支持对象。
5. 统一观测模型字段规范是否先以数据库和 Web 两类场景收敛。

---

## 13. 结论

从可落地性、维护成本和生态复用角度看，UOF 首版建议采用以下技术组合：

* eBPF：`Rust + Aya`
* Agent / Control Plane：`Rust`
* Pipeline：`OpenTelemetry Collector + OTLP/gRPC`
* Storage：`Prometheus + Tempo + Loki`
* Visualization：`Grafana`
* Plugin Packaging：`OCI Artifact + manifest`
* Metadata Store：`PostgreSQL`

这套组合的优点是：

1. 统一 Rust 技术栈，降低跨模块开发和维护成本。
2. 兼顾性能、安全性和工程一致性。
3. 云原生生态兼容好。
4. 有利于先做出“通用底座 + 场景模板”的 MVP。
5. 便于后续演进到更强的插件生态和诊断能力。
