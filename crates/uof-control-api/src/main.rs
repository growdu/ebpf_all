use uof_control_api::routes::router;
use uof_control_plane::state::AppState;

#[tokio::main]
async fn main() {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let base_url = std::env::var("CONTROL_PLANE_URL")
        .unwrap_or_else(|_| format!("http://{}", bind_addr));

    let state = AppState::new(base_url);

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!(addr = %bind_addr, "control plane listening");

    axum::serve(listener, app)
        .await
        .unwrap();
}
