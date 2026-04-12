use axum::extract::ws::{Message, WebSocket};
use std::sync::{Arc, RwLock};
use crate::types::SessionHandle;
use std::collections::{HashMap};
use uuid::Uuid;

use crate::{
    messages::{FullStateSyncPayload, JoinSessionPayload, ServerEvent, UserJoinedPayload, UserLeftPayload},
    types::{AppState, User, COLOR_PALETTE},
};

use super::helpers::{broadcast, now_ms};

// join_session handler is responsible for:
// First cleaning up if this connection_id is already tracked (which can happen if a client reconnects without properly closing the previous connection, or if there are duplicate connection_ids for some reason). This involves removing the old connection's membership from the session and broadcasting a UserLeft event for the old session.
// 1) Adding the user to the session's user list (creating the session if it doesn't exist).
// 2) Sending the current session state to the joining user for synchronization.
//   - This is done by acquiring read locks on the SessionHandle's internal state, cloning the data into a new Session struct, and sending it as a FullStateSync event.
// 3) Broadcasting a UserJoined event to all other users in the session.

pub async fn handle(socket :&mut WebSocket, state: &AppState, connection_id: Uuid, payload: JoinSessionPayload) {
    // Re-join safety: if this connection was already tracked, clean old membership first.
    if let Some((_, (old_sid, old_uid))) = state.connection_meta.remove(&connection_id) {
        tracing::warn!(
            connection_id = %connection_id,
            old_session_id = %old_sid,
            old_user_id = %old_uid,
        "connection re-joining while still tracked; cleaning stale membership"
    );

    // Remove connection from old session->connections index
    let mut remove_old_session_entry = false;
    if let Some(mut conns) = state.session_connections.get_mut(&old_sid) {
        conns.remove(&connection_id);
        remove_old_session_entry = conns.is_empty();
    }
    if remove_old_session_entry {
        state.session_connections.remove(&old_sid);
    }

    // Remove stale user presence from old session users map
    let mut reclaim_old_session = false;
    if let Some(old_session) = state.sessions.get(&old_sid) {
        let mut users = match old_session.users.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Old session users lock poisoned during re-join cleanup, recovering");
                poisoned.into_inner()
            }
        };
        users.remove(&old_uid);
        reclaim_old_session = users.is_empty() && old_sid != payload.session_id;
    }

    if reclaim_old_session {
        state.sessions.remove(&old_sid);
        tracing::info!(
            event_type = "SessionReclaimed",
            session_id = %old_sid,
            "reclaimed empty stale session during re-join cleanup"
        );
    }

    // Broadcast UserLeft for old session 
    let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload { user_id: old_uid }))
        .expect("UserLeft serialization failed during re-join cleanup");
    let count = broadcast(state, &old_sid, &left_json, Some(connection_id));
    tracing::info!(
        connection_id = %connection_id,
        old_session_id = %old_sid,
        old_user_id = %old_uid,
        recipient_count = count,
        "broadcast UserLeft for stale session during re-join cleanup", 
    );
    tracing::warn!("user has left session due to re-joining while still tracked; if this happens frequently, consider investigating client connection stability or adding more aggressive backpressure eviction")
    }

    let session = state
        .sessions
        .entry(payload.session_id.clone())
        .or_insert_with(|| {
            tracing::info!(session_id = %payload.session_id, "session created");
            // Initialize a new SessionHandle with empty state and wrap it in an Arc for shared ownership.
            Arc::new(SessionHandle {
                objects: RwLock::new(HashMap::new()),
                users: RwLock::new(HashMap::new()),
                session_id: payload.session_id.clone(),
            })
        });

    let user_id = Uuid::new_v4();
    let color = {
        let mut users = match session.users.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session users lock poisoned (write), recovering");
                poisoned.into_inner()
            }
        };
        let color = COLOR_PALETTE[users.len() % COLOR_PALETTE.len()];
        users.insert(
            user_id,
            User {
                display_name: payload.display_name.clone(),
                color,
                selected_object: None,
                connected_at: now_ms(),
            },
        );
        color
    };

    state
        .connection_meta
        .insert(connection_id, (payload.session_id.clone(), user_id));
    
    state
        .session_connections
        .entry(payload.session_id.clone())
        .or_default()
        .insert(connection_id);

    tracing::info!(
        event_type = "JoinSession",
        session_id = %payload.session_id,
        user_id = %user_id,
        display_name = %payload.display_name,
        connection_id = %connection_id,
        "user joined session"
    );

    let sync_json = serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
        session: (session.session_snapshot()),
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
