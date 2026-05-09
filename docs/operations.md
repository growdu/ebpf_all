# UOF 运维部署指南

## 目录

1. [概述](#概述)
2. [系统要求](#系统要求)
3. [部署架构](#部署架构)
4. [Kubernetes 部署](#kubernetes-部署)
5. [裸机/虚拟机部署](#裸机虚拟机部署)
6. [配置参考](#配置参考)
7. [安全配置](#安全配置)
8. [监控与告警](#监控与告警)
9. [备份与恢复](#备份与恢复)
10. [升级与回滚](#升级与回滚)
11. [故障排查](#故障排查)

---

## 概述

本文档提供 UOF (Universal Observability Framework) 的运维和部署指南，涵盖生产环境所需的各项配置和操作。

### 组件说明

| 组件 | 说明 | 部署形态 |
|------|------|----------|
| uof-agent | 节点常驻进程，负责探针管理和数据采集 | DaemonSet |
| uof-control-api | HTTP API 层 | Deployment |
| uof-control-plane | 控制面核心逻辑 | Deployment |
| uof-registry | OCI Registry 客户端 | 内置 |
| PostgreSQL | 控制面元数据存储 | StatefulSet 或外部托管 |
| OTEL Collector | 数据管道 | Deployment |
| Prometheus/Tempo/Loki | 时序/链路/日志存储 | 独立部署 |
| Grafana | 可视化 | Deployment |

---

## 系统要求

### 最低要求

| 资源 | 要求 |
|------|------|
| CPU | 4 核 |
| 内存 | 8 GB |
| 磁盘 | 50 GB (根据数据量调整) |
| 内核版本 | 4.18+ (支持 eBPF) |

### 推荐配置

| 资源 | 推荐 |
|------|------|
| CPU | 8 核+ |
| 内存 | 16 GB+ |
| 磁盘 | 100 GB+ SSD |
| 内核版本 | 5.0+ |

### 支持的操作系统

- Ubuntu 20.04+
- Debian 11+
- RHEL 8+
- Rocky Linux 8+
- Amazon Linux 2 (内核 4.14+)

---

## 部署架构

### 生产环境推荐架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                       │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Grafana     │  │ Prometheus  │  │ Loki/Tempo  │             │
│  │ Dashboard   │  │ Metrics    │  │ Logs/Traces│             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                    │
│         └────────────────┼────────────────┘                    │
│                          │                                     │
│               ┌──────────┴──────────┐                         │
│               │   OTEL Collector   │                          │
│               └──────────┬──────────┘                         │
│                          │                                     │
│         ┌────────────────┼────────────────┐                    │
│         │                │                │                    │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────┴──────┐           │
│  │uof-control  │  │uof-control  │  │   PostgreSQL│           │
│  │    -api     │  │   -plane    │  │   (元数据)   │           │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────────────┐
│                      Linux Worker Nodes                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ uof-agent  │  │ uof-agent  │  │ uof-agent  │             │
│  │ (DaemonSet)│  │ (DaemonSet)│  │ (DaemonSet)│             │
│  └─────────────┘  └─────────────┘  └─────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Kubernetes 部署

### 前置要求

- Kubernetes 1.24+
- Helm 3.8+
- kubectl configured
- PV provisioner (用于 PostgreSQL)

### 使用 Helm 部署

#### 添加 Helm Repo

```bash
helm repo add uof https://charts.uof.io
helm repo update
```

#### 安装 UOF

```bash
# 安装控制面组件
helm install uof-control \
  uof/uof-control \
  --namespace uof-system \
  --create-namespace \
  --set controlPlane.api.port=8080 \
  --set controlPlane.bindAddress=0.0.0.0

# 安装 Agent (每节点)
helm install uof-agent \
  uof/uof-agent \
  --namespace uof-system \
  -f values-agent.yaml
```

#### values-agent.yaml 示例

```yaml
agent:
  controlPlaneEndpoint: "http://uof-control-api:8080"
  resources:
    limits:
      cpu: "2"
      memory: "2Gi"
    requests:
      cpu: "500m"
      memory: "512Mi"

  probes:
    enabled:
      - syscall
      - sched
      - io
    samplingRate: 1000

  securityContext:
    privileged: false
    capabilities:
      add:
        - BPF
        - PERFMON
```

### 验证部署

```bash
# 检查 Pod 状态
kubectl get pods -n uof-system

# 检查 Control Plane 健康
kubectl port-forward -n uof-system svc/uof-control-api 8080:8080
curl http://localhost:8080/healthz

# 查看 Agent 日志
kubectl logs -n uof-system -l app=uof-agent
```

---

## 裸机/虚拟机部署

### 方式一：systemd 服务

#### 创建用户和目录

```bash
sudo useradd -r -s /usr/sbin/nologin uof
sudo mkdir -p /opt/uof/{bin,config,data,logs}
```

#### 下载和安装

```bash
# 下载二进制文件
curl -L https://github.com/your-org/uof/releases/latest/download/uof-linux-amd64.tar.gz | tar xz

# 安装二进制
sudo cp uof-{agent,control-api,cli} /opt/uof/bin/
sudo chmod +x /opt/uof/bin/*
sudo chown -R uof:uof /opt/uof
```

#### Control Plane systemd unit

```ini
# /etc/systemd/system/uof-control-api.service
[Unit]
Description=UOF Control Plane API
After=network.target postgresql.service

[Service]
Type=simple
User=uof
Group=uof
ExecStart=/opt/uof/bin/uof-control-api --config /opt/uof/config/control-api.toml
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

#### Agent systemd unit

```ini
# /etc/systemd/system/uof-agent.service
[Unit]
Description=UOF Agent
After=network.target

[Service]
Type=simple
User=uof
Group=uof
ExecStart=/opt/uof/bin/uof-agent --config /opt/uof/config/agent.toml
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

#### 启动服务

```bash
sudo systemctl daemon-reload
sudo systemctl enable uof-control-api
sudo systemctl enable uof-agent
sudo systemctl start uof-control-api
sudo systemctl start uof-agent
```

### 方式二：Docker Compose (开发/测试环境)

```yaml
# docker-compose.yml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: uof
      POSTGRES_USER: uof
      POSTGRES_PASSWORD: uof_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  control-api:
    image: uof/control-api:latest
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://uof:uof_password@postgres:5432/uof
      BIND_ADDR: 0.0.0.0:8080
    depends_on:
      - postgres

  agent:
    image: uof/agent:latest
    privileged: true
    environment:
      UOF_CONTROL_PLANE_ENDPOINT: http://control-api:8080
    volumes:
      - /sys/kernel/debug:/sys/kernel/debug:ro
    depends_on:
      - control-api

volumes:
  postgres_data:
```

启动：
```bash
docker-compose up -d
```

---

## 配置参考

### Control Plane 配置

```toml
# /opt/uof/config/control-api.toml
[server]
bind_address = "0.0.0.0:8080"
read_timeout = "30s"
write_timeout = "30s"

[database]
url = "postgres://uof:password@localhost:5432/uof"
max_connections = 20
min_connections = 5

[registry]
default_registry = "ghcr.io"
auth_enabled = true

[logging]
level = "info"
format = "json"
```

### Agent 配置

```toml
# /opt/uof/config/agent.toml
[control_plane]
endpoint = "http://localhost:8080"
heartbeat_interval = "30s"
register_timeout = "10s"

[agent]
hostname = "auto"  # 自动获取
data_dir = "/var/lib/uof"
log_level = "info"

[probes]
enabled = ["syscall", "sched", "io", "net"]
sampling_rate = 1000

[ebpf]
ring_buffer_size = 8192
probe_auto_attach = true

[otel]
endpoint = "http://localhost:4317"
protocol = "grpc"
```

### 环境变量参考

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `UOF_BIND_ADDR` | Control Plane 绑定地址 | `127.0.0.1:8080` |
| `UOF_CONTROL_PLANE_ENDPOINT` | Agent 连接的 Control Plane 地址 | - |
| `DATABASE_URL` | PostgreSQL 连接字符串 | - |
| `RUST_LOG` | 日志级别 | `info` |
| `RUST_BACKTRACE` | 是否输出 backtrace | `1` |

---

## 安全配置

### eBPF 权限最小化

尽量使用细粒度权限，避免 `CAP_SYS_ADMIN`：

```yaml
securityContext:
  privileged: false
  capabilities:
    add:
      - BPF          # 加载 BPF 程序
      - PERFMON      # 性能监控
    # 不需要 NET_ADMIN，除非采集网络流量
```

### TLS 配置

```toml
[tls]
enabled = true
cert_file = "/etc/uof/tls/server.crt"
key_file = "/etc/uof/tls/server.key"
```

### 插件签名校验

```toml
[plugin]
require_signed = true
allowed_publishers = ["uof-official", "verified-publisher"]
```

### 数据脱敏

```toml
[sensitive_data]
 redact_sql_params = true
 redact_file_paths = true
 redact_http_headers = ["Authorization", "Cookie"]
 max_sql_length = 1000
```

---

## 监控与告警

### UOF 自身可观测性

Agent 提供以下自监控指标：

| 指标 | 说明 | 告警阈值 |
|------|------|----------|
| `uof_agent_cpu_percent` | Agent CPU 使用率 | > 20% |
| `uof_agent_memory_bytes` | Agent 内存使用 | > 1GB |
| `uof_ring_buffer_lost_events` | Ring Buffer 丢包数 | > 0 |
| `uof_probe_load_failures` | 探针加载失败次数 | > 0 |
| `uof_otel_export_errors` | OTLP 导出失败数 | > 100/min |

### 健康检查端点

```bash
# Control Plane
curl http://localhost:8080/healthz

# Agent
curl http://localhost:8081/healthz   # 健康状态
curl http://localhost:8081/readyz    # 就绪状态
curl http://localhost:8081/debug/probes  # 探针列表
```

### 日志级别调整

运行时调整日志级别：

```bash
# 通过 API
curl -X POST http://localhost:8081/config/log \
  -H "Content-Type: application/json" \
  -d '{"level": "debug"}'

# 通过环境变量 (需重启)
RUST_LOG=debug cargo run -p uof-agent
```

---

## 备份与恢复

### PostgreSQL 备份

```bash
# 每日全量备份
pg_dump -U uof -d uof -F c -f /backup/uof-$(date +%Y%m%d).dump

# 保留策略: 保留最近 30 天
find /backup -name "uof-*.dump" -mtime +30 -delete
```

### 恢复

```bash
pg_restore -U uof -d uof -c /backup/uof-20240101.dump
```

### 配置备份

```bash
# 备份配置
tar czf uof-config-$(date +%Y%m%d).tar.gz /opt/uof/config/

# 备份插件
tar czf uof-plugins-$(date +%Y%m%d).tar.gz /opt/uof/plugins/
```

---

## 升级与回滚

### 升级步骤

#### 1. 更新 Control Plane

```bash
# 更新 Helm
helm repo update
helm upgrade uof-control uof/uof-control -n uof-system

# 或使用二进制
sudo systemctl stop uof-control-api
sudo cp new-uof-control-api /opt/uof/bin/
sudo systemctl start uof-control-api
```

#### 2. 更新 Agent

```bash
# 滚动更新 (Kubernetes)
kubectl rollout restart daemonset/uof-agent -n uof-system

# 验证
kubectl rollout status daemonset/uof-agent -n uof-system
```

#### 3. 验证

```bash
# 检查版本
curl http://localhost:8080/api/v1/version
curl http://localhost:8081/version

# 检查功能
curl http://localhost:8080/healthz
curl http://localhost:8081/debug/probes
```

### 回滚步骤

```bash
# Helm 回滚
helm rollback uof-control -n uof-system

# 二进制回滚
sudo systemctl stop uof-control-api
sudo cp /opt/uof/bin/uof-control-api.bak /opt/uof/bin/uof-control-api
sudo systemctl start uof-control-api
```

### 插件版本管理

```bash
# 查看已安装插件版本
curl http://localhost:8080/api/v1/plugins

# 回滚插件版本
curl -X POST http://localhost:8080/api/v1/plugins/{id}/rollback \
  -d '{"target_version": "0.1.0"}'
```

---

## 故障排查

### 常见问题

#### 1. Agent 无法注册

```bash
# 检查网络
curl http://control-plane:8080/healthz

# 检查 Agent 日志
journalctl -u uof-agent -n 100

# 检查端点配置
cat /opt/uof/config/agent.toml | grep endpoint
```

#### 2. eBPF 探针加载失败

```bash
# 检查内核支持
uname -r
cat /proc/sys/kernel/bpf_stats_enabled

# 检查权限
getpcaps $(pidof uof-agent)

# 检查 dmesg
dmesg | grep -i bpf
```

#### 3. 数据不完整

```bash
# 检查 Ring Buffer
curl http://localhost:8081/debug/probes | jq '.[] | select(.status=="running")'

# 检查 OTLP 连接
curl -X POST http://localhost:8081/debug/otel-stats

# 检查采样率
curl http://localhost:8081/config/sampling
```

### 调试模式

启用调试模式获取更详细的日志：

```bash
# 临时启用
RUST_LOG=debug sudo systemctl restart uof-agent

# 查看日志
journalctl -u uof-agent -f --since "5 minutes ago"
```

### 内核调试

```bash
# bpftrace 脚本示例
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }'

# 查看 BPF map
sudo bpftool map show
sudo bpftool prog show
```

---

## 下一步

- 查看[用户使用指南](./user_guide.md)了解日常操作
- 查看[详细设计文档](./detailed_design.md)了解架构细节