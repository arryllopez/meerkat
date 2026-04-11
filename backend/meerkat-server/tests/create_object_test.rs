use tokio_tungstenite::connect_async;
use uuid::Uuid;

use meerkat_server::messages::{
    ClientEvent, CreateObjectPayload, JoinSessionPayload, ServerEvent,
};
use meerkat_server::types::{ObjectType, Transform};

mod common;

use common::{recv, send, start_test_server, try_recv};

#[tokio::test]
async fn test_duplicate_object_id_is_rejected() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.expect("A: connect failed");
    send(
        &mut ws_a,
        ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "dup-create".to_string(),
            display_name: "Alice".to_string(),
        }),
    )
    .await;
    let sync_a = recv(&mut ws_a).await;
    assert!(matches!(sync_a, ServerEvent::FullStateSync(_)));

    let (mut ws_b, _) = connect_async(&url).await.expect("B: connect failed");
    send(
        &mut ws_b,
        ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "dup-create".to_string(),
            display_name: "Bob".to_string(),
        }),
    )
    .await;
    let sync_b = recv(&mut ws_b).await;
    assert!(matches!(sync_b, ServerEvent::FullStateSync(_)));
    let joined_a = recv(&mut ws_a).await;
    assert!(matches!(joined_a, ServerEvent::UserJoined(_)));

    let object_id = Uuid::new_v4();

    // Initial create succeeds and is broadcast to both clients.
    send(
        &mut ws_a,
        ClientEvent::CreateObject(CreateObjectPayload {
            object_id,
            name: "OriginalCube".to_string(),
            object_type: ObjectType::Cube,
            asset_id: None,
            asset_library: None,
            transform: Transform {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            properties: None,
        }),
    )
    .await;

    let created_a = recv(&mut ws_a).await;
    assert!(matches!(created_a, ServerEvent::ObjectCreated(_)));
    let created_b = recv(&mut ws_b).await;
    assert!(matches!(created_b, ServerEvent::ObjectCreated(_)));

    // Duplicate create is rejected for sender.
    send(
        &mut ws_a,
        ClientEvent::CreateObject(CreateObjectPayload {
            object_id,
            name: "DuplicateCube".to_string(),
            object_type: ObjectType::Cube,
            asset_id: None,
            asset_library: None,
            transform: Transform {
                position: [3.0, 3.0, 3.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            properties: None,
        }),
    )
    .await;

    let err_a = recv(&mut ws_a).await;
    match err_a {
        ServerEvent::Error(e) => assert_eq!(e.code, "DUPLICATE_OBJECT_ID"),
        other => panic!("A: expected Error(DUPLICATE_OBJECT_ID), got {:?}", other),
    }

    // No duplicate create broadcast to peers.
    assert!(
        try_recv(&mut ws_b).await.is_none(),
        "B: should not receive any event for duplicate create"
    );

    // Canonical object should still be the original one.
    send(&mut ws_a, ClientEvent::RequestStateSync).await;
    let sync_after = recv(&mut ws_a).await;
    match sync_after {
        ServerEvent::FullStateSync(p) => {
            let obj = p
                .session
                .objects
                .get(&object_id)
                .expect("expected object to exist after duplicate rejection");
            assert_eq!(obj.name, "OriginalCube");
            assert_eq!(obj.transform.position, [0.0, 0.0, 0.0]);
        }
        other => panic!("A: expected FullStateSync after RequestStateSync, got {:?}", other),
    }
}
