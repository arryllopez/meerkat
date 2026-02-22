mod types;
mod messages;
mod websocket;

use std::sync::Arc;
use dashmap::DashMap;
use crate::types::AppState;
use crate::websocket::handler;


use axum::{
    routing::any,
    Router,
};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
    };

    tracing_subscriber::fmt() 
        .json() 
        .init(); 

    let app : Router = Router::new()
    .route("/ws", any(handler))
    .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    tracing::info!("Server started listening on 0.0.0.8000");
    axum::serve(listener, app).await.unwrap();



}
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    