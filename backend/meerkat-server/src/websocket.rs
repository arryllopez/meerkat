use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use tokio::sync::mpsc;
use uuid::Uuid;
use axum::extract::ws::Message;

use crate::{
    handlers::{
        self,
        helpers::broadcast,
    },
    messages::{ClientEvent, ServerEvent, UserLeftPayload, parse_client_message},
    types::AppState,
};

// ── HTTP upgrade entry-point ──────────────────────────────────────────────────

pub async fn handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

// ── Per-connection event loop ─────────────────────────────────────────────────

pub async fn handle_connection(mut socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    let (tx, mut rx) = mpsc::channel::<String>(32);
    state.connections.insert(connection_id, tx);

    tracing::info!(connection_id = %connection_id, "connection opened");

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                let text = match msg {
                    Ok(Message::Text(t)) => t.to_string(),
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => continue,
                };
                match parse_client_message(&text) {
                    Ok(event) => dispatch(&mut socket, &state, connection_id, event).await,
                    Err(e) => {
                        tracing::warn!(
                            connection_id = %connection_id,
                            error = %e,
                            "failed to parse client message"
                        );
                    }
                }
            }
            Some(text) = rx.recv() => {
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    // ── Disconnect cleanup ────────────────────────────────────────────────────
    state.connections.remove(&connection_id);

    // If the client was in a session (did not call LeaveSession cleanly), clean up now.
    if let Some((_, (sid, uid))) = state.connection_meta.remove(&connection_id) {
        if let Some(session) = state.sessions.get(&sid) {
            session.users.remove(&uid);
        }

        let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload {
            user_id: uid,
        }))
        .expect("UserLeft serialization failed");

        let count = broadcast(&state, &sid, &left_json, None);
        tracing::info!(
            connection_id = %connection_id,
            session_id = %sid,
            user_id = %uid,
            recipient_count = count,
            "connection closed — broadcast UserLeft"
        );
    } else {
        tracing::info!(connection_id = %connection_id, "connection closed (no active session)");
    }
}

// ── Event dispatcher ──────────────────────────────────────────────────────────

async fn dispatch(
    socket: &mut WebSocket,
    state: &AppState,
    connection_id: Uuid,
    event: ClientEvent,
) {
    match event {
        ClientEvent::JoinSession(p)      => handlers::join_session::handle(socket, state, connection_id, p).await,
        ClientEvent::LeaveSession        => handlers::leave_session::handle(state, connection_id).await,
        ClientEvent::CreateObject(p)     => handlers::create_object::handle(state, connection_id, p).await,
        ClientEvent::DeleteObject(p)     => handlers::delete_object::handle(state, connection_id, p).await,
        ClientEvent::UpdateTransform(p)  => handlers::update_transform::handle(state, connection_id, p).await,
        ClientEvent::UpdateProperties(p) => handlers::update_properties::handle(state, connection_id, p).await,
        ClientEvent::UpdateName(p)       => handlers::update_name::handle(state, connection_id, p).await,
        ClientEvent::SelectObject(p)     => handlers::select_object::handle(state, connection_id, p).await,
        ClientEvent::RequestStateSync    => handlers::request_state_sync::handle(socket, state, connection_id).await,
        ClientEvent::UpdateCursor(p)     => handlers::update_cursor::handle(state, connection_id, p).await,
    }
}
