use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use bytes::Bytes;
use std::collections::HashMap;
use uuid::Uuid;

use uof_control_plane::state::AppState;
use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest},
    desired_state::AckRequest,
    plugin::{CreatePluginRequest, CreatePluginVersionRequest, ReleasePluginRequest},
    template::{CreateTemplateBindingRequest, CreateTemplateRequest},
};
use uof_registry::{OciClient, OciRef, digest_bytes, media_type};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        // Test routes
        .route("/api/v1/test-simple", get(test_simple))
        .route("/api/v1/test/{name}", get(test_param))
        // Agent routes - IDs passed via query string to work around Axum Path bug
        .route("/api/v1/agents/register", post(register_agent))
        .route("/api/v1/agents/heartbeat", post(agent_heartbeat))
        .route("/api/v1/agents/desired-state", get(get_desired_state))
        .route("/api/v1/agents/ack", post(ack_desired_state))
        // Plugin routes
        .route("/api/v1/plugins", get(list_plugins).post(create_plugin))
        .route("/api/v1/plugins/get", get(get_plugin))
        // Template routes
        .route("/api/v1/templates", get(list_templates).post(create_template))
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn test_param(Path(name): Path<String>) -> impl IntoResponse {
    tracing::info!("test_param called with name: {}", name);
    Json(serde_json::json!({ "param": name }))
}

async fn test_catch_all(Path(path): Path<String>) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "path": path })))
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
#[derive(serde::Deserialize)]
struct PullPluginBody {
    registry: String,
    repo: String,
    tag: Option<String>,
}

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

/// GET /api/v1/plugins/{plugin_id}/versions/{version_id}/artifact — admin-facing proxy to OCI.
async fn serve_plugin_artifact(
    Path((plugin_id, version_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let (registry, repo) = match state.get_version_oci_ref(plugin_id, version_id).await {
        Some(oci_ref) => parse_oci_ref(&oci_ref),
        None => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "plugin version not found" }))).into_response();
        }
    };

    let client = match OciClient::new(&registry) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(registry = %registry, "failed to create OCI client: {e}");
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    tracing::info!(plugin_id = %plugin_id, version_id = %version_id, registry = %registry, repo = %repo, "proxying artifact pull from OCI registry");

    let bytes = match client.pull(&repo, OciRef::parse("latest"), media_type::EBPF_BINARY).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(plugin_id = %plugin_id, version_id = %version_id, "OCI proxy pull failed: {e}");
            let status = if e.is_not_found() { StatusCode::NOT_FOUND } else { StatusCode::BAD_GATEWAY };
            return (status, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    let digest = digest_bytes(&bytes);
    tracing::info!(plugin_id = %plugin_id, version_id = %version_id, bytes = bytes.len(), digest = %digest, "artifact proxied successfully");

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

/// GET /api/v1/agents/{agent_id}/plugins/{plugin_id}/artifact — agent-facing artifact download.
async fn serve_agent_plugin_artifact(
    Path((agent_id, plugin_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let version_string = params.get("version").map(|s| s.as_str()).unwrap_or("latest");

    let version_id = match state.find_version_by_string(plugin_id, version_string).await {
        Some((vid, _)) => vid,
        None => {
            tracing::warn!(plugin_id = %plugin_id, version = version_string, "plugin version not found");
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "plugin version not found" }))).into_response();
        }
    };

    let (registry, repo) = match state.get_version_oci_ref(plugin_id, version_id).await {
        Some(oci_ref) => parse_oci_ref(&oci_ref),
        None => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "plugin version not found" }))).into_response();
        }
    };

    let client = match OciClient::new(&registry) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(registry = %registry, "failed to create OCI client: {e}");
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    tracing::info!(agent_id = %agent_id, plugin_id = %plugin_id, registry = %registry, repo = %repo, "agent requesting plugin artifact");

    let bytes = match client.pull(&repo, OciRef::parse(version_string), media_type::EBPF_BINARY).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(agent_id = %agent_id, plugin_id = %plugin_id, "OCI pull failed: {e}");
            let status = if e.is_not_found() { StatusCode::NOT_FOUND } else { StatusCode::BAD_GATEWAY };
            return (status, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    let digest = digest_bytes(&bytes);
    tracing::info!(agent_id = %agent_id, plugin_id = %plugin_id, bytes = bytes.len(), digest = %digest, "artifact served to agent");

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

/// Parse an OCI reference string into (registry, repo) components.
fn parse_oci_ref(oci_ref: &str) -> (String, String) {
    let (without_digest, _) = oci_ref.split_once('@').unwrap_or((oci_ref, ""));
    let (without_tag, _) = without_digest.split_once(':').unwrap_or((without_digest, ""));

    let parts: Vec<&str> = without_tag.split('/').collect();
    if parts.len() == 1 {
        ("docker.io".to_string(), oci_ref.to_string())
    } else if parts[0].contains('.') || parts[0].contains(':') {
        let registry = parts[0].to_string();
        let repo = parts[1..].join("/");
        (registry, repo)
    } else {
        ("docker.io".to_string(), oci_ref.to_string())
    }
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

async fn create_template_binding(
    State(state): State<AppState>,
    Json(request): Json<CreateTemplateBindingRequest>,
) -> impl IntoResponse {
    (StatusCode::CREATED, Json(state.create_template_binding(request).await))
}

async fn delete_template_binding(
    Path(binding_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.delete_template_binding(binding_id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
