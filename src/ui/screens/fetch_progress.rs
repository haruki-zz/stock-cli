use crate::config::RegionConfig;
use crate::services::{AsyncStockFetcher, StockData};
use crate::storage::StockDatabase;
use crate::ui::{components::utils::centered_rect, TerminalGuard};
use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};

/// Marker error used when the user aborts the fetch mid-flight.
#[derive(Debug)]
pub struct FetchCancelled;

impl std::fmt::Display for FetchCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fetch cancelled by user")
    }
}

impl std::error::Error for FetchCancelled {}

pub async fn run_fetch_progress(
    snapshots_dir: &str,
    stock_codes: &[String],
    region_config: RegionConfig,
    static_names: std::collections::HashMap<String, String>,
) -> Result<(Vec<StockData>, String)> {
    let fetcher = AsyncStockFetcher::new(stock_codes.to_vec(), region_config, static_names);
    let progress = fetcher.progress_counter.clone();
    let total = fetcher.total_stocks;
    let handle = tokio::spawn(async move { fetcher.fetch_data().await });

    // Keep terminal raw/alternate state well-scoped to the progress screen.
    let mut guard = TerminalGuard::new()?;
    let mut cancelled = false;

    loop {
        let done = progress.load(std::sync::atomic::Ordering::SeqCst);
        let ratio = if total == 0 {
            0.0
        } else {
            (done as f64 / total as f64).clamp(0.0, 1.0)
        };

        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let area = centered_rect(60, 20, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Fetching latest data...");
            f.render_widget(block.clone(), area);
            let inner = block.inner(area);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            let label = format!(
                "Progress: {} / {} ({:.0}%)",
                done.min(total),
                total,
                ratio * 100.0
            );
            f.render_widget(
                Paragraph::new("Please wait while we fetch data").alignment(Alignment::Center),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(label).alignment(Alignment::Center),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("Esc to cancel")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center),
                chunks[2],
            );
        })?;

        if handle.is_finished() {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if matches!(k.code, KeyCode::Esc)
                    || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL))
                {
                    cancelled = true;
                    handle.abort();
                    break;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }

    if cancelled {
        guard.restore()?;
        let _ = handle.await;
        return Err(FetchCancelled.into());
    }

    let res = handle
        .await
        .map_err(|e| anyhow!("Fetch task failed: {}", e))?;
    let data = res?;
    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M");
    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", snapshots_dir, timestamp);
    database.save_to_csv(&filename)?;

    guard.terminal_mut().draw(|f| {
        let size = f.size();
        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title("Done");
        f.render_widget(block.clone(), area);
        let inner = block.inner(area);
        let msg = Paragraph::new(format!(
            "Fetched {} records. Saved to {}\nPress Enter to continue.",
            data.len(),
            filename
        ))
        .alignment(Alignment::Center);
        f.render_widget(msg, inner);
    })?;
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if matches!(
                    k.code,
                    KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r')
                ) {
                    break;
                }
                if matches!(k.code, KeyCode::Esc)
                    || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }
            }
        }
    }

    guard.restore()?;
    Ok((data, filename))
}
