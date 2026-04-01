use std::sync::Arc;
use uuid::Uuid;

use crate::{
    messages::{ServerEvent, TransformUpdatedPayload, UpdateTransformPayload},
    types::AppState,
};

use super::helpers::{broadcast, now_ms};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: UpdateTransformPayload) {
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
        let Some(obj) = objects.get_mut(&payload.object_id) else {
            tracing::debug!(
                object_id = %payload.object_id,
                session_id = %sid,
                "object not found for transform update"
            );
            return;
        };
        obj.transform = payload.transform.clone();
        obj.last_updated_by = uid;
        obj.last_updated_at = now;
    }

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
