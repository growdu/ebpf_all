#!/bin/bash
# Prometheus startup script

cd /home/ubuntu/ebpf_all

source ./deploy/prometheus/env

echo "Starting Prometheus..."
/home/ubuntu/otel/prometheus-3.11.3.linux-amd64/prometheus \
    --config.file=./deploy/prometheus/prometheus.yml \
    --storage.tsdb.path=./deploy/prometheus/data \
    --web.listen-address=0.0.0.0:9091 \
    --web.enable-lifecycle