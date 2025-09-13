use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::Path;

use crate::config:: Config;
use crate::database::StockDatabase;
use crate::ui::menu_main::{Menu, MenuAction};
use crate::action::{render_main_menu_full, find_latest_csv, do_update, do_show, do_set_thresholds, do_filter, do_load};
use crossterm::{cursor, terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand};

pub async fn run() -> Result<()> {
    let config_path = "config.json";
    let stock_codes_path = "stock_code.csv";
    let region = "CN";

    // Load configuration
    let config = Config::load(config_path).context("Failed to load configuration")?;

    let region_config = config
        .get_region_config(region)
        .context("Region not found in config")?
        .clone();

    let info_indices = config
        .get_valid_info_indices(region)
        .context("No valid info indices found")?;

    let mut thresholds = config.get_valid_thresholds(region).unwrap_or_default();

    // Load stock codes
    let stock_codes = load_stock_codes(stock_codes_path)?;

    // Create raw data directory
    let raw_data_dir = "raw_data";
    if !Path::new(raw_data_dir).exists() {
        fs::create_dir_all(raw_data_dir).context("Failed to create raw_data directory")?;
    }

    // Prepare database; load later based on user choice
    let mut database = StockDatabase::new(Vec::new());

    // Enter shared alternate screen + raw mode once
    {
        let mut out = std::io::stdout();
        out.execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
    }

    // Render main menu at top (full-screen clear once)
    let mut menu = Menu::new();
    render_main_menu_full(&mut menu)?;

    // Compute subcontent top row (below menu)
    let sub_top: u16 = {
        let menu_rows = menu.items.len() as u16;
        // banner: BANNER_HEIGHT (10) + gap (1); menu starts at 11, so sub_top after menu + one blank line
        10 + 1 + menu_rows + 2
    };

    // Initial previous-data prompt shown below the main menu
    if let Some((latest_path, latest_name)) = find_latest_csv(raw_data_dir) {
        let mut out = std::io::stdout();
        out.queue(cursor::MoveTo(0, sub_top))?;
        out.queue(terminal::Clear(ClearType::FromCursorDown))?;
        use std::io::Write;
        write!(
            out,
            "Found previous data: {}. Load it? [y/N]: \r\n",
            latest_name
        )?;
        out.flush()?;

        // Temporarily disable raw to read input
        terminal::disable_raw_mode()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        terminal::enable_raw_mode()?;
        let choice = input.trim().to_lowercase();

        out.queue(cursor::MoveTo(0, sub_top))?;
        out.queue(terminal::Clear(ClearType::FromCursorDown))?;
        if choice == "y" {
            match StockDatabase::load_from_csv(latest_path.to_str().unwrap_or("")) {
                Ok(db) => {
                    database = db;
                    menu.loaded_file = Some(latest_name.clone());
                    write!(out, "Data loaded from {}\r\n", latest_name)?;
                }
                Err(e) => {
                    write!(out, "Failed to load data: {}\r\n", e)?;
                }
            }
        } else {
            write!(out, "Skipped loading previous data.\r\n")?;
        }
        out.flush()?;

        // Redraw clean main menu
        render_main_menu_full(&mut menu)?;
    }

    // Main interactive loop sharing the same screen
    loop {
        // Ensure raw mode is enabled before capturing navigation input
        let _ = terminal::enable_raw_mode();
        let action = menu.choose_action()?;

        match action {
            MenuAction::Update => {
                do_update(
                    &mut database,
                    &mut menu,
                    raw_data_dir,
                    &stock_codes,
                    region_config.clone(),
                    info_indices.clone(),
                    sub_top,
                )
                .await?;
            }
            MenuAction::Show => {
                do_show(&database, &mut menu, sub_top)?;
            }
            MenuAction::SetThresholds => {
                do_set_thresholds(&mut thresholds, &mut menu, sub_top)?;
            }
            MenuAction::Filter => {
                do_filter(&database, &thresholds, &mut menu, sub_top)?;
            }
            MenuAction::Load => {
                do_load(&mut database, &mut menu, raw_data_dir, sub_top)?;
            }
            MenuAction::Exit => {
                println!("Goodbye.");
                break;
            }
        }
    }

    // Cleanup screen
    {
        let mut out = std::io::stdout();
        let _ = terminal::disable_raw_mode();
        let _ = out.execute(LeaveAlternateScreen);
    }

    Ok(())
}

fn load_stock_codes(file_path: &str) -> Result<Vec<String>> {
    if !Path::new(file_path).exists() {
        anyhow::bail!("Stock codes file not found: {}", file_path);
    }

    let mut reader = csv::Reader::from_path(file_path).context("Failed to open stock codes file")?;

    let mut codes = Vec::new();
    for result in reader.records() {
        let record = result.context("Failed to read CSV record")?;
        if let Some(code) = record.get(0) {
            codes.push(code.to_string());
        }
    }

    if codes.is_empty() {
        anyhow::bail!("Stock codes file is empty: {}", file_path);
    }

    Ok(codes)
}
