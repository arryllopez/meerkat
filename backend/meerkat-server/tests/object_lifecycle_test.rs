use tokio_tungstenite::connect_async;
use uuid::Uuid;

use meerkat_server::{
    messages::{
        ClientEvent, CreateObjectPayload, CreateSessionPayload, DeleteObjectPayload, JoinSessionPayload, ServerEvent,
    },
    types::{ObjectType, Transform},
};

mod common;

use common::{recv, send, start_test_server};

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
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "test-01".to_string(),
        display_name: "Alice".to_string(),
        password: "somepassword".to_string(),
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
        password: "somepassword".to_string(),
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
