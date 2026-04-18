use axum::extract::ws::{Message, WebSocket};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};
use uuid::Uuid;

use crate::{
    messages::{CreateSessionPayload, FullStateSyncPayload, ServerEvent, ErrorPayload,},
    types::{AppState, SessionHandle},
};
use super::helpers::{cleanup_stale_membership, add_user_to_session};

pub async fn handle (socket: &mut WebSocket, state: &AppState, connection_id: Uuid, payload: CreateSessionPayload) {
    if state.sessions.contains_key(&payload.session_id) {
        let err_json = serde_json::to_string(&ServerEvent::Error(ErrorPayload {
            code: "SESSION_ALREADY_EXISTS".to_string(),
            message: format!("Session with id '{}' already exists", payload.session_id),
        }));
        if let Ok(json) = err_json {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        return;
    }

    let hashed = match bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(err) => {
            tracing::error!(error=%err, "failed to hash password");
            let err_json = serde_json::to_string(&ServerEvent::Error(ErrorPayload {
                code: "InternalError".to_string(),
                message: "Failed to create session due to internal error".to_string(),
            }));
            if let Ok(json) = err_json {
                let _ = socket.send(Message::Text(json.into())).await;
            }
            return;
        }
    };

    cleanup_stale_membership(state, connection_id, &payload.session_id);

    let session_handle = Arc::new(SessionHandle {
        objects: RwLock::new(HashMap::new()),
        users: RwLock::new(HashMap::new()),
        session_id: payload.session_id.clone(),
        password_hash: hashed,
    });

    state.sessions.insert(payload.session_id.clone(), session_handle.clone());

    let (user_id, _) = add_user_to_session(
        state, &session_handle, connection_id, &payload.session_id, &payload.display_name,
    );

    let sync_json = match serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
        session: session_handle.session_snapshot(),
        your_user_id: user_id,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
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
            session_id = %payload.session_id,
            user_id = %user_id,
            connection_id = %connection_id,
            error = %err,
            "failed to send FullStateSync to session creator"
        );
    }
}


