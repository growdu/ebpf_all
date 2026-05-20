-- Desired state storage tables

-- Desired states table (stores the full desired state for each agent)
CREATE TABLE desired_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    generation BIGINT NOT NULL DEFAULT 1,
    sampling JSONB NOT NULL DEFAULT '{}',
    exporter JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(agent_id, generation)
);

-- Desired state plugins (plugins that should be active for an agent)
CREATE TABLE desired_state_plugins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    desired_state_id UUID NOT NULL REFERENCES desired_states(id) ON DELETE CASCADE,
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    action VARCHAR(50) NOT NULL DEFAULT 'enable',
    artifact_url TEXT,
    artifact_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Desired state templates
CREATE TABLE desired_state_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    desired_state_id UUID NOT NULL REFERENCES desired_states(id) ON DELETE CASCADE,
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    variables JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_desired_states_agent_id ON desired_states(agent_id);
CREATE INDEX idx_desired_state_plugins_state_id ON desired_state_plugins(desired_state_id);
CREATE INDEX idx_desired_state_plugins_plugin_id ON desired_state_plugins(plugin_id);
CREATE INDEX idx_desired_state_templates_state_id ON desired_state_templates(desired_state_id);
