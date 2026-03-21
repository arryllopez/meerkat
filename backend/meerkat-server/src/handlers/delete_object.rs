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
    let Some(mut session) = state.sessions.get_mut(&sid) else {
        return;
    };

    session.objects.remove(&payload.object_id);
    let log_entry = LogEntry {
        timestamp: now,
        event_type: "DeleteObject".to_string(),
        payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
    };
    session.event_log.push(log_entry.clone());
    drop(session);
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
