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
        ClientEvent, FullStateSyncPayload, ServerEvent, UserJoinedPayload, parse_client_message, UserLeftPayload,
        ObjectCreatedPayload, ObjectDeletedPayload, TransformUpdatedPayload, PropertiesUpdatedPayload,
        NameUpdatedPayload, UserSelectedPayload,
    },
    types::{AppState, Session, User, SceneObject},
};

use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH}; 

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
                                let json = serde_json::to_string(&sync).expect("serialize to json failed");
                                socket.send(Message::Text(json.into())).await.ok();

                                // broadcast UserJoined to everyone else
                                let joined_json = serde_json::to_string(&ServerEvent::UserJoined(UserJoinedPayload {
                                    user_id: new_user_id,
                                    display_name: payload.display_name.clone(),
                                    color: [255, 0, 0],
                                })).expect("serialize to json failed");

                                for entry in state.connections.iter() {
                                    if *entry.key() == connection_id { continue; }
                                    let _ = entry.value().try_send(joined_json.clone());
                                }
                            }
                            ClientEvent::LeaveSession => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                if let Some(session) = state.sessions.get(sid) {
                                    session.users.remove(&uid);
                                    let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload {
                                        user_id: uid,
                                    })).expect("serialize to json failed");
                                    for entry in state.connections.iter() {
                                        if *entry.key() == connection_id { continue; }
                                        let _ = entry.value().try_send(left_json.clone());
                                    }
                                }
                                user_id = None;
                                session_id = None;
                            }
                            ClientEvent::CreateObject(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                                if let Some(session) = state.sessions.get(sid) {
                                    let scene_object = SceneObject {
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
                                    session.objects.insert(scene_object.object_id, scene_object.clone());
                                    let json = serde_json::to_string(&ServerEvent::ObjectCreated(ObjectCreatedPayload {
                                        object: scene_object,
                                        created_by: uid,
                                    })).expect("serialize to json failed");
                                    for entry in state.connections.iter() {
                                        let _ = entry.value().try_send(json.clone());
                                    }
                                }
                            }
                            ClientEvent::DeleteObject(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                if let Some(session) = state.sessions.get(sid) {
                                    session.objects.remove(&payload.object_id);
                                    let json = serde_json::to_string(&ServerEvent::ObjectDeleted(ObjectDeletedPayload {
                                        object_id: payload.object_id,
                                        deleted_by: uid,
                                    })).expect("serialize to json failed");
                                    for entry in state.connections.iter() {
                                        let _ = entry.value().try_send(json.clone());
                                    }
                                }
                            }
                            ClientEvent::UpdateTransform(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                                if let Some(session) = state.sessions.get(sid) {
                                    if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                                        obj.transform = payload.transform.clone();
                                        obj.last_updated_by = uid;
                                        obj.last_updated_at = now;
                                        let json = serde_json::to_string(&ServerEvent::TransformUpdated(TransformUpdatedPayload {
                                            object_id: payload.object_id,
                                            transform: payload.transform,
                                            updated_by: uid,
                                        })).expect("serialize to json failed");
                                        for entry in state.connections.iter() {
                                            let _ = entry.value().try_send(json.clone());
                                        }
                                    }
                                }
                            }
                            ClientEvent::UpdateProperties(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                                if let Some(session) = state.sessions.get(sid) {
                                    if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                                        obj.properties = Some(payload.properties.clone());
                                        obj.last_updated_by = uid;
                                        obj.last_updated_at = now;
                                        let json = serde_json::to_string(&ServerEvent::PropertiesUpdated(PropertiesUpdatedPayload {
                                            object_id: payload.object_id,
                                            properties: payload.properties,
                                            updated_by: uid,
                                        })).expect("serialize to json failed");
                                        for entry in state.connections.iter() {
                                            let _ = entry.value().try_send(json.clone());
                                        }
                                    }
                                }
                            }
                            ClientEvent::UpdateName(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                                if let Some(session) = state.sessions.get(sid) {
                                    if let Some(mut obj) = session.objects.get_mut(&payload.object_id) {
                                        obj.name = payload.name.clone();
                                        obj.last_updated_by = uid;
                                        obj.last_updated_at = now;
                                        let json = serde_json::to_string(&ServerEvent::NameUpdated(NameUpdatedPayload {
                                            object_id: payload.object_id,
                                            name: payload.name,
                                            updated_by: uid,
                                        })).expect("serialize to json failed");
                                        for entry in state.connections.iter() {
                                            let _ = entry.value().try_send(json.clone());
                                        }
                                    }
                                }
                            }
                            ClientEvent::SelectObject(payload) => {
                                let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) else { continue; };
                                if let Some(session) = state.sessions.get(sid) {
                                    if let Some(mut user) = session.users.get_mut(&uid) {
                                        user.selected_object = payload.object_id;
                                    }
                                    let json = serde_json::to_string(&ServerEvent::UserSelected(UserSelectedPayload {
                                        user_id: uid,
                                        object_id: payload.object_id,
                                    })).expect("serialize to json failed");
                                    for entry in state.connections.iter() {
                                        let _ = entry.value().try_send(json.clone());
                                    }
                                }
                            }
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
    if let (Some(uid), Some(sid)) = (user_id, session_id.as_deref()) {
        if let Some(session) = state.sessions.get(sid) {
            session.users.remove(&uid);
            let left_json = serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload {
                user_id: uid,
            })).expect("serialize to json failed");
            for entry in state.connections.iter() {
                let _ = entry.value().try_send(left_json.clone());
            }
        }
    }
}
