use axum::extract::ws::{Message, WebSocket};
use uuid::Uuid;

use crate::{
    messages::{FullStateSyncPayload, ServerEvent},
    types::AppState,
};

pub async fn handle(socket: &mut WebSocket, state: &AppState, connection_id: Uuid) {
    let Some((sid, uid)) = state
        .connection_meta
        .get(&connection_id)
        .map(|r| r.value().clone())
    else {
        return;
    };

    if let Some(session) = state.sessions.get(&sid) {
        let sync_json = match serde_json::to_string(&ServerEvent::FullStateSync(FullStateSyncPayload {
            session: session.session_snapshot(),
            your_user_id: uid,
        })) {
            Ok(json) => json,
            Err(err) => {
                tracing::error!(
                    event_type = "RequestStateSync",
                    session_id = %sid,
                    connection_id = %connection_id,
                    error = %err,
                    "failed to serialize FullStateSync"
                );
                return;
            }
        };

        if let Err(err) = socket.send(Message::Text(sync_json.into())).await {
            tracing::warn!(
                event_type = "RequestStateSync",
                session_id = %sid,
                connection_id = %connection_id,
                error = %err,
                "failed to send FullStateSync to requesting client"
            );
        }

        tracing::info!(
            event_type = "RequestStateSync",
            session_id = %sid,
            "sent FullStateSync to requesting client"
        );
    }
}
