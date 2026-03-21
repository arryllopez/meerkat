use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    event_log::open_log_file,
    messages::{FullStateSyncPayload, JoinSessionPayload, ServerEvent, UserJoinedPayload},
    types::{AppState, Session, User, COLOR_PALETTE},
};

use super::helpers::{broadcast, now_ms};

pub async fn handle(
    socket: &mut WebSocket,
    state: &AppState,
    connection_id: Uuid,
    payload: JoinSessionPayload,
) {
    let session = state
        .sessions
        .entry(payload.session_id.clone())
        .or_insert_with(|| {
            tracing::info!(session_id = %payload.session_id, "session created");
            state
                .log_files
                .entry(payload.session_id.clone())
                .or_insert_with(|| open_log_file(&payload.session_id));
            Session {
                session_id: payload.session_id.clone(),
                objects: DashMap::new(),
                users: DashMap::new(),
                event_log: Vec::new(),
            }
        });

    let user_id = Uuid::new_v4();
    let color = COLOR_PALETTE[session.users.len() % COLOR_PALETTE.len()];

    session.users.insert(
        user_id,
        User {
            display_name: payload.display_name.clone(),
            color,
            selected_object: None,
            connected_at: now_ms(),
        },
    );

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

    let sync_json = serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
        session: session.clone(),
    }))
    .expect("FullStateSync serialization failed");
    socket.send(Message::Text(sync_json.into())).await.ok();

    let joined_json = serde_json::to_string(&ServerEvent::UserJoined(UserJoinedPayload {
        user_id,
        display_name: payload.display_name,
        color,
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
