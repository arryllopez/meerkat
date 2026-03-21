use uuid::Uuid;

use crate::{
    messages::{ServerEvent, UserLeftPayload},
    types::AppState,
};

use super::helpers::broadcast;

pub async fn handle(state: &AppState, connection_id: Uuid) {
    let Some((_, (sid, uid))) = state.connection_meta.remove(&connection_id) else {
        return;
    };

    if let Some(session) = state.sessions.get(&sid) {
        session.users.remove(&uid);
    }

    tracing::info!(
        event_type = "LeaveSession",
        session_id = %sid,
        user_id = %uid,
        "user left session"
    );

    let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload { user_id: uid }))
        .expect("UserLeft serialization failed");

    let count = broadcast(state, &sid, &left_json, Some(connection_id));
    tracing::info!(
        event_type = "UserLeft",
        session_id = %sid,
        recipient_count = count,
        "broadcast UserLeft"
    );
}
