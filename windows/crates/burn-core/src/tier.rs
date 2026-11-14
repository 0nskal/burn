//! Port of `CodexTierResolver.swift`: the service tier of Codex turns whose rollout carried no
//! `thread_settings_applied`, read from T3 Code's own database. Read-only; empty when it's missing.

use crate::parsers::Speed;
use crate::paths::Paths;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Request {
    pub session: String,
    pub turn: String,
    pub model: String,
}

pub fn db_path(paths: &Paths) -> PathBuf {
    paths.t3_userdata().join("statev2.sqlite")
}

/// Size and mtime of T3's database and its WAL; changes when T3 recorded anything new (T3 writes to the
/// WAL first, the main file only changes on a checkpoint).
pub fn db_stamp(paths: &Paths) -> String {
    let db = db_path(paths);
    let mut wal = db.clone().into_os_string();
    wal.push("-wal");
    [db, PathBuf::from(wal)]
        .iter()
        .map(|p| match std::fs::metadata(p) {
            Ok(m) => {
                let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
                format!("{}.{}.{}", m.len(), t.as_secs(), t.subsec_nanos())
            }
            Err(_) => "none".into(),
        })
        .collect::<Vec<_>>()
        .join("|")
}

const SQL: &str = "
SELECT json_extract(attempt.payload_json, '$.nativeThreadId') AS native_thread_id,
  json_extract(turn.payload_json, '$.nativeTurnRef.nativeId') AS native_turn_id,
  (
    SELECT json_extract(event.payload_json, '$.modelSelection')
    FROM orchestration_events AS event
    WHERE event.aggregate_kind = 'thread'
      AND event.application_event_version = 2
      AND event.stream_id = attempt.thread_id
      AND event.event_type = 'run.updated'
      AND json_extract(event.payload_json, '$.id') = attempt.run_id
      AND json_extract(event.payload_json, '$.activeAttemptId') = attempt.attempt_id
      AND json_extract(event.payload_json, '$.status') = 'running'
    ORDER BY event.sequence
    LIMIT 1
  ) AS selection_json
FROM orchestration_v2_projection_provider_turns AS turn
LEFT JOIN orchestration_v2_projection_run_attempts AS attempt
  ON attempt.attempt_id = turn.run_attempt_id
  AND attempt.thread_id = turn.thread_id
  AND attempt.provider_thread_id = turn.provider_thread_id
WHERE json_extract(turn.payload_json, '$.nativeTurnRef.driver') = 'codex'
  AND json_extract(turn.payload_json, '$.nativeTurnRef.nativeId') IN (SELECT value FROM json_each(?))
";

struct Match {
    thread: Option<String>,
    selection: Option<String>,
}

pub fn resolve(paths: &Paths, requests: &[Request]) -> HashMap<Request, Speed> {
    let path = db_path(paths);
    if requests.is_empty() || !path.exists() {
        return HashMap::new();
    }
    query(&path, requests).unwrap_or_default()
}

fn query(path: &std::path::Path, requests: &[Request]) -> rusqlite::Result<HashMap<Request, Speed>> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    db.busy_timeout(Duration::from_secs(3))?;
    let turns: Vec<&str> = requests.iter().map(|r| r.turn.as_str()).collect::<HashSet<_>>().into_iter().collect();
    let json = serde_json::to_string(&turns).unwrap_or_else(|_| "[]".into());

    let mut stmt = db.prepare(SQL)?;
    let mut rows = stmt.query([json])?;
    // None: the turn id is ambiguous.
    let mut by_turn: HashMap<String, Option<Match>> = HashMap::new();
    while let Some(row) = rows.next()? {
        let Some(turn) = row.get::<_, Option<String>>(1)? else { continue };
        let m = Match { thread: row.get(0)?, selection: row.get(2)? };
        by_turn.entry(turn).and_modify(|e| *e = None).or_insert(Some(m));
    }

    let mut out = HashMap::new();
    for r in requests {
        let Some(Some(m)) = by_turn.get(&r.turn) else { continue };
        if m.thread.as_deref() != Some(r.session.as_str()) {
            continue;
        }
        let Some(sel) = m.selection.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()) else { continue };
        let Some(sel) = sel.as_object() else { continue };
        if sel.get("model").and_then(Value::as_str) != Some(r.model.as_str()) {
            continue;
        }
        if let Some(speed) = speed_of(sel.get("options")) {
            out.insert(r.clone(), speed);
        }
    }
    Ok(out)
}

/// `None`: the selection doesn't say (or says something unknown), leave the turn unresolved.
fn speed_of(options: Option<&Value>) -> Option<Speed> {
    let empty = Vec::new();
    let options = options.and_then(Value::as_array).unwrap_or(&empty);
    let option = |id: &str| options.iter().find(|o| o.get("id").and_then(Value::as_str) == Some(id));
    let is_tier = |o: &&Value| o.get("id").and_then(Value::as_str) == Some("serviceTier");
    if options.iter().filter(is_tier).any(|o| !o.get("value").is_some_and(Value::is_string)) {
        return None;
    }
    let mut tier = option("serviceTier").and_then(|o| o.get("value")).and_then(Value::as_str);
    if tier.is_none() && option("fastMode").and_then(|o| o.get("value")).and_then(Value::as_bool) == Some(true) {
        tier = Some("fast");
    }
    match tier {
        Some("fast" | "priority") => Some(Speed::Fast),
        Some("ultrafast") => Some(Speed::Ultrafast),
        None | Some("default" | "standard") => Some(Speed::Standard),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn speed_from_selection_options() {
        assert_eq!(speed_of(Some(&json!([{"id": "serviceTier", "value": "priority"}]))), Some(Speed::Fast));
        assert_eq!(speed_of(Some(&json!([{"id": "serviceTier", "value": "ultrafast"}]))), Some(Speed::Ultrafast));
        assert_eq!(speed_of(Some(&json!([{"id": "fastMode", "value": true}]))), Some(Speed::Fast));
        assert_eq!(speed_of(Some(&json!([{"id": "fastMode", "value": 1}]))), Some(Speed::Standard));
        assert_eq!(speed_of(Some(&json!([{"id": "serviceTier", "value": null}]))), None);
        assert_eq!(speed_of(Some(&json!([{"id": "serviceTier", "value": "flex"}]))), None);
        assert_eq!(speed_of(None), Some(Speed::Standard));
    }
}
