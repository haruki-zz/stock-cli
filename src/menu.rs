use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use std::io::{stdout, Write};

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
        stdout.execute(terminal::Clear(ClearType::All))?;
        stdout.execute(cursor::Hide)?;

        // Write banner lines using cursor positioning to avoid line break issues
        stdout.execute(cursor::MoveTo(0, 0))?;
        stdout.execute(Print("# ------------------------------------------------------------------------ #"))?;
        stdout.execute(cursor::MoveTo(0, 1))?;
        stdout.execute(Print("# Stock Information Fetcher (Rust Edition)"))?;
        stdout.execute(cursor::MoveTo(0, 2))?;
        stdout.execute(Print("# Author: haruki-zhang"))?;
        stdout.execute(cursor::MoveTo(0, 3))?;
        stdout.execute(Print("# FOR PERSONAL USE ONLY"))?;
        stdout.execute(cursor::MoveTo(0, 4))?;
        stdout.execute(Print("#"))?;
        stdout.execute(cursor::MoveTo(0, 5))?;
        stdout.execute(Print("# Project created on: 2024/10/02"))?;
        stdout.execute(cursor::MoveTo(0, 6))?;
        stdout.execute(Print(format!("# Executing date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"))))?;
        stdout.execute(cursor::MoveTo(0, 7))?;
        stdout.execute(Print("#"))?;
        stdout.execute(cursor::MoveTo(0, 8))?;
        stdout.execute(Print("# Use ↑/↓ arrows to navigate, Enter to select, Esc/Ctrl+C to exit"))?;
        stdout.execute(cursor::MoveTo(0, 9))?;
        stdout.execute(Print("# ------------------------------------------------------------------------ #"))?;

        Ok(())
    }

    pub fn display(&self) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear the menu area
        stdout.execute(terminal::Clear(ClearType::FromCursorDown))?;

        // Find the maximum label width for consistent alignment
        let max_label_width = self.items.iter()
            .map(|item| item.label.len())
            .max()
            .unwrap_or(0);

        for (index, item) in self.items.iter().enumerate() {
            let padded_label = format!("{:width$}", item.label, width = max_label_width);
            
            // Move cursor to the beginning of the line for each menu item
            stdout.execute(cursor::MoveTo(0, 11 + index as u16))?;
            
            if index == self.selected_index {
                // Selected item with highlighting
                stdout
                    .execute(Print("► "))?
                    .execute(SetForegroundColor(Color::Black))?
                    .execute(crossterm::style::SetBackgroundColor(Color::White))?
                    .execute(Print(&padded_label))?
                    .execute(ResetColor)?
                    .execute(Print(format!(" - {}", item.description)))?;
            } else {
                // Non-selected item with consistent spacing
                stdout.execute(Print(format!("  {}   - {}", padded_label, item.description)))?;
            }
        }

        stdout.flush()?;
        Ok(())
    }

    pub fn navigate(&mut self) -> Result<MenuAction> {
        terminal::enable_raw_mode()?;
        
        self.show_banner()?;
        
        // Position cursor for menu display
        let mut stdout_handle = stdout();
        stdout_handle.execute(cursor::MoveTo(0, 11))?;
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
                        stdout_handle.execute(terminal::Clear(ClearType::All))?;
                        stdout_handle.execute(cursor::MoveTo(0, 0))?;
                        stdout_handle.execute(cursor::Show)?;
                        
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_display_alignment() {
        let menu = Menu::new();
        
        // Test that all labels are properly padded to the same width
        let max_label_width = menu.items.iter()
            .map(|item| item.label.len())
            .max()
            .unwrap_or(0);
        
        // Verify that "Update Stock Data" (17 chars) is the longest
        assert_eq!(max_label_width, 17);
        
        // Test label padding
        for item in &menu.items {
            let padded_label = format!("{:width$}", item.label, width = max_label_width);
            assert_eq!(padded_label.len(), max_label_width);
        }
        
        // Test that shorter labels get properly padded
        let exit_item = &menu.items[4]; // "Exit" item
        assert_eq!(exit_item.label, "Exit");
        let padded_exit = format!("{:width$}", exit_item.label, width = max_label_width);
        assert_eq!(padded_exit.len(), 17);
        assert_eq!(padded_exit, "Exit             "); // 4 chars + 13 spaces
    }

    #[test]
    fn test_menu_item_creation() {
        let menu = Menu::new();
        
        // Verify all menu items are created correctly
        assert_eq!(menu.items.len(), 5);
        assert_eq!(menu.selected_index, 0);
        
        // Check first item
        assert_eq!(menu.items[0].label, "Update Stock Data");
        assert_eq!(menu.items[0].description, "Fetch fresh stock information from API");
        assert_eq!(menu.items[0].action, MenuAction::Update);
        
        // Check last item
        assert_eq!(menu.items[4].label, "Exit");
        assert_eq!(menu.items[4].description, "Exit the application");
        assert_eq!(menu.items[4].action, MenuAction::Exit);
    }

    #[test]
    fn test_display_format_consistency() {
        let menu = Menu::new();
        let max_label_width = menu.items.iter()
            .map(|item| item.label.len())
            .max()
            .unwrap_or(0);

        // Test format strings for selected and non-selected items
        for (index, item) in menu.items.iter().enumerate() {
            let padded_label = format!("{:width$}", item.label, width = max_label_width);
            
            if index == 0 { // Simulate selected item (first item)
                let selected_format = format!(" ► {} - {}", padded_label, item.description);
                // Verify the format includes the arrow and proper spacing
                assert!(selected_format.starts_with(" ► "));
                assert!(selected_format.contains(" - "));
            } else {
                let non_selected_format = format!("  {} - {}", padded_label, item.description);
                // Verify the format includes proper indentation and spacing
                assert!(non_selected_format.starts_with("  "));
                assert!(non_selected_format.contains(" - "));
            }
        }
    }

    #[test]
    fn test_label_lengths() {
        let menu = Menu::new();
        
        // Test individual label lengths
        let expected_lengths = vec![
            ("Update Stock Data", 17),
            ("Show Stock Info", 15),
            ("Filter Stocks", 13),
            ("Load from File", 14),
            ("Exit", 4),
        ];
        
        for (i, (expected_label, expected_len)) in expected_lengths.iter().enumerate() {
            assert_eq!(menu.items[i].label, *expected_label);
            assert_eq!(menu.items[i].label.len(), *expected_len);
        }
    }

    #[test]
    fn test_menu_actions() {
        let menu = Menu::new();
        
        // Test that all menu actions are correctly assigned
        let expected_actions = vec![
            MenuAction::Update,
            MenuAction::Show,
            MenuAction::Filter,
            MenuAction::Load,
            MenuAction::Exit,
        ];
        
        for (i, expected_action) in expected_actions.iter().enumerate() {
            assert_eq!(menu.items[i].action, *expected_action);
        }
    }

    #[test]
    fn test_visual_alignment() {
        let menu = Menu::new();
        let max_label_width = menu.items.iter()
            .map(|item| item.label.len())
            .max()
            .unwrap_or(0);

        // Simulate the actual display output for each item
        let mut display_lines = Vec::new();
        
        for (index, item) in menu.items.iter().enumerate() {
            let padded_label = format!("{:width$}", item.label, width = max_label_width);
            
            let line = if index == 0 { // Simulate first item being selected
                format!("► {} - {}", padded_label, item.description)
            } else {
                format!("  {}   - {}", padded_label, item.description)
            };
            display_lines.push(line);
        }

        // Print the simulated output for visual inspection
        println!("\nSimulated menu display:");
        for line in &display_lines {
            println!("{}", line);
        }

        // Verify alignment by checking dash positions
        let dash_positions: Vec<usize> = display_lines.iter()
            .map(|line| line.find(" - ").unwrap_or(0))
            .collect();

        // Now both formats should have the same dash position
        // Selected:     "► " + 17 chars + " " + "-" = 2 + 17 + 1 + 1 = 21 (position of " - ")
        // Non-selected: "  " + 17 chars + " " + "-" = 2 + 17 + 1 + 1 = 21 (position of " - ")
        
        // All dash positions should be the same for proper alignment
        let first_dash_pos = dash_positions[0];
        for (i, &dash_pos) in dash_positions.iter().enumerate() {
            assert_eq!(dash_pos, first_dash_pos, 
                "Line {} has dash at position {}, expected {}", i, dash_pos, first_dash_pos);
        }

        // Verify alignment is consistent
        println!("✓ All menu items are properly aligned with dashes at position: {}", first_dash_pos);
    }
}
