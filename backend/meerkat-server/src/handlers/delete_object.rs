use std::sync::Arc;
use uuid::Uuid;

use crate::{
    messages::{DeleteObjectPayload, ObjectDeletedPayload, ServerEvent},
    types::{AppState, LogEntry},
};

use super::helpers::{broadcast, now_ms, write_log};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: DeleteObjectPayload) {
    let Some((sid, uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };
    let now = now_ms();
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

    let log_entry = LogEntry {
        timestamp: now,
        event_type: "DeleteObject".to_string(),
        payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
    };
    {
        let mut event_log = match session.event_log.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session event_log lock poisoned, recovering");
                poisoned.into_inner()
            }
        };
        event_log.push(log_entry.clone());
    }
    write_log(state, &sid, &log_entry);

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
