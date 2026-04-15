use std::sync::Arc;
use dashmap::DashMap;
use axum::{routing::any, Router};
use tokio::net::TcpListener;
use meerkat_server::{types::AppState, websocket::tcp_socket_upgrade};

pub fn logging_init() { 
    tracing_subscriber::fmt()
        .json()
        .init();
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    logging_init();

    let state = AppState {
        sessions: Arc::new(DashMap::new()),              // K: session_id: String | V: Arc<SessionHandle>
        connections: Arc::new(DashMap::new()),           // K: connection_id: Uuid | V: mpsc::Sender<String>
        connection_meta: Arc::new(DashMap::new()),       // K: connection_id: Uuid | V: (session id string user id uuid) 
        connection_backpressure: Arc::new(DashMap::new()), // K: connection_id: Uuid | V: LagState {strikes: u8, last_full_at_ms: u64}
        session_connections: Arc::new(DashMap::new()),   // K: session_id: String | V: HashSet<connection_id: Uuid>
    };

    let app: Router = Router::new()
        .route("/ws", any(tcp_socket_upgrade))
        .with_state(state);

    let listener = match TcpListener::bind("0.0.0.0:8000").await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "failed to bind to 0.0.0.0:8000");
            return;
        }
    };

    tracing::info!(addr = "0.0.0.0:8000", "server listening");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }
}
