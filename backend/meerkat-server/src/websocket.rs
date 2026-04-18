use axum::{
    extract::{
        State,
        ws::{CloseCode, CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use tokio::sync::mpsc;
use tokio::select; 
use uuid::Uuid;


use crate::{
    handlers::{
        self,
        helpers::broadcast,
    },

    messages::{ClientEvent, ServerEvent, UserLeftPayload, parse_client_message},
    types::AppState,
};

const EVICTED_CLOSE_CODE: CloseCode = 4008;

// tcp_socket_ugprade upgrades a TCP connection to a Websocket 

pub async fn tcp_socket_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_failed_upgrade(|error| {
        tracing::error!(error = %error, "WebSocket upgrade failed");
    })
    .on_upgrade(|socket| async move {
        handle_connection(socket, state).await; 
    })
}

// ── Per-connection event loop ─────────────────────────────────────────────────

pub async fn handle_connection(mut socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();
    let (tx, mut rx) = mpsc::channel::<String>(64);
    state.connections.insert(connection_id, tx); 

    tracing::info!(connection_id = %connection_id, "connection opened");

    loop {
        select! {
            // Branch 1, client sends something
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        match parse_client_message(&t) {
                            Ok(event) => {
                                // High-frequency events (cursor) demoted to trace to avoid log spam.
                                if matches!(event, ClientEvent::UpdateCursor(_)) {
                                    tracing::trace!(connection_id = %connection_id, event_type = ?event, "parsed client event");
                                } else {
                                    tracing::info!(connection_id = %connection_id, raw_message = %t, "received client message");
                                    tracing::info!(connection_id = %connection_id, event_type = ?event, "parsed client event");
                                }
                                dispatch(&mut socket, &state, connection_id, event).await
                            },
                            Err(e) => {
                                tracing::warn!(
                                    connection_id = %connection_id,
                                    error = %e,
                                    raw_message = %t,
                                    "failed to parse client message"
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(Some(frame)))) => {
                        tracing::info!(
                            connection_id = %connection_id,
                            code = %frame.code,
                            reason = %frame.reason,
                            "client sent close frame"
                        );
                        break;
                    }
                    Some(Ok(Message::Close(None))) => {
                        tracing::info!(
                            connection_id = %connection_id,
                            "client sent close frame with no payload"
                        );
                        break;
                    }
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => break,
                    None => { 
                        // This none case means the server cant read client messages 
                        break;
                    }
                }
            }
            // Branch 2 server sends to client 
            msg = rx.recv() => {
                match msg { 
                    Some (text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        let _ = socket.send(Message::Close(Some(CloseFrame {
                            code: EVICTED_CLOSE_CODE,
                            reason: "client was dropped from broadcast due to full/closed channel or missing sender".into(),
                        }))).await;
                        break;
                    }
                }
            }
        }
    }

    // ── Disconnect cleanup ────────────────────────────────────────────────────
    state.connections.remove(&connection_id);
    state.connection_backpressure.remove(&connection_id);

    // If the client was in a session (did not call LeaveSession cleanly), clean up now.
    if let Some((_, (sid, uid))) = state.connection_meta.remove(&connection_id) {
        let mut remove_session_entry = false;
        if let Some(mut conns) = state.session_connections.get_mut(&sid) {
            conns.remove(&connection_id);
            remove_session_entry = conns.is_empty();
        }
        if remove_session_entry {
            state.session_connections.remove(&sid);
        }

        let mut reclaim_session = false;
        if let Some(session) = state.sessions.get(&sid) {
            let mut users = match session.users.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::warn!("Session users lock poisoned during disconnect cleanup, recovering");
                    poisoned.into_inner()
                }
            };
            users.remove(&uid);
            reclaim_session = users.is_empty();
        }

        if reclaim_session {
            state.sessions.remove(&sid);
            tracing::info!(
                event_type = "SessionReclaimed",
                session_id = %sid,
                "reclaimed empty session after disconnect"
            );
        }

        match serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload {
            user_id: uid,
        })) {
            Ok(left_json) => {
                let count = broadcast(&state, &sid, &left_json, None);
                tracing::info!(
                    connection_id = %connection_id,
                    session_id = %sid,
                    user_id = %uid,
                    recipient_count = count,
                    "connection closed — broadcast UserLeft"
                );
            }
            Err(err) => {
                tracing::error!(
                    connection_id = %connection_id,
                    session_id = %sid,
                    user_id = %uid,
                    error = %err,
                    "failed to serialize UserLeft during disconnect cleanup"
                );
            }
        }
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
        ClientEvent::CreateSession(p)    => handlers::create_session::handle(socket, state, connection_id, p).await,
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
