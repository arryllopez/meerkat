use uuid::Uuid;

use crate::{
    messages::{CursorPayload, ServerEvent, UpdatedCursor},
    types::AppState,
};

use super::helpers::broadcast;

pub async fn handle(state: &AppState, connection_id: Uuid, payload: CursorPayload) {
    let Some((sid, uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };

    let json = match serde_json::to_string(&ServerEvent::CursorUpdated(UpdatedCursor {
        position: payload.position,
        user_id: uid,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
                event_type = "CursorUpdated",
                session_id = %sid,
                user_id = %uid,
                error = %err,
                "failed to serialize CursorUpdated event"
            );
            return;
        }
    };

    broadcast(state, &sid, &json, Some(connection_id));
}
