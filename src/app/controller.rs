use std::path::Path;
use std::sync::Arc;

use crate::app::{market_registry::MarketRegistry, state::RegionState};
use crate::config::RegionConfig;
use crate::error::{AppError, Result};
use crate::ui::{
    run_csv_picker, run_fetch_progress, run_filters_menu, run_main_menu, run_market_picker,
    run_preset_picker, run_results_table, run_thresholds_editor, FilterMenuAction, MenuAction,
};
use crate::utils::sanitize_preset_name;

/// Coordinates configuration, region state, and TUI flows.
pub struct AppController {
    markets: Arc<MarketRegistry>,
}

enum ControllerOutcome {
    Exit,
    SwitchRegion(String),
}

impl AppController {
    pub fn new(markets: Arc<MarketRegistry>) -> Result<Self> {
        if markets.available_regions().is_empty() {
            return Err(AppError::message(
                "No regions configured in the application.",
            ));
        }
        Ok(Self { markets })
    }

    pub async fn run(self) -> Result<()> {
        let mut current_region = match self.select_initial_region()? {
            Some(code) => code,
            None => return Ok(()),
        };

        loop {
            let region_config = self.region_config(&current_region)?;

            let mut region_state = RegionState::new(region_config.clone()).await?;
            self.load_previous_snapshot(&mut region_state);

            if region_state.database().data.is_empty() {
                self.fetch_and_persist(
                    &mut region_state,
                    "Fetch cancelled.",
                    "Failed to fetch data",
                )
                .await;
            }

            match self
                .drive_region(
                    &mut region_state,
                    self.markets.available_regions().len() > 1,
                    &current_region,
                )
                .await?
            {
                ControllerOutcome::Exit => return Ok(()),
                ControllerOutcome::SwitchRegion(next) => {
                    current_region = next;
                }
            }
        }
    }

    async fn drive_region(
        &self,
        region_state: &mut RegionState,
        allow_region_switch: bool,
        current_region: &str,
    ) -> Result<ControllerOutcome> {
        loop {
            match run_main_menu(
                region_state.loaded_file(),
                allow_region_switch,
                &region_state.config().code,
                &region_state.config().name,
            )? {
                MenuAction::Update => {
                    self.fetch_and_persist(
                        region_state,
                        "Update cancelled.",
                        "Failed to refresh data",
                    )
                    .await;
                }
                MenuAction::Filter => {
                    let codes = region_state
                        .database()
                        .filter_stocks(region_state.thresholds());
                    run_results_table(region_state.config(), region_state.database(), &codes)?;
                }
                MenuAction::Filters => {
                    self.handle_filters(region_state)?;
                }
                MenuAction::Load => {
                    self.handle_snapshot_load(region_state)?;
                }
                MenuAction::SwitchRegion => {
                    if !allow_region_switch {
                        println!("Only one market configured; cannot switch regions.");
                        continue;
                    }

                    match self.prompt_region_switch(current_region)? {
                        Some(next_region) => {
                            return Ok(ControllerOutcome::SwitchRegion(next_region))
                        }
                        None => continue,
                    }
                }
                MenuAction::Exit => return Ok(ControllerOutcome::Exit),
            }
        }
    }

    fn handle_filters(&self, region_state: &mut RegionState) -> Result<()> {
        loop {
            match run_filters_menu()? {
                FilterMenuAction::Adjust => {
                    let presets_dir = region_state.records().presets_dir().to_path_buf();
                    let mut save_cb = move |raw_name: &str,
                                            thresholds: &std::collections::HashMap<
                        String,
                        crate::config::Threshold,
                    >|
                          -> Result<String> {
                        let trimmed = raw_name.trim();
                        if trimmed.is_empty() {
                            return Err(AppError::message("Preset name cannot be empty."));
                        }

                        let Some(file_name) = sanitize_preset_name(trimmed) else {
                            return Err(AppError::message(
                                "Preset name must contain letters, numbers, spaces, '-' or '_'.",
                            ));
                        };

                        let mut normalized = thresholds.clone();
                        crate::records::ensure_metric_thresholds(&mut normalized);
                        crate::records::presets::save_thresholds(
                            std::path::Path::new(&presets_dir),
                            &file_name,
                            &normalized,
                        )?;

                        Ok(file_name)
                    };

                    run_thresholds_editor(region_state.thresholds_mut(), Some(&mut save_cb))?;
                }
                FilterMenuAction::Load => {
                    let (_, filters_dir) = region_state.directories();
                    match run_preset_picker(&filters_dir)? {
                        Some(path) => {
                            match region_state
                                .records()
                                .load_threshold_preset(Path::new(&path))
                            {
                                Ok(loaded) => {
                                    region_state.set_thresholds(loaded);
                                    let name = Path::new(&path)
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(&path);
                                    println!("Applied filters from {}", name);
                                }
                                Err(err) => {
                                    eprintln!("Failed to load filters: {}", err);
                                }
                            }
                        }
                        None => {}
                    }
                }
                FilterMenuAction::Back => break,
            }
        }
        Ok(())
    }

    fn handle_snapshot_load(&self, region_state: &mut RegionState) -> Result<()> {
        let (snapshots_dir, _) = region_state.directories();
        if let Some(filename) = run_csv_picker(&snapshots_dir)? {
            match region_state.records().load_snapshot(&filename) {
                Ok(loaded) => {
                    region_state.replace_database(loaded);
                    println!("Loaded: {}", filename);
                    let name = Path::new(&filename)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&filename);
                    region_state.set_loaded_file(Some(name.to_string()));
                }
                Err(err) => eprintln!("Load failed for {}: {}", filename, err),
            }
        }
        Ok(())
    }

    async fn fetch_and_persist(
        &self,
        region_state: &mut RegionState,
        cancel_message: &str,
        error_message: &str,
    ) {
        match run_fetch_progress(
            region_state.stock_codes(),
            region_state.config().clone(),
            region_state.stock_names().clone(),
        )
        .await
        {
            Ok(data) => match region_state.apply_snapshot(data) {
                Ok(saved_path) => println!("Saved: {}", saved_path.display()),
                Err(err) => eprintln!("Failed to persist snapshot: {}", err),
            },
            Err(AppError::Cancelled) => println!("{}", cancel_message),
            Err(err) => eprintln!("{}: {}", error_message, err),
        }
    }

    fn load_previous_snapshot(&self, region_state: &mut RegionState) {
        match region_state.records().latest_snapshot() {
            Ok(Some((path, name))) => match region_state.records().load_snapshot(&path) {
                Ok(database) => {
                    println!(
                        "Loaded latest {} data from {}",
                        region_state.config().code,
                        name
                    );
                    region_state.replace_database(database);
                    region_state.set_loaded_file(Some(name));
                }
                Err(err) => eprintln!("Failed to load previous data: {}", err),
            },
            Ok(None) => {}
            Err(err) => eprintln!("Failed to inspect snapshots: {}", err),
        }
    }

    fn prompt_region_switch(&self, current_region: &str) -> Result<Option<String>> {
        let options = self.region_options();
        match run_market_picker(&options) {
            Ok(code) => {
                if code != current_region {
                    Ok(Some(code))
                } else {
                    Ok(None)
                }
            }
            Err(AppError::Cancelled) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn region_options(&self) -> Vec<(String, String)> {
        self.markets
            .available_regions()
            .into_iter()
            .map(|summary| (summary.code, summary.name))
            .collect()
    }

    fn select_initial_region(&self) -> Result<Option<String>> {
        let summaries = self.markets.available_regions();
        if summaries.is_empty() {
            return Err(AppError::message(
                "No regions configured in the application.",
            ));
        }

        if summaries.len() == 1 {
            return Ok(Some(summaries[0].code.clone()));
        }

        let options: Vec<(String, String)> = summaries
            .into_iter()
            .map(|summary| (summary.code, summary.name))
            .collect();

        match run_market_picker(&options) {
            Ok(code) => Ok(Some(code)),
            Err(AppError::Cancelled) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn region_config(&self, code: &str) -> Result<RegionConfig> {
        self.markets.ensure_region(code)
    }
}
