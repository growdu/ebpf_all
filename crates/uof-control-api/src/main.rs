use uof_control_api::routes::router;
use uof_control_plane::state::AppState;

#[tokio::main]
async fn main() {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let base_url = std::env::var("CONTROL_PLANE_URL")
        .unwrap_or_else(|_| format!("http://{}", bind_addr));

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/uof".to_string());

    let pool = uof_control_plane::db::create_pool(&database_url, 10)
        .await
        .expect("failed to create database pool");

    uof_control_plane::db::run_migrations(&pool)
        .await
        .expect("failed to run migrations");

    let state = AppState::new(pool, base_url).await;

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!(addr = %bind_addr, "control plane listening");

    axum::serve(listener, app)
        .await
        .unwrap();
}
