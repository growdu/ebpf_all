-- UOF control plane schema (MVP draft)

CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY,
    hostname VARCHAR(255) NOT NULL,
    node_name VARCHAR(255),
    ip INET,
    kernel_version VARCHAR(128) NOT NULL,
    os_release VARCHAR(255),
    arch VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'online',
    labels JSONB NOT NULL DEFAULT '{}'::jsonb,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_agents_last_seen_at ON agents(last_seen_at DESC);
CREATE INDEX IF NOT EXISTS idx_agents_labels_gin ON agents USING GIN(labels);

CREATE TABLE IF NOT EXISTS agent_heartbeats (
    id BIGSERIAL PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL,
    health_summary JSONB NOT NULL DEFAULT '{}'::jsonb,
    probe_status JSONB NOT NULL DEFAULT '[]'::jsonb,
    plugin_status JSONB NOT NULL DEFAULT '[]'::jsonb,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agent_heartbeats_agent_id_sent_at
    ON agent_heartbeats(agent_id, sent_at DESC);

CREATE TABLE IF NOT EXISTS plugins (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    kind VARCHAR(64) NOT NULL,
    publisher VARCHAR(255) NOT NULL,
    default_version_id UUID,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS plugin_versions (
    id UUID PRIMARY KEY,
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(64) NOT NULL,
    digest VARCHAR(255) NOT NULL,
    oci_ref TEXT NOT NULL,
    manifest JSONB NOT NULL,
    compat_matrix JSONB NOT NULL DEFAULT '{}'::jsonb,
    signature_status VARCHAR(32) NOT NULL DEFAULT 'pending',
    published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(plugin_id, version)
);

CREATE INDEX IF NOT EXISTS idx_plugin_versions_plugin_id ON plugin_versions(plugin_id);
CREATE INDEX IF NOT EXISTS idx_plugin_versions_published ON plugin_versions(published);

ALTER TABLE plugins
    ADD CONSTRAINT fk_plugins_default_version
    FOREIGN KEY (default_version_id) REFERENCES plugin_versions(id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE IF NOT EXISTS templates (
    id UUID PRIMARY KEY,
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(64) NOT NULL,
    target_software VARCHAR(128) NOT NULL,
    scenario VARCHAR(128),
    manifest JSONB NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(plugin_id, name, version)
);

CREATE INDEX IF NOT EXISTS idx_templates_target_software ON templates(target_software);

CREATE TABLE IF NOT EXISTS template_bindings (
    id UUID PRIMARY KEY,
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    selector JSONB NOT NULL,
    target JSONB NOT NULL,
    policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_template_bindings_template_id ON template_bindings(template_id);
CREATE INDEX IF NOT EXISTS idx_template_bindings_selector_gin ON template_bindings USING GIN(selector);

CREATE TABLE IF NOT EXISTS policies (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    type VARCHAR(64) NOT NULL,
    spec JSONB NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS desired_states (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    generation BIGINT NOT NULL,
    spec JSONB NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acked_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    UNIQUE(agent_id, generation)
);

CREATE INDEX IF NOT EXISTS idx_desired_states_agent_id_generation
    ON desired_states(agent_id, generation DESC);

CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    actor_type VARCHAR(64) NOT NULL,
    actor_id VARCHAR(255) NOT NULL,
    action VARCHAR(128) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
