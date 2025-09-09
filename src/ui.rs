use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

/// Generic list navigation loop without touching raw mode.
/// Caller is responsible for enabling/disabling raw mode and rendering.
///
/// - `selected`: initial selected index
/// - `total`: closure returning total number of items
/// - `render`: called after any state change and on resize with current selection
///
/// Returns Some(selected_index) on Enter, None on Esc.
pub fn navigate_list<FTotal, FRender>(
    mut selected: usize,
    total: FTotal,
    mut render: FRender,
) -> Result<Option<usize>>
where
    FTotal: Fn() -> usize,
    FRender: FnMut(usize) -> Result<()>,
{
    // Initial draw
    render(selected)?;

    loop {
        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, kind, .. }) => {
                // Ignore key release events
                if matches!(kind, event::KeyEventKind::Release) {
                    continue;
                }
                let count = total().max(1);
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = (selected + count - 1) % count;
                        render(selected)?;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % count;
                        render(selected)?;
                    }
                    KeyCode::Home => {
                        selected = 0;
                        render(selected)?;
                    }
                    KeyCode::End => {
                        selected = count - 1;
                        render(selected)?;
                    }
                    KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                        return Ok(Some(selected));
                    }
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None)
                    }
                    _ => {}
                }
            }
            Event::Resize(_, _) => {
                render(selected)?;
            }
            _ => {}
        }
    }
}

