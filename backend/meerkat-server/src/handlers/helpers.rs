use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use std::sync::Arc;

use crate::messages::{ServerEvent, UserLeftPayload};
use crate::types::{AppState, LagState, SessionHandle, User, COLOR_PALETTE};

const BACKPRESSURE_RESET_MS: u64 = 5_000;
const BACKPRESSURE_EVICT_STRIKES: u8 = 3;

pub fn now_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        Err(err) => {
            tracing::warn!(error = %err, "system clock before unix epoch; falling back to 0ms timestamp");
            0
        }
    }
}

pub fn broadcast(state: &AppState, session_id: &str, json: &str, exclude: Option<Uuid>) -> usize {
    let mut delivered = 0;
    let mut dropped_full = 0;
    let mut dropped_closed = 0;
    let mut missing_tx = 0;

    // Initialize a vector to track connections that should be evicted due to full or closed channels
    let mut to_evict = Vec::new();

    let conn_ids: Vec<Uuid> = state
    .session_connections
    .get(session_id)
    .map(|conns| conns.iter().copied().collect())
    .unwrap_or_default();

    for conn_id in conn_ids {
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

        if let Some((_, (session_id, _user_id))) = state.connection_meta.remove(conn_id) {
            let mut remove_session_entry = false;

            if let Some(mut conns) = state.session_connections.get_mut(&session_id) {
                conns.remove(conn_id);
                remove_session_entry = conns.is_empty();
            }

            if remove_session_entry {
                state.session_connections.remove(&session_id);
            }
        }
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

/// Clean up stale membership when a connection is already tracked in another session.
/// Used by both join_session and create_session handlers.
pub fn cleanup_stale_membership(state: &AppState, connection_id: Uuid, new_session_id: &str) {
    if let Some((_, (old_sid, old_uid))) = state.connection_meta.remove(&connection_id) {
        tracing::warn!(
            connection_id = %connection_id,
            old_session_id = %old_sid,
            old_user_id = %old_uid,
            "connection re-joining while still tracked; cleaning stale membership"
        );

        // Remove connection from old session->connections index
        let mut remove_old_session_entry = false;
        if let Some(mut conns) = state.session_connections.get_mut(&old_sid) {
            conns.remove(&connection_id);
            remove_old_session_entry = conns.is_empty();
        }
        if remove_old_session_entry {
            state.session_connections.remove(&old_sid);
        }

        // Remove stale user presence from old session users map
        let mut reclaim_old_session = false;
        if let Some(old_session) = state.sessions.get(&old_sid) {
            let mut users = match old_session.users.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::warn!("Old session users lock poisoned during re-join cleanup, recovering");
                    poisoned.into_inner()
                }
            };
            users.remove(&old_uid);
            reclaim_old_session = users.is_empty() && old_sid != new_session_id;
        }

        if reclaim_old_session {
            state.sessions.remove(&old_sid);
            tracing::info!(
                event_type = "SessionReclaimed",
                session_id = %old_sid,
                "reclaimed empty stale session during re-join cleanup"
            );
        }

        // Broadcast UserLeft for old session
        match serde_json::to_string(&ServerEvent::UserLeft(UserLeftPayload { user_id: old_uid })) {
            Ok(left_json) => {
                let count = broadcast(state, &old_sid, &left_json, Some(connection_id));
                tracing::info!(
                    connection_id = %connection_id,
                    old_session_id = %old_sid,
                    old_user_id = %old_uid,
                    recipient_count = count,
                    "broadcast UserLeft for stale session during re-join cleanup",
                );
            }
            Err(err) => {
                tracing::error!(
                    connection_id = %connection_id,
                    old_session_id = %old_sid,
                    old_user_id = %old_uid,
                    error = %err,
                    "failed to serialize UserLeft during stale re-join cleanup"
                );
            }
        }
        tracing::warn!("user has left session due to re-joining while still tracked; if this happens frequently, consider investigating client connection stability or adding more aggressive backpressure eviction");
    }
}

/// Add a user to a session and track the connection.
/// Returns (user_id, color) for use in FullStateSync and UserJoined broadcasts.
pub fn add_user_to_session(state: &AppState, session: &Arc<SessionHandle>, connection_id: Uuid, session_id: &str, display_name: &str,) -> (Uuid, [u8; 3]) {
    let user_id = Uuid::new_v4();
    let color = {
        let mut users = match session.users.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session users lock poisoned (write), recovering");
                poisoned.into_inner()
            }
        };
        let color = COLOR_PALETTE[users.len() % COLOR_PALETTE.len()];
        users.insert(
            user_id,
            User {
                display_name: display_name.to_string(),
                color,
                selected_object: None,
                connected_at: now_ms(),
            },
        );
        color
    };

    state
        .connection_meta
        .insert(connection_id, (session_id.to_string(), user_id));

    state
        .session_connections
        .entry(session_id.to_string())
        .or_default()
        .insert(connection_id);

    tracing::info!(
        event_type = "UserAdded",
        session_id = %session_id,
        user_id = %user_id,
        display_name = %display_name,
        connection_id = %connection_id,
        "user added to session"
    );

    (user_id, color)
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
