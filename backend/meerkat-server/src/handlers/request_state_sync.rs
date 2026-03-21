use axum::extract::ws::{Message, WebSocket};
use uuid::Uuid;

use crate::{
    messages::{FullStateSyncPayload, ServerEvent},
    types::AppState,
};

pub async fn handle(socket: &mut WebSocket, state: &AppState, connection_id: Uuid) {
    let Some((sid, _uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };

    if let Some(session) = state.sessions.get(&sid) {
        let sync_json = serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
            session: session.clone(),
        }))
        .expect("FullStateSync serialization failed");
        socket.send(Message::Text(sync_json.into())).await.ok();

        tracing::info!(
            event_type = "RequestStateSync",
            session_id = %sid,
            "sent FullStateSync to requesting client"
        );
    }
}
