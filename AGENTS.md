# Repository Guidelines

## Project Structure & Module Organization
The CLI is a Rust crate with app entry in `src/main.rs` and domain modules under `src/app`, `src/config`, `src/fetch`, `src/records`, `src/ui`, and `src/utils`. UI widgets live in `src/ui`, while data ingestion and persistence sit in `src/fetch` and `src/records`. 
Assets land in `assets/snapshots/<market>/` and filter presets in `assets/filters/<market>/`. 
Docs live in `docs/` for architecture and styling notes. 
Keep new modules under `src/` and expose them via the nearest `mod.rs`.

### Core functions
- 1. Grabs live snapshots: loads tickers from `assets/.markets/<region>.csv`, pulls real-time quotes, save the newest dataset to CSV file in `assets/snapshots/<market>/`, and lets users filter those rows interactively in the TUI.
- 2. Builds price history views: spins a background worker that downloads up to 420 days of candles per selected stock and pipes the data into the Ratatui candlestick chart for multi-range analysis inside the results screen.

## Architecture
See `./docs/architecture.md`

## Build, Test, and Development Commands
`cargo run` runs the TUI with dev logging.
`cargo build --release` compiles an optimized binary to `target/release/stock-cli`.
`cargo fmt` applies formatting; run before committing.
`cargo clippy -- -D warnings` enforces lint-clean builds.
`cargo test` executes unit tests kept in `#[cfg(test)]` modules.
`./build_macos_intel_release.sh` cross-builds the macOS Intel artifact.

## Coding Principles

You write only the most **concise, elegant, and efficient code**. 

### Generic
- Keep It Simple: most systems work and are understood better if they are kept simple rather than made complex.
- You Aren't Gonna Need It: don't implement something until it is necessary.
- Keep things DRY (Don't Repeat Yourself): each significant piece of functionality in a program should be implemented in just one place in the source code.
- Leave code cleaner than you found it: When making changes to an existing codebase make sure it does not degrade the codebase quality.
### Modules, functions, classes, components, entities
- Single Responsibility Principle: every class should have a single responsibility, and that responsibility should be entirely encapsulated by the class.
- Hide Implementation Details: software module hides information (i.e. implementation details) by providing an interface, and not leak any unnecessary information
### Relationships between modules, functions, classes, components, entities
- Minimise coupling
- Prefer composition over inheritance
### Naming Conventions
Modules and files stay `snake_case`; types and traits use `PascalCase`; functions, variables, and filter keys remain `snake_case`. Keep enums for UI states (for example, `AppState`) and prefer small helpers in `src/utils/` for shared formatting or parsing. Rely on `cargo fmt` and follow the ratatui styling helpers summarized in `docs/styles.md`.

## TUI style conventions

See `./docs/styles.md`.

## TUI code conventions

- Use concise styling helpers from ratatui’s Stylize trait.
  - Basic spans: use "text".into()
  - Styled spans: use "text".red(), "text".green(), "text".magenta(), "text".dim(), etc.
  - Prefer these over constructing styles with `Span::styled` and `Style` directly.
  - Example: patch summary file lines
    - Desired: vec!["  └ ".into(), "M".red(), " ".dim(), "tui/src/app.rs".dim()]

### TUI Styling (ratatui)
- Prefer Stylize helpers: use "text".dim(), .bold(), .cyan(), .italic(), .underlined() instead of manual Style where possible.
- Prefer simple conversions: use "text".into() for spans and vec![…].into() for lines; when inference is ambiguous (e.g., Paragraph::new/Cell::from), use Line::from(spans) or Span::from(text).
- Computed styles: if the Style is computed at runtime, using `Span::styled` is OK (`Span::from(text).set_style(style)` is also acceptable).
- Avoid hardcoded white: do not use `.white()`; prefer the default foreground (no color).
- Chaining: combine helpers by chaining for readability (e.g., url.cyan().underlined()).
- Single items: prefer "text".into(); use Line::from(text) or Span::from(text) only when the target type isn’t obvious from context, or when using .into() would require extra type annotations.
- Building lines: use vec![…].into() to construct a Line when the target type is obvious and no extra type annotations are needed; otherwise use Line::from(vec![…]).
- Avoid churn: don’t refactor between equivalent forms (Span::styled ↔ set_style, Line::from ↔ .into()) without a clear readability or functional gain; follow file‑local conventions and do not introduce type annotations solely to satisfy .into().
- Compactness: prefer the form that stays on one line after rustfmt; if only one of Line::from(vec![…]) or vec![…].into() avoids wrapping, choose that. If both wrap, pick the one with fewer wrapped lines.

### Text wrapping
- Always use textwrap::wrap to wrap plain strings.
- If you have a ratatui Line and you want to wrap it, use the helpers in tui/src/wrapping.rs, e.g. word_wrap_lines / word_wrap_line.
- If you need to indent wrapped lines, use the initial_indent / subsequent_indent options from RtOptions if you can, rather than writing custom logic.
- If you have a list of lines and you need to prefix them all with some prefix (optionally different on the first vs subsequent lines), use the `prefix_lines` helper from line_utils.

## Testing Guidelines
Add focused unit tests beside the code under test using `mod <name>_tests`. Favor table-driven assertions for filters and request builders. New features should cover happy paths and error branches for fetch failures. Run `cargo test` locally and keep any fixture CSVs under `assets/snapshots/<market>/` when required; avoid committing generated output.

## Commit & Pull Request Guidelines
Commits follow the existing imperative, sentence-case style (`Add market switch prompt`). Keep subject lines under ~60 characters and explain intent in the body when behavior changes. Pull requests should: 1) summarize the user-facing impact, 2) list manual or automated tests run (`cargo test`, `cargo fmt`), 3) link related issues or docs, and 4) attach screenshots or GIFs for UI tweaks. Request review once CI is green and conflicts are resolved.
