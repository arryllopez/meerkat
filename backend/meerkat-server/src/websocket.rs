use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use uuid::Uuid;
use tokio::sync::mpsc;
use axum::extract::ws::Message;

use crate::{
    messages::{
        ClientEvent, FullStateSyncPayload, ServerEvent, UserJoinedPayload, parse_client_message,
    },
    types::{AppState, Session, User},
};

use dashmap::DashMap;

pub async fn handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

pub async fn handle_connection(mut socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4();

    let (tx, mut rx) = mpsc::channel::<String>(32);
    state.connections.insert(connection_id, tx);

    let mut user_id: Option<Uuid> = None;
    let mut session_id: Option<String> = None;

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                let msg = match msg {
                    Ok(Message::Text(text)) => text.to_string(),
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => continue,
                };

                let parsed = parse_client_message(&msg);
                match parsed {
                    Ok(event) => {
                        match event {
                            ClientEvent::JoinSession(payload) => {
                                let session = state.sessions
                                    .entry(payload.session_id.clone())
                                    .or_insert_with(|| Session {
                                        session_id: payload.session_id.clone(),
                                        objects: DashMap::new(),
                                        users: DashMap::new(),
                                        event_log: Vec::new(),
                                    });

                                let new_user_id = Uuid::new_v4();
                                session.users.insert(new_user_id, User {
                                    display_name: payload.display_name.clone(),
                                    color: [255, 0, 0],
                                    selected_object: None,
                                    connected_at: 0,
                                });

                                user_id = Some(new_user_id);
                                session_id = Some(payload.session_id.clone());

                                // send full state to joining client
                                let sync = ServerEvent::FullStateSync(FullStateSyncPayload {
                                    session: session.clone(),
                                });
                                let json = serde_json::to_string(&sync).unwrap();
                                socket.send(Message::Text(json.into())).await.ok();

                                // broadcast UserJoined to everyone else
                                let joined_json = serde_json::to_string(&ServerEvent::UserJoined(UserJoinedPayload {
                                    user_id: new_user_id,
                                    display_name: payload.display_name.clone(),
                                    color: [255, 0, 0],
                                })).unwrap();

                                for entry in state.connections.iter() {
                                    if *entry.key() == connection_id { continue; }
                                    let _ = entry.value().try_send(joined_json.clone());
                                }
                            }
                            ClientEvent::LeaveSession              => {}
                            ClientEvent::CreateObject(payload)     => {}
                            ClientEvent::DeleteObject(payload)     => {}
                            ClientEvent::UpdateTransform(payload)  => {}
                            ClientEvent::UpdateProperties(payload) => {}
                            ClientEvent::UpdateName(payload)       => {}
                            ClientEvent::SelectObject(payload)     => {}
                        }
                    }
                    Err(_) => { continue; }
                }
            }
            Some(text) = rx.recv() => {
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    // cleanup on disconnect
    state.connections.remove(&connection_id);
    // TODO: if session_id and user_id are Some, run leave_session + broadcast USER_LEFT
}
