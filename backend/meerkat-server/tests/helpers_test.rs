use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use meerkat_server::{
    handlers::helpers::broadcast,
    types::{AppState, SessionHandle},
};

#[test]
fn broadcast_drops_when_connection_queue_is_full() {
    let session_id = "backlog-test".to_string();
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let sessions = Arc::new(DashMap::new());
    sessions.insert(
        session_id.clone(),
        Arc::new(SessionHandle {
            objects: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
            session_id: session_id.clone(),
        }),
    );

    let connections = Arc::new(DashMap::new());
    let (tx, mut rx) = mpsc::channel::<String>(32);
    connections.insert(connection_id, tx);

    let connection_meta = Arc::new(DashMap::new());
    connection_meta.insert(connection_id, (session_id.clone(), user_id));

    let state = AppState {
        sessions,
        connections,
        connection_meta,
    };

    for _ in 0..32 {
        let delivered = broadcast(&state, &session_id, "{\"event_type\":\"Test\"}", None);
        assert_eq!(
            delivered, 1,
            "expected message to be queued while capacity remains"
        );
    }

    let delivered = broadcast(&state, &session_id, "{\"event_type\":\"Test\"}", None);
    assert_eq!(
        delivered, 0,
        "expected send to fail once queue reaches capacity"
    );

    let mut drained = 0;
    while rx.try_recv().is_ok() {
        drained += 1;
    }
    assert_eq!(drained, 32, "expected exactly 32 queued messages");
}
