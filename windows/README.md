# Burn for Windows (work in progress)

Two crates:

- `burn-taskbar`: the widget embedded in the Windows 11 taskbar next to the notification area, a tray
  icon fallback and a placeholder panel. Still shows demo data.
- `burn-core`: the data layer, a port of the Mac app's `Sources/Burn/Data`: live limits for Claude,
  Codex and Cursor, usage import from local session logs and API-price costs. Per-record results match
  the Mac app exactly.

## Try it

Download the `burn-windows` artifact from the latest run of the **Windows** workflow.

- `burn-spike.exe`: the taskbar widget. Right-click it or the tray icon for **Next demo values**,
  **Open log** and **Quit Burn**. The log (`%LOCALAPPDATA%\Burn\spike.log`) lists the taskbar's
  windows and every placement.
- `burn-cli.exe`: run it from a terminal.
  - `burn-cli dump`: live limits of each provider, then usage per period.
  - `burn-cli import`: imports `%USERPROFILE%\.claude\projects` and `%USERPROFILE%\.codex\sessions`
    into `%LOCALAPPDATA%\Burn\usage.sqlite`, then prints usage per period.
  - `burn-cli json`: the same data as JSON.

Where Burn reads credentials (read only, tokens are never refreshed or printed):

| Provider | File |
|---|---|
| Claude | `%USERPROFILE%\.claude\.credentials.json` |
| Codex | `%USERPROFILE%\.codex\auth.json` |
| Cursor | `%APPDATA%\Cursor\User\globalStorage\state.vscdb` |

## Develop

```sh
cd windows
cargo test                                        # burn-core and the shared layout code, on macOS too
cargo run -p burn-core --bin burn-cli -- import   # real data on a Mac; the store goes to ~/.burn-dev
```

`cargo test` includes `tests/fixtures.rs`, which imports a fixture home and compares every row with
`expected-usage.tsv`, written by the Mac app's importer. After changing the fixtures or the Swift
parsers, regenerate it on a Mac with `scripts/update-usage-fixture.sh`.

`cargo clippy -p burn-taskbar --target x86_64-pc-windows-msvc` type-checks the widget from a Mac.
`burn-core` bundles SQLite (C), which needs the Windows SDK, so check it on CI: the **Windows**
workflow lints, tests and builds everything on `windows-latest`.
