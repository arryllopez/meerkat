use std::sync::Arc;

use axum::{routing::any, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream,
    WebSocketStream,
};
use uuid::Uuid;

use meerkat_server::{
    messages::{ClientEvent, CreateObjectPayload, JoinSessionPayload, ServerEvent},
    types::{AppState, ObjectType, Transform},
    websocket::handler,
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn start_test_server() -> String {
    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections: Arc::new(DashMap::new()),
        connection_meta: Arc::new(DashMap::new()),
        connection_backpressure: Arc::new(DashMap::new()),
        session_connections: Arc::new(DashMap::new()),
    };

    let app = Router::new().route("/ws", any(handler)).with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("ws://127.0.0.1:{}/ws", port)
}

async fn send(ws: &mut WsStream, event: ClientEvent) {
    let json = serde_json::to_string(&event).expect("ClientEvent serialization failed");
    ws.send(Message::Text(json.into())).await.expect("send failed");
}

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
        transform: Transform {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        },
        properties: None,
    }
}

#[tokio::test]
async fn test_rejoin_same_connection_cleans_old_membership() {
    let url = start_test_server().await;

    // A joins room-1.
    let (mut ws_a, _) = connect_async(&url).await.expect("A: connect failed");
    send(
        &mut ws_a,
        ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "room-1".to_string(),
            display_name: "Alice".to_string(),
        }),
    )
    .await;

    let first_sync = recv(&mut ws_a).await;
    let alice_user_id = match first_sync {
        ServerEvent::FullStateSync(p) => {
            assert_eq!(p.session.users.len(), 1, "expected only Alice in room-1");
            *p.session
                .users
                .keys()
                .next()
                .expect("expected Alice user_id in FullStateSync")
        }
        other => panic!("A: expected FullStateSync, got {:?}", other),
    };

    // B joins room-1.
    let (mut ws_b, _) = connect_async(&url).await.expect("B: connect failed");
    send(
        &mut ws_b,
        ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "room-1".to_string(),
            display_name: "Bob".to_string(),
        }),
    )
    .await;

    let b_sync = recv(&mut ws_b).await;
    assert!(
        matches!(b_sync, ServerEvent::FullStateSync(_)),
        "B: expected FullStateSync on join"
    );

    let a_user_joined = recv(&mut ws_a).await;
    assert!(
        matches!(a_user_joined, ServerEvent::UserJoined(_)),
        "A: expected UserJoined when Bob joined"
    );

    // A re-joins into room-2 on the SAME socket.
    send(
        &mut ws_a,
        ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "room-2".to_string(),
            display_name: "Alice".to_string(),
        }),
    )
    .await;

    // A should receive a fresh room-2 snapshot.
    let second_sync = recv(&mut ws_a).await;
    assert!(
        matches!(second_sync, ServerEvent::FullStateSync(_)),
        "A: expected FullStateSync after re-join"
    );

    // B should be told Alice left room-1.
    let b_left = recv(&mut ws_b).await;
    match b_left {
        ServerEvent::UserLeft(p) => {
            assert_eq!(p.user_id, alice_user_id, "B: wrong user_id in UserLeft")
        }
        other => panic!("B: expected UserLeft after A re-join, got {:?}", other),
    }

    // Ensure no cross-session leak: A creates object in room-2, B should not see it.
    let object_id = Uuid::new_v4();
    send(
        &mut ws_a,
        ClientEvent::CreateObject(cube_payload(object_id)),
    )
    .await;

    // A receives its own ObjectCreated echo.
    let a_created = recv(&mut ws_a).await;
    assert!(
        matches!(a_created, ServerEvent::ObjectCreated(_)),
        "A: expected ObjectCreated echo in room-2"
    );

    let b_maybe = try_recv(&mut ws_b).await;
    assert!(
        b_maybe.is_none(),
        "B: should not receive room-2 ObjectCreated after A re-join"
    );
}
