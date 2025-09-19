use anyhow::Result;
use crossterm::{execute, terminal};
use ratatui::{prelude::*, widgets::*};
use crate::config::Threshold;
use crate::ui::utils::centered_rect;

pub fn run_thresholds_editor(thresholds: &mut std::collections::HashMap<String, Threshold>) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut keys: Vec<String> = thresholds.keys().cloned().collect();
    keys.sort();
    let mut selected = 0usize;

    #[derive(Clone)]
    enum Mode { List, Edit { name: String, lower: String, upper: String, field: usize, orig_lower: f64, orig_upper: f64 }, }
    let mut mode = Mode::List;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)])
                .split(size);

            let title = Paragraph::new("Edit thresholds — \u{2191}/\u{2193}/j/k navigate, Enter edit, Esc back")
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(title, chunks[0]);

            let mut items_vec: Vec<ListItem> = keys.iter().map(|k|{
                let thr = thresholds.get(k).unwrap();
                ListItem::new(format!("{:<12}  [{:>6.2}, {:>6.2}]", k, thr.lower, thr.upper))
            }).collect();
            items_vec.push(ListItem::new("Add filter"));
            items_vec.push(ListItem::new("Back"));

            let list = List::new(items_vec.into_iter().enumerate().map(|(i, mut it)|{
                if i==selected { it = it.style(Style::default().add_modifier(Modifier::REVERSED)); }
                it
            }).collect::<Vec<_>>())
            .block(Block::default().borders(Borders::ALL).title("Thresholds"));
            f.render_widget(list, chunks[1]);

            f.render_widget(Paragraph::new("Enter edit • Esc back").style(Style::default().fg(Color::Gray)), chunks[2]);

            if let Mode::Edit { name, lower, upper, field, .. } = &mode {
                let area = centered_rect(60, 40, size);
                f.render_widget(Clear, area);
                let block = Block::default().borders(Borders::ALL).title(format!("Edit '{}'", name));
                f.render_widget(block.clone(), area);
                let inner = block.inner(area);
                let v = Layout::default().direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
                    .split(inner);
                let l_style = if *field==0 { Style::default().add_modifier(Modifier::REVERSED)} else {Style::default()};
                let u_style = if *field==1 { Style::default().add_modifier(Modifier::REVERSED)} else {Style::default()};
                f.render_widget(Paragraph::new(format!("Lower: {}", lower)).style(l_style), v[0]);
                f.render_widget(Paragraph::new(format!("Upper: {}", upper)).style(u_style), v[1]);
                f.render_widget(Paragraph::new("Enter save • Esc cancel • Tab/↑/↓/j/k switch").style(Style::default().fg(Color::Gray)), v[2]);
            }
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(200))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                match (&mode, k.code) {
                    (Mode::List, crossterm::event::KeyCode::Up) | (Mode::List, crossterm::event::KeyCode::Char('k')) => { if selected==0 { selected = keys.len()+1; } else { selected -=1; } }
                    (Mode::List, crossterm::event::KeyCode::Down) | (Mode::List, crossterm::event::KeyCode::Char('j')) => { selected = (selected + 1) % (keys.len()+2); }
                    (Mode::List, crossterm::event::KeyCode::Enter) => {
                        if selected < keys.len() {
                            let name = keys[selected].clone();
                            if let Some(t) = thresholds.get(&name) {
                                mode = Mode::Edit { name, lower: String::new(), upper: String::new(), field: 0, orig_lower: t.lower, orig_upper: t.upper };
                            }
                        } else if selected == keys.len() {
                            let name = "custom".to_string();
                            thresholds.entry(name.clone()).or_insert(Threshold{lower:0.0, upper:0.0, valid:true});
                            if !keys.contains(&name) { keys.push(name.clone()); keys.sort(); }
                            mode = Mode::Edit { name, lower: String::new(), upper: String::new(), field: 0, orig_lower: 0.0, orig_upper: 0.0 };
                        } else { terminal::disable_raw_mode()?; let mut out=std::io::stdout(); let _=execute!(out, terminal::LeaveAlternateScreen); return Ok(()); }
                    }
                    (Mode::List, crossterm::event::KeyCode::Esc) => { terminal::disable_raw_mode()?; let mut out=std::io::stdout(); let _=execute!(out, terminal::LeaveAlternateScreen); return Ok(()); }
                    (Mode::Edit { name, lower, upper, field, orig_lower, orig_upper }, key) => {
                        let mut nm = name.clone(); let mut lo = lower.clone(); let mut up = upper.clone(); let mut fld = *field;
                        match key {
                            crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => { fld = (fld + 1) % 2; }
                            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => { fld = (fld + 1) % 2; }
                            crossterm::event::KeyCode::Backspace => { if fld==0 { lo.pop(); } else { up.pop(); } }
                            crossterm::event::KeyCode::Char(c) if c.is_ascii_digit() || c=='.' || c=='-' => { if fld==0 { lo.push(c);} else {up.push(c);} }
                            crossterm::event::KeyCode::Enter => {
                                let lo_v = if lo.is_empty() {*orig_lower} else { lo.parse::<f64>().unwrap_or(*orig_lower) };
                                let up_v = if up.is_empty() {*orig_upper} else { up.parse::<f64>().unwrap_or(*orig_upper) };
                                let (a,b) = if lo_v<=up_v {(lo_v,up_v)} else {(up_v,lo_v)};
                                thresholds.insert(nm.clone(), Threshold{lower:a, upper:b, valid:true});
                                mode = Mode::List; continue;
                            }
                            crossterm::event::KeyCode::Esc => { mode = Mode::List; continue; }
                            _ => {}
                        }
                        mode = Mode::Edit { name: nm, lower: lo, upper: up, field: fld, orig_lower: *orig_lower, orig_upper: *orig_upper };
                    }
                    _ => {}
                }
            }
        }
    }
}

