/// Central routing types for the TUI flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRoute {
    MainMenu,
    FiltersMenu,
    Results,
    CsvPicker,
    PresetPicker,
    SavePreset,
    Thresholds,
    FetchProgress,
    MarketPicker,
    Exit,
}

impl UiRoute {
    /// Human readable label used by headers and logs.
    pub fn title(self) -> &'static str {
        match self {
            UiRoute::MainMenu => "Main Menu",
            UiRoute::FiltersMenu => "Filters",
            UiRoute::Results => "Filtered Results",
            UiRoute::CsvPicker => "Load CSV",
            UiRoute::PresetPicker => "Load Filters",
            UiRoute::SavePreset => "Save Filters",
            UiRoute::Thresholds => "Edit Thresholds",
            UiRoute::FetchProgress => "Fetching Data",
            UiRoute::MarketPicker => "Select Market",
            UiRoute::Exit => "Exit",
        }
    }
}

/// Navigation outcomes from the main menu screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Update,
    Filter,
    Filters,
    Load,
    SwitchRegion,
    Exit,
}

/// Secondary menu outcomes when managing filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMenuAction {
    Adjust,
    Save,
    Load,
    Back,
}
