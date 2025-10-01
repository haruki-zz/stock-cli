use std::borrow::Cow;

use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// Accent color used for prompts, highlights, and status badges.
pub const ACCENT: Color = Color::Indexed(208);

/// Build a styled text block for headers.
pub fn header_text<'a>(text: impl Into<Cow<'a, str>>) -> Text<'a> {
    let owned = text.into().into_owned();
    Text::from(owned.bold().fg(ACCENT))
}

/// Produce a dimmed line for secondary descriptions and hints.
pub fn secondary_line<'a>(text: impl Into<Cow<'a, str>>) -> Line<'a> {
    let owned = text.into().into_owned();
    Line::from(owned.dim())
}

/// Dimmed text chunk for inline usage.
pub fn secondary_span<'a>(text: impl Into<Cow<'a, str>>) -> Span<'a> {
    let owned = text.into().into_owned();
    Span::from(owned).dim()
}

/// Apply the accent and bold modifiers for list selections.
pub fn selection_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}
