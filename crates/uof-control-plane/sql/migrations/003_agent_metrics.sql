-- Agent metrics storage tables

-- Raw metrics from agents (for time-series storage)
CREATE TABLE agent_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    metric_name VARCHAR(100) NOT NULL,
    metric_type VARCHAR(20) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB NOT NULL DEFAULT '{}',
    collected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Aggregated metrics summary per agent per time period
CREATE TABLE agent_metrics_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    syscall_count BIGINT NOT NULL DEFAULT 0,
    io_count BIGINT NOT NULL DEFAULT 0,
    sched_count BIGINT NOT NULL DEFAULT 0,
    net_count BIGINT NOT NULL DEFAULT 0,
    lock_count BIGINT NOT NULL DEFAULT 0,
    syscall_latency_avg_us DOUBLE PRECISION NOT NULL DEFAULT 0,
    io_latency_avg_us DOUBLE PRECISION NOT NULL DEFAULT 0,
    net_bytes_total BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(agent_id, period_start)
);

-- Indexes for efficient querying
CREATE INDEX idx_agent_metrics_agent_id ON agent_metrics(agent_id);
CREATE INDEX idx_agent_metrics_collected_at ON agent_metrics(collected_at);
CREATE INDEX idx_agent_metrics_name ON agent_metrics(metric_name);
CREATE INDEX idx_agent_metrics_summary_agent_period ON agent_metrics_summary(agent_id, period_start DESC);
