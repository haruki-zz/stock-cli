use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::storage::{
    ensure_metric_thresholds, load_threshold_preset, save_threshold_preset, StockDatabase,
};
use crate::ui::{
    run_csv_picker, run_fetch_progress, run_filters_menu, run_main_menu, run_preset_picker,
    run_results_table, run_save_preset_dialog, run_thresholds_editor, FetchCancelled,
    FilterMenuAction, MenuAction,
};

/// Find the most recently modified CSV file within the given directory.
fn find_latest_csv(dir: &str) -> Option<(std::path::PathBuf, String)> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut latest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("csv") {
            if let Ok(meta) = e.metadata() {
                if let Ok(modified) = meta.modified() {
                    if latest.as_ref().map(|(t, _)| &modified > t).unwrap_or(true) {
                        latest = Some((modified, p));
                    }
                }
            }
        }
    }
    latest.map(|(_, p)| {
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        (p, name)
    })
}

pub async fn run() -> Result<()> {
    let stock_codes_path = "stock_code.csv";
    let region = "CN";

    // Load configuration and region-specific metadata that drive fetching and filtering.
    let config = Config::builtin();

    let region_config = config
        .get_region_config(region)
        .context("Region not found in config")?
        .clone();

    let info_indices = config
        .get_valid_info_indices(region)
        .context("No valid info indices found")?;

    let mut thresholds = region_config.thre.clone();
    ensure_metric_thresholds(&mut thresholds);

    // Load stock codes
    let stock_codes = load_stock_codes(stock_codes_path)?;

    // Create data directories
    let raw_data_dir = "raw_data";
    if !Path::new(raw_data_dir).exists() {
        fs::create_dir_all(raw_data_dir).context("Failed to create raw_data directory")?;
    }
    let filters_dir = "filters";
    if !Path::new(filters_dir).exists() {
        fs::create_dir_all(filters_dir).context("Failed to create filters directory")?;
    }

    // Prepare database; load later based on user choice
    let mut database = StockDatabase::new(Vec::new());
    let mut loaded_file: Option<String> = None;
    // Fixed subcontent top row used by legacy action screens
    // Automatically load the most recent snapshot if available; otherwise fetch fresh data.
    if let Some((latest_path, latest_name)) = find_latest_csv(raw_data_dir) {
        match StockDatabase::load_from_csv(latest_path.to_str().unwrap_or("")) {
            Ok(db) => {
                println!("Loaded latest data from {}", latest_name);
                database = db;
                loaded_file = Some(latest_name);
            }
            Err(e) => {
                eprintln!("Failed to load previous data: {}", e);
            }
        }
    }

    if database.data.is_empty() {
        match run_fetch_progress(
            raw_data_dir,
            &stock_codes,
            region_config.clone(),
            info_indices.clone(),
        )
        .await
        {
            Ok((data, saved_file)) => {
                database.update(data);
                println!("Saved: {}", saved_file);
                if let Some(name) = std::path::Path::new(&saved_file)
                    .file_name()
                    .and_then(|s| s.to_str())
                {
                    loaded_file = Some(name.to_string());
                } else {
                    loaded_file = Some(saved_file);
                }
            }
            Err(err) => {
                if err.downcast_ref::<FetchCancelled>().is_some() {
                    println!("Fetch cancelled.");
                } else {
                    eprintln!("Failed to fetch data: {}", err);
                }
            }
        }
    }

    // Main interactive loop using Ratatui. Each menu action owns a dedicated screen so the
    // core loop can stay focused on state transitions rather than input handling details.
    loop {
        match run_main_menu(loaded_file.as_deref())? {
            MenuAction::Update => {
                match run_fetch_progress(
                    raw_data_dir,
                    &stock_codes,
                    region_config.clone(),
                    info_indices.clone(),
                )
                .await
                {
                    Ok((data, saved_file)) => {
                        database.update(data);
                        println!("Saved: {}", saved_file);
                        if let Some(name) = std::path::Path::new(&saved_file)
                            .file_name()
                            .and_then(|s| s.to_str())
                        {
                            loaded_file = Some(name.to_string());
                        } else {
                            loaded_file = Some(saved_file);
                        }
                    }
                    Err(err) => {
                        if err.downcast_ref::<FetchCancelled>().is_some() {
                            println!("Update cancelled.");
                        } else {
                            eprintln!("Failed to refresh data: {}", err);
                        }
                    }
                }
            }
            MenuAction::Filter => {
                // Precompute the matching codes; the results view stays read-only.
                let codes = database.filter_stocks(&thresholds);
                run_results_table(&database, &codes)?;
            }
            MenuAction::Filters => loop {
                match run_filters_menu()? {
                    FilterMenuAction::Adjust => {
                        run_thresholds_editor(&mut thresholds)?;
                    }
                    FilterMenuAction::Save => match run_save_preset_dialog()? {
                        Some(name) => match sanitize_preset_name(&name) {
                            Some(file_name) => {
                                if let Err(err) = save_threshold_preset(
                                    Path::new(filters_dir),
                                    &file_name,
                                    &thresholds,
                                ) {
                                    eprintln!("Failed to save filters: {}", err);
                                } else {
                                    println!("Filters saved as '{}'.", file_name);
                                }
                            }
                            None => println!(
                                "Preset name must contain letters, numbers, spaces, '-' or '_'."
                            ),
                        },
                        None => println!("Save filters cancelled."),
                    },
                    FilterMenuAction::Load => match run_preset_picker(filters_dir)? {
                        Some(path) => match load_threshold_preset(Path::new(&path)) {
                            Ok(mut loaded) => {
                                ensure_metric_thresholds(&mut loaded);
                                thresholds = loaded;
                                println!(
                                    "Applied filters from {}",
                                    Path::new(&path)
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(&path)
                                );
                            }
                            Err(err) => {
                                eprintln!("Failed to load filters: {}", err);
                            }
                        },
                        None => {}
                    },
                    FilterMenuAction::Back => break,
                }
            },
            MenuAction::Load => {
                if let Some(filename) = run_csv_picker(raw_data_dir)? {
                    match StockDatabase::load_from_csv(&filename) {
                        Ok(loaded_db) => {
                            database = loaded_db;
                            println!("Loaded: {}", filename);
                            if let Some(name) = std::path::Path::new(&filename)
                                .file_name()
                                .and_then(|s| s.to_str())
                            {
                                loaded_file = Some(name.to_string());
                            } else {
                                loaded_file = Some(filename);
                            }
                        }
                        Err(e) => {
                            eprintln!("Load failed for {}: {}", filename, e);
                        }
                    }
                }
            }
            MenuAction::Exit => break,
        }
    }

    Ok(())
}

/// Read the first column of the CSV into a list of tradable codes.
fn load_stock_codes(file_path: &str) -> Result<Vec<String>> {
    if !Path::new(file_path).exists() {
        anyhow::bail!("Stock codes file not found: {}", file_path);
    }

    let mut reader =
        csv::Reader::from_path(file_path).context("Failed to open stock codes file")?;

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

fn sanitize_preset_name(name: &str) -> Option<String> {
    let mut result = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch);
        } else if matches!(ch, ' ' | '-' | '_') {
            result.push(match ch {
                ' ' => '_',
                other => other,
            });
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result.to_lowercase())
    }
}
