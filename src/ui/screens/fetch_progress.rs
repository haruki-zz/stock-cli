use crate::config::RegionConfig;
use crate::error::{AppError, Result};
use crate::fetch::{SnapshotFetcher, StockData};
use crate::ui::{
    components::utils::{centered_rect, split_vertical},
    TerminalGuard,
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::Stylize;
use ratatui::{prelude::*, widgets::*};

pub async fn run_fetch_progress(
    stock_codes: &[String],
    region_config: RegionConfig,
    static_names: std::collections::HashMap<String, String>,
) -> Result<Vec<StockData>> {
    let fetcher = SnapshotFetcher::new(stock_codes.to_vec(), region_config, static_names);
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
            let chunks = split_vertical(
                inner,
                &[
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ],
            );
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
                Paragraph::new("Esc to cancel".gray()).alignment(Alignment::Center),
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
        return Err(AppError::Cancelled);
    }

    let res = handle.await?;
    let data = res?;
    guard.terminal_mut().draw(|f| {
        let size = f.size();
        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title("Done");
        f.render_widget(block.clone(), area);
        let inner = block.inner(area);
        let msg = Paragraph::new(format!(
            "Fetched {} records.\nPress Enter to continue.",
            data.len()
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
    Ok(data)
}
