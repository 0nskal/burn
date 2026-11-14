//! Cursor usage (`GetCurrentPeriodUsage`). The token comes from the Keychain (macOS) or from
//! Cursor's state database.

use super::http::{self, json_f64, now_ms};
use crate::model::{LimitWindow, LimitsError, ProviderId, ProviderLimits};
use crate::paths::Paths;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

fn keychain_token() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        http::keychain_password("cursor-access-token", Some("cursor-user"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn is_connected(paths: &Paths) -> bool {
    #[cfg(target_os = "macos")]
    if http::keychain_item_exists("cursor-access-token", Some("cursor-user")) {
        return true;
    }
    item_value(&paths.cursor_state_db(), "cursorAuth/accessToken").is_some()
}

pub fn fetch(paths: &Paths) -> Result<ProviderLimits, LimitsError> {
    let db = paths.cursor_state_db();
    let token =
        keychain_token().or_else(|| item_value(&db, "cursorAuth/accessToken")).ok_or(LimitsError::NotConnected)?;
    let bearer = format!("Bearer {token}");
    let (status, body) = http::post_json(
        "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage",
        &[("Authorization", &bearer), ("Connect-Protocol-Version", "1")],
        "{}",
    )?;
    if status == 401 || status == 403 {
        return Err(LimitsError::TokenExpired);
    }
    if status != 200 {
        return Err(LimitsError::Http(status));
    }
    let json = http::parse_json_object(&body)?;
    let membership = item_value(&db, "cursorAuth/stripeMembershipType");
    parse_cursor(&json, membership.as_deref(), now_ms())
}

pub(crate) fn parse_cursor(json: &Value, membership: Option<&str>, now: i64) -> Result<ProviderLimits, LimitsError> {
    if json.get("code").and_then(Value::as_str) == Some("unauthenticated") {
        return Err(LimitsError::TokenExpired);
    }
    let reset = json_f64(json.get("billingCycleEnd")).map(|ms| ms as i64);
    let usage = json.get("planUsage");
    let field = |k: &str| json_f64(usage.and_then(|u| u.get(k)));
    let mut windows = Vec::new();
    let mut total = field("totalPercentUsed");
    if total.is_none()
        && let (Some(inc), Some(lim)) = (field("includedSpend"), field("limit"))
        && lim > 0.0
    {
        total = Some(inc / lim * 100.0);
    }
    if let Some(t) = total {
        windows.push(LimitWindow {
            id: "total".into(),
            label: "Month".into(),
            percent: t,
            resets_at: reset,
            minor: false,
        });
    }
    if let Some(api) = field("apiPercentUsed") {
        windows.push(LimitWindow {
            id: "api".into(),
            label: "API".into(),
            percent: api,
            resets_at: reset,
            minor: false,
        });
    }
    if windows.is_empty() {
        return Err(LimitsError::BadResponse);
    }
    Ok(ProviderLimits { provider: ProviderId::Cursor, plan: plan(membership), windows, fetched_at: now })
}

pub(crate) fn plan(raw: Option<&str>) -> Option<String> {
    let raw = raw.filter(|s| !s.is_empty())?;
    Some(match raw.to_lowercase().as_str() {
        "free" => "Free".into(),
        "pro" => "Pro".into(),
        "pro_plus" => "Pro+".into(),
        "ultra" => "Ultra".into(),
        "free_trial" => "Trial".into(),
        "business" | "team" => "Team".into(),
        _ => http::capitalized(&raw.replace('_', " ")),
    })
}

/// One ItemTable value from Cursor's state DB, opened read-only (Cursor may have it open).
fn item_value(db_path: &Path, key: &str) -> Option<String> {
    if !db_path.is_file() {
        return None;
    }
    // Plain read-only open (no URI), so Windows drive letters and spaces need no escaping.
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .ok()?;
    conn.busy_timeout(Duration::from_secs(1)).ok()?;
    let raw: Vec<u8> = conn
        .query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |r| {
            // Cursor stores values as TEXT, sometimes BLOB.
            match r.get_ref(0)? {
                rusqlite::types::ValueRef::Text(t) | rusqlite::types::ValueRef::Blob(t) => Ok(t.to_vec()),
                _ => Ok(Vec::new()),
            }
        })
        .ok()?;
    let s = String::from_utf8_lossy(&raw);
    let v = s.trim_matches(|c: char| c.is_whitespace() || c == '"');
    (!v.is_empty()).then(|| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn total_percent_and_api() {
        let j =
            json!({"billingCycleEnd": "1800000000000", "planUsage": {"totalPercentUsed": 42.5, "apiPercentUsed": 10}});
        let l = parse_cursor(&j, Some("pro_plus"), 1).unwrap();
        assert_eq!(l.plan.as_deref(), Some("Pro+"));
        assert_eq!(l.windows.len(), 2);
        assert_eq!((l.windows[0].id.as_str(), l.windows[0].percent), ("total", 42.5));
        assert_eq!(l.windows[0].resets_at, Some(1_800_000_000_000));
        assert_eq!((l.windows[1].id.as_str(), l.windows[1].label.as_str()), ("api", "API"));
    }

    #[test]
    fn included_spend_fallback() {
        let j = json!({"planUsage": {"includedSpend": 5, "limit": 20}});
        let l = parse_cursor(&j, None, 1).unwrap();
        assert_eq!(l.windows.len(), 1);
        assert_eq!(l.windows[0].percent, 25.0);
        assert_eq!(l.plan, None);
        assert_eq!(
            parse_cursor(&json!({"planUsage": {"includedSpend": 5, "limit": 0}}), None, 1),
            Err(LimitsError::BadResponse)
        );
    }

    #[test]
    fn api_only_and_unauthenticated() {
        let l = parse_cursor(&json!({"planUsage": {"apiPercentUsed": 3}}), None, 1).unwrap();
        assert_eq!(l.windows[0].id, "api");
        assert_eq!(parse_cursor(&json!({"code": "unauthenticated"}), None, 1), Err(LimitsError::TokenExpired));
    }

    #[test]
    fn plans() {
        assert_eq!(plan(Some("pro_plus")).as_deref(), Some("Pro+"));
        assert_eq!(plan(Some("free_trial")).as_deref(), Some("Trial"));
        assert_eq!(plan(Some("business")).as_deref(), Some("Team"));
        assert_eq!(plan(Some("enterprise_plus")).as_deref(), Some("Enterprise Plus"));
        assert_eq!(plan(Some("")), None);
    }

    #[test]
    fn reads_item_table() {
        let dir = std::env::temp_dir().join(format!("burn-cursor-test {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.vscdb");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE ItemTable (key TEXT UNIQUE, value BLOB); INSERT INTO ItemTable VALUES ('a', ' \"tok\" ');").unwrap();
        }
        assert_eq!(item_value(&path, "a").as_deref(), Some("tok"));
        assert_eq!(item_value(&path, "missing"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
