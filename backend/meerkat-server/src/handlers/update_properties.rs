use uuid::Uuid;

use crate::{
    messages::{PropertiesUpdatedPayload, ServerEvent, UpdatePropertiesPayload},
    types::{AppState, LogEntry},
};

use super::helpers::{broadcast, now_ms, write_log};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: UpdatePropertiesPayload) {
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
        obj.properties = Some(payload.properties.clone());
        obj.last_updated_by = uid;
        obj.last_updated_at = now;
    }
    let log_entry = LogEntry {
        timestamp: now,
        event_type: "UpdateProperties".to_string(),
        payload: serde_json::to_value(&payload).expect("LogEntry serialization failed"),
    };
    session.event_log.push(log_entry.clone());
    drop(session);
    write_log(state, &sid, &log_entry);

    tracing::info!(
        event_type = "UpdateProperties",
        session_id = %sid,
        user_id = %uid,
        object_id = %payload.object_id,
        "properties updated"
    );

    let json = serde_json::to_string(&ServerEvent::PropertiesUpdated(PropertiesUpdatedPayload {
        object_id: payload.object_id,
        properties: payload.properties,
        updated_by: uid,
    }))
    .expect("PropertiesUpdated serialization failed");

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "PropertiesUpdated",
        session_id = %sid,
        recipient_count = count,
        "broadcast PropertiesUpdated"
    );
}
