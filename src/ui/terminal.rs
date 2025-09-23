use anyhow::Result;
use crossterm::{execute, terminal, ExecutableCommand};
use ratatui::{backend::CrosstermBackend, Terminal};

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    restored: bool,
}

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<std::io::Stdout>> {
        &mut self.terminal
    }

    pub fn restore(&mut self) -> Result<()> {
        if !self.restored {
            self.terminal.show_cursor()?;
            self.terminal
                .backend_mut()
                .execute(terminal::LeaveAlternateScreen)?;
            terminal::disable_raw_mode()?;
            self.restored = true;
        }
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
