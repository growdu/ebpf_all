mod routes;

use axum::Router;
use tower_http::trace::TraceLayer;
use uof_common::{telemetry::init_tracing, AppConfig};
use uof_control_plane::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = AppConfig::default();
    let state = AppState::default();

    let app = Router::new()
        .merge(routes::router(state))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("{} listening on {}", config.service_name, config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
