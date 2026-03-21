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

    let json = serde_json::to_string(&ServerEvent::CursorUpdated(UpdatedCursor {
        position: payload.position,
        user_id: uid,
    }))
    .expect("CursorUpdated serialization failed");

    broadcast(state, &sid, &json, Some(connection_id));
}
