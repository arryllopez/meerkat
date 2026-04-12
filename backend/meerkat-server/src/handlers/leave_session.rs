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

    let mut remove_session_entry = false;
    if let Some(mut conns) = state.session_connections.get_mut(&sid) {
        conns.remove(&connection_id);
        remove_session_entry = conns.is_empty();
    }
    if remove_session_entry {
        state.session_connections.remove(&sid);
    }

    let mut reclaim_session = false;
    if let Some(session) = state.sessions.get(&sid) {
        let mut users = match session.users.write(){ 
            Ok(guard) => guard, 
            Err(poisoned) => { 
                tracing::warn!("Session users lock poisoned, recovering with potentially inconsistent data."); 
                poisoned.into_inner()
            }
        };
        users.remove(&uid);
        reclaim_session = users.is_empty();
    }

    // reclaim session in memory if it is empty 
    if reclaim_session {
        state.sessions.remove(&sid);
        tracing::info!(
            event_type = "SessionReclaimed",
            session_id = %sid,
            "reclaimed empty session after leave"
        );
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
