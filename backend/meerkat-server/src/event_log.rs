use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use uuid::Uuid;

use crate::messages::{
    CreateObjectPayload, DeleteObjectPayload, UpdateNamePayload, UpdatePropertiesPayload,
    UpdateTransformPayload,
};
use crate::types::{LogEntry, SceneObject, Session};

const DATA_DIR: &str = "./data";

/// Ensure the data directory exists.
pub fn ensure_data_dir() {
    fs::create_dir_all(DATA_DIR).expect("failed to create data directory");
}

/// Open (or create) the append-only log file for a session.
pub fn open_log_file(session_id: &str) -> Mutex<BufWriter<File>> {
    let path = log_path(session_id);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("failed to open log file {}: {}", path.display(), e));
    Mutex::new(BufWriter::new(file))
}

/// Append a LogEntry as a JSON line and flush.
pub fn append_entry(writer: &Mutex<BufWriter<File>>, entry: &LogEntry) {
    let mut w = writer.lock().expect("log file mutex poisoned");
    let line = serde_json::to_string(entry).expect("LogEntry serialization failed");
    writeln!(w, "{}", line).expect("failed to write log entry");
    w.flush().expect("failed to flush log file");
}

/// Scan ./data/ for .log files and replay each into a Session.
/// Returns a map of session_id → Session.
pub fn replay_all_logs() -> HashMap<String, Session> {
    let dir = Path::new(DATA_DIR);
    if !dir.exists() {
        return HashMap::new();
    }

    let mut sessions = HashMap::new();

    let entries = fs::read_dir(dir).expect("failed to read data directory");
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }

        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        match replay_log(&path, &session_id) {
            Ok(session) => {
                let obj_count = session.objects.len();
                tracing::info!(
                    session_id = %session_id,
                    objects = obj_count,
                    "replayed session from event log"
                );
                sessions.insert(session_id, session);
            }
            Err(e) => {
                tracing::error!(
                    session_id = %session_id,
                    error = %e,
                    "failed to replay event log"
                );
            }
        }
    }

    sessions
}

/// Replay a single .log file into a Session.
fn replay_log(path: &PathBuf, session_id: &str) -> Result<Session, String> {
    let file = File::open(path).map_err(|e| format!("open: {}", e))?;
    let reader = BufReader::new(file);

    let mut session = Session {
        session_id: session_id.to_string(),
        objects: HashMap::new(),
        users: HashMap::new(),
        event_log: Vec::new(),
    };

    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(
                    session_id = %session_id,
                    line = line_num + 1,
                    error = %e,
                    "skipping unreadable log line"
                );
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let entry: LogEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    session_id = %session_id,
                    line = line_num + 1,
                    error = %e,
                    "skipping malformed log entry"
                );
                continue;
            }
        };

        replay_entry(&mut session, &entry);
    }

    Ok(session)
}

/// Apply a single log entry to a session (no networking, no broadcast).
fn replay_entry(session: &mut Session, entry: &LogEntry) {
    match entry.event_type.as_str() {
        "CreateObject" => {
            let payload: CreateObjectPayload = match serde_json::from_value(entry.payload.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to deserialize CreateObject during replay");
                    return;
                }
            };
            let object = SceneObject {
                object_id: payload.object_id,
                name: payload.name,
                object_type: payload.object_type,
                asset_id: payload.asset_id,
                asset_library: payload.asset_library,
                transform: payload.transform,
                properties: payload.properties,
                created_by: Uuid::nil(),
                last_updated_by: Uuid::nil(),
                last_updated_at: entry.timestamp,
            };
            session.objects.insert(object.object_id, object);
        }

        "DeleteObject" => {
            let payload: DeleteObjectPayload = match serde_json::from_value(entry.payload.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to deserialize DeleteObject during replay");
                    return;
                }
            };
            session.objects.remove(&payload.object_id);
        }

        "UpdateTransform" => {
            let payload: UpdateTransformPayload =
                match serde_json::from_value(entry.payload.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to deserialize UpdateTransform during replay");
                        return;
                    }
                };
            if let Some(obj) = session.objects.get_mut(&payload.object_id) {
                obj.transform = payload.transform;
                obj.last_updated_at = entry.timestamp;
            }
        }

        "UpdateProperties" => {
            let payload: UpdatePropertiesPayload =
                match serde_json::from_value(entry.payload.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to deserialize UpdateProperties during replay");
                        return;
                    }
                };
            if let Some(obj) = session.objects.get_mut(&payload.object_id) {
                obj.properties = Some(payload.properties);
                obj.last_updated_at = entry.timestamp;
            }
        }

        "UpdateName" => {
            let payload: UpdateNamePayload = match serde_json::from_value(entry.payload.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to deserialize UpdateName during replay");
                    return;
                }
            };
            if let Some(obj) = session.objects.get_mut(&payload.object_id) {
                obj.name = payload.name;
                obj.last_updated_at = entry.timestamp;
            }
        }

        other => {
            tracing::warn!(event_type = other, "unknown event type during replay — skipping");
        }
    }
}

fn log_path(session_id: &str) -> PathBuf {
    Path::new(DATA_DIR).join(format!("{}.log", session_id))
}
