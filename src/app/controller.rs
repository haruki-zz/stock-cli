use std::path::Path;

use crate::app::state::RegionState;
use crate::config::Config;
use crate::error::{AppError, Context, Result};
use crate::ui::{
    run_csv_picker, run_fetch_progress, run_filters_menu, run_main_menu, run_market_picker,
    run_preset_picker, run_results_table, run_save_preset_dialog, run_thresholds_editor,
    FilterMenuAction, MenuAction,
};
use crate::utils::sanitize_preset_name;

/// Coordinates configuration, region state, and TUI flows.
pub struct AppController {
    config: Config,
}

enum ControllerOutcome {
    Exit,
    SwitchRegion(String),
}

impl AppController {
    pub fn new(config: Config) -> Result<Self> {
        if config.available_regions().is_empty() {
            return Err(AppError::message(
                "No regions configured in the application.",
            ));
        }
        Ok(Self { config })
    }

    pub async fn run(self) -> Result<()> {
        let regions = self.config.available_regions();
        let allow_region_switch = regions.len() > 1;

        let mut current_region = if allow_region_switch {
            let options = self.region_options();
            match run_market_picker(&options) {
                Ok(code) => code,
                Err(AppError::Cancelled) => return Ok(()),
                Err(err) => return Err(err),
            }
        } else {
            regions
                .get(0)
                .map(|region| region.code.clone())
                .context("Missing region configuration despite availability check")?
        };

        loop {
            let region_config = self
                .config
                .get_region_config(&current_region)
                .context("Region not found in config")?
                .clone();

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
                .drive_region(&mut region_state, allow_region_switch, &current_region)
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
                    run_results_table(region_state.database(), &codes)?;
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
                    run_thresholds_editor(region_state.thresholds_mut())?;
                }
                FilterMenuAction::Save => match run_save_preset_dialog()? {
                    Some(name) => match sanitize_preset_name(&name) {
                        Some(file_name) => {
                            if let Err(err) = region_state
                                .records()
                                .save_threshold_preset(&file_name, region_state.thresholds())
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
            Ok(data) => {
                region_state.database_mut().update(data);
                match region_state
                    .records()
                    .save_snapshot(region_state.database())
                {
                    Ok(saved_path) => {
                        println!("Saved: {}", saved_path.display());
                        let name = saved_path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| saved_path.to_string_lossy().to_string());
                        region_state.set_loaded_file(Some(name));
                    }
                    Err(err) => eprintln!("Failed to persist snapshot: {}", err),
                }
            }
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
        self.config
            .available_regions()
            .into_iter()
            .map(|region| (region.code.clone(), region.name.clone()))
            .collect()
    }
}
