use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use serde::Deserialize;
use bytes::Bytes;
use uuid::Uuid;

use uof_control_plane::state::AppState;
use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest},
    desired_state::AckRequest,
    plugin::{CreatePluginRequest, CreatePluginVersionRequest, ReleasePluginRequest},
    template::CreateTemplateRequest,
};
use uof_registry::{OciClient, OciRef, digest_bytes, media_type};

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/healthz", get(healthz))
        // Test routes
        .route("/api/v1/test-simple", get(test_simple))
        .route("/api/v1/test/{name}", get(test_param))
        // Agent routes - IDs passed via query string to work around Axum Path bug
        .route("/api/v1/agents", get(list_agents))
        .route("/api/v1/agents/register", post(register_agent))
        .route("/api/v1/agents/heartbeat", post(agent_heartbeat))
        .route("/api/v1/agents/desired-state", get(get_desired_state))
        .route("/api/v1/agents/ack", post(ack_desired_state))
        // Plugin routes
        .route("/api/v1/plugins", get(list_plugins).post(create_plugin))
        .route("/api/v1/plugins/get", get(get_plugin))
        // Template routes
        .route("/api/v1/templates", get(list_templates).post(create_template))
        // Metrics endpoint (simulated for demo)
        .route("/api/v1/metrics/summary", get(get_metrics_summary))
        .route("/api/v1/metrics/syscall", get(get_syscall_metrics))
        .route("/api/v1/metrics/io", get(get_io_metrics))
        .route("/api/v1/metrics/network", get(get_network_metrics))
        .layer(cors)
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn test_param(Path(name): Path<String>) -> impl IntoResponse {
    tracing::info!("test_param called with name: {}", name);
    Json(serde_json::json!({ "param": name }))
}

async fn test_simple() -> impl IntoResponse {
    Json(serde_json::json!({ "simple": true }))
}

async fn register_agent(
    State(state): State<AppState>,
    Json(request): Json<AgentRegisterRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(state.register_agent(request).await))
}

async fn list_agents(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.list_agents().await))
}

async fn agent_heartbeat(
    Query(qs): Query<HeartbeatQs>,
    State(state): State<AppState>,
    Json(request): Json<AgentHeartbeatRequest>,
) -> impl IntoResponse {
    let agent_id = match Uuid::parse_str(&qs.id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if state.heartbeat(agent_id, request).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn get_desired_state(
    Query(qs): Query<HeartbeatQs>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let agent_id = match Uuid::parse_str(&qs.id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    match state.desired_state(agent_id).await {
        Some(ds) => (StatusCode::OK, Json(ds)).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn ack_desired_state(
    Query(qs): Query<HeartbeatQs>,
    State(state): State<AppState>,
    Json(request): Json<AckRequest>,
) -> impl IntoResponse {
    let agent_id = match Uuid::parse_str(&qs.id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if state.ack_desired_state(agent_id, request).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

#[derive(Debug, Deserialize)]
struct HeartbeatQs {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PluginQs {
    id: String,
}

async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.list_plugins().await))
}

async fn create_plugin(
    State(state): State<AppState>,
    Json(request): Json<CreatePluginRequest>,
) -> impl IntoResponse {
    (StatusCode::CREATED, Json(state.create_plugin(request).await))
}

async fn get_plugin(
    Query(qs): Query<PluginQs>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let plugin_id = match Uuid::parse_str(&qs.id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    tracing::debug!(plugin_id = %plugin_id, "get_plugin called");
    match state.get_plugin(plugin_id).await {
        Some(plugin) => {
            tracing::debug!(plugin_id = %plugin_id, "plugin found");
            (StatusCode::OK, Json(plugin)).into_response()
        }
        None => {
            tracing::debug!(plugin_id = %plugin_id, "plugin not found");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

#[allow(dead_code)]
async fn create_plugin_version(
    Path(plugin_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreatePluginVersionRequest>,
) -> impl IntoResponse {
    match state.create_plugin_version(plugin_id, request).await {
        Some(version) => (StatusCode::CREATED, Json(version)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[allow(dead_code)]
async fn release_plugin_version(
    Path(plugin_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<ReleasePluginRequest>,
) -> impl IntoResponse {
    let version_id = state.find_version_by_string(plugin_id, &request.version).await;

    if !state.release_plugin_version(plugin_id, request).await {
        return StatusCode::NOT_FOUND.into_response();
    }

    if let Some((vid, ver)) = version_id {
        state.notify_agents_plugin_released(plugin_id, vid, ver).await;
    }

    StatusCode::ACCEPTED.into_response()
}

/// POST /api/v1/plugins/pull — admin-facing direct OCI pull.
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct PullPluginBody {
    registry: String,
    repo: String,
    tag: Option<String>,
}

#[allow(dead_code)]
async fn pull_plugin(
    State(_state): State<AppState>,
    Json(body): Json<PullPluginBody>,
) -> impl IntoResponse {
    let tag = body.tag.as_deref().unwrap_or("latest");

    let client = match OciClient::new(&body.registry) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(registry = %body.registry, "failed to create OCI client: {e}");
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    tracing::info!(registry = %body.registry, repo = %body.repo, %tag, "pulling plugin from OCI registry");

    let bytes = match client
        .pull(&body.repo, OciRef::parse(tag), media_type::EBPF_BINARY)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(registry = %body.registry, repo = %body.repo, %tag, "OCI pull failed: {e}");
            let status = if e.is_not_found() { StatusCode::NOT_FOUND } else { StatusCode::BAD_GATEWAY };
            return (status, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    let digest = digest_bytes(&bytes);
    tracing::info!(registry = %body.registry, repo = %body.repo, bytes = bytes.len(), digest = %digest, "plugin streamed successfully");

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        axum::http::header::CONTENT_LENGTH,
        HeaderValue::from(bytes.len()),
    );
    let etag = format!("sha256:{digest}");
    headers.insert(
        axum::http::header::ETAG,
        HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    let mut resp = axum::response::Response::new(Body::from(Bytes::from(bytes)));
    *resp.headers_mut() = headers;
    *resp.status_mut() = StatusCode::OK;
    resp
}

async fn list_templates(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.list_templates().await))
}

async fn create_template(
    State(state): State<AppState>,
    Json(request): Json<CreateTemplateRequest>,
) -> impl IntoResponse {
    (StatusCode::CREATED, Json(state.create_template(request).await))
}

#[derive(serde::Serialize)]
struct MetricsSummary {
    total_events_per_sec: f64,
    syscall_per_sec: f64,
    io_per_sec: f64,
    network_per_sec: f64,
    avg_cpu_usage: f64,
    memory_used_mb: u64,
    timestamp_ms: u64,
}

async fn get_metrics_summary(State(state): State<AppState>) -> impl IntoResponse {
    let summary = state.get_metrics_summary(None).await;
    (StatusCode::OK, Json(summary))
}

#[derive(serde::Serialize)]
struct SyscallMetrics {
    calls: Vec<SyscallCall>,
    top_files: Vec<FileAccess>,
    top_processes: Vec<ProcessAccess>,
}

#[derive(serde::Serialize)]
struct SyscallCall {
    syscall: String,
    count: u64,
    avg_latency_us: f64,
    errors: u64,
}

#[derive(serde::Serialize)]
struct FileAccess {
    path: String,
    count: u64,
    read_bytes: u64,
    write_bytes: u64,
}

#[derive(serde::Serialize)]
struct ProcessAccess {
    pid: u32,
    comm: String,
    syscall_count: u64,
}

async fn get_syscall_metrics() -> impl IntoResponse {
    let metrics = SyscallMetrics {
        calls: vec![
            SyscallCall { syscall: "read".to_string(), count: 45230, avg_latency_us: 12.5, errors: 3 },
            SyscallCall { syscall: "write".to_string(), count: 38120, avg_latency_us: 8.3, errors: 0 },
            SyscallCall { syscall: "open".to_string(), count: 29450, avg_latency_us: 15.2, errors: 12 },
            SyscallCall { syscall: "close".to_string(), count: 72100, avg_latency_us: 2.1, errors: 0 },
            SyscallCall { syscall: "poll".to_string(), count: 18500, avg_latency_us: 145.0, errors: 0 },
        ],
        top_files: vec![
            FileAccess { path: "/var/log/syslog".to_string(), count: 4521, read_bytes: 204800, write_bytes: 1536000 },
            FileAccess { path: "/proc/net/tcp".to_string(), count: 3890, read_bytes: 45678, write_bytes: 0 },
            FileAccess { path: "/etc/nginx/nginx.conf".to_string(), count: 1205, read_bytes: 8912, write_bytes: 0 },
        ],
        top_processes: vec![
            ProcessAccess { pid: 1234, comm: "nginx".to_string(), syscall_count: 45230 },
            ProcessAccess { pid: 5678, comm: "postgres".to_string(), syscall_count: 38120 },
            ProcessAccess { pid: 9012, comm: "sshd".to_string(), syscall_count: 12450 },
        ],
    };

    (StatusCode::OK, Json(metrics))
}

#[derive(serde::Serialize)]
struct IoMetrics {
    total_ops_per_sec: f64,
    read_ops_per_sec: f64,
    write_ops_per_sec: f64,
    avg_latency_us: f64,
    top_devices: Vec<DeviceIo>,
}

#[derive(serde::Serialize)]
struct DeviceIo {
    device: String,
    read_ops: u64,
    write_ops: u64,
    read_mb: f64,
    write_mb: f64,
    avg_latency_us: f64,
}

async fn get_io_metrics() -> impl IntoResponse {
    let metrics = IoMetrics {
        total_ops_per_sec: 1520.5,
        read_ops_per_sec: 980.3,
        write_ops_per_sec: 540.2,
        avg_latency_us: 245.0,
        top_devices: vec![
            DeviceIo {
                device: "sda".to_string(),
                read_ops: 850,
                write_ops: 420,
                read_mb: 125.5,
                write_mb: 89.2,
                avg_latency_us: 230.0,
            },
            DeviceIo {
                device: "nvme0n1".to_string(),
                read_ops: 130,
                write_ops: 120,
                read_mb: 45.8,
                write_mb: 38.1,
                avg_latency_us: 85.0,
            },
        ],
    };

    (StatusCode::OK, Json(metrics))
}

#[derive(serde::Serialize)]
struct NetworkMetrics {
    total_packets_per_sec: f64,
    tcp_packets_per_sec: f64,
    udp_packets_per_sec: f64,
    top_connections: Vec<ConnectionStats>,
}

#[derive(serde::Serialize)]
struct ConnectionStats {
    local_addr: String,
    remote_addr: String,
    state: String,
    packets_in: u64,
    packets_out: u64,
    bytes_in: u64,
    bytes_out: u64,
}

async fn get_network_metrics() -> impl IntoResponse {
    let metrics = NetworkMetrics {
        total_packets_per_sec: 2700.0,
        tcp_packets_per_sec: 2450.0,
        udp_packets_per_sec: 250.0,
        top_connections: vec![
            ConnectionStats {
                local_addr: "10.1.0.3:443".to_string(),
                remote_addr: "192.168.1.100:54321".to_string(),
                state: "ESTABLISHED".to_string(),
                packets_in: 125000,
                packets_out: 98000,
                bytes_in: 45678900,
                bytes_out: 12345600,
            },
            ConnectionStats {
                local_addr: "10.1.0.3:80".to_string(),
                remote_addr: "192.168.1.101:45678".to_string(),
                state: "ESTABLISHED".to_string(),
                packets_in: 89000,
                packets_out: 112000,
                bytes_in: 23456700,
                bytes_out: 67890100,
            },
        ],
    };

    (StatusCode::OK, Json(metrics))
}
