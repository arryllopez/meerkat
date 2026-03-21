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
