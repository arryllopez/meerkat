use std::sync::Arc;

use axum::{routing::any, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::Message,
};
use uuid::Uuid;

use meerkat_server::{
    messages::{ClientEvent, CreateObjectPayload, CreateSessionPayload, JoinSessionPayload, ServerEvent},
    types::{AppState, ObjectType, Transform},
    websocket::tcp_socket_upgrade,
};

pub const TEST_PASSWORD: &str = "testpassword123";

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn start_test_server() -> String {
    start_test_server_with_state().await.0
}

pub async fn start_test_server_with_state() -> (String, AppState) {
    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
        connection_backpressure: Arc::new(DashMap::new()),
        session_connections: Arc::new(DashMap::new()),
    };
    let app = Router::new().route("/ws", any(tcp_socket_upgrade)).with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("ws://127.0.0.1:{}/ws", port), state)
}

pub async fn send(ws: &mut WsStream, event: ClientEvent) {
    // Server expects MessageEnvelope format: {event_type, timestamp, source_user_id, payload}
    // Serialize the ClientEvent to get {event_type, payload}, then inject envelope fields.
    let tagged: serde_json::Value = serde_json::to_value(&event).expect("ClientEvent serialization failed");
    let envelope = serde_json::json!({
        "event_type": tagged["event_type"],
        "payload": tagged["payload"],
        "timestamp": 0u64,
        "source_user_id": Uuid::new_v4().to_string(),
    });
    let json = serde_json::to_string(&envelope).expect("envelope serialization failed");
    ws.send(Message::Text(json.into())).await.expect("send failed");
}

pub async fn recv(ws: &mut WsStream) -> ServerEvent {
    loop {
        let msg = timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("recv timed out after 5s")
            .expect("WebSocket stream closed unexpectedly")
            .expect("WebSocket error on recv");
        if let Message::Text(text) = msg {
            return serde_json::from_str(&text).expect("invalid ServerEvent JSON");
        }
    }
}

pub async fn try_recv(ws: &mut WsStream) -> Option<ServerEvent> {
    match timeout(Duration::from_millis(300), ws.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            Some(serde_json::from_str(&text).expect("invalid ServerEvent JSON"))
        }
        _ => None,
    }
}

pub fn cube_payload(object_id: Uuid) -> CreateObjectPayload {
    CreateObjectPayload {
        object_id,
        name: "Cube".to_string(),
        object_type: ObjectType::Cube,
        asset_id: None,
        asset_library: None,
        transform: Transform { position: [0.0; 3], rotation: [0.0; 3], scale: [1.0; 3] },
        properties: None,
    }
}

pub fn asset_payload(object_id: Uuid, name : &str,  asset_id : Option<String>,  asset_library : Option<String>) -> CreateObjectPayload {
    CreateObjectPayload {
        object_id,
        name : name.to_string(),
        object_type: ObjectType::AssetRef,
        asset_id,
        asset_library,
        transform: Transform { position: [0.0; 3], rotation: [0.0; 3], scale: [1.0; 3] },
        properties: None,

    }
}

pub fn extract_object_id(event: ServerEvent) -> Uuid {
    match event {
        ServerEvent::ObjectCreated(p) => p.object.object_id,
        other => panic!("expected ObjectCreated, got {:?}", other),
    }
}

// Create a new session (first client). Returns the FullStateSync payload.
pub async fn create_session(ws: &mut WsStream, session_id: &str, display_name: &str) {
    send(ws, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: session_id.to_string(),
        display_name: display_name.to_string(),
        password: TEST_PASSWORD.to_string(),
    })).await;
    let msg = recv(ws).await;
    assert!(matches!(msg, ServerEvent::FullStateSync(_)), "expected FullStateSync after CreateSession, got {:?}", msg);
}

/// Join an existing session (subsequent clients). Returns FullStateSync.
pub async fn join_session(ws: &mut WsStream, session_id: &str, display_name: &str) {
    send(ws, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: session_id.to_string(),
        display_name: display_name.to_string(),
        password: TEST_PASSWORD.to_string(),
    })).await;
    let msg = recv(ws).await;
    assert!(matches!(msg, ServerEvent::FullStateSync(_)), "expected FullStateSync after JoinSession, got {:?}", msg);
}
