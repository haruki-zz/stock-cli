You write only the most **concise, elegant, and efficient code**. 

### Generic
- Keep It Simple: prefer small, composable helpers over large monoliths.
- You Aren't Gonna Need It: add new configuration flags or UI routes only when a user story requires them.
- Keep things DRY: share fetch/persistence logic via `records`, `fetch`, and `utils` modules instead of cloning code into UI flows.
- Leave code cleaner than you found it: align naming, imports, and formatting with adjacent code before exiting the file.

### Modules, functions, components
- Single Responsibility: flows orchestrate UI, `records` handles persistence, `fetch` drives HTTP – avoid leaking concerns across layers.
- Hide Implementation Details: expose only the constructors/functions needed by other modules; keep helper structs private where possible.
- Prepare Data Early: normalise threshold maps with `ensure_metric_thresholds` before passing them to UI code.

### Async & network
- Use the provided concurrency helpers (`ensure_concurrency_limit`, semaphores) when adding new fetchers.
- Bubble up errors with context via `Context`/`AppError` so the TUI can surface actionable messages.
- If new providers are introduced, keep request-building helpers reusable and prefer sharing the existing fetch pipelines.

### UI patterns
- Build blocks with the existing helpers in `ui::styles` and `ui::components::utils` for consistency.
- Keep key hints visible; prefer integrating new shortcuts into the active screen rather than adding extra menus.
- Ensure any modal restores terminal state by using `TerminalGuard`.

### Naming Conventions
Modules and files stay `snake_case`; types and traits use `PascalCase`; functions, variables, and filter keys remain `snake_case`. Keep enums for UI states (for example, `AppState`) and prefer small helpers in `src/utils/` for shared formatting or parsing. Rely on `cargo fmt` and follow the ratatui styling helpers summarized in `docs/styles.md`.
