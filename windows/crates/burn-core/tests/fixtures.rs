//! Imports the fixture home and compares every stored row with what the Mac app's importer
//! produced for the same files (`expected-usage.tsv`, see `windows/scripts/update-usage-fixture.sh`).

use burn_core::database::UsageDatabase;
use burn_core::importer;
use burn_core::{Paths, Period};
use rusqlite::Connection;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("burn-core-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[derive(Debug, PartialEq)]
enum Cell {
    Text(String),
    Num(f64),
}

/// Rows as the Swift importer wrote them; costs parsed back to the exact doubles.
fn expected() -> Vec<Vec<Cell>> {
    let tsv = std::fs::read_to_string(fixtures().join("expected-usage.tsv")).unwrap();
    tsv.lines()
        .map(|line| {
            line.split('\t')
                .enumerate()
                .map(|(i, v)| match i {
                    12 | 15 if !v.is_empty() => Cell::Num(v.parse().unwrap()),
                    _ => Cell::Text(v.to_string()),
                })
                .collect()
        })
        .collect()
}

fn actual(db: &Connection, home: &str) -> Vec<Vec<Cell>> {
    let norm = |s: String| s.replace(home, "<HOME>").replace('\\', "/");
    let mut stmt = db
        .prepare(
            "SELECT id, provider, model, ts_ms, ifnull(session_id, ''), ifnull(project, ''), input, cache_read, cache_write, \
             output, reasoning, speed, reported_cost, ifnull(rate_model, ''), ifnull(tier_turn, ''), cost_usd, origin, source \
             FROM usage",
        )
        .unwrap();
    let mut rows: Vec<Vec<Cell>> = stmt
        .query_map([], |r| {
            let mut out = Vec::new();
            for i in 0..18 {
                out.push(match i {
                    12 => r.get::<_, Option<f64>>(i)?.map(Cell::Num).unwrap_or(Cell::Text(String::new())),
                    15 => Cell::Num(r.get(i)?),
                    3 | 6..=10 => Cell::Text(r.get::<_, i64>(i)?.to_string()),
                    0 | 17 => Cell::Text(norm(r.get(i)?)),
                    _ => Cell::Text(r.get(i)?),
                });
            }
            Ok(out)
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    // Same order as the TSV (`ORDER BY id`, SQLite's binary collation), after normalizing paths.
    rows.sort_by(|a, b| match (&a[0], &b[0]) {
        (Cell::Text(x), Cell::Text(y)) => x.as_bytes().cmp(y.as_bytes()),
        _ => std::cmp::Ordering::Equal,
    });
    rows
}

#[test]
fn import_matches_the_mac_app() {
    let home = fixtures().join("home");
    let data = temp_dir("fixtures");
    let paths = Paths::with_home(&home, &data);
    let first = importer::run(&paths, &|_| {}).unwrap();
    let db = Connection::open(paths.db_path()).unwrap();
    let want = expected();
    let got = actual(&db, &home.to_string_lossy());
    assert_eq!(got.len(), want.len(), "row count");
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g, w);
    }
    assert_eq!(first.total_rows, want.len() as i64);

    // A second run finds nothing new and changes nothing.
    let second = importer::run(&paths, &|_| {}).unwrap();
    assert_eq!((second.parsed_files, second.new_rows), (0, 0));
    assert_eq!(actual(&db, &home.to_string_lossy()), want);

    // All time covers every row.
    let all = UsageDatabase::new(paths.db_path()).summary(Period::All).unwrap();
    let total: f64 = want.iter().map(|r| if let Cell::Num(c) = r[15] { c } else { 0.0 }).sum();
    assert!((all.cost - total).abs() < 1e-12, "{} vs {total}", all.cost);
    assert!((all.buckets.iter().map(|b| b.total()).sum::<f64>() - all.cost).abs() < 1e-12);
    let _ = std::fs::remove_dir_all(&data);
}
