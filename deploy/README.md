# UOF Observability Stack Deployment Guide

## 部署方式

### 方式一：Systemd 服务（推荐用于直接部署）

```bash
# 启动 OTEL Collector
sudo systemctl start otelcol-contrib
sudo systemctl enable otelcol-contrib

# 启动 Prometheus
sudo systemctl start prometheus
sudo systemctl enable prometheus

# 检查状态
sudo systemctl status otelcol-contrib
sudo systemctl status prometheus
```

### 方式二：直接启动脚本

```bash
# OTEL Collector
./deploy/otel/start.sh

# Prometheus (另一个终端)
./deploy/prometheus/start.sh
```

### 方式三：Docker Compose

```bash
cd deploy
docker-compose up -d
```

## 端口说明

| 服务 | 端口 | 说明 |
|------|------|------|
| OTEL Collector (gRPC) | 4317 | OTLP 接收端点 |
| OTEL Collector (HTTP) | 4318 | OTLP 接收端点 |
| OTEL Collector (Metrics) | 8889 | Prometheus 指标 |
| Prometheus | 9091 | Web UI |

## 配置说明

### OTEL Collector
- 配置文件: `deploy/otel/otelcol-config.yaml`
- 接收来自 UOF Agent 的 OTLP 数据
- 导出 metrics 到 Prometheus

### Prometheus
- 配置文件: `deploy/prometheus/prometheus.yml`
- 抓取 OTEL Collector 的指标
- 数据存储在 `deploy/prometheus/data/`

## 验证

```bash
# 检查 OTEL Collector 指标
curl http://localhost:8889/metrics

# 检查 Prometheus
curl http://localhost:9091/-/healthy
```