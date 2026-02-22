use std::sync::Arc;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    connect_async,
    tungstenite::Message,
};
use tokio::net::TcpStream;
use axum::{routing::any, Router};
use uuid::Uuid;

use meerkat_server::{
    messages::{
        ClientEvent, CreateObjectPayload, DeleteObjectPayload, JoinSessionPayload, ServerEvent,
        SelectObjectPayload, UpdateNamePayload, UpdatePropertiesPayload, UpdateTransformPayload,
    },
    types::{AppState, ObjectProperties, ObjectType, PointLightProperties, Transform},
    websocket::handler,
};

// ── Test helpers ──────────────────────────────────────────────────────────────

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn start_test_server() -> String {
    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
    };
    let app = Router::new().route("/ws", any(handler)).with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("ws://127.0.0.1:{}/ws", port)
}

/// Sends a ClientEvent as JSON text over the WebSocket.
async fn send(ws: &mut WsStream, event: ClientEvent) {
    let json = serde_json::to_string(&event).expect("ClientEvent serialization failed");
    ws.send(Message::Text(json.into())).await.expect("send failed");
}

/// Receives the next text frame and deserializes it as a ServerEvent.
/// Panics if no message arrives within 5 seconds.
async fn recv(ws: &mut WsStream) -> ServerEvent {
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

/// Like `recv`, but returns `None` if nothing arrives within 300ms.
/// Used to assert that a client does NOT receive a message.
async fn try_recv(ws: &mut WsStream) -> Option<ServerEvent> {
    match timeout(Duration::from_millis(300), ws.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            Some(serde_json::from_str(&text).expect("invalid ServerEvent JSON"))
        }
        _ => None,
    }
}

fn cube_payload(object_id: Uuid) -> CreateObjectPayload {
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

// ── Phase 1 acceptance test ───────────────────────────────────────────────────

/// Walks through the full Phase 1 scenario from the PRD:
///   1. Client A joins session "test-01" → receives FullStateSync
///   2. Client B joins "test-01" → receives FullStateSync; A receives UserJoined
///   3. A creates a Cube → both clients receive ObjectCreated with matching UUID
///   4. A deletes the Cube → both clients receive ObjectDeleted with matching UUID
///   5. A disconnects → B receives UserLeft
#[tokio::test]
async fn test_phase_1_full_flow() {
    let url = start_test_server().await;

    // ── Step 1: Client A joins ────────────────────────────────────────────────
    let (mut ws_a, _) = connect_async(&url).await.expect("A: connect failed");
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "test-01".to_string(),
        display_name: "Alice".to_string(),
    })).await;

    let msg = recv(&mut ws_a).await;
    assert!(
        matches!(msg, ServerEvent::FullStateSync(_)),
        "A: expected FullStateSync on join, got {:?}", msg
    );

    // ── Step 2: Client B joins the same session ───────────────────────────────
    let (mut ws_b, _) = connect_async(&url).await.expect("B: connect failed");
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "test-01".to_string(),
        display_name: "Bob".to_string(),
    })).await;

    let msg_b = recv(&mut ws_b).await;
    assert!(
        matches!(msg_b, ServerEvent::FullStateSync(_)),
        "B: expected FullStateSync on join, got {:?}", msg_b
    );

    let msg_a = recv(&mut ws_a).await;
    assert!(
        matches!(msg_a, ServerEvent::UserJoined(_)),
        "A: expected UserJoined after B joined, got {:?}", msg_a
    );

    // ── Step 3: Client A creates a Cube ──────────────────────────────────────
    let object_id = Uuid::new_v4();
    send(&mut ws_a, ClientEvent::CreateObject(CreateObjectPayload {
        object_id,
        name: "TestCube".to_string(),
        object_type: ObjectType::Cube,
        asset_id: None,
        asset_library: None,
        transform: Transform {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
            scale:    [1.0, 1.0, 1.0],
        },
        properties: None,
    })).await;

    // Broadcast includes the sender, so both A and B receive ObjectCreated.
    let created_a = recv(&mut ws_a).await;
    let created_b = recv(&mut ws_b).await;

    match &created_a {
        ServerEvent::ObjectCreated(p) => assert_eq!(p.object.object_id, object_id, "A: wrong object_id"),
        _ => panic!("A: expected ObjectCreated, got {:?}", created_a),
    }
    match &created_b {
        ServerEvent::ObjectCreated(p) => assert_eq!(p.object.object_id, object_id, "B: wrong object_id"),
        _ => panic!("B: expected ObjectCreated, got {:?}", created_b),
    }

    // ── Step 4: Client A deletes the Cube ────────────────────────────────────
    send(&mut ws_a, ClientEvent::DeleteObject(DeleteObjectPayload { object_id })).await;

    let deleted_a = recv(&mut ws_a).await;
    let deleted_b = recv(&mut ws_b).await;

    match &deleted_a {
        ServerEvent::ObjectDeleted(p) => assert_eq!(p.object_id, object_id, "A: deleted wrong object"),
        _ => panic!("A: expected ObjectDeleted, got {:?}", deleted_a),
    }
    match &deleted_b {
        ServerEvent::ObjectDeleted(p) => assert_eq!(p.object_id, object_id, "B: deleted wrong object"),
        _ => panic!("B: expected ObjectDeleted, got {:?}", deleted_b),
    }

    // ── Step 5: Client A disconnects — B should receive UserLeft ─────────────
    drop(ws_a); // dropping the stream closes the underlying TCP connection

    let user_left = recv(&mut ws_b).await;
    assert!(
        matches!(user_left, ServerEvent::UserLeft(_)),
        "B: expected UserLeft after A disconnected, got {:?}", user_left
    );
}

// ── Handler coverage ──────────────────────────────────────────────────────────

/// Exercises UpdateTransform, UpdateName, UpdateProperties, and SelectObject
/// in sequence, asserting that both clients receive every broadcast.
#[tokio::test]
async fn test_update_handlers() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "update-test".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "update-test".to_string(),
        display_name: "Bob".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync
    recv(&mut ws_a).await; // UserJoined(Bob)

    // Create an object so the update handlers have something to mutate.
    let object_id = Uuid::new_v4();
    send(&mut ws_a, ClientEvent::CreateObject(cube_payload(object_id))).await;
    recv(&mut ws_a).await; // ObjectCreated (echo to sender)
    recv(&mut ws_b).await; // ObjectCreated

    // ── UpdateTransform ───────────────────────────────────────────────────────
    let new_transform = Transform { position: [5.0, 10.0, 15.0], rotation: [0.1, 0.2, 0.3], scale: [2.0; 3] };
    send(&mut ws_a, ClientEvent::UpdateTransform(UpdateTransformPayload {
        object_id,
        transform: new_transform.clone(),
    })).await;

    let tf_a = recv(&mut ws_a).await;
    let tf_b = recv(&mut ws_b).await;

    match &tf_a {
        ServerEvent::TransformUpdated(p) => {
            assert_eq!(p.object_id, object_id);
            assert_eq!(p.transform.position, new_transform.position);
        }
        _ => panic!("A: expected TransformUpdated, got {:?}", tf_a),
    }
    assert!(matches!(tf_b, ServerEvent::TransformUpdated(_)), "B: expected TransformUpdated");

    // ── UpdateName ────────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::UpdateName(UpdateNamePayload {
        object_id,
        name: "renamed_cube".to_string(),
    })).await;

    let name_a = recv(&mut ws_a).await;
    let name_b = recv(&mut ws_b).await;

    match &name_a {
        ServerEvent::NameUpdated(p) => {
            assert_eq!(p.object_id, object_id);
            assert_eq!(p.name, "renamed_cube");
        }
        _ => panic!("A: expected NameUpdated, got {:?}", name_a),
    }
    assert!(matches!(name_b, ServerEvent::NameUpdated(_)), "B: expected NameUpdated");

    // ── UpdateProperties ──────────────────────────────────────────────────────
    let props = ObjectProperties::PointLight(PointLightProperties {
        color: [1.0, 0.5, 0.0],
        temperature: 6500.0,
        exposure: 0.0,
        power: 100.0,
        radius: 0.1,
        soft_falloff: true,
        normalize: false,
    });
    send(&mut ws_a, ClientEvent::UpdateProperties(UpdatePropertiesPayload {
        object_id,
        properties: props,
    })).await;

    let props_a = recv(&mut ws_a).await;
    let props_b = recv(&mut ws_b).await;

    match &props_a {
        ServerEvent::PropertiesUpdated(p) => assert_eq!(p.object_id, object_id),
        _ => panic!("A: expected PropertiesUpdated, got {:?}", props_a),
    }
    assert!(matches!(props_b, ServerEvent::PropertiesUpdated(_)), "B: expected PropertiesUpdated");

    // ── SelectObject ──────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::SelectObject(SelectObjectPayload {
        object_id: Some(object_id),
    })).await;

    let sel_a = recv(&mut ws_a).await;
    let sel_b = recv(&mut ws_b).await;

    match &sel_a {
        ServerEvent::UserSelected(p) => assert_eq!(p.object_id, Some(object_id)),
        _ => panic!("A: expected UserSelected, got {:?}", sel_a),
    }
    assert!(matches!(sel_b, ServerEvent::UserSelected(_)), "B: expected UserSelected");

    // ── Deselect ──────────────────────────────────────────────────────────────
    send(&mut ws_a, ClientEvent::SelectObject(SelectObjectPayload { object_id: None })).await;
    let desel_a = recv(&mut ws_a).await;
    match &desel_a {
        ServerEvent::UserSelected(p) => assert!(p.object_id.is_none(), "expected deselect (None)"),
        _ => panic!("A: expected UserSelected(None), got {:?}", desel_a),
    }
    recv(&mut ws_b).await; // UserSelected(None)
}

/// Verifies that an explicit LeaveSession cleans up the user and broadcasts
/// UserLeft, and that the connection stays open for a potential rejoin.
#[tokio::test]
async fn test_explicit_leave_session() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "leave-test".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "leave-test".to_string(),
        display_name: "Bob".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync
    recv(&mut ws_a).await; // UserJoined(Bob)

    // A explicitly leaves (not a disconnect).
    send(&mut ws_a, ClientEvent::LeaveSession).await;

    let left_b = recv(&mut ws_b).await;
    assert!(
        matches!(left_b, ServerEvent::UserLeft(_)),
        "B: expected UserLeft after A left, got {:?}", left_b
    );

    // A's connection is still open — it should be able to rejoin a new session.
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "leave-test-2".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    let sync = recv(&mut ws_a).await;
    assert!(
        matches!(sync, ServerEvent::FullStateSync(_)),
        "A: expected FullStateSync after rejoining, got {:?}", sync
    );
}

// ── Session isolation ─────────────────────────────────────────────────────────

/// Two clients in separate sessions. Events from one session must never
/// reach the other.
#[tokio::test]
async fn test_session_isolation() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "iso-alpha".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "iso-beta".to_string(),
        display_name: "Bob".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync

    // A creates an object in iso-alpha.
    send(&mut ws_a, ClientEvent::CreateObject(cube_payload(Uuid::new_v4()))).await;
    recv(&mut ws_a).await; // ObjectCreated (A's own echo)

    // B must not receive anything.
    assert!(
        try_recv(&mut ws_b).await.is_none(),
        "B: received a message from a different session — isolation broken"
    );

    // B creates an object in iso-beta.
    send(&mut ws_b, ClientEvent::CreateObject(cube_payload(Uuid::new_v4()))).await;
    recv(&mut ws_b).await; // ObjectCreated (B's own echo)

    // A must not receive anything.
    assert!(
        try_recv(&mut ws_a).await.is_none(),
        "A: received a message from a different session — isolation broken"
    );
}

// ── Concurrent writes ─────────────────────────────────────────────────────────

/// Two users in separate sessions both write concurrently.
/// Each should receive only their own events with no cross-session bleed.
#[tokio::test]
async fn test_concurrent_writes_separate_sessions() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "concurrent-1".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "concurrent-2".to_string(),
        display_name: "Bob".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync

    let obj_a = Uuid::new_v4();
    let obj_b = Uuid::new_v4();

    // Both fire CreateObject at the same time.
    tokio::join!(
        send(&mut ws_a, ClientEvent::CreateObject(cube_payload(obj_a))),
        send(&mut ws_b, ClientEvent::CreateObject(cube_payload(obj_b)))
    );

    let msg_a = recv(&mut ws_a).await;
    let msg_b = recv(&mut ws_b).await;

    match &msg_a {
        ServerEvent::ObjectCreated(p) => assert_eq!(p.object.object_id, obj_a, "A received B's object"),
        _ => panic!("A: expected ObjectCreated, got {:?}", msg_a),
    }
    match &msg_b {
        ServerEvent::ObjectCreated(p) => assert_eq!(p.object.object_id, obj_b, "B received A's object"),
        _ => panic!("B: expected ObjectCreated, got {:?}", msg_b),
    }

    // Confirm no stray messages crossed the session boundary.
    assert!(try_recv(&mut ws_a).await.is_none(), "A: stray message from session-2");
    assert!(try_recv(&mut ws_b).await.is_none(), "B: stray message from session-1");
}

/// Two users in the SAME session both write concurrently.
/// Both should receive both events (4 messages total).
#[tokio::test]
async fn test_concurrent_writes_same_session() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "concurrent-shared".to_string(),
        display_name: "Alice".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "concurrent-shared".to_string(),
        display_name: "Bob".to_string(),
    })).await;
    recv(&mut ws_b).await; // FullStateSync
    recv(&mut ws_a).await; // UserJoined(Bob)

    let obj_a = Uuid::new_v4();
    let obj_b = Uuid::new_v4();

    // Both create objects simultaneously in the same session.
    tokio::join!(
        send(&mut ws_a, ClientEvent::CreateObject(cube_payload(obj_a))),
        send(&mut ws_b, ClientEvent::CreateObject(cube_payload(obj_b)))
    );

    // Collect two ObjectCreated events per client (order not guaranteed).
    let mut ids_seen_by_a = vec![
        extract_object_id(recv(&mut ws_a).await),
        extract_object_id(recv(&mut ws_a).await),
    ];
    let mut ids_seen_by_b = vec![
        extract_object_id(recv(&mut ws_b).await),
        extract_object_id(recv(&mut ws_b).await),
    ];

    ids_seen_by_a.sort();
    ids_seen_by_b.sort();
    let mut expected = vec![obj_a, obj_b];
    expected.sort();

    assert_eq!(ids_seen_by_a, expected, "A did not receive both ObjectCreated events");
    assert_eq!(ids_seen_by_b, expected, "B did not receive both ObjectCreated events");
}

fn extract_object_id(event: ServerEvent) -> Uuid {
    match event {
        ServerEvent::ObjectCreated(p) => p.object.object_id,
        other => panic!("expected ObjectCreated, got {:?}", other),
    }
}
