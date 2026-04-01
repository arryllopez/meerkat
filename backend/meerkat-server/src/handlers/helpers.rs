use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use crate::types::AppState;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as u64
}

pub fn broadcast(state: &AppState, session_id: &str, json: &str, exclude: Option<Uuid>) -> usize {
    let mut delivered = 0;
    let mut dropped_full = 0;
    let mut dropped_closed = 0;
    let mut missing_tx = 0;

    for entry in state.connection_meta.iter() {
        let (conn_session, _) = entry.value();
        if conn_session.as_str() != session_id {
            continue;
        }
        let conn_id = *entry.key();
        if exclude == Some(conn_id) {
            continue;
        }
        if let Some(tx) = state.connections.get(&conn_id) {
            match tx.try_send(json.to_owned()) {
                Ok(()) => {
                    delivered += 1;
                }
                Err(TrySendError::Full(_)) => {
                    dropped_full += 1;
                    tracing::warn!(
                        session_id = %session_id,
                        connection_id = %conn_id,
                        queue_capacity = tx.capacity(),
                        "dropped outbound message: receiver queue is full"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    dropped_closed += 1;
                    tracing::warn!(
                        session_id = %session_id,
                        connection_id = %conn_id,
                        "dropped outbound message: receiver channel is closed"
                    );
                }
            }
        } else {
            missing_tx += 1;
        }
    }

    if dropped_full > 0 || dropped_closed > 0 || missing_tx > 0 {
        tracing::warn!(
            session_id = %session_id,
            delivered,
            dropped_full,
            dropped_closed,
            missing_tx,
            "broadcast delivery shortfall"
        );
    }

    delivered
}
