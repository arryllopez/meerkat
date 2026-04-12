use std::sync::Arc;
use uuid::Uuid;

use crate::{
    messages::{DeleteObjectPayload, ObjectDeletedPayload, ServerEvent},
    types::AppState,
};

use super::helpers::broadcast;

pub async fn handle(state: &AppState, connection_id: Uuid, payload: DeleteObjectPayload) {
    let Some((sid, uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };
    let session = match state.sessions.get(&sid) {
        Some(s) => Arc::clone(s.value()),
        None => return,
    };

    {
        let mut objects = match session.objects.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session objects lock poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if objects.remove(&payload.object_id).is_none() {
            tracing::debug!(
                object_id = %payload.object_id,
                session_id = %sid,
                "object not found for deletion"
            );
            return;
        }
    }

    tracing::info!(
        event_type = "DeleteObject",
        session_id = %sid,
        user_id = %uid,
        object_id = %payload.object_id,
        "object deleted"
    );

    let json = match serde_json::to_string(&ServerEvent::ObjectDeleted(ObjectDeletedPayload {
        object_id: payload.object_id,
        deleted_by: uid,
    })) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(
                event_type = "ObjectDeleted",
                session_id = %sid,
                user_id = %uid,
                object_id = %payload.object_id,
                error = %err,
                "failed to serialize ObjectDeleted event"
            );
            return;
        }
    };

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "ObjectDeleted",
        session_id = %sid,
        recipient_count = count,
        "broadcast ObjectDeleted"
    );
}
