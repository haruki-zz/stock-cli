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
  - `stock_code.csv` (required, one code per line)
- Run the program
  - `./target/release/stock-cli`

Notes
- `raw_data/` is created on first fetch and stores timestamped CSVs.
- The app errors if `stock_code.csv` is missing or empty (no built‑in defaults).

## Usage

Start the app
- `stock-cli` (or `cargo run` during development)

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
- Set Filters — Adjust threshold ranges used for filtering
  - Third-level editor (inline modal):
    - Tab/↑/↓/j/k: switch between Lower and Upper
    - Type numbers (digits/./-), Backspace to edit, Enter to save, Esc to cancel
    - Values display with two decimals
- Refresh Data — Fetch latest data and save to `raw_data/`
  - Progress screen shows “Please wait…” and a textual percentage; Enter to continue when done
- Load CSV — Pick a CSV from `raw_data/` using the same keys (↑/↓/j/k, Enter/Esc)
- Quit — Exit

Tips
- On startup, if a recent CSV exists and you skip loading it, the app automatically fetches fresh data.
- After each action, press Enter to return to the main menu.

## Features

- Async fetching with progress and error handling (Tokio + anyhow)
- Ratatui-powered TUI with clear, consistent key bindings and selectable tables
- Inline historical charts (1Y/6M/3M/1M/1W) alongside filtered results
- CSV persistence with timestamped filenames under `raw_data/`
- Powerful filtering by configurable thresholds
- Built-in CSV picker to load past datasets
- Unicode-friendly rendering for names and labels

Highlighted structure
- `src/application/` — App wiring and lifecycle
- `src/services/` — Async HTTP clients and historical data fetchers
- `src/storage/` — In-memory store, filtering logic, and CSV I/O
- `src/ui/` — Ratatui components (main menu, CSV picker, thresholds editor, progress, results, charts)
