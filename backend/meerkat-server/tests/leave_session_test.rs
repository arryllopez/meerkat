use tokio_tungstenite::connect_async;
use tokio::time::{timeout, Duration};

use meerkat_server::messages::{ClientEvent, JoinSessionPayload, ServerEvent};

mod common;

use common::{recv, send, start_test_server, start_test_server_with_state};

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

#[tokio::test]
async fn test_reclaim_session_on_last_user_leave() {
    let (url, state) = start_test_server_with_state().await;
    let session_id = "reclaim-on-last-leave";

    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: session_id.to_string(),
        display_name: "Alice".to_string(),
    })).await;

    let sync = recv(&mut ws).await;
    assert!(matches!(sync, ServerEvent::FullStateSync(_)));

    send(&mut ws, ClientEvent::LeaveSession).await;

    let reclaimed = timeout(Duration::from_secs(2), async {
        loop {
            if state.sessions.get(session_id).is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    assert!(
        reclaimed.is_ok(),
        "expected session to be reclaimed after last user left"
    );
}
