use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;
use axum::extract::ws::Message;

use crate::{
    messages::{
        ClientEvent, FullStateSyncPayload, NameUpdatedPayload, ObjectCreatedPayload,
        ObjectDeletedPayload, PropertiesUpdatedPayload, ServerEvent, TransformUpdatedPayload,
        UserJoinedPayload, UserLeftPayload, UserSelectedPayload, parse_client_message,
    },
    types::{AppState, LogEntry, SceneObject, Session, User},
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as u64
}

/// Sends `json` to every connection in `session_id`, excluding `exclude` if provided.
/// Returns the number of recipients the message was dispatched to.
fn broadcast(state: &AppState, session_id: &str, json: &str, exclude: Option<Uuid>) -> usize {
    let mut count = 0;
    for entry in state.connection_meta.iter() {
        let (conn_session, _) = entry.value();
        if conn_session.as_str() != session_id {
            continue;
        }
        let conn_id = *entry.key();
        if exclude == Some(conn_id) {
            continue;
        }
        if let Some(tx) = state.connections.get(&conn_id) {
            if tx.try_send(json.to_owned()).is_ok() {
                count += 1;
            }
        }
    }
    count
}

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
        // ── JoinSession ───────────────────────────────────────────────────────
        ClientEvent::JoinSession(payload) => {
            let session = state
                .sessions
                .entry(payload.session_id.clone())
                .or_insert_with(|| {
                    tracing::info!(session_id = %payload.session_id, "session created");
                    Session {
                        session_id: payload.session_id.clone(),
                        objects: DashMap::new(),
                        users: DashMap::new(),
                        event_log: Vec::new(),
                    }
                });

            let user_id = Uuid::new_v4();
            session.users.insert(user_id, User {
                display_name: payload.display_name.clone(),
                color: [255, 0, 0], // palette assignment in Phase 5
                selected_object: None,
                connected_at: now_ms(),
            });

            state
                .connection_meta
                .insert(connection_id, (payload.session_id.clone(), user_id));

            tracing::info!(
                event_type = "JoinSession",
                session_id = %payload.session_id,
                user_id = %user_id,
                display_name = %payload.display_name,
                connection_id = %connection_id,
                "user joined session"
            );

            let sync_json = serde_json::to_string(&ServerEvent::FullStateSync(
                FullStateSyncPayload { session: session.clone() },
            ))
            .expect("FullStateSync serialization failed");
            socket.send(Message::Text(sync_json.into())).await.ok();

            let joined_json = serde_json::to_string(&ServerEvent::UserJoined(UserJoinedPayload {
                user_id,
                display_name: payload.display_name,
                color: [255, 0, 0],
            }))
            .expect("UserJoined serialization failed");

            let count = broadcast(state, &payload.session_id, &joined_json, Some(connection_id));
            tracing::info!(
                event_type = "UserJoined",
                session_id = %payload.session_id,
                recipient_count = count,
                "broadcast UserJoined"
            );
        }

        // ── LeaveSession ──────────────────────────────────────────────────────
        ClientEvent::LeaveSession => {
            let Some((_, (sid, uid))) = state.connection_meta.remove(&connection_id) else {
                return;
            };

            if let Some(session) = state.sessions.get(&sid) {
                session.users.remove(&uid);
            }

            tracing::info!(
                event_type = "LeaveSession",
                session_id = %sid,
                user_id = %uid,
                "user left session"
            );

            let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload {
                user_id: uid,
            }))
            .expect("UserLeft serialization failed");

            let count = broadcast(state, &sid, &left_json, Some(connection_id));
            tracing::info!(
                event_type = "UserLeft",
                session_id = %sid,
                recipient_count = count,
                "broadcast UserLeft"
            );
        }

        // ── CreateObject ──────────────────────────────────────────────────────
        ClientEvent::CreateObject(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };
            let now = now_ms();
            let Some(mut session) = state.sessions.get_mut(&sid) else {
                return;
            };

            let object = SceneObject {
                object_id: payload.object_id,
                name: payload.name.clone(),
                object_type: payload.object_type.clone(),
                asset_id: payload.asset_id.clone(),
                asset_library: payload.asset_library.clone(),
                transform: payload.transform.clone(),
                properties: payload.properties.clone(),
                created_by: uid,
                last_updated_by: uid,
                last_updated_at: now,
            };
            session.objects.insert(object.object_id, object.clone());
            session.event_log.push(LogEntry {
                timestamp: now,
                event_type: "CreateObject".to_string(),
                payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
            });
            drop(session); // release DashMap shard lock before broadcasting

            tracing::info!(
                event_type = "CreateObject",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                "object created"
            );

            let json = serde_json::to_string(&ServerEvent::ObjectCreated(ObjectCreatedPayload {
                object,
                created_by: uid,
            }))
            .expect("ObjectCreated serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "ObjectCreated",
                session_id = %sid,
                recipient_count = count,
                "broadcast ObjectCreated"
            );
        }

        // ── DeleteObject ──────────────────────────────────────────────────────
        ClientEvent::DeleteObject(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };
            let now = now_ms();
            let Some(mut session) = state.sessions.get_mut(&sid) else {
                return;
            };

            session.objects.remove(&payload.object_id);
            session.event_log.push(LogEntry {
                timestamp: now,
                event_type: "DeleteObject".to_string(),
                payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
            });
            drop(session);

            tracing::info!(
                event_type = "DeleteObject",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                "object deleted"
            );

            let json = serde_json::to_string(&ServerEvent::ObjectDeleted(ObjectDeletedPayload {
                object_id: payload.object_id,
                deleted_by: uid,
            }))
            .expect("ObjectDeleted serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "ObjectDeleted",
                session_id = %sid,
                recipient_count = count,
                "broadcast ObjectDeleted"
            );
        }

        // ── UpdateTransform ───────────────────────────────────────────────────
        ClientEvent::UpdateTransform(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };
            let now = now_ms();
            let Some(mut session) = state.sessions.get_mut(&sid) else {
                return;
            };

            if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                obj.transform = payload.transform.clone();
                obj.last_updated_by = uid;
                obj.last_updated_at = now;
            }
            session.event_log.push(LogEntry {
                timestamp: now,
                event_type: "UpdateTransform".to_string(),
                payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
            });
            drop(session);

            tracing::info!(
                event_type = "UpdateTransform",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                "transform updated"
            );

            let json = serde_json::to_string(&ServerEvent::TransformUpdated(
                TransformUpdatedPayload {
                    object_id: payload.object_id,
                    transform: payload.transform,
                    updated_by: uid,
                },
            ))
            .expect("TransformUpdated serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "TransformUpdated",
                session_id = %sid,
                recipient_count = count,
                "broadcast TransformUpdated"
            );
        }

        // ── UpdateProperties ──────────────────────────────────────────────────
        ClientEvent::UpdateProperties(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };
            let now = now_ms();
            let Some(mut session) = state.sessions.get_mut(&sid) else {
                return;
            };

            if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                obj.properties = Some(payload.properties.clone());
                obj.last_updated_by = uid;
                obj.last_updated_at = now;
            }
            session.event_log.push(LogEntry {
                timestamp: now,
                event_type: "UpdateProperties".to_string(),
                payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
            });
            drop(session);

            tracing::info!(
                event_type = "UpdateProperties",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                "properties updated"
            );

            let json = serde_json::to_string(&ServerEvent::PropertiesUpdated(
                PropertiesUpdatedPayload {
                    object_id: payload.object_id,
                    properties: payload.properties,
                    updated_by: uid,
                },
            ))
            .expect("PropertiesUpdated serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "PropertiesUpdated",
                session_id = %sid,
                recipient_count = count,
                "broadcast PropertiesUpdated"
            );
        }

        // ── UpdateName ────────────────────────────────────────────────────────
        ClientEvent::UpdateName(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };
            let now = now_ms();
            let Some(mut session) = state.sessions.get_mut(&sid) else {
                return;
            };

            if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                obj.name = payload.name.clone();
                obj.last_updated_by = uid;
                obj.last_updated_at = now;
            }
            session.event_log.push(LogEntry {
                timestamp: now,
                event_type: "UpdateName".to_string(),
                payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
            });
            drop(session);

            tracing::info!(
                event_type = "UpdateName",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                "name updated"
            );

            let json = serde_json::to_string(&ServerEvent::NameUpdated(NameUpdatedPayload {
                object_id: payload.object_id,
                name: payload.name,
                updated_by: uid,
            }))
            .expect("NameUpdated serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "NameUpdated",
                session_id = %sid,
                recipient_count = count,
                "broadcast NameUpdated"
            );
        }

        // ── SelectObject ──────────────────────────────────────────────────────
        ClientEvent::SelectObject(payload) => {
            let Some((sid, uid)) = state.connection_meta.get(&connection_id).map(|r| r.value().clone()) else {
                return;
            };

            if let Some(session) = state.sessions.get(&sid) {
                if let Some(mut user) = session.users.get_mut(&uid) {
                    user.selected_object = payload.object_id;
                }
            }

            tracing::info!(
                event_type = "SelectObject",
                session_id = %sid,
                user_id = %uid,
                object_id = ?payload.object_id,
                "selection updated"
            );

            let json = serde_json::to_string(&ServerEvent::UserSelected(UserSelectedPayload {
                user_id: uid,
                object_id: payload.object_id,
            }))
            .expect("UserSelected serialization failed");

            let count = broadcast(state, &sid, &json, None);
            tracing::info!(
                event_type = "UserSelected",
                session_id = %sid,
                recipient_count = count,
                "broadcast UserSelected"
            );
        }
    }
}
