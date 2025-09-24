// Individual Ratatui screens and helpers exposed to the application layer.
pub mod chart;
pub mod csv_picker;
pub mod main_menu_ratatui;
pub mod menu_main;
pub mod progress;
pub mod results;
pub mod terminal;
pub mod thresholds;
pub mod utils;

// Re-export the entry points expected by the application layer to keep call sites tidy.
pub use csv_picker::run_csv_picker;
pub use main_menu_ratatui::run_main_menu;
pub use progress::{run_fetch_progress, FetchCancelled};
pub use results::run_results_table;
pub use terminal::TerminalGuard;
pub use thresholds::run_thresholds_editor;
