use std::sync::Arc;
use uuid::Uuid;
use std::collections::hash_map::Entry;

use crate::{
    messages::{CreateObjectPayload, ErrorPayload, ObjectCreatedPayload, ServerEvent},
    types::{AppState, SceneObject},
};

use super::helpers::{broadcast, now_ms};

pub async fn handle(state: &AppState, connection_id: Uuid, payload: CreateObjectPayload) {
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

    let object = SceneObject {
        object_id: payload.object_id,
        name: payload.name.clone(),
        object_type: payload.object_type.clone(),
        asset_id: payload.asset_id.clone(),
        asset_library: payload.asset_library.clone(),
        transform: payload.transform.clone(),
        properties: payload.properties.clone(),
        created_by: uid,
        last_updated_by: uid,
        last_updated_at: now,
    };

    let inserted: bool = {
        let mut objects = match session.objects.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session objects lock poisoned, recovering");
                poisoned.into_inner()
            }
        };
        match objects.entry(object.object_id) {
            Entry::Vacant(v) => {
                v.insert(object.clone());
                true
            }
            Entry::Occupied(_) => false,
        }
    };

    if !inserted {
        tracing::warn!(
            event_type = "CreateObject",
            session_id = %sid,
            user_id = %uid,
            object_id = %payload.object_id,
            "duplicate object_id on create; ignoring"
        );

        if let Some(tx) = state.connections.get(&connection_id) {
            let error_json = serde_json::to_string(&ServerEvent::Error(ErrorPayload {
                code: "DUPLICATE_OBJECT_ID".to_string(),
                message: format!(
                    "CreateObject rejected: object_id {} already exists in session {}",
                    payload.object_id, sid
                ),
            }))
            .expect("Error serialization failed");

            if tx.try_send(error_json).is_err() {
                tracing::warn!(
                    event_type = "CreateObject",
                    session_id = %sid,
                    user_id = %uid,
                    object_id = %payload.object_id,
                    "failed to deliver duplicate-object error to sender"
                );
            }
        }

        return;
    }

    tracing::info!(
        event_type = "CreateObject",
        session_id = %sid,
        user_id = %uid,
        object_id = %payload.object_id,
        "object created"
    );

    let json = serde_json::to_string(&ServerEvent::ObjectCreated(ObjectCreatedPayload {
        object,
        created_by: uid,
    }))
    .expect("ObjectCreated serialization failed");

    let count = broadcast(state, &sid, &json, None);
    tracing::info!(
        event_type = "ObjectCreated",
        session_id = %sid,
        recipient_count = count,
        "broadcast ObjectCreated"
    );
}
