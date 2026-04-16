use axum::extract::ws::{Message, WebSocket};
use uuid::Uuid;

use crate::{
    messages::{ErrorPayload, FullStateSyncPayload, JoinSessionPayload, ServerEvent, UserJoinedPayload},
    types::AppState,
};

use super::helpers::{add_user_to_session, broadcast, cleanup_stale_membership};

// join_session handler is responsible for:
// 1) Looking up existing session by ID, rejecting if not found.
// 2) Verifying the password against the stored bcrypt hash, rejecting if wrong.
// 3) Cleaning up stale membership if this connection was already tracked.
// 4) Adding the user to the session's user list.
// 5) Sending FullStateSync to the joining user.
// 6) Broadcasting UserJoined to all other users in the session.

pub async fn handle(socket :&mut WebSocket, state: &AppState, connection_id: Uuid, payload: JoinSessionPayload) {
    // Re-join safety: if this connection was already tracked, clean old membership first.
    cleanup_stale_membership(state, connection_id, &payload.session_id);

    let session = match state.sessions.get(&payload.session_id) {
        Some(s) => s,
        None => {
            let err_json = serde_json::to_string(&ServerEvent::Error(ErrorPayload {
                code: "SessionNotFound".to_string(),
                message: format!("Session with id '{}' not found", payload.session_id),
            }));
            if let Ok(json) = err_json {
                let _ = socket.send(Message::Text(json.into())).await;
            }
            return;
        }
    };

    let password_valid = match bcrypt::verify(&payload.password, &session.password_hash) {
        Ok(valid) => valid,
        Err(err) => {
            tracing::error!(error = %err, "bcrypt verify failed");
            false
        }
    };
    if !password_valid {
        let err_json = serde_json::to_string(&ServerEvent::Error(ErrorPayload {
            code: "WRONG_PASSWORD".to_string(),
            message: "Invalid password".to_string(),
        }));
        if let Ok(json) = err_json {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        return;
    }

    let (user_id, color) = add_user_to_session(
        state, &session, connection_id, &payload.session_id, &payload.display_name,
    );

    let sync_json = match serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
        session: (session.session_snapshot()),
        your_user_id: user_id,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
                event_type = "FullStateSync",
                session_id = %payload.session_id,
                user_id = %user_id,
                connection_id = %connection_id,
                error = %err,
                "failed to serialize FullStateSync"
            );
            return;
        }
    };
    if let Err(err) = socket.send(Message::Text(sync_json.into())).await {
        tracing::warn!(
            event_type = "FullStateSync",
            session_id = %payload.session_id,
            user_id = %user_id,
            connection_id = %connection_id,
            error = %err,
            "failed to send FullStateSync to joining client"
        );
    }

    let joined_json = match serde_json::to_string(&ServerEvent::UserJoined(UserJoinedPayload {
        user_id,
        display_name: payload.display_name,
        color,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
                event_type = "UserJoined",
                session_id = %payload.session_id,
                user_id = %user_id,
                connection_id = %connection_id,
                error = %err,
                "failed to serialize UserJoined"
            );
            return;
        }
    };

    let count = broadcast(state, &payload.session_id, &joined_json, Some(connection_id));
    tracing::info!(
        event_type = "UserJoined",
        session_id = %payload.session_id,
        recipient_count = count,
        "broadcast UserJoined"
    );
}

