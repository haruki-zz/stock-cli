use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::{Config, ProviderConfig, RegionConfig};
use crate::services::fetch_japan_stock_codes;
use crate::storage::{
    ensure_metric_thresholds, load_threshold_preset, save_threshold_preset, StockDatabase,
};
use crate::ui::{
    run_csv_picker, run_fetch_progress, run_filters_menu, run_main_menu, run_market_picker,
    run_preset_picker, run_results_table, run_save_preset_dialog, run_thresholds_editor,
    FetchCancelled, FilterMenuAction, MenuAction,
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
    // Load configuration and region-specific metadata that drive fetching and filtering.
    let config = Config::builtin();
    let available_regions = config.available_regions();

    if available_regions.is_empty() {
        anyhow::bail!("No regions configured in the application.");
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

        let mut thresholds = region_config.thresholds.clone();
        ensure_metric_thresholds(&mut thresholds);

        // Create per-market data directories
        let snapshots_dir = format!("assets/snapshots/{}", region_config.code.to_lowercase());
        if !Path::new(&snapshots_dir).exists() {
            fs::create_dir_all(&snapshots_dir)
                .with_context(|| format!("Failed to create directory {}", snapshots_dir))?;
        }
        let filters_dir = format!("assets/filters/{}", region_config.code.to_lowercase());
        if !Path::new(&filters_dir).exists() {
            fs::create_dir_all(&filters_dir)
                .with_context(|| format!("Failed to create directory {}", filters_dir))?;
        }

        // Prepare database; load later based on user choice
        let mut database = StockDatabase::new(Vec::new());
        let mut loaded_file: Option<String> = None;
        if let Some((latest_path, latest_name)) = find_latest_csv(&snapshots_dir) {
            match StockDatabase::load_from_csv(latest_path.to_str().unwrap_or("")) {
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
            match run_fetch_progress(
                &snapshots_dir,
                &stock_codes,
                region_config.clone(),
                stock_names.clone(),
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

        loop {
            match run_main_menu(
                loaded_file.as_deref(),
                available_regions.len() > 1,
                &region_config.code,
                &region_config.name,
            )? {
                MenuAction::Update => {
                    match run_fetch_progress(
                        &snapshots_dir,
                        &stock_codes,
                        region_config.clone(),
                        stock_names.clone(),
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
                                        Path::new(&filters_dir),
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
                        FilterMenuAction::Load => match run_preset_picker(&filters_dir)? {
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
                    if let Some(filename) = run_csv_picker(&snapshots_dir)? {
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
                            if !err
                                .to_string()
                                .to_lowercase()
                                .contains("cancelled")
                            {
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
        anyhow::bail!("Stock codes file not found: {}", file_path.display());
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
        anyhow::bail!("Stock codes file is empty: {}", file_path.display());
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
                anyhow::bail!(
                    "Stock codes file not found for region {}: {}",
                    region_config.code,
                    path.display()
                );
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
