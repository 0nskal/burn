#!/usr/bin/env bash
# Regenerates crates/burn-core/tests/fixtures/expected-usage.tsv with the Mac app's importer,
# the reference implementation. Run on a Mac after changing the fixtures or the Swift parsers;
# `cargo test -p burn-core --test fixtures` then checks the Rust port against it.
set -euo pipefail
cd "$(dirname "$0")/../.."
swift build -q
fixtures="$PWD/windows/crates/burn-core/tests/fixtures"
home="$fixtures/home"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
# The Swift app finds the home through CFFIXED_USER_HOME. Its rate table is the fixture's T3 copy
# (dated 2100, so always fresh); no network.
CFFIXED_USER_HOME="$home" BURN_DB="$tmp/usage.sqlite" .build/debug/Burn --import >/dev/null
sqlite3 -separator $'\t' "$tmp/usage.sqlite" "
  SELECT replace(id, '$home', '<HOME>'), provider, model, ts_ms, ifnull(session_id, ''), ifnull(project, ''),
         input, cache_read, cache_write, output, reasoning, speed, CASE WHEN reported_cost IS NULL THEN '' ELSE printf('%!.17g', reported_cost) END,
         ifnull(rate_model, ''), ifnull(tier_turn, ''), printf('%!.17g', cost_usd), origin, replace(source, '$home', '<HOME>')
  FROM usage ORDER BY id" > "$fixtures/expected-usage.tsv"
echo "wrote $(wc -l < "$fixtures/expected-usage.tsv" | tr -d ' ') rows to $fixtures/expected-usage.tsv"
