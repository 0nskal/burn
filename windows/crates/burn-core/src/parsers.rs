//! Ports of `UsageParsers.swift` (itself a port of T3 Code's usage readers). Per-record tokens,
//! speed and reported cost must stay identical to the Swift version.

use crate::rates::ModelRates;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Speed {
    Standard,
    Fast,
    Ultrafast,
}

impl Speed {
    /// Index used by T3's scan cache.
    pub const CACHE_ORDER: [Speed; 3] = [Speed::Standard, Speed::Fast, Speed::Ultrafast];

    pub fn as_str(self) -> &'static str {
        match self {
            Speed::Standard => "standard",
            Speed::Fast => "fast",
            Speed::Ultrafast => "ultrafast",
        }
    }

    pub fn parse(s: &str) -> Option<Speed> {
        Self::CACHE_ORDER.into_iter().find(|v| v.as_str() == s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TokenTotals {
    pub uncached: i64,
    pub cached: i64,
    pub cache_creation: i64,
    pub output: i64,
    pub reasoning: i64,
}

impl TokenTotals {
    /// Reasoning is already inside output, so it is not part of the total.
    pub fn total(&self) -> i64 {
        self.uncached + self.cached + self.cache_creation + self.output
    }
}

/// One parsed usage record, before pricing.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageRecord {
    pub provider: String,
    pub ts_ms: i64,
    pub model: String,
    pub rate_model: Option<String>,
    pub session_id: String,
    pub project: Option<String>,
    pub totals: TokenTotals,
    pub reported_cost: Option<f64>,
    pub speed: Speed,
    pub dedupe_key: Option<String>,
    /// Codex turn whose service tier is unknown (resolved later from T3's database).
    pub tier_turn: Option<String>,
}

/// One usage record, ready to be written to the store.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageRow {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub ts_ms: i64,
    /// Local time, `%Y-%m-%dT%H:%M:%S`.
    pub ts: String,
    /// Local date, `YYYY-MM-DD`.
    pub day: String,
    pub session: Option<String>,
    pub project: Option<String>,
    pub totals: TokenTotals,
    pub speed: Speed,
    pub reported_cost: Option<f64>,
    pub rate_model: Option<String>,
    pub tier_turn: Option<String>,
    pub cost: f64,
    pub origin: String,
    pub source: String,
}

impl UsageRow {
    pub fn new(r: &UsageRecord, id: String, project: Option<&str>, origin: &str, source: &str, rates: &ModelRates) -> UsageRow {
        let _ = (r, id, project, origin, source, rates);
        todo!("port UsageRow.init")
    }
}

/// ISO-8601 to epoch ms, like the Swift `isoToMs`.
pub fn iso_to_ms(s: &str) -> Option<i64> {
    let _ = s;
    todo!()
}

pub fn parse_claude_records(path: &Path) -> io::Result<Vec<UsageRecord>> {
    let _ = path;
    todo!()
}

pub fn dedupe_within_file(records: Vec<UsageRecord>) -> Vec<UsageRecord> {
    let _ = records;
    todo!()
}

pub fn parse_codex_records(path: &Path) -> io::Result<Vec<UsageRecord>> {
    let _ = path;
    todo!()
}

/// Store ids that reproduce T3's dedupe. `path` is the source path as stored in the database.
pub fn make_rows(records: &[UsageRecord], path: &str, origin: &str, project: Option<&str>, rates: &ModelRates) -> Vec<UsageRow> {
    let _ = (records, path, origin, project, rates);
    todo!()
}

/// One transcript's records as T3 saved them.
#[derive(Clone, Debug)]
pub struct T3CacheFile {
    pub path: String,
    pub provider: String,
    pub size: i64,
    pub records: Vec<UsageRecord>,
}

/// Cache format version from a file name (`usage-scan-cache-v7.json` -> 7, unversioned -> 0).
pub fn t3_cache_version(name: &str) -> i64 {
    let _ = name;
    todo!()
}

pub fn parse_t3_cache(path: &Path) -> io::Result<Vec<T3CacheFile>> {
    let _ = path;
    todo!()
}
