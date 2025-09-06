use anyhow::Result;
use crossterm::{
    cursor, event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, execute, queue, style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor}, terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand
};
use tokio::time::sleep;
use std::{io::{stdout, Write}, mem::take, usize};
use unicode_width::UnicodeWidthStr;

const BANNER_HEIGHT: u16 = 10;
const MENU_GAP: u16 = 1;

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

    fn menu_top() -> u16 {
        BANNER_HEIGHT + MENU_GAP
    }

    pub fn show_banner(&self) -> Result<()> {
        let (cols, _) = terminal::size().unwrap_or((80, 24));
        let mut stdout = stdout();
        
        // Clear screen and hide cursor first
        stdout.queue(terminal::Clear(ClearType::All))?;
        stdout.queue(cursor::Hide)?;

        let line = "#".to_string() + &" ".repeat(1) + &"-".repeat(cols.saturating_sub(4) as usize) + &" ".repeat(1) + "#";

        // Queue all banner lines for batch processing
        stdout.queue(cursor::MoveTo(0, 0))?;
        stdout.queue(Print(&line))?;
        stdout.queue(cursor::MoveTo(0, 1))?;
        stdout.queue(Print("# Stock Information Fetcher (Rust Edition)"))?;
        stdout.queue(cursor::MoveTo(0, 2))?;
        stdout.queue(Print("# Author: haruki-zhang"))?;
        stdout.queue(cursor::MoveTo(0, 3))?;
        stdout.queue(Print("# FOR PERSONAL USE ONLY"))?;
        stdout.queue(cursor::MoveTo(0, 4))?;
        stdout.queue(Print("#"))?;
        stdout.queue(cursor::MoveTo(0, 5))?;
        stdout.queue(Print("# Project created on: 2024/10/02"))?;
        stdout.queue(cursor::MoveTo(0, 6))?;
        stdout.queue(Print(format!("# Executing date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"))))?;
        stdout.queue(cursor::MoveTo(0, 7))?;
        stdout.queue(Print("#"))?;
        stdout.queue(cursor::MoveTo(0, 8))?;
        stdout.queue(Print("# Use ↑/↓ arrows to navigate, Enter to select, Esc/Ctrl+C to exit"))?;
        stdout.queue(cursor::MoveTo(0, 9))?;
        stdout.queue(Print(&line))?;
       
        // Flush all queued operations at once
        stdout.flush()?;
        Ok(())
    }

    pub fn display(&self) -> Result<()> {
        let (cols, _) = terminal::size().unwrap_or((80, 24));
        let mut stdout = stdout();

        let max_label_width = self.items.iter()
            .map(|it| UnicodeWidthStr::width(it.label.as_str()))
            .max().unwrap_or(0);
        
        for (index, item) in self.items.iter().enumerate() {
            let y = Self::menu_top() + index as u16;
            
            stdout.queue(cursor::MoveTo(0, y))?;
            stdout.queue(terminal::Clear(ClearType::CurrentLine))?;

            let label_w = UnicodeWidthStr::width(item.label.as_str());
            let pad = max_label_width.saturating_sub(label_w);
            let padded_label = format!("{}{}", item.label, " ".repeat(pad));

            let head = if index == self.selected_index { "► " } else { "  " };
            let tail = format!("   - {}", item.description);

            stdout.queue(Print(head))?;

            if index == self.selected_index {
                stdout.queue(SetAttribute(Attribute::Reverse))?;
            }
            stdout.queue(Print(&padded_label))?;
            if index == self.selected_index {
                stdout.queue(SetAttribute(Attribute::Reset))?;
            }

            stdout.queue(Print(&tail))?;

            let used_w = UnicodeWidthStr::width(
                format!("{head}{padded_label}{tail}").as_str()
            );
            let term_cols = cols as usize;
            let fill = term_cols.saturating_sub(used_w);
            if fill > 0 {
                stdout.queue(Print(" ".repeat(fill)))?;
            }
        }

        // Flush all queued operations at once
        stdout.flush()?;
        Ok(())
    }

    pub fn navigate(&mut self) -> Result<MenuAction> {
        let mut out = stdout();
        out.execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;

        self.show_banner()?;
        self.display()?;

        let mut result: Option<MenuAction> = None;
        loop {
            match event::read()? {
                Event::Key(KeyEvent { code, modifiers, kind, .. }) => {
                    if matches!(kind, event::KeyEventKind::Release) {
                        continue;
                    }
                    match code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.selected_index = 
                                (self.selected_index + self.items.len() - 1) % self.items.len();
                            self.display()?;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.selected_index = (self.selected_index + 1) % self.items.len();
                            self.display()?;
                        }
                        KeyCode::Home => {
                            self.selected_index = 0;
                            self.display()?;
                        }
                        KeyCode::End => {
                            self.selected_index = self.items.len() - 1;
                            self.display()?;
                        }
                        KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
                            let action = self.items[self.selected_index].action.clone();

                            terminal::disable_raw_mode()?;
                            let mut out = stdout();
                            out.execute(LeaveAlternateScreen)?;
                            out.execute(cursor::Show)?;

                            return Ok(action);
                        }                        
                        KeyCode::Esc => {
                            result = Some(MenuAction::Exit);
                            break;
                        }
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                            result = Some(MenuAction::Exit);
                            break;
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    self.show_banner()?;
                    self.display()?;
                }
                _ => {}
            }
        }
        terminal::disable_raw_mode()?;
        out.execute(LeaveAlternateScreen)?;
        out.execute(cursor::Show)?;
        Ok(result.expect("Unreachable: result is always set befor break."))
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = stdout().execute(cursor::Show);
    }
}

