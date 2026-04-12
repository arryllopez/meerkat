use uuid::Uuid;

use crate::{
    messages::{SelectObjectPayload, ServerEvent, UserSelectedPayload},
    types::AppState,
};

use super::helpers::broadcast;

pub async fn handle(state: &AppState, connection_id: Uuid, payload: SelectObjectPayload) {
    let Some((sid, uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };

    let updated = if let Some(session) = state.sessions.get(&sid) {
        let mut users = match session.users.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session users lock poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if let Some(user) = users.get_mut(&uid) {
            user.selected_object = payload.object_id;
            true
        } else {
            false
        }
    } else {
        false
    };

    if !updated {
        tracing::warn!(
            session_id = %sid,
            user_id = %uid,
            "failed to update selection: session or user not found"
        );
        return;
    }

    tracing::info!(
        event_type = "SelectObject",
        session_id = %sid,
        user_id = %uid,
        object_id = ?payload.object_id,
        "selection updated"
    );

    let json = match serde_json::to_string(&ServerEvent::UserSelected(UserSelectedPayload {
        user_id: uid,
        object_id: payload.object_id,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
                event_type = "UserSelected",
                session_id = %sid,
                user_id = %uid,
                object_id = ?payload.object_id,
                error = %err,
                "failed to serialize UserSelected event"
            );
            return;
        }
    };

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "UserSelected",
        session_id = %sid,
        recipient_count = count,
        "broadcast UserSelected"
    );
}
