use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::{Config, ProviderConfig, RegionConfig};
use crate::error::{AppError, Context, Result};
use crate::fetch::fetch_japan_stock_codes;
use crate::records::{Records, StockDatabase};
use crate::ui::{
    run_csv_picker, run_fetch_progress, run_filters_menu, run_main_menu, run_market_picker,
    run_preset_picker, run_results_table, run_save_preset_dialog, run_thresholds_editor,
    FilterMenuAction, MenuAction,
};
use crate::utils::sanitize_preset_name;

pub async fn run() -> Result<()> {
    // Load configuration and region-specific metadata that drive fetching and filtering.
    let config = Config::builtin();
    let available_regions = config.available_regions();

    if available_regions.is_empty() {
        return Err(AppError::message(
            "No regions configured in the application.",
        ));
    }

    let mut current_region_code = if available_regions.len() == 1 {
        available_regions[0].code.clone()
    } else {
        let options: Vec<(String, String)> = available_regions
            .iter()
            .map(|region| (region.code.clone(), region.name.clone()))
            .collect();
        match run_market_picker(&options) {
            Ok(code) => code,
            Err(err) => {
                if err.to_string().to_lowercase().contains("cancelled") {
                    return Ok(());
                }
                return Err(err);
            }
        }
    };

    'app: loop {
        let region_config = config
            .get_region_config(&current_region_code)
            .context("Region not found in config")?
            .clone();

        let LoadedStockCodes {
            codes: stock_codes,
            names: stock_names,
        } = prepare_stock_codes(&region_config).await?;

        let records = Records::for_region(&region_config);
        records.prepare()?;

        let snapshots_dir = records.snapshots_dir().to_string_lossy().to_string();
        let filters_dir = records.presets_dir().to_string_lossy().to_string();

        let mut thresholds = records.initial_thresholds(&region_config);

        let mut database = StockDatabase::new(Vec::new());
        let mut loaded_file: Option<String> = None;
        if let Some((latest_path, latest_name)) = records.latest_snapshot()? {
            match records.load_snapshot(&latest_path) {
                Ok(db) => {
                    println!(
                        "Loaded latest {} data from {}",
                        region_config.code, latest_name
                    );
                    database = db;
                    loaded_file = Some(latest_name);
                }
                Err(e) => {
                    eprintln!("Failed to load previous data: {}", e);
                }
            }
        }

        if database.data.is_empty() {
            match run_fetch_progress(&stock_codes, region_config.clone(), stock_names.clone()).await
            {
                Ok(data) => {
                    database.update(data);
                    match records.save_snapshot(&database) {
                        Ok(saved_path) => {
                            println!("Saved: {}", saved_path.display());
                            if let Some(name) = saved_path.file_name().and_then(|s| s.to_str()) {
                                loaded_file = Some(name.to_string());
                            } else {
                                loaded_file = Some(saved_path.to_string_lossy().to_string());
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to persist snapshot: {}", err);
                        }
                    }
                }
                Err(err) => match err {
                    AppError::Cancelled => println!("Fetch cancelled."),
                    other => eprintln!("Failed to fetch data: {}", other),
                },
            }
        }

        loop {
            match run_main_menu(
                loaded_file.as_deref(),
                available_regions.len() > 1,
                &region_config.code,
                &region_config.name,
            )? {
                MenuAction::Update => {
                    match run_fetch_progress(
                        &stock_codes,
                        region_config.clone(),
                        stock_names.clone(),
                    )
                    .await
                    {
                        Ok(data) => {
                            database.update(data);
                            match records.save_snapshot(&database) {
                                Ok(saved_path) => {
                                    println!("Saved: {}", saved_path.display());
                                    if let Some(name) =
                                        saved_path.file_name().and_then(|s| s.to_str())
                                    {
                                        loaded_file = Some(name.to_string());
                                    } else {
                                        loaded_file =
                                            Some(saved_path.to_string_lossy().to_string());
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Failed to persist snapshot: {}", err);
                                }
                            }
                        }
                        Err(err) => match err {
                            AppError::Cancelled => println!("Update cancelled."),
                            other => eprintln!("Failed to refresh data: {}", other),
                        },
                    }
                }
                MenuAction::Filter => {
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
                                    if let Err(err) =
                                        records.save_threshold_preset(&file_name, &thresholds)
                                    {
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
                        FilterMenuAction::Load => match run_preset_picker(&filters_dir)? {
                            Some(path) => match records.load_threshold_preset(Path::new(&path)) {
                                Ok(loaded) => {
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
                    if let Some(filename) = run_csv_picker(&snapshots_dir)? {
                        match records.load_snapshot(&filename) {
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
                MenuAction::SwitchRegion => {
                    if available_regions.len() <= 1 {
                        println!("Only one market configured; cannot switch regions.");
                        continue;
                    }

                    let options: Vec<(String, String)> = available_regions
                        .iter()
                        .map(|region| (region.code.clone(), region.name.clone()))
                        .collect();

                    match run_market_picker(&options) {
                        Ok(new_code) => {
                            if new_code != current_region_code {
                                current_region_code = new_code;
                                continue 'app;
                            }
                        }
                        Err(err) => {
                            if !err.to_string().to_lowercase().contains("cancelled") {
                                return Err(err);
                            }
                        }
                    }
                }
                MenuAction::Exit => break 'app,
            }
        }
    }

    Ok(())
}

/// Read the stock codes file, collecting codes and optional names.
fn load_stock_codes(file_path: &Path) -> Result<LoadedStockCodes> {
    if !file_path.exists() {
        return Err(AppError::message(format!(
            "Stock codes file not found: {}",
            file_path.display()
        )));
    }

    let mut reader = csv::Reader::from_path(file_path)
        .with_context(|| format!("Failed to open stock codes file {}", file_path.display()))?;

    let mut codes = Vec::new();
    let mut names = HashMap::new();

    for result in reader.records() {
        let record = result.context("Failed to read CSV record")?;
        if let Some(code_raw) = record.get(0) {
            let code = code_raw.trim();
            if code.is_empty() {
                continue;
            }
            let code_string = code.to_string();
            if let Some(name_raw) = record.get(1) {
                let name = name_raw.trim();
                if !name.is_empty() {
                    names.insert(code_string.clone(), name.to_string());
                }
            }
            codes.push(code_string);
        }
    }

    if codes.is_empty() {
        return Err(AppError::message(format!(
            "Stock codes file is empty: {}",
            file_path.display()
        )));
    }

    Ok(LoadedStockCodes { codes, names })
}

struct LoadedStockCodes {
    codes: Vec<String>,
    names: HashMap<String, String>,
}

async fn prepare_stock_codes(region_config: &RegionConfig) -> Result<LoadedStockCodes> {
    let path = Path::new(&region_config.stock_code_file);

    if !path.exists() {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {}", parent.display()))?;
            }
        }

        match &region_config.provider {
            ProviderConfig::Stooq(_) => {
                let entries = fetch_japan_stock_codes().await?;
                write_stock_codes(path, &entries)?;
            }
            ProviderConfig::Tencent(_) => {
                return Err(AppError::message(format!(
                    "Stock codes file not found for region {}: {}",
                    region_config.code,
                    path.display()
                )));
            }
        }
    }

    load_stock_codes(path)
}

fn write_stock_codes(path: &Path, entries: &[(String, String)]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)
        .with_context(|| format!("Failed to create stock codes file {}", path.display()))?;

    writer.write_record(["code", "name"])?;
    for (code, name) in entries {
        writer.write_record([code, name])?;
    }
    writer.flush()?;
    Ok(())
}
