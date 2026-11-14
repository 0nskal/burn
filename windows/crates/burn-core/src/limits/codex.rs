//! Codex (ChatGPT) usage, with an offline fallback to the newest rate_limits snapshot in the
//! Codex CLI's session logs.

use super::http::{self, json_f64, now_ms};
use crate::model::{LimitWindow, LimitsError, ProviderId, ProviderLimits};
use crate::paths::Paths;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

struct Auth {
    token: String,
    account_id: Option<String>,
}

fn read_auth(paths: &Paths) -> Option<Auth> {
    let raw = std::fs::read_to_string(paths.codex_home().join("auth.json")).ok()?;
    let obj: Value = serde_json::from_str(&raw).ok()?;
    let tokens = obj.get("tokens")?;
    let token = tokens.get("access_token")?.as_str().filter(|t| !t.is_empty())?.to_string();
    let account_id = tokens.get("account_id").and_then(Value::as_str).map(str::to_string);
    Some(Auth { token, account_id })
}

pub fn is_connected(paths: &Paths) -> bool {
    read_auth(paths).is_some()
}

pub fn fetch(paths: &Paths) -> Result<ProviderLimits, LimitsError> {
    let auth = read_auth(paths).ok_or(LimitsError::NotConnected)?;
    match fetch_remote(&auth) {
        Err(e @ LimitsError::Network(_)) => from_sessions(&paths.codex_sessions()).ok_or(e),
        Err(e @ LimitsError::Http(code)) if code >= 500 => from_sessions(&paths.codex_sessions()).ok_or(e),
        other => other,
    }
}

fn fetch_remote(auth: &Auth) -> Result<ProviderLimits, LimitsError> {
    let bearer = format!("Bearer {}", auth.token);
    let mut headers = vec![("Authorization", bearer.as_str()), ("User-Agent", "codex_cli_rs")];
    if let Some(id) = &auth.account_id {
        headers.push(("ChatGPT-Account-Id", id.as_str()));
    }
    let (status, body) = http::get("https://chatgpt.com/backend-api/wham/usage", &headers)?;
    if status == 401 || status == 403 {
        return Err(LimitsError::TokenExpired);
    }
    if status != 200 {
        return Err(LimitsError::Http(status));
    }
    parse_codex_usage(&http::parse_json_object(&body)?, now_ms())
}

pub(crate) fn parse_codex_usage(json: &Value, now: i64) -> Result<ProviderLimits, LimitsError> {
    let rate = json.get("rate_limit");
    let mut windows: Vec<LimitWindow> = Vec::new();
    for key in ["primary_window", "secondary_window"] {
        let Some(w) = rate.and_then(|r| r.get(key)).filter(|v| v.is_object()) else { continue };
        let Some(pct) = json_f64(w.get("used_percent")) else { continue };
        let (id, label) = window_name(json_f64(w.get("limit_window_seconds")).unwrap_or(0.0));
        windows.push(LimitWindow {
            id,
            label,
            percent: pct,
            resets_at: json_f64(w.get("reset_at")).map(|s| (s * 1000.0) as i64),
            minor: false,
        });
    }
    if windows.is_empty() {
        return Err(LimitsError::BadResponse);
    }
    // Stable sort; a missing reset sorts last.
    windows.sort_by_key(|w| w.resets_at.unwrap_or(i64::MAX));
    Ok(ProviderLimits {
        provider: ProviderId::Codex,
        plan: plan(json.get("plan_type").and_then(Value::as_str)),
        windows,
        fetched_at: now,
    })
}

pub(crate) fn window_name(seconds: f64) -> (String, String) {
    if seconds == 18000.0 {
        return ("5h".into(), "5h".into());
    }
    if seconds == 604_800.0 {
        return ("week".into(), "Week".into());
    }
    if (28.0 * 86400.0..=31.0 * 86400.0).contains(&seconds) {
        return ("month".into(), "Month".into());
    }
    let hours = (seconds / 3600.0).round() as i64;
    if hours >= 48 && hours % 24 == 0 {
        let d = format!("{}d", hours / 24);
        return (d.clone(), d);
    }
    let h = format!("{hours}h");
    (h.clone(), h)
}

pub(crate) fn plan(raw: Option<&str>) -> Option<String> {
    let raw = raw.filter(|s| !s.is_empty())?;
    Some(match raw.to_lowercase().as_str() {
        "free" => "Free".into(),
        "plus" => "Plus".into(),
        "pro" => "Pro".into(),
        "prolite" => "Pro Lite".into(),
        "team" => "Team".into(),
        _ => http::capitalized(raw),
    })
}

// Offline fallback: latest rate_limits snapshot written by the Codex CLI.

fn collect_jsonl(dir: &Path, cutoff: SystemTime, out: &mut Vec<(PathBuf, SystemTime)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            collect_jsonl(&path, cutoff, out);
        } else if path.extension().is_some_and(|e| e == "jsonl")
            && let Ok(m) = entry.metadata().and_then(|m| m.modified())
            && m > cutoff
        {
            out.push((path, m));
        }
    }
}

fn from_sessions(root: &Path) -> Option<ProviderLimits> {
    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(14 * 86400))?;
    let mut files = Vec::new();
    collect_jsonl(root, cutoff, &mut files);
    files.sort_by_key(|f| std::cmp::Reverse(f.1));
    let now = now_ms();
    for (path, _) in files.into_iter().take(5) {
        let Ok(data) = std::fs::read(&path) else { continue };
        for line in data.rsplit(|b| *b == b'\n') {
            if memchr::memmem::find(line, b"\"rate_limits\"").is_none() {
                continue;
            }
            if let Some(l) = parse_codex_snapshot(line, now) {
                return Some(l);
            }
        }
    }
    None
}

pub(crate) fn parse_codex_snapshot(line: &[u8], now: i64) -> Option<ProviderLimits> {
    let obj: Value = serde_json::from_slice(line).ok()?;
    let rl = obj.pointer("/payload/rate_limits").filter(|v| v.is_object())?;
    let mut windows = Vec::new();
    for key in ["primary", "secondary"] {
        let Some(w) = rl.get(key).filter(|v| v.is_object()) else { continue };
        let Some(pct) = json_f64(w.get("used_percent")) else { continue };
        let (id, label) = window_name(json_f64(w.get("window_minutes")).unwrap_or(0.0) * 60.0);
        let reset = json_f64(w.get("resets_at")).map(|s| (s * 1000.0) as i64);
        let expired = reset.is_some_and(|r| r < now);
        windows.push(LimitWindow {
            id,
            label,
            percent: if expired { 0.0 } else { pct },
            resets_at: reset,
            minor: false,
        });
    }
    if windows.is_empty() {
        return None;
    }
    Some(ProviderLimits {
        provider: ProviderId::Codex,
        plan: plan(rl.get("plan_type").and_then(Value::as_str)),
        windows,
        fetched_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn window_names() {
        assert_eq!(window_name(18000.0), ("5h".into(), "5h".into()));
        assert_eq!(window_name(604_800.0), ("week".into(), "Week".into()));
        assert_eq!(window_name(30.0 * 86400.0), ("month".into(), "Month".into()));
        assert_eq!(window_name(2.0 * 86400.0), ("2d".into(), "2d".into()));
        assert_eq!(window_name(3.0 * 3600.0), ("3h".into(), "3h".into()));
        assert_eq!(window_name(0.0), ("0h".into(), "0h".into()));
    }

    #[test]
    fn plans() {
        assert_eq!(plan(Some("prolite")).as_deref(), Some("Pro Lite"));
        assert_eq!(plan(Some("PLUS")).as_deref(), Some("Plus"));
        assert_eq!(plan(Some("team")).as_deref(), Some("Team"));
        assert_eq!(plan(Some("enterprise")).as_deref(), Some("Enterprise"));
        assert_eq!(plan(Some("")), None);
    }

    #[test]
    fn usage_sorted_by_reset() {
        let j = json!({
            "plan_type": "pro",
            "rate_limit": {
                "primary_window": {"used_percent": 10, "limit_window_seconds": 604800, "reset_at": 2000},
                "secondary_window": {"used_percent": "55.5", "limit_window_seconds": 18000, "reset_at": 1000}
            }
        });
        let l = parse_codex_usage(&j, 5).unwrap();
        assert_eq!(l.plan.as_deref(), Some("Pro"));
        assert_eq!(l.windows[0].id, "5h");
        assert_eq!(l.windows[0].percent, 55.5);
        assert_eq!(l.windows[0].resets_at, Some(1_000_000));
        assert_eq!(l.windows[1].id, "week");
        assert_eq!(parse_codex_usage(&json!({}), 0), Err(LimitsError::BadResponse));
    }

    #[test]
    fn snapshot_expired_reset_is_zero() {
        let line = br#"{"payload":{"rate_limits":{"plan_type":"plus","primary":{"used_percent":80,"window_minutes":300,"resets_at":1000},"secondary":{"used_percent":20,"window_minutes":10080,"resets_at":9999999999}}}}"#;
        let l = parse_codex_snapshot(line, 2_000_000).unwrap();
        assert_eq!(l.plan.as_deref(), Some("Plus"));
        assert_eq!(l.windows[0].id, "5h");
        assert_eq!(l.windows[0].percent, 0.0);
        assert_eq!(l.windows[1].id, "week");
        assert_eq!(l.windows[1].percent, 20.0);
        assert!(parse_codex_snapshot(b"{\"payload\":{}}", 0).is_none());
    }
}
