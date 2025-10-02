# Refactor Roadmap

- [x] `src/fetch`: separate snapshot and history retrieval, adopting shared error handling and concurrency limits consistent with the new `fetch::snapshots` and `fetch::history` layout.
- [x] `src/records`: isolate persistence concerns (CSV snapshots and preset storage) behind a cohesive API, guaranteeing thresholds remain synchronized with configurable metrics.
- [x] `src/utils`: consolidate cross-cutting helpers for file management, text munging, and time formatting, keeping interface-only utilities here to reduce duplication.
- [x] `src/error.rs`: introduce a crate-wide error enum with `thiserror`, harmonizing error propagation between fetch, records, and UI layers.
- [x] `src/ui/components`: refactor reusable widgets (table, chart, terminal guard, helpers) to comply with the Stylize conventions and reduce repeated layout calculations.
- [x] `src/ui/flows`: rebuild each TUI screen (main menu, fetch progress, results, presets, thresholds) to consume the refactored state/controller interfaces while respecting routing rules.
- [x] `src/ui`: reorganize UI scaffolding to expose the routes and styling helpers specified in `docs/styles.md`, introducing `navigation`, `styles`, and a clean public surface in `mod.rs`.
- [x] `src/app`: split responsibilities into `bootstrap`, `controller`, and `state` modules, ensuring each coordinates configuration, data fetchers, and UI transitions per the architecture document.
- [x] `src/config`: centralize region metadata under `mod.rs`, extracting China/Japan specifics into dedicated modules and normalizing threshold serialization.
- [ ] `src/main.rs`: align the entrypoint with `app::bootstrap` by delegating startup, wiring logging, and trimming any direct business logic in favor of the controller-oriented flow described in `docs/architecture.md`.
