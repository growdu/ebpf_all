use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};

use crate::{
    health::AgentHealthSnapshot,
    probe_manager::{InMemoryProbeManager, ProbeManager},
};

#[derive(Clone)]
pub struct AdminState {
    pub probe_manager: Arc<InMemoryProbeManager>,
}

pub fn router(state: AdminState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/debug/probes", get(list_probes))
        .with_state(state)
}

async fn healthz(State(state): State<AdminState>) -> impl IntoResponse {
    let probes = state.probe_manager.list_status().await.unwrap_or_default();
    Json(AgentHealthSnapshot {
        status: "ok",
        loaded_plugins: probes.iter().filter(|p| p.plugin_id.is_some()).count(),
        active_probes: probes.iter().filter(|p| p.state == "running").count(),
    })
}

async fn readyz() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ready" }))
}

async fn list_probes(State(state): State<AdminState>) -> impl IntoResponse {
    Json(state.probe_manager.list_status().await.unwrap_or_default())
}
