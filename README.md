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
  - `config.json` (required)
  - `stock_code.csv` (required, one code per line)
- Run the program
  - `./target/release/stock-cli`

Notes
- `raw_data/` is created on first fetch and stores timestamped CSVs.
- The app errors if `stock_code.csv` is missing or empty (no built‑in defaults).

## Usage

Start the app
- `stock-cli` (or `cargo run` during development)

Navigation
- ↑/↓ navigate • Enter select • Esc back • Ctrl+C exit

Menu actions
- Show Filtered — List stocks matching current filters
- Set Filters — Adjust threshold ranges used for filtering
- Refresh Data — Fetch latest data and save to CSV under `raw_data/`
- View Stocks — Enter space-separated codes to view details
- Load CSV — Pick an existing CSV from `raw_data/` and load it
- Quit — Exit

Tips
- On startup, if a recent CSV exists, you can load it with a single keypress.
- After each action, press Enter to return to the main menu.

## Features

- Async fetching with progress and error handling (Tokio + anyhow)
- Menu-driven TUI with clear, consistent key bindings
- CSV persistence with timestamped filenames under `raw_data/`
- Powerful filtering by configurable thresholds
- Built-in CSV picker to load past datasets
- Unicode-friendly rendering for names and labels

Highlighted structure
- `src/fetcher.rs` — Async HTTP client and JSON parsing
- `src/database.rs` — In-memory store, display, filtering, CSV I/O
- `src/ui/` — TUI components (`menu_main`, `menu_sub_threshold_setting`, `menu_sub_load_csv`, `select`, `navigation`)
- `src/action.rs` — Encapsulated implementations for menu actions
- `src/app.rs` — App wiring and lifecycle
