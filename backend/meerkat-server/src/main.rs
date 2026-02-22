use std::sync::Arc;
use dashmap::DashMap;
use axum::{routing::any, Router};
use meerkat_server::{types::AppState, websocket::handler};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .init();

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
    };

    let app: Router = Router::new()
        .route("/ws", any(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    tracing::info!(addr = "0.0.0.0:8000", "server listening");
    axum::serve(listener, app).await.unwrap();
}
