use std::sync::{Arc, RwLock};
use dashmap::DashMap;
use axum::{routing::any, Router};
use meerkat_server::{event_log, types::{AppState, SessionHandle}, websocket::handler};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .init();

    // Ensure ./data/ directory exists
    event_log::ensure_data_dir();

    // Replay any existing event logs to restore sessions from a previous run
    let restored = event_log::replay_all_logs();
    let sessions = Arc::new(DashMap::new());
    let log_files = Arc::new(DashMap::new());

    for (session_id, session) in restored {
        // Re-open the log file handle for continued appending
        let writer = event_log::open_log_file(&session_id);
        log_files.insert(session_id.clone(), writer);
        // Convert the replayed Session into a SessionHandle wrapped in Arc
        sessions.insert(session_id, Arc::new(SessionHandle {
            objects: RwLock::new(session.objects),
            users: RwLock::new(session.users),
            event_log: RwLock::new(session.event_log),
            session_id: session.session_id,
        }));
    }

    let state = AppState {
        sessions,
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
        log_files,
    };

    let app: Router = Router::new()
        .route("/ws", any(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    tracing::info!(addr = "0.0.0.0:8000", "server listening");
    axum::serve(listener, app).await.unwrap();
}
