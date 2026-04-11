use tokio_tungstenite::connect_async;

use meerkat_server::messages::{ClientEvent, JoinSessionPayload, ServerEvent};

mod common;

use common::{recv, send, start_test_server};

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
