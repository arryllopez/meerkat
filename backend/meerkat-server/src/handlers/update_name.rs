use std::sync::Arc;
use uuid::Uuid;

use crate::{
    messages::{NameUpdatedPayload, ServerEvent, UpdateNamePayload},
    types::AppState,
};

use super::helpers::{broadcast, now_ms};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: UpdateNamePayload) {
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
        if let Some(obj) = objects.get_mut(&payload.object_id) {
            obj.name = payload.name.clone();
            obj.last_updated_by = uid;
            obj.last_updated_at = now;
        } else {
            tracing::debug!(
                object_id = %payload.object_id,
                session_id = %sid,
                "object not found for name update"
            );
            return;
        }
    }

    tracing::info!(
        event_type = "UpdateName",
        session_id = %sid,
        user_id = %uid,
        object_id = %payload.object_id,
        "name updated"
    );

    let json = serde_json::to_string(&ServerEvent::NameUpdated(NameUpdatedPayload {
        object_id: payload.object_id,
        name: payload.name,
        updated_by: uid,
    }))
    .expect("NameUpdated serialization failed");

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "NameUpdated",
        session_id = %sid,
        recipient_count = count,
        "broadcast NameUpdated"
    );
}
