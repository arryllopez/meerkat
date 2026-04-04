use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc::{self, error::TryRecvError};
use uuid::Uuid;

use meerkat_server::{handlers::helpers::broadcast, types::AppState};

#[test]
fn broadcast_evicts_connection_after_three_full_strikes() {
    let session_id = "overflow-session".to_string();
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let (tx, mut rx) = mpsc::channel::<String>(32);
    for i in 0..32 {
        tx.try_send(format!("prefill-{i}"))
            .expect("queue prefill should fit capacity");
    }

    let connections = Arc::new(DashMap::new());
    connections.insert(connection_id, tx);

    let connection_meta = Arc::new(DashMap::new());
    connection_meta.insert(connection_id, (session_id.clone(), user_id));

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        connections,
        connection_meta,
        connection_backpressure: Arc::new(DashMap::new()),
    };

    for _ in 0..2 {
        let delivered = broadcast(&state, &session_id, "{\"event_type\":\"Test\"}", None);
        assert_eq!(delivered, 0, "full queue should not accept another message");
        assert!(
            state.connections.get(&connection_id).is_some(),
            "connection should stay until strike threshold"
        );
    }

    let delivered = broadcast(&state, &session_id, "{\"event_type\":\"Test\"}", None);
    assert_eq!(delivered, 0, "third full strike should still drop the send");

    assert!(
        state.connections.get(&connection_id).is_none(),
        "full queue connection should be evicted on third strike"
    );
    assert!(
        state.connection_meta.get(&connection_id).is_none(),
        "evicted connection metadata should be removed"
    );

    let mut drained = 0;
    while rx.try_recv().is_ok() {
        drained += 1;
    }
    assert_eq!(drained, 32, "expected original queued messages to remain");
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Disconnected)),
        "receiver should be disconnected after eviction drops sender"
    );
}
