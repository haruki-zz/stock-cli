# Repository Guidelines

## Overview
- Entry point: `src/main.rs` boots `AppController`, which wires config, fetchers, `RegionState`, and the Ratatui UI.
- Core modules: `src/app` (control flow), `src/config` (region metadata), `src/fetch` (snapshots + history), `src/records` (CSV + preset storage), `src/ui` (components + flows), `src/utils` (shared helpers), `src/error.rs` (AppError & Result alias).
- Markets live under `assets/.markets/<code>.csv`; snapshots & presets are written to `assets/snapshots/<code>/` and `assets/filters/<code>/`.
- Market data uses Tencent endpoints end-to-end. If another region is introduced, follow the CN module as a template and register it in `config::mod`.

## Behaviour Highlights
- Snapshots: streamed concurrently with retries/autodetected firewall errors, then persisted as timestamped CSVs.
- History: uses Tencent's daily API to hydrate candlestick charts for the active market.
- Threshold editor: list + modal UI with inline validation. Users press **S / Ctrl+S** to open the preset save dialog and persist JSON presets without leaving the screen.
- Results view: sortable table (`s` cycles columns, `d` toggles order) with an optional multi-range candlestick chart (`Enter` to show/hide, ←/→ navigate ranges).
- CSV/preset pickers: re-use generic list navigation (↑/↓/j/k, Enter confirms, Esc backs out).

## Development Commands
- `cargo fmt` — formatting (run before committing).
- `cargo clippy -- -D warnings` — lint gate.
- `cargo test` — unit tests (fetch/history/ UI helpers).
- `cargo run` — launch TUI with dev logging.
- `cargo build --release` — production binary under `target/release/stock-cli`.
- `./build_macos_intel_release.sh` — optional cross-build script.

## Coding & Style
- Follow `docs/coding_principles.md` for layering, error handling, and async guidance.
- Use helpers from `src/ui/styles.rs` (`header_text`, `secondary_line`, `selection_style`, `ACCENT`) and `Stylize` chaining instead of manual `Style` construction. See `docs/styles.md`.
- Share new utilities via the nearest `mod.rs`; keep modules focused on one responsibility.
- Normalise thresholds with `ensure_metric_thresholds` before serialising or passing into UI flows.
- When adding providers or regions, keep shared helpers in `fetch`/`records`, and register the module in `config::mod` so the controller can surface it.

## Testing & Assets
- Add targeted tests alongside the code (e.g. `mod history_tests`). Use table-driven assertions for data transforms.
- Place generated CSVs/presets only inside `assets/snapshots/<code>/` and `assets/filters/<code>/`; avoid committing runtime output.
- Keep documentation (`docs/architecture.md`, `docs/styles.md`) current when workflows or modules change.
