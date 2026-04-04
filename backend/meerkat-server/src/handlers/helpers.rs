use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use crate::types::{AppState, LagState};

const BACKPRESSURE_RESET_MS: u64 = 5_000;
const BACKPRESSURE_EVICT_STRIKES: u8 = 3;

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
                    decay_lag_strikes_on_ok_send(state, conn_id, now_ms());
                }
                Err(TrySendError::Full(_)) => {
                    dropped_full += 1;
                    let strikes = record_full_strike(state, conn_id, now_ms());
                    if strikes >= BACKPRESSURE_EVICT_STRIKES {
                        tracing::warn!(
                            session_id = %session_id,
                            connection_id = %conn_id,
                            strikes,
                            "evicting connection: receiver channel repeatedly full"
                        );
                        to_evict.push(conn_id);
                    } else {
                        tracing::debug!(
                            session_id = %session_id,
                            connection_id = %conn_id,
                            strikes,
                            "dropped outbound message: receiver channel is full"
                        );
                    }
                }
                Err(TrySendError::Closed(_)) => {
                    dropped_closed += 1;
                    tracing::debug!(
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
            tracing::debug!(
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
        state.connection_backpressure.remove(conn_id);
    }
}

fn record_full_strike(state: &AppState, connection_id: Uuid, now_ms: u64) -> u8 {
    if let Some(mut lag) = state.connection_backpressure.get_mut(&connection_id) {
        if now_ms.saturating_sub(lag.last_full_at_ms) > BACKPRESSURE_RESET_MS {
            lag.strikes = 0;
        }
        lag.strikes = lag.strikes.saturating_add(1);
        lag.last_full_at_ms = now_ms;
        lag.strikes
    } else {
        state.connection_backpressure.insert(
            connection_id,
            LagState {
                strikes: 1,
                last_full_at_ms: now_ms,
            },
        );
        1
    }
}

fn decay_lag_strikes_on_ok_send(state: &AppState, connection_id: Uuid, now_ms: u64) {
    let mut should_remove = false;

    if let Some(mut lag) = state.connection_backpressure.get_mut(&connection_id) {
        if now_ms.saturating_sub(lag.last_full_at_ms) > BACKPRESSURE_RESET_MS {
            lag.strikes = 0;
        } else {
            lag.strikes = lag.strikes.saturating_sub(1);
        }
        should_remove = lag.strikes == 0;
    }

    if should_remove {
        state
            .connection_backpressure
            .remove_if(&connection_id, |_, lag| lag.strikes == 0);
    }
}
