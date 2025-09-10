use anyhow::Result;
use crossterm::{
    cursor,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use crate::ui::navigate_list;
use std::io::{stdout, Write};
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
    SetThresholds,
    Filter,
    Load,
    Exit,
}

pub struct Menu {
    pub items: Vec<MenuItem>,
    pub selected_index: usize,
}

impl Menu {
    /// Creates a new menu with default stock application options
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
                label: "Set Thresholds".to_string(),
                description: "Configure filtering thresholds interactively".to_string(),
                action: MenuAction::SetThresholds,
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

    /// Calculate the top position of the menu based on banner height and gap
    fn menu_top() -> u16 {
        BANNER_HEIGHT + MENU_GAP
    }

    /// Get the maximum display width of all menu item labels
    fn max_label_width(&self) -> usize {
        self.items
            .iter()
            .map(|item| UnicodeWidthStr::width(item.label.as_str()))
            .max()
            .unwrap_or(0)
    }

    /// Create a padded version of a label with Unicode-aware width calculation
    fn pad_label(&self, label: &str, target_width: usize) -> String {
        let current_width = UnicodeWidthStr::width(label);
        let padding = target_width.saturating_sub(current_width);
        format!("{}{}", label, " ".repeat(padding))
    }

    /// Create the banner border line that adapts to terminal width
    fn create_border_line(terminal_width: u16) -> String {
        let inner_width = terminal_width.saturating_sub(4) as usize;
        format!("# {} #", "-".repeat(inner_width))
    }

    /// Display the application banner with border and information
    pub fn show_banner(&self) -> Result<()> {
        let (cols, _) = terminal::size().unwrap_or((80, 24));
        let mut stdout = stdout();

        // Hide cursor (do not clear whole screen so subcontent remains)
        stdout.queue(cursor::Hide)?;

        let border_line = Self::create_border_line(cols);

        let banner_lines = [
            &border_line,
            "# Stock Information Fetcher (Rust Edition)",
            "# Author: haruki-zhang", 
            "# FOR PERSONAL USE ONLY",
            "#",
            "# Project created on: 2024/10/02",
            &format!("# Executing date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M")),
            "#",
            "# Use ↑/↓ arrows to navigate, Enter to select, Esc/Ctrl+C to exit",
            &border_line,
        ];

        // Queue all banner lines for batch processing
        for (row, line) in banner_lines.iter().enumerate() {
            stdout.queue(cursor::MoveTo(0, row as u16))?;
            stdout.queue(terminal::Clear(ClearType::CurrentLine))?;
            stdout.queue(Print(line))?;
        }
       
        stdout.flush()?;
        Ok(())
    }

    /// Choose an action using shared screen (no terminal init/cleanup)
    pub fn choose_action(&mut self) -> Result<MenuAction> {
        let total = self.items.len();
        let initial = self.selected_index;
        let render = |sel: usize| -> anyhow::Result<()> {
            self.selected_index = sel;
            self.show_banner()?;
            self.display()?;
            Ok(())
        };
        let result = match navigate_list(initial, || total, render)? {
            Some(sel) => self.items[sel].action.clone(),
            None => MenuAction::Exit,
        };
        Ok(result)
    }

    /// Render a single menu item with proper formatting and highlighting
    fn render_menu_item(
        &self,
        stdout: &mut std::io::Stdout,
        index: usize,
        item: &MenuItem,
        max_label_width: usize,
        terminal_cols: usize,
    ) -> Result<()> {
        let y_position = Self::menu_top() + index as u16;
        
        stdout.queue(cursor::MoveTo(0, y_position))?;
        stdout.queue(terminal::Clear(ClearType::CurrentLine))?;

        let padded_label = self.pad_label(&item.label, max_label_width);
        let is_selected = index == self.selected_index;
        
        let prefix = if is_selected { "► " } else { "  " };
        let suffix = format!("   - {}", item.description);

        stdout.queue(Print(prefix))?;

        if is_selected {
            stdout.queue(SetAttribute(Attribute::Reverse))?;
        }
        stdout.queue(Print(&padded_label))?;
        if is_selected {
            stdout.queue(SetAttribute(Attribute::Reset))?;
        }

        stdout.queue(Print(&suffix))?;

        // Fill remaining space to prevent artifacts on terminal resize
        self.fill_line_remainder(stdout, &format!("{prefix}{padded_label}{suffix}"), terminal_cols)?;
        
        Ok(())
    }

    /// Fill the remainder of a line with spaces to prevent display artifacts
    fn fill_line_remainder(
        &self,
        stdout: &mut std::io::Stdout,
        content: &str,
        terminal_cols: usize,
    ) -> Result<()> {
        let used_width = UnicodeWidthStr::width(content);
        let remaining_space = terminal_cols.saturating_sub(used_width);
        
        if remaining_space > 0 {
            stdout.queue(Print(" ".repeat(remaining_space)))?;
        }
        
        Ok(())
    }

    /// Display the menu items with proper formatting and highlighting
    pub fn display(&self) -> Result<()> {
        let (cols, _) = terminal::size().unwrap_or((80, 24));
        let mut stdout = stdout();

        let max_label_width = self.max_label_width();
        let terminal_cols = cols as usize;
        
        for (index, item) in self.items.iter().enumerate() {
            self.render_menu_item(&mut stdout, index, item, max_label_width, terminal_cols)?;
        }

        stdout.flush()?;
        Ok(())
    }
 
    /// Initialize terminal for interactive navigation
    fn initialize_terminal(&self) -> Result<()> {
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        Ok(())
    }

    /// Cleanup terminal after navigation
    fn cleanup_terminal(&self) -> Result<()> {
        terminal::disable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(LeaveAlternateScreen)?;
        stdout.execute(cursor::Show)?;
        Ok(())
    }

    /// Main navigation loop with keyboard input handling
    pub fn navigate(&mut self) -> Result<MenuAction> {
        self.initialize_terminal()?;

        // Use shared navigator for selection; we manage drawing via closure
        let total = self.items.len();
        let initial = self.selected_index;
        let render = |sel: usize| -> anyhow::Result<()> {
            self.selected_index = sel;
            self.show_banner()?;
            self.display()?;
            Ok(())
        };
        let result = match navigate_list(initial, || total, render)? {
            Some(sel) => self.items[sel].action.clone(),
            None => MenuAction::Exit,
        };

        self.cleanup_terminal()?;
        Ok(result)
    }
}

impl Drop for Menu {
    /// Ensure terminal is properly cleaned up when Menu is dropped
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = stdout().execute(cursor::Show);
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}
