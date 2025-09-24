/// Logical actions triggered from the main menu screen.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Update,
    SetThresholds,
    Filter,
    Load,
    Exit,
}
