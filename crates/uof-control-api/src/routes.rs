use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use uof_control_plane::state::AppState;
use uof_model::{
    agent::{AgentHeartbeatRequest, AgentRegisterRequest},
    desired_state::AckRequest,
    plugin::{CreatePluginRequest, CreatePluginVersionRequest, ReleasePluginRequest},
    template::{CreateTemplateBindingRequest, CreateTemplateRequest},
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/agents/register", post(register_agent))
        .route("/api/v1/agents/{agent_id}/heartbeat", post(agent_heartbeat))
        .route("/api/v1/agents/{agent_id}/desired-state", get(get_desired_state))
        .route("/api/v1/agents/{agent_id}/ack", post(ack_desired_state))
        .route("/api/v1/plugins", get(list_plugins).post(create_plugin))
        .route("/api/v1/plugins/{plugin_id}", get(get_plugin))
        .route("/api/v1/plugins/{plugin_id}/versions", post(create_plugin_version))
        .route("/api/v1/plugins/{plugin_id}/release", post(release_plugin_version))
        .route("/api/v1/templates", get(list_templates).post(create_template))
        .route("/api/v1/template-bindings", post(create_template_binding))
        .route("/api/v1/template-bindings/{binding_id}", delete(delete_template_binding))
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn register_agent(
    State(state): State<AppState>,
    Json(request): Json<AgentRegisterRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(state.register_agent(request).await))
}

async fn agent_heartbeat(
    Path(agent_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<AgentHeartbeatRequest>,
) -> impl IntoResponse {
    if state.heartbeat(agent_id, request).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn get_desired_state(
    Path(agent_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.desired_state(agent_id).await {
        Some(desired_state) => (StatusCode::OK, Json(desired_state)).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn ack_desired_state(
    Path(agent_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<AckRequest>,
) -> impl IntoResponse {
    if state.ack_desired_state(agent_id, request).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
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
    Path(plugin_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.get_plugin(plugin_id).await {
        Some(plugin) => (StatusCode::OK, Json(plugin)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
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
    if state.release_plugin_version(plugin_id, request).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
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
