#!/bin/bash
# OTEL Collector startup script

cd /home/ubuntu/ebpf_all

source ./deploy/otel/env

echo "Starting OTEL Collector..."
/usr/bin/otelcol-contrib --config ./deploy/otel/otelcol-config.yaml