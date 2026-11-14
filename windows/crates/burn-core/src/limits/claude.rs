//! Claude Code OAuth usage (`/api/oauth/usage`). Never refreshes the token.

use super::http::{self, json_f64, now_ms, parse_iso_ms};
use crate::model::{LimitWindow, LimitsError, ProviderId, ProviderLimits};
use crate::paths::Paths;
use serde_json::Value;

#[cfg(target_os = "macos")]
const SERVICE: &str = "Claude Code-credentials";

/// Raw credentials JSON: the file first, on macOS the Keychain as fallback.
fn read_credentials(paths: &Paths) -> Option<String> {
    if let Ok(s) = std::fs::read_to_string(paths.claude_credentials()) {
        return Some(s);
    }
    #[cfg(target_os = "macos")]
    {
        http::keychain_password(SERVICE, None)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn is_connected(paths: &Paths) -> bool {
    if paths.claude_credentials().is_file() {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        http::keychain_item_exists(SERVICE, None)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn fetch(paths: &Paths) -> Result<ProviderLimits, LimitsError> {
    let raw = read_credentials(paths).ok_or(LimitsError::NotConnected)?;
    let obj: Value = serde_json::from_str(&raw).map_err(|_| LimitsError::NotConnected)?;
    let oauth = obj.get("claudeAiOauth").filter(|v| v.is_object()).ok_or(LimitsError::NotConnected)?;
    let token = oauth.get("accessToken").and_then(Value::as_str).ok_or(LimitsError::NotConnected)?;

    if let Some(exp) = json_f64(oauth.get("expiresAt"))
        && exp < now_ms() as f64
    {
        return Err(LimitsError::TokenExpired);
    }

    let auth = format!("Bearer {token}");
    let (status, body) = http::get(
        "https://api.anthropic.com/api/oauth/usage",
        &[("Authorization", &auth), ("anthropic-beta", "oauth-2025-04-20")],
    )?;
    if status == 401 {
        return Err(LimitsError::TokenExpired);
    }
    if status != 200 {
        return Err(LimitsError::Http(status));
    }
    let json = http::parse_json_object(&body)?;
    parse_claude(&json, oauth, now_ms())
}

pub(crate) fn parse_claude(json: &Value, oauth: &Value, now: i64) -> Result<ProviderLimits, LimitsError> {
    const KEYS: [(&str, &str, &str); 4] = [
        ("five_hour", "5h", "5h"),
        ("seven_day", "week", "Week"),
        ("seven_day_opus", "week_opus", "Opus"),
        ("seven_day_sonnet", "week_sonnet", "Sonnet"),
    ];
    let mut windows: Vec<LimitWindow> = Vec::new();
    for (key, id, label) in KEYS {
        let Some(w) = json.get(key).filter(|v| v.is_object()) else { continue };
        let Some(pct) = json_f64(w.get("utilization")) else { continue };
        windows.push(LimitWindow {
            id: id.into(),
            label: label.into(),
            percent: pct,
            resets_at: w.get("resets_at").and_then(Value::as_str).and_then(parse_iso_ms),
            minor: false,
        });
    }
    // Per-model weekly caps (e.g. Fable) only come in the newer `limits` array.
    if let Some(list) = json.get("limits").and_then(Value::as_array) {
        for l in list {
            if l.get("kind").and_then(Value::as_str) != Some("weekly_scoped") {
                continue;
            }
            let Some(pct) = json_f64(l.get("percent")) else { continue };
            let Some(name) = l.pointer("/scope/model/display_name").and_then(Value::as_str).filter(|n| !n.is_empty())
            else {
                continue;
            };
            let id = format!("week_{}", name.to_lowercase().replace(' ', "_"));
            if windows.iter().any(|w| w.id == id) {
                continue;
            }
            windows.push(LimitWindow {
                id,
                label: name.into(),
                percent: pct,
                resets_at: l.get("resets_at").and_then(Value::as_str).and_then(parse_iso_ms),
                minor: true,
            });
        }
    }
    if windows.is_empty() {
        return Err(LimitsError::BadResponse);
    }
    Ok(ProviderLimits {
        provider: ProviderId::Claude,
        plan: plan(
            oauth.get("subscriptionType").and_then(Value::as_str),
            oauth.get("rateLimitTier").and_then(Value::as_str),
        ),
        windows,
        fetched_at: now,
    })
}

pub(crate) fn plan(sub: Option<&str>, tier: Option<&str>) -> Option<String> {
    let sub = sub.filter(|s| !s.is_empty())?;
    if sub.eq_ignore_ascii_case("max")
        && let Some(t) = tier
    {
        if t.contains("20x") {
            return Some("Max 20x".into());
        }
        if t.contains("5x") {
            return Some("Max 5x".into());
        }
    }
    Some(http::capitalized(sub))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_windows_and_fable() {
        let usage = json!({
            "five_hour": {"utilization": 12.5, "resets_at": "2026-01-01T05:00:00.123Z"},
            "seven_day": {"utilization": "40", "resets_at": "2026-01-05T00:00:00Z"},
            "seven_day_opus": {"utilization": 7, "resets_at": null},
            "seven_day_sonnet": {"resets_at": "2026-01-05T00:00:00Z"},
            "limits": [
                {"kind": "weekly_scoped", "percent": 33, "resets_at": "2026-01-05T00:00:00Z",
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "session", "percent": 1},
                {"kind": "weekly_scoped", "percent": 9, "scope": {"model": {"display_name": ""}}}
            ]
        });
        let oauth = json!({"subscriptionType": "max", "rateLimitTier": "default_claude_max_5x"});
        let l = parse_claude(&usage, &oauth, 1).unwrap();
        assert_eq!(l.plan.as_deref(), Some("Max 5x"));
        let ids: Vec<_> = l.windows.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["5h", "week", "week_opus", "week_fable"]);
        assert_eq!(l.windows[0].percent, 12.5);
        assert_eq!(l.windows[0].resets_at, Some(1_767_243_600_123));
        assert_eq!(l.windows[1].percent, 40.0);
        assert_eq!(l.windows[2].resets_at, None);
        assert!(l.windows[3].minor && !l.windows[0].minor);
        assert_eq!(l.windows[3].label, "Fable");
    }

    #[test]
    fn empty_is_bad_response() {
        assert_eq!(parse_claude(&json!({}), &json!({}), 0), Err(LimitsError::BadResponse));
    }

    #[test]
    fn plan_names() {
        assert_eq!(plan(Some("max"), Some("x_20x")).as_deref(), Some("Max 20x"));
        assert_eq!(plan(Some("max"), Some("x_5x")).as_deref(), Some("Max 5x"));
        assert_eq!(plan(Some("max"), None).as_deref(), Some("Max"));
        assert_eq!(plan(Some("pro"), Some("5x")).as_deref(), Some("Pro"));
        assert_eq!(plan(Some(""), None), None);
        assert_eq!(plan(None, None), None);
    }
}
