use tokio_tungstenite::connect_async;
use uuid::Uuid;

use meerkat_server::messages::{ClientEvent, CreateSessionPayload};

mod common;

use common::{cube_payload, recv, send, start_test_server, try_recv};

/// Two clients in separate sessions. Events from one session must never
/// reach the other.
#[tokio::test]
async fn test_session_isolation() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "iso-alpha".to_string(),
        display_name: "Alice".to_string(),
        password: "somepassword".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "iso-beta".to_string(),
        display_name: "Bob".to_string(),
        password: "somepassword".to_string(),
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
