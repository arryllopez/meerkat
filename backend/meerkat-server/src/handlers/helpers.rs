use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{
    event_log::append_entry,
    types::{AppState, LogEntry},
};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as u64
}

pub fn write_log(state: &AppState, session_id: &str, entry: &LogEntry) {
    if let Some(writer) = state.log_files.get(session_id) {
        append_entry(writer.value(), entry);
    }
}

pub fn broadcast(state: &AppState, session_id: &str, json: &str, exclude: Option<Uuid>) -> usize {
    let mut count = 0;
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
            if tx.try_send(json.to_owned()).is_ok() {
                count += 1;
            }
        }
    }
    count
}
