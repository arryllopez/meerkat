use tokio_tungstenite::connect_async;

use meerkat_server::messages::{ClientEvent, CreateSessionPayload, JoinSessionPayload, ServerEvent};

mod common;

use common::{recv, send, start_test_server};

#[tokio::test]
async fn test_create_session_returns_full_state_sync() {
    let url = start_test_server().await;

    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "auth-create".to_string(),
        display_name: "Alice".to_string(),
        password: "secret123".to_string(),
    })).await;

    let msg = recv(&mut ws).await;
    match msg {
        ServerEvent::FullStateSync(p) => {
            assert_eq!(p.session.session_id, "auth-create");
            assert_eq!(p.session.users.len(), 1, "creator should be the only user");
        }
        other => panic!("expected FullStateSync, got {:?}", other),
    }
}

#[tokio::test]
async fn test_join_with_correct_password_succeeds() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "auth-join-ok".to_string(),
        display_name: "Alice".to_string(),
        password: "correctpassword".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "auth-join-ok".to_string(),
        display_name: "Bob".to_string(),
        password: "correctpassword".to_string(),
    })).await;

    let msg = recv(&mut ws_b).await;
    match msg {
        ServerEvent::FullStateSync(p) => {
            assert_eq!(p.session.users.len(), 2, "both Alice and Bob should be in session");
        }
        other => panic!("expected FullStateSync, got {:?}", other),
    }
}

#[tokio::test]
async fn test_join_with_wrong_password_rejected() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "auth-wrong-pw".to_string(),
        display_name: "Alice".to_string(),
        password: "correctpassword".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "auth-wrong-pw".to_string(),
        display_name: "Bob".to_string(),
        password: "wrongpassword".to_string(),
    })).await;

    let msg = recv(&mut ws_b).await;
    match msg {
        ServerEvent::Error(e) => {
            assert_eq!(e.code, "WRONG_PASSWORD");
        }
        other => panic!("expected Error(WRONG_PASSWORD), got {:?}", other),
    }
}

#[tokio::test]
async fn test_join_nonexistent_session_rejected() {
    let url = start_test_server().await;

    let (mut ws, _) = connect_async(&url).await.unwrap();
    send(&mut ws, ClientEvent::JoinSession(JoinSessionPayload {
        session_id: "does-not-exist".to_string(),
        display_name: "Alice".to_string(),
        password: "whatever".to_string(),
    })).await;

    let msg = recv(&mut ws).await;
    match msg {
        ServerEvent::Error(e) => {
            assert_eq!(e.code, "SessionNotFound");
        }
        other => panic!("expected Error(SessionNotFound), got {:?}", other),
    }
}

#[tokio::test]
async fn test_create_duplicate_session_rejected() {
    let url = start_test_server().await;

    let (mut ws_a, _) = connect_async(&url).await.unwrap();
    send(&mut ws_a, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "auth-dup".to_string(),
        display_name: "Alice".to_string(),
        password: "password1".to_string(),
    })).await;
    recv(&mut ws_a).await; // FullStateSync

    let (mut ws_b, _) = connect_async(&url).await.unwrap();
    send(&mut ws_b, ClientEvent::CreateSession(CreateSessionPayload {
        session_id: "auth-dup".to_string(),
        display_name: "Bob".to_string(),
        password: "password2".to_string(),
    })).await;

    let msg = recv(&mut ws_b).await;
    match msg {
        ServerEvent::Error(e) => {
            assert_eq!(e.code, "SessionAlreadyExists");
        }
        other => panic!("expected Error(SessionAlreadyExists), got {:?}", other),
    }
}
