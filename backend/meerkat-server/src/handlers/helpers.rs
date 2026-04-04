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

    // Initialize a vector to track connections that should be evicted due to full or closed channels
    let mut to_evict = Vec::new();

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
                        "dropped outbound message: receiver channel is full"
                    );
                    to_evict.push(conn_id); 
                }
                Err(TrySendError::Closed(_)) => {
                    dropped_closed += 1;
                    tracing::warn!(
                        session_id = %session_id,
                        connection_id = %conn_id,
                        "dropped outbound message: receiver channel is closed"
                    );
                    to_evict.push(conn_id);
                }
            }
        } else {
            missing_tx += 1; 
            to_evict.push(conn_id);
            tracing::warn!(
                session_id = %session_id,
                connection_id = %conn_id,   
                "dropped outbound message: no sender channel found for connection"
            );
        }
    }

    // Evict all connections with full/closed channels or missing senders after processing to avoid holding up the broadcast loop
    evict_connection(state, &to_evict);

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

pub fn evict_connection(state: &AppState, connection_ids: &[Uuid]) {
    for conn_id in connection_ids {
        state.connections.remove(conn_id);
        state.connection_meta.remove(conn_id);
    }
}
