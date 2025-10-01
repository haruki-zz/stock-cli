use crate::error::Result;
use crate::ui::styles::{secondary_line, ACCENT};
use crate::ui::{
    components::utils::{centered_rect, split_vertical},
    TerminalGuard, UiRoute,
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

/// Prompt the user for a preset name inside the application's alternate screen.
pub fn run_save_preset_dialog() -> Result<Option<String>> {
    let mut guard = TerminalGuard::new()?;
    let mut buffer = String::new();
    let mut error: Option<&'static str> = None;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default().borders(Borders::ALL).title(format!(
                "{} — Enter preset name",
                UiRoute::SavePreset.title()
            ));
            f.render_widget(block.clone(), area);
            let inner = block.inner(area);

            let chunks = split_vertical(
                inner,
                &[
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ],
            );

            let instructions = Paragraph::new(secondary_line(
                "Allowed characters: letters, numbers, space, '-' and '_'",
            ));
            f.render_widget(instructions, chunks[0]);

            let mut display = buffer.clone();
            display.push('_');
            let input = Paragraph::new(display)
                .style(Style::default().fg(ACCENT))
                .block(Block::default().borders(Borders::ALL).title("Preset name"));
            f.render_widget(input, chunks[1]);

            let message = error.unwrap_or("Enter to save • Esc to cancel • Backspace delete");
            let message_widget = Paragraph::new(secondary_line(message));
            f.render_widget(message_widget, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(None);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let trimmed = buffer.trim();
                        if trimmed.is_empty() {
                            error = Some("Name cannot be empty");
                        } else {
                            guard.restore()?;
                            return Ok(Some(trimmed.to_string()));
                        }
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                        error = None;
                    }
                    KeyCode::Char(ch) => {
                        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_') {
                            buffer.push(ch);
                            error = None;
                        } else {
                            error = Some("Unsupported character");
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
