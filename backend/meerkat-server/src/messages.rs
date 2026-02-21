use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::types::{Transform, ObjectType, ObjectProperties, SceneObject, Session};

// ── Envelope ──────────────────────────────────────────────────────────────────

/// Every message on the wire is wrapped in this envelope.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageEnvelope {
    pub event_type: String,
    pub timestamp: u64,
    pub source_user_id: Uuid,
    pub payload: serde_json::Value,
}

// ── Client → Server payloads ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoinSessionPayload {
    pub session_id: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateObjectPayload {
    pub object_id: Uuid,
    pub name: String,
    pub object_type: ObjectType,
    pub asset_id: Option<String>,
    pub asset_library: Option<String>,
    pub transform: Transform,
    pub properties: Option<ObjectProperties>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteObjectPayload {
    pub object_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateTransformPayload {
    pub object_id: Uuid,
    pub transform: Transform,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdatePropertiesPayload {
    pub object_id: Uuid,
    pub properties: ObjectProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateNamePayload {
    pub object_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SelectObjectPayload {
    pub object_id: Option<Uuid>, // None means deselect
}

// ── Client event enum ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event_type", content = "payload")]
pub enum ClientEvent {
    JoinSession(JoinSessionPayload),
    LeaveSession,
    CreateObject(CreateObjectPayload),
    DeleteObject(DeleteObjectPayload),
    UpdateTransform(UpdateTransformPayload),
    UpdateProperties(UpdatePropertiesPayload),
    UpdateName(UpdateNamePayload),
    SelectObject(SelectObjectPayload),
}

// ── Server → Client payloads ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullStateSyncPayload {
    pub session: Session,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectCreatedPayload {
    pub object: SceneObject,
    pub created_by: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectDeletedPayload {
    pub object_id: Uuid,
    pub deleted_by: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransformUpdatedPayload {
    pub object_id: Uuid,
    pub transform: Transform,
    pub updated_by: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PropertiesUpdatedPayload {
    pub object_id: Uuid,
    pub properties: ObjectProperties,
    pub updated_by: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NameUpdatedPayload {
    pub object_id: Uuid,
    pub name: String,
    pub updated_by: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserJoinedPayload {
    pub user_id: Uuid,
    pub display_name: String,
    pub color: [u8; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserLeftPayload {
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserSelectedPayload {
    pub user_id: Uuid,
    pub object_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

// ── Server event enum ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event_type", content = "payload")]
pub enum ServerEvent {
    FullStateSync(FullStateSyncPayload),
    ObjectCreated(ObjectCreatedPayload),
    ObjectDeleted(ObjectDeletedPayload),
    TransformUpdated(TransformUpdatedPayload),
    PropertiesUpdated(PropertiesUpdatedPayload),
    NameUpdated(NameUpdatedPayload),
    UserJoined(UserJoinedPayload),
    UserLeft(UserLeftPayload),
    UserSelected(UserSelectedPayload),
    Error(ErrorPayload),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Deserializes a raw JSON string into a ClientEvent.
pub fn parse_client_message(raw: &str) -> Result<ClientEvent, serde_json::Error> {
    serde_json::from_str(raw)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Transform, ObjectType};
    use uuid::Uuid;

    fn round_trip_client(event: &ClientEvent) {
        let json = serde_json::to_string(event).expect("serialize failed");
        let back: ClientEvent = serde_json::from_str(&json).expect("deserialize failed");
        // Compare via re-serialized JSON (enums don't impl PartialEq by default)
        let json2 = serde_json::to_string(&back).expect("re-serialize failed");
        assert_eq!(json, json2);
    }

    fn round_trip_server(event: &ServerEvent) {
        let json = serde_json::to_string(event).expect("serialize failed");
        let back: ServerEvent = serde_json::from_str(&json).expect("deserialize failed");
        let json2 = serde_json::to_string(&back).expect("re-serialize failed");
        assert_eq!(json, json2);
    }

    fn dummy_transform() -> Transform {
        Transform {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    // ── Client events ──────────────────────────────────────────────────────

    #[test]
    fn test_join_session() {
        round_trip_client(&ClientEvent::JoinSession(JoinSessionPayload {
            session_id: "shot-01".to_string(),
            display_name: "Alice".to_string(),
        }));
    }

    #[test]
    fn test_leave_session() {
        round_trip_client(&ClientEvent::LeaveSession);
    }

    #[test]
    fn test_create_object() {
        round_trip_client(&ClientEvent::CreateObject(CreateObjectPayload {
            object_id: Uuid::new_v4(),
            name: "Cube".to_string(),
            object_type: ObjectType::Cube,
            asset_id: None,
            asset_library: None,
            transform: dummy_transform(),
            properties: None,
        }));
    }

    #[test]
    fn test_delete_object() {
        round_trip_client(&ClientEvent::DeleteObject(DeleteObjectPayload {
            object_id: Uuid::new_v4(),
        }));
    }

    #[test]
    fn test_update_transform() {
        round_trip_client(&ClientEvent::UpdateTransform(UpdateTransformPayload {
            object_id: Uuid::new_v4(),
            transform: dummy_transform(),
        }));
    }

    #[test]
    fn test_update_name() {
        round_trip_client(&ClientEvent::UpdateName(UpdateNamePayload {
            object_id: Uuid::new_v4(),
            name: "hero_chair".to_string(),
        }));
    }

    #[test]
    fn test_select_object() {
        round_trip_client(&ClientEvent::SelectObject(SelectObjectPayload {
            object_id: Some(Uuid::new_v4()),
        }));
    }

    #[test]
    fn test_deselect_object() {
        round_trip_client(&ClientEvent::SelectObject(SelectObjectPayload {
            object_id: None,
        }));
    }

    // ── Server events ──────────────────────────────────────────────────────

    #[test]
    fn test_object_deleted_server() {
        round_trip_server(&ServerEvent::ObjectDeleted(ObjectDeletedPayload {
            object_id: Uuid::new_v4(),
            deleted_by: Uuid::new_v4(),
        }));
    }

    #[test]
    fn test_transform_updated_server() {
        round_trip_server(&ServerEvent::TransformUpdated(TransformUpdatedPayload {
            object_id: Uuid::new_v4(),
            transform: dummy_transform(),
            updated_by: Uuid::new_v4(),
        }));
    }

    #[test]
    fn test_user_joined_server() {
        round_trip_server(&ServerEvent::UserJoined(UserJoinedPayload {
            user_id: Uuid::new_v4(),
            display_name: "Bob".to_string(),
            color: [255, 100, 0],
        }));
    }

    #[test]
    fn test_user_left_server() {
        round_trip_server(&ServerEvent::UserLeft(UserLeftPayload {
            user_id: Uuid::new_v4(),
        }));
    }

    #[test]
    fn test_user_selected_server() {
        round_trip_server(&ServerEvent::UserSelected(UserSelectedPayload {
            user_id: Uuid::new_v4(),
            object_id: Some(Uuid::new_v4()),
        }));
    }

    #[test]
    fn test_error_server() {
        round_trip_server(&ServerEvent::Error(ErrorPayload {
            code: "SESSION_FULL".to_string(),
            message: "Session has reached max users".to_string(),
        }));
    }
}
