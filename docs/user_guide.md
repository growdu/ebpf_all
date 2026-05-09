# UOF 用户使用指南

## 目录

1. [概述](#概述)
2. [快速开始](#快速开始)
3. [Agent 管理](#agent-管理)
4. [插件管理](#插件管理)
5. [模板管理](#模板管理)
6. [监控数据查看](#监控数据查看)
7. [告警配置](#告警配置)
8. [常见问题](#常见问题)

---

## 概述

UOF (Universal Observability Framework) 是一个基于 eBPF + OpenTelemetry 的通用可观测性框架，支持对 Linux 系统上的各类软件（数据库、Web服务、中间件等）进行统一观测。

### 核心能力

- **统一观测模型**：将 Metrics、Trace、Log、Event 统一建模
- **低开销采集**：基于 eBPF 内核探针，实时采集系统级信号
- **模板化交付**：通过模板和插件快速适配不同软件场景
- **插件扩展**：支持自定义探针、指标和诊断视图

---

## 快速开始

### 前置要求

- Linux 内核 4.18+ (支持 eBPF)
- Rust 1.75+ (仅开发/构建需要)
- PostgreSQL (控制面元数据存储，可选)

### 安装构建

```bash
# 克隆项目
git clone https://github.com/your-org/uof.git
cd uof

# 构建所有 crate
cargo build

# 构建 release 版本
cargo build --release
```

### 启动 Control Plane

```bash
# 使用默认配置启动 (监听 127.0.0.1:8080)
cargo run -p uof-control-api

# 指定绑定地址
UOF_BIND_ADDR=0.0.0.0:8080 cargo run -p uof-control-api
```

### 启动 Agent

```bash
# 启动 agent (连接 control plane)
cargo run -p uof-agent

# 启动 agent (独立模式，不连接 control plane)
UOF_CONTROL_PLANE_ENDPOINT=http://127.0.0.1:8080 cargo run -p uof-agent
```

### 使用 CLI

```bash
# 查看版本
cargo run -p uof-cli -- version

# 查看帮助
cargo run -p uof-cli -- --help
```

---

## Agent 管理

### Agent 注册

Agent 启动时会自动向 Control Plane 注册：

```bash
curl -X POST http://localhost:8080/api/v1/agents/register \
  -H "Content-Type: application/json" \
  -d '{
    "hostname": "node-01",
    "version": "0.1.0",
    "capabilities": ["ebpf", "otel"]
  }'
```

### Agent 心跳

Agent 定期发送心跳保持连接：

```bash
curl -X POST http://localhost:8080/api/v1/agents/{agent_id}/heartbeat \
  -H "Content-Type: application/json" \
  -d '{
    "status": "running",
    "probe_states": {
      "syscall": "running",
      "sched": "running"
    }
  }'
```

### 拉取期望配置

```bash
curl http://localhost:8080/api/v1/agents/{agent_id}/desired-state
```

响应示例：
```json
{
  "enabled_probes": ["syscall", "sched", "io"],
  "sampling_rate": 1000,
  "plugins": ["postgres-observability"]
}
```

### 上报配置应用结果

```bash
curl -X POST http://localhost:8080/api/v1/agents/{agent_id}/ack \
  -H "Content-Type: application/json" \
  -d '{
    "applied_probes": ["syscall", "sched"],
    "failed_probes": [],
    "plugins_loaded": ["postgres-observability"]
  }'
```

---

## 插件管理

### 查看可用插件

```bash
curl http://localhost:8080/api/v1/plugins
```

### 上传插件

使用 CLI 打包并上传插件：

```bash
# 打包插件
cargo run -p uof-cli -- plugin pack \
  --dir ./plugins/postgres/ \
  --output postgres.tar.gz

# 上传插件
cargo run -p uof-cli -- plugin push \
  --registry ghcr.io \
  --repo myorg/postgres \
  --tag 0.1.0 \
  --artifact postgres.tar.gz
```

### 从 Registry 拉取插件

```bash
cargo run -p uof-cli -- plugin pull \
  --registry ghcr.io \
  --repo myorg/postgres \
  --tag 0.1.0 \
  --output ./plugins/postgres/
```

或通过 API 拉取：

```bash
curl -X POST http://localhost:8080/api/v1/plugins/pull \
  -H "Content-Type: application/json" \
  -d '{
    "registry": "ghcr.io",
    "repo": "myorg/postgres",
    "tag": "0.1.0"
  }'
```

### 插件 manifest.yaml 示例

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

---

## 模板管理

### 查看可用模板

```bash
curl http://localhost:8080/api/v1/templates
```

### 创建模板绑定

```bash
curl -X POST http://localhost:8080/api/v1/template-bindings \
  -H "Content-Type: application/json" \
  -d '{
    "template_id": "dba-diagnostic-v1",
    "target": {
      "type": "host",
      "selector": {
        "label": "role=database"
      }
    },
    "config": {
      "sampling_rate": 500,
      "enabled_probes": ["syscall", "io", "lock"]
    }
  }'
```

### 删除模板绑定

```bash
curl -X DELETE http://localhost:8080/api/v1/template-bindings/{binding_id}
```

---

## 监控数据查看

### 通过 Grafana 查看

UOF 集成 Grafana，提供以下默认 Dashboard：

| Dashboard | 内容 |
|-----------|------|
| Host Overview | 主机 CPU、内存、IO、网络概览 |
| Process Details | 进程级系统调用、文件IO、网络活动 |
| Database (Template) | 慢查询、锁等待、连接数、缓存命中率 |
| Plugin Metrics | 各插件自定义指标 |

访问 Grafana：`http://localhost:3000` (默认)

### 通过 API 查询

#### 查询健康状态

```bash
# Control Plane 健康检查
curl http://localhost:8080/healthz

# Agent 健康检查
curl http://localhost:8081/healthz

# Agent 就绪检查
curl http://localhost:8081/readyz

# 探针列表
curl http://localhost:8081/debug/probes
```

---

## 告警配置

### 默认告警规则

UOF 提供以下默认告警规则：

| 告警名称 | 条件 | 严重程度 |
|----------|------|----------|
| HighCPU | CPU > 80% | Warning |
| HighMemory | Memory > 85% | Warning |
| SlowQuery | Query latency > 1s | Critical |
| LockWait | Lock wait > 5s | Warning |
| DiskIOHigh | IO utilization > 90% | Warning |

### 自定义告警

通过 Grafana Alerting 配置自定义告警规则：

1. 进入 Grafana → Alerting → Alert rules
2. 点击 "New alert rule"
3. 选择数据源和查询条件
4. 设置告警条件和建议
5. 配置通知渠道

---

## 常见问题

### Q: Agent 无法注册到 Control Plane

检查以下内容：
1. 网络连通性：`curl http://control-plane:8080/healthz`
2. Control Plane 是否正常运行
3. Agent 配置的 `UOF_CONTROL_PLANE_ENDPOINT` 是否正确

### Q: eBPF 探针加载失败

常见原因：
1. 内核版本低于 4.18，不支持 eBPF
2. 缺少 BPF 权限
3. 内核模块签名问题（需关闭 SECURE_BOOT）

解决方案：
```bash
# 检查 eBPF 支持
cat /proc/sys/kernel/bpf_stats_enabled

# 检查内核版本
uname -r
```

### Q: 插件拉取失败

1. 确认 Registry 认证配置正确
2. 检查网络能否访问 Registry
3. 确认插件版本存在

### Q: 性能开销过高

调整采样率：
```bash
# 降低采样率，减少开销
curl -X PATCH http://localhost:8081/config/sampling \
  -d '{"rate": 5000}'
```

---

## 下一步

- 查看[详细设计文档](./detailed_design.md)了解架构细节
- 查看[概要设计文档](./overview_design.md)了解技术选型
- 查看[产品文档](./product.md)了解产品定位