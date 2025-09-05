use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use std::io::{stdout, Write};
use unicode_width::UnicodeWidthStr;

pub struct MenuItem {
    pub label: String,
    pub description: String,
    pub action: MenuAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Update,
    Show,
    Filter,
    Load,
    Exit,
}

pub struct Menu {
    pub items: Vec<MenuItem>,
    pub selected_index: usize,
}

impl Menu {
    /// Pad a string to a specific display width using Unicode-aware padding
    fn pad_string_unicode(s: &str, width: usize) -> String {
        let current_width = s.width();
        if current_width >= width {
            s.to_string()
        } else {
            let padding = width - current_width;
            format!("{}{}", s, " ".repeat(padding))
        }
    }

    pub fn new() -> Self {
        let items = vec![
            MenuItem {
                label: "Update Stock Data".to_string(),
                description: "Fetch fresh stock information from API".to_string(),
                action: MenuAction::Update,
            },
            MenuItem {
                label: "Show Stock Info".to_string(),
                description: "Display information for specific stock codes".to_string(),
                action: MenuAction::Show,
            },
            MenuItem {
                label: "Filter Stocks".to_string(),
                description: "Show stocks matching default thresholds".to_string(),
                action: MenuAction::Filter,
            },
            MenuItem {
                label: "Load from File".to_string(),
                description: "Load stock data from CSV file".to_string(),
                action: MenuAction::Load,
            },
            MenuItem {
                label: "Exit".to_string(),
                description: "Exit the application".to_string(),
                action: MenuAction::Exit,
            },
        ];

        Self {
            items,
            selected_index: 0,
        }
    }

    pub fn show_banner(&self) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear screen and hide cursor first
        stdout.execute(terminal::Clear(ClearType::All))?;
        stdout.execute(cursor::Hide)?;

        // Queue all banner lines for batch processing
        queue!(stdout, cursor::MoveTo(0, 0), Print("# ------------------------------------------------------------------------ #"))?;
        queue!(stdout, cursor::MoveTo(0, 1), Print("# Stock Information Fetcher (Rust Edition)"))?;
        queue!(stdout, cursor::MoveTo(0, 2), Print("# Author: haruki-zhang"))?;
        queue!(stdout, cursor::MoveTo(0, 3), Print("# FOR PERSONAL USE ONLY"))?;
        queue!(stdout, cursor::MoveTo(0, 4), Print("#"))?;
        queue!(stdout, cursor::MoveTo(0, 5), Print("# Project created on: 2024/10/02"))?;
        queue!(stdout, cursor::MoveTo(0, 6), Print(format!("# Executing date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"))))?;
        queue!(stdout, cursor::MoveTo(0, 7), Print("#"))?;
        queue!(stdout, cursor::MoveTo(0, 8), Print("# Use ↑/↓ arrows to navigate, Enter to select, Esc/Ctrl+C to exit"))?;
        queue!(stdout, cursor::MoveTo(0, 9), Print("# ------------------------------------------------------------------------ #"))?;
        
        // Flush all queued operations at once
        stdout.flush()?;
        Ok(())
    }

    pub fn display(&self) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear the menu area
        stdout.execute(terminal::Clear(ClearType::FromCursorDown))?;

        // Find the maximum label width for consistent alignment using Unicode width
        let max_label_width = self.items.iter()
            .map(|item| item.label.width())
            .max()
            .unwrap_or(0);

        for (index, item) in self.items.iter().enumerate() {
            let padded_label = Self::pad_string_unicode(&item.label, max_label_width);
            
            // Clear the entire line first to remove any artifacts
            queue!(stdout, cursor::MoveTo(0, 11 + index as u16))?;
            queue!(stdout, terminal::Clear(ClearType::UntilNewLine))?;
            
            if index == self.selected_index {
                // Selected item: "► " + padded_label + " - " + description
                queue!(stdout, Print("► "))?;
                queue!(stdout, SetForegroundColor(Color::Black))?;
                queue!(stdout, crossterm::style::SetBackgroundColor(Color::White))?;
                queue!(stdout, Print(&padded_label))?;
                queue!(stdout, ResetColor)?;
                queue!(stdout, Print(format!(" - {}", item.description)))?;
            } else {
                // Non-selected item: "  " + padded_label + " - " + description (same total width)
                queue!(stdout, Print(format!("  {} - {}", padded_label, item.description)))?;
            }
        }

        // Flush all queued operations at once
        stdout.flush()?;
        Ok(())
    }

    pub fn navigate(&mut self) -> Result<MenuAction> {
        terminal::enable_raw_mode()?;
        
        self.show_banner()?;
        
        // Position cursor for menu display
        let mut stdout_handle = stdout();
        queue!(stdout_handle, cursor::MoveTo(0, 11))?;
        stdout_handle.flush()?;
        self.display()?;

        loop {
            if let Event::Key(KeyEvent { 
                code, 
                modifiers, 
                kind: event::KeyEventKind::Press,
                .. 
            }) = event::read()? {
                match code {
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        } else {
                            self.selected_index = self.items.len() - 1;
                        }
                        self.display()?;
                    }
                    KeyCode::Down => {
                        if self.selected_index < self.items.len() - 1 {
                            self.selected_index += 1;
                        } else {
                            self.selected_index = 0;
                        }
                        self.display()?;
                    }
                    KeyCode::Enter => {
                        let action = self.items[self.selected_index].action.clone();
                        terminal::disable_raw_mode()?;
                        
                        // Clear screen and show cursor
                        let mut stdout_handle = stdout();
                        queue!(stdout_handle, terminal::Clear(ClearType::All))?;
                        queue!(stdout_handle, cursor::MoveTo(0, 0))?;
                        queue!(stdout_handle, cursor::Show)?;
                        stdout_handle.flush()?;
                        
                        return Ok(action);
                    }
                    KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        return Ok(MenuAction::Exit);
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        return Ok(MenuAction::Exit);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = stdout().execute(cursor::Show);
    }
}

