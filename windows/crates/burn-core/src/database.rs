//! Port of `UsageDatabase.swift`: per-period summaries from Burn's usage store.

use crate::model::{BucketUnit, Period, ProviderId, UsageBucket, UsageSummary};
use crate::store;
use anyhow::Result;
use chrono::{DateTime, Datelike, Days, Local, NaiveDate, TimeZone};
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct UsageDatabase {
    path: PathBuf,
}

impl UsageDatabase {
    pub fn new(path: impl Into<PathBuf>) -> UsageDatabase {
        UsageDatabase { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// None until the first import has created the `usage` table.
    fn open(&self) -> Result<Option<Connection>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let db = store::open_ro(&self.path)?;
        let ready = db
            .query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'usage'", [], |_| Ok(()))
            .optional()?
            .is_some();
        Ok(ready.then_some(db))
    }

    /// True once at least one usage record has been imported.
    pub fn has_any_usage(&self) -> Result<bool> {
        let Some(db) = self.open()? else { return Ok(false) };
        Ok(db.query_row("SELECT 1 FROM usage LIMIT 1", [], |_| Ok(())).optional()?.is_some())
    }

    pub fn summary(&self, period: Period) -> Result<UsageSummary> {
        self.summary_at(period, Local::now())
    }

    /// Weeks start on Monday. Buckets are contiguous and zero-filled; a day bucket is 23 or 25 hours on DST days.
    pub fn summary_at(&self, period: Period, now: DateTime<Local>) -> Result<UsageSummary> {
        // No store yet (first launch): behave like an empty one.
        let db = self.open()?;
        let today = now.date_naive();
        let tomorrow = start_of_day(today + Days::new(1));

        let (unit, starts, end) = match period {
            Period::Today => {
                let start = start_of_day(today);
                let mut starts = Vec::new();
                let mut cursor = start;
                while cursor < tomorrow {
                    starts.push(cursor);
                    cursor += 3_600_000;
                }
                (BucketUnit::Hour, starts, tomorrow)
            }
            Period::Week => (BucketUnit::Day, day_starts(today - Days::new(6), today, 1), tomorrow),
            Period::Month => (BucketUnit::Day, day_starts(today - Days::new(29), today, 1), tomorrow),
            Period::All => {
                let first: Option<i64> = match &db {
                    Some(db) => db.query_row("SELECT MIN(ts_ms) FROM usage", [], |r| r.get(0))?,
                    None => None,
                };
                let first_day = first
                    .and_then(|ms| Local.timestamp_millis_opt(ms).single())
                    .map(|d| d.min(now))
                    .unwrap_or(now)
                    .date_naive();
                let this_monday = monday_of(today);
                let starts = day_starts(monday_of(first_day), this_monday, 7);
                (BucketUnit::Week, starts, start_of_day(this_monday + Days::new(7)))
            }
        };

        let mut costs: Vec<BTreeMap<ProviderId, f64>> = vec![BTreeMap::new(); starts.len()];
        let mut by_provider: BTreeMap<ProviderId, f64> = BTreeMap::new();
        let mut tokens: i64 = 0;
        let mut total = 0.0;
        if let Some(db) = &db {
            let mut stmt = db.prepare(
                "SELECT ts_ms, provider, cost_usd, input + cache_read + cache_write + output FROM usage WHERE ts_ms >= ? AND ts_ms < ?",
            )?;
            let mut rows = stmt.query([starts[0], end])?;
            while let Some(r) = rows.next()? {
                let provider: Option<String> = r.get(1)?;
                let Some(p) = provider.as_deref().and_then(ProviderId::parse) else { continue };
                if p == ProviderId::Cursor {
                    continue;
                }
                let ts: i64 = r.get(0)?;
                let cost: f64 = r.get(2)?;
                // Last bucket whose start <= ts.
                let i = starts.partition_point(|&s| s <= ts).saturating_sub(1);
                *costs[i].entry(p).or_default() += cost;
                *by_provider.entry(p).or_default() += cost;
                total += cost;
                tokens += r.get::<_, i64>(3)?;
            }
        }

        let buckets = starts.into_iter().zip(costs).map(|(start, cost)| UsageBucket { start, cost }).collect();
        Ok(UsageSummary { period, cost: total, tokens, by_provider, buckets, unit })
    }
}

/// Epoch ms of the first moment of `day` in local time (midnight, or later when DST skips it).
fn start_of_day(day: NaiveDate) -> i64 {
    let mut t = day.and_hms_opt(0, 0, 0).expect("midnight");
    for _ in 0..48 {
        if let Some(d) = Local.from_local_datetime(&t).earliest() {
            return d.timestamp_millis();
        }
        t += chrono::Duration::minutes(30);
    }
    Local.from_utc_datetime(&day.and_hms_opt(0, 0, 0).expect("midnight")).timestamp_millis()
}

/// Start of each day from `first` through `last`, `step` days apart.
fn day_starts(first: NaiveDate, last: NaiveDate, step: u64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut d = first;
    while d <= last {
        out.push(start_of_day(d));
        d = d + Days::new(step);
    }
    out
}

fn monday_of(day: NaiveDate) -> NaiveDate {
    day - Days::new(u64::from(day.weekday().num_days_from_monday()))
}
