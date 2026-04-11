use tokio_tungstenite::connect_async;
use uuid::Uuid;

use meerkat_server::messages::{ClientEvent, JoinSessionPayload, ServerEvent};

mod common;

use common::{asset_payload, cube_payload, extract_object_id, recv, send, start_test_server, try_recv};

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
/// Testing with an asset reference within a shared library
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
    let joined = recv(&mut ws_a).await;
    assert!(
        matches!(joined, ServerEvent::UserJoined(_)),
        "A: expected UserJoined(Bob), got {:?}",
        joined
    );

    let obj_a = Uuid::new_v4();
    let obj_b = Uuid::new_v4();

    //definitions for assets
    let asset_a : Option<String> = Some("dragon".to_string());
    let asset_b : Option<String> = Some("tree".to_string());
    let asset_library_a : Option<String> = Some("sharedLibrary".to_string());
    let asset_library_b : Option<String> = Some("sharedLibrary".to_string());

    // Both create objects simultaneously in the same session.
    tokio::join!(
        send(&mut ws_a, ClientEvent::CreateObject(asset_payload(obj_a, "Dragon", asset_a, asset_library_a ))),
        send(&mut ws_b, ClientEvent::CreateObject(asset_payload(obj_b, "Tree", asset_b, asset_library_b))),
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
