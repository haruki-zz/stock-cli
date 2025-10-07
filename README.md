# Stock CLI

A fast, terminal-based tool for fetching, viewing, and filtering stock data. It fetches live data, saves timestamped CSVs, and offers an intuitive menu-driven TUI.

## Installation

Prerequisites
- Rust toolchain (stable) with `cargo`

Steps
- Clone the repository and enter the folder
  - `git clone <repo-url> && cd stock-cli`
- Build a release binary
  - `cargo build --release`
- Prepare required files next to the binary (or run from project root)
  - `stock_code.csv` for the China A-share market (required, one code per line)
  - Additional market CSVs can be added under `assets/.markets/` when new regions are configured
- Run the program
  - `./target/release/stock-cli`

Notes
- `assets/snapshots/<market>/` is created on first fetch and stores timestamped CSVs per market.
- The app errors if the configured stock code file is missing (e.g., `stock_code.csv` for CN) or empty.

## Usage

Start the app
- `stock-cli` (or `cargo run` during development)
- Pick the desired market when prompted; data, presets, and raw CSVs are stored per market (`assets/snapshots/<market>/`, `assets/filters/<market>/`)

Global navigation
- ↑/↓ or j/k: move selection (vertical)
- ←/→ or h/l: move selection (horizontal, where available)
- Enter: confirm / drill down • Esc: back • Ctrl+C: exit

Menu actions
- Show Filtered — Browse stocks matching current filters
  - ↓/j and ↑/k move the highlight; PageDown/PageUp jump by a page; Home/End jump to first/last row
  - Press `s` to cycle the sort column and `d` to flip ascending/descending; the active header shows an arrow
  - Press Enter to toggle the inline price chart; ←/→ or Enter cycle across 1Y/6M/3M/1M/1W ranges; X closes the chart
  - When the chart is open it fills the lower half of the screen while the table continues to show the full metric columns; Esc returns to the menu
  - The chart header shows timeframe shortcuts; the detail panel lists price/turnover metrics while the chart is visible
- Filters — Manage thresholds
  - Set Filters — Adjust ranges using the editor
  - Save Filters — Store the current thresholds as a named preset (`assets/filters/`)
  - Load Filters — Pick a saved preset to apply immediately
- Set Filters — Adjust threshold ranges used for filtering
  - Third-level editor (inline modal):
    - Tab/↑/↓/j/k: switch between Lower and Upper
    - Type numbers (digits/./-), Backspace to edit, Enter to save, Esc to cancel
    - Values display with two decimals
- Refresh Data — Fetch latest data and save to `assets/snapshots/`
  - Progress screen shows “Please wait…” and a textual percentage; Enter to continue when done
- Load CSV — Pick a CSV from `assets/snapshots/` using the same keys (↑/↓/j/k, Enter/Esc)
- Switch Market — Change the active region without restarting (only shown when multiple regions are available)
- Quit — Exit

Tips
- On startup, if a recent CSV exists and you skip loading it, the app automatically fetches fresh data.
- After each action, press Enter to return to the main menu.

## Features

- Async fetching with progress and error handling (Tokio + anyhow)
- Ratatui-powered TUI with clear, consistent key bindings and selectable tables
- Inline historical charts (1Y/6M/3M/1M/1W) alongside filtered results
- CSV persistence with timestamped filenames under `assets/snapshots/`
- Powerful filtering by configurable thresholds
- Built-in CSV picker to load past datasets
- Live region switching without restarting when multiple markets are configured
- Unicode-friendly rendering for names and labels

Highlighted structure
- `src/application/` — App wiring and lifecycle
- `src/services/` — Async HTTP clients and historical data fetchers
- `src/storage/` — In-memory store, filtering logic, and CSV I/O
- `src/ui/` — Ratatui components (main menu, CSV picker, thresholds editor, progress, results, charts)
