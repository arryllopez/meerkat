use uuid::Uuid;

use crate::{
    messages::{ServerEvent, TransformUpdatedPayload, UpdateTransformPayload},
    types::{AppState, LogEntry},
};

use super::helpers::{broadcast, now_ms, write_log};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: UpdateTransformPayload) {
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

    if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
        obj.transform = payload.transform.clone();
        obj.last_updated_by = uid;
        obj.last_updated_at = now;
    }
    let log_entry = LogEntry {
        timestamp: now,
        event_type: "UpdateTransform".to_string(),
        payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
    };
    session.event_log.push(log_entry.clone());
    drop(session);
    write_log(state, &sid, &log_entry);

    tracing::info!(
        event_type = "UpdateTransform",
        session_id = %sid,
        user_id = %uid,
        object_id = %payload.object_id,
        "transform updated"
    );

    let json = serde_json::to_string(&ServerEvent::TransformUpdated(TransformUpdatedPayload {
        object_id: payload.object_id,
        transform: payload.transform,
        updated_by: uid,
    }))
    .expect("TransformUpdated serialization failed");

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "TransformUpdated",
        session_id = %sid,
        recipient_count = count,
        "broadcast TransformUpdated"
    );
}
