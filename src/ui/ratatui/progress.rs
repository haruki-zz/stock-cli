use anyhow::Result;
use crossterm::{execute, terminal};
use ratatui::{prelude::*, widgets::*};
use crate::config::{RegionConfig, InfoIndex};
use crate::fetcher::{AsyncStockFetcher, StockData};
use crate::database::StockDatabase;
use super::utils::centered_rect;

pub async fn run_fetch_progress(
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: RegionConfig,
    info_indices: std::collections::HashMap<String, InfoIndex>,
) -> Result<(Vec<StockData>, String)> {
    let fetcher = AsyncStockFetcher::new(stock_codes.to_vec(), region_config, info_indices);
    let progress = fetcher.progress_counter.clone();
    let total = fetcher.total_stocks;
    let handle = tokio::spawn(async move { fetcher.fetch_data().await });

    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    loop {
        if handle.is_finished() { break; }
        let done = progress.load(std::sync::atomic::Ordering::SeqCst);
        let ratio = if total==0 {0.0} else {(done as f64 / total as f64).clamp(0.0,1.0)};
        terminal.draw(|f| {
            let size = f.size(); let area = centered_rect(60, 20, size); f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).title("Fetching latest data..."); f.render_widget(block.clone(), area);
            let inner = block.inner(area);
            let chunks = Layout::default().direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)]).split(inner);
            let label = format!("Progress: {} / {} ({:.0}%)", done.min(total), total, ratio*100.0);
            f.render_widget(Paragraph::new("Please wait while we fetch data").alignment(Alignment::Center), chunks[0]);
            f.render_widget(Paragraph::new(label).alignment(Alignment::Center), chunks[1]);
            f.render_widget(Paragraph::new("Esc to cancel").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center), chunks[2]);
        })?;
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                if matches!(k.code, crossterm::event::KeyCode::Esc)
                    || (k.code == crossterm::event::KeyCode::Char('c') && k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL))
                { /* best-effort cancel */ }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }

    let res = handle.await.expect("fetch join");
    let data = res?;
    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M");
    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", raw_data_dir, timestamp);
    database.save_to_csv(&filename)?;

    terminal.draw(|f| {
        let size = f.size(); let area = centered_rect(60, 20, size); f.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title("Done"); f.render_widget(block.clone(), area);
        let inner = block.inner(area);
        let msg = Paragraph::new(format!("Fetched {} records. Saved to {}\nPress Enter to continue.", data.len(), filename)).alignment(Alignment::Center);
        f.render_widget(msg, inner);
    })?;
    loop { if crossterm::event::poll(std::time::Duration::from_millis(100))? { if let crossterm::event::Event::Key(k)=crossterm::event::read()? { if matches!(k.code, crossterm::event::KeyCode::Enter|crossterm::event::KeyCode::Char('\n')|crossterm::event::KeyCode::Char('\r')) { break; } } } }

    terminal::disable_raw_mode()?;
    let mut out = std::io::stdout(); let _ = execute!(out, terminal::LeaveAlternateScreen);
    Ok((data, filename))
}
