use axum::{ 
    extract::{ws::{WebSocketUpgrade, WebSocket}, State},
    response::Response,
};
use uuid::Uuid;

use tokio::sync::mpsc;

use axum::extract::ws::Message;

use crate::{
    messages::{parse_client_message, ClientEvent},
    types::AppState,
};

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
                    Ok(event) => 
                    {
                        match event 
                        { 
                            ClientEvent::JoinSession(payload)      => {}
                            ClientEvent::LeaveSession              => {}
                            ClientEvent::CreateObject(payload)     => {}
                            ClientEvent::DeleteObject(payload)     => {}
                            ClientEvent::UpdateTransform(payload)  => {}
                            ClientEvent::UpdateProperties(payload) => {}
                            ClientEvent::UpdateName(payload)       => {}
                            ClientEvent::SelectObject(payload)     => {}
                        }
                    }
                    Err(_) => {continue;}
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


