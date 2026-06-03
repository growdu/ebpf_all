pub mod agent;
pub mod desired_state;
pub mod plugin;
pub mod template;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_agent_register_request_serde() {
        let request = agent::AgentRegisterRequest {
            hostname: "test-host".to_string(),
            node_name: Some("test-node".to_string()),
            ip: Some("192.168.1.100".to_string()),
            kernel_version: "5.4.0".to_string(),
            os_release: Some("Ubuntu 20.04".to_string()),
            arch: "x86_64".to_string(),
            labels: std::collections::BTreeMap::new(),
            capabilities: serde_json::json!({}),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: agent::AgentRegisterRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.hostname, request.hostname);
        assert_eq!(parsed.kernel_version, request.kernel_version);
        assert_eq!(parsed.arch, request.arch);
    }

    #[test]
    fn test_agent_register_response_serde() {
        let response = agent::AgentRegisterResponse {
            agent_id: Uuid::new_v4(),
            status: "active".to_string(),
            poll_interval_seconds: 30,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: agent::AgentRegisterResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.agent_id, response.agent_id);
        assert_eq!(parsed.status, response.status);
        assert_eq!(parsed.poll_interval_seconds, response.poll_interval_seconds);
    }

    #[test]
    fn test_agent_heartbeat_request_serde() {
        let request = agent::AgentHeartbeatRequest {
            status: "healthy".to_string(),
            health_summary: serde_json::json!({"cpu": "ok", "memory": "ok"}),
            probe_status: vec![],
            plugin_status: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: agent::AgentHeartbeatRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status, request.status);
    }

    #[test]
    fn test_desired_state_serde() {
        let state = desired_state::DesiredState {
            generation: 1,
            plugins: vec![],
            templates: vec![],
            sampling: serde_json::json!({"rate": 100}),
            exporter: serde_json::json!({"type": "otlp"}),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: desired_state::DesiredState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.generation, state.generation);
    }

    #[test]
    fn test_ack_request_serde() {
        let request = desired_state::AckRequest {
            generation: 42,
            status: desired_state::AckStatus::Applied,
            message: Some("Applied successfully".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: desired_state::AckRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.generation, request.generation);
        assert!(matches!(parsed.status, desired_state::AckStatus::Applied));
    }

    #[test]
    fn test_plugin_activation_serde() {
        let activation = desired_state::PluginActivation {
            plugin_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            action: desired_state::PluginAction::Enable,
            artifact_url: Some("https://example.com/plugin.tar.gz".to_string()),
            artifact_digest: Some("abc123".to_string()),
        };

        let json = serde_json::to_string(&activation).unwrap();
        let parsed: desired_state::PluginActivation = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.plugin_id, activation.plugin_id);
        assert_eq!(parsed.version, activation.version);
        assert!(matches!(parsed.action, desired_state::PluginAction::Enable));
    }

    #[test]
    fn test_plugin_action_disable_serde() {
        let action = desired_state::PluginAction::Disable;
        let json = serde_json::to_string(&action).unwrap();
        let parsed: desired_state::PluginAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, desired_state::PluginAction::Disable));
    }

    #[test]
    fn test_plugin_serde() {
        let plugin = plugin::Plugin {
            id: Uuid::new_v4(),
            name: "test-plugin".to_string(),
            kind: "ebpf".to_string(),
            publisher: "test".to_string(),
            status: "active".to_string(),
        };

        let json = serde_json::to_string(&plugin).unwrap();
        let parsed: plugin::Plugin = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, plugin.name);
        assert_eq!(parsed.kind, plugin.kind);
    }

    #[test]
    fn test_plugin_version_serde() {
        let version = plugin::PluginVersion {
            id: Uuid::new_v4(),
            plugin_id: Uuid::new_v4(),
            version: "2.0.0".to_string(),
            digest: "sha256:def456".to_string(),
            oci_ref: "oci://example.com/plugin:2.0.0".to_string(),
            signature_status: "verified".to_string(),
            published: true,
        };

        let json = serde_json::to_string(&version).unwrap();
        let parsed: plugin::PluginVersion = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, version.version);
        assert_eq!(parsed.digest, version.digest);
        assert!(parsed.published);
    }

    #[test]
    fn test_template_serde() {
        let tmpl = template::Template {
            id: Uuid::new_v4(),
            plugin_id: Uuid::new_v4(),
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            target_software: "nginx".to_string(),
            scenario: Some("http流量".to_string()),
            status: "active".to_string(),
        };

        let json = serde_json::to_string(&tmpl).unwrap();
        let parsed: template::Template = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, tmpl.name);
        assert_eq!(parsed.target_software, tmpl.target_software);
    }

    #[test]
    fn test_template_binding_serde() {
        let binding = template::TemplateBinding {
            id: Uuid::new_v4(),
            template_id: Uuid::new_v4(),
            selector: serde_json::json!({"env": "prod"}),
            target: serde_json::json!({"hostname": "*.example.com"}),
            policy: serde_json::json!({"rate": 1000}),
            enabled: true,
        };

        let json = serde_json::to_string(&binding).unwrap();
        let parsed: template::TemplateBinding = serde_json::from_str(&json).unwrap();

        assert!(parsed.enabled);
    }

    #[test]
    fn test_create_template_request_serde() {
        let request = template::CreateTemplateRequest {
            plugin_id: Uuid::new_v4(),
            name: "create-test".to_string(),
            version: "1.0.0".to_string(),
            target_software: "apache".to_string(),
            scenario: None,
            manifest: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: template::CreateTemplateRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, request.name);
    }

    #[test]
    fn test_create_plugin_request_serde() {
        let request = plugin::CreatePluginRequest {
            name: "new-plugin".to_string(),
            kind: "probe".to_string(),
            publisher: "acme".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: plugin::CreatePluginRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, request.name);
    }

    #[test]
    fn test_artifact_request_serde() {
        let request = desired_state::PluginArtifactRequest {
            plugin_id: Uuid::new_v4(),
            version: "3.0.0".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: desired_state::PluginArtifactRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, request.version);
    }

    #[test]
    fn test_skip_serializing_artifact_url() {
        let activation = desired_state::PluginActivation {
            plugin_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            action: desired_state::PluginAction::Disable,
            artifact_url: None,
            artifact_digest: None,
        };

        let json = serde_json::to_string(&activation).unwrap();
        assert!(!json.contains("artifact_url"));
        assert!(!json.contains("artifact_digest"));
    }

    #[test]
    fn test_plugin_detail_flatten() {
        let detail = plugin::PluginDetail {
            plugin: plugin::Plugin {
                id: Uuid::new_v4(),
                name: "detail-plugin".to_string(),
                kind: "exporter".to_string(),
                publisher: "acme".to_string(),
                status: "active".to_string(),
            },
            default_version_id: Some(Uuid::new_v4()),
            versions: vec![],
        };

        let json = serde_json::to_string(&detail).unwrap();
        let parsed: plugin::PluginDetail = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.plugin.name, detail.plugin.name);
    }
}

