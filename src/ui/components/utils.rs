use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = split_vertical(
        r,
        &[
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ],
    );
    let horizontal = split_horizontal(
        vertical[1],
        &[
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ],
    );
    horizontal[1]
}

pub fn split_vertical(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.iter().copied().collect::<Vec<_>>())
        .split(area)
        .to_vec()
}

pub fn split_horizontal(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints.iter().copied().collect::<Vec<_>>())
        .split(area)
        .to_vec()
}
