use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::{
    config::Threshold,
    storage::{ensure_metric_thresholds, FILTERABLE_METRICS},
    ui::{components::utils::centered_rect, TerminalGuard},
};

pub fn run_thresholds_editor(
    thresholds: &mut std::collections::HashMap<String, Threshold>,
) -> Result<()> {
    // Guard terminal state while the modal editor is active.
    let mut guard = TerminalGuard::new()?;

    ensure_metric_thresholds(thresholds);

    let mut keys: Vec<String> = FILTERABLE_METRICS
        .iter()
        .map(|(key, _)| (*key).to_string())
        .collect();
    let mut extra_keys: Vec<String> = thresholds
        .keys()
        .filter(|key| {
            !FILTERABLE_METRICS
                .iter()
                .any(|(known, _)| known == &key.as_str())
        })
        .cloned()
        .collect();
    extra_keys.sort();
    for extra in extra_keys {
        if !keys.contains(&extra) {
            keys.push(extra);
        }
    }

    fn sort_metric_keys(
        keys: &mut Vec<String>,
        thresholds: &std::collections::HashMap<String, Threshold>,
    ) {
        keys.sort_by(|a, b| {
            let a_valid = thresholds.get(a).map(|t| t.valid).unwrap_or(false);
            let b_valid = thresholds.get(b).map(|t| t.valid).unwrap_or(false);
            match (a_valid, b_valid) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
    }

    sort_metric_keys(&mut keys, thresholds);
    let mut selected = 0usize;

    // The editor either shows the list or a focused edit modal for a single threshold.
    #[derive(Clone)]
    enum Mode {
        List,
        Edit {
            name: String,
            lower: String,
            upper: String,
            valid: bool,
            field: usize,
            orig_lower: f64,
            orig_upper: f64,
        },
    }
    let mut mode = Mode::List;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            let title = Paragraph::new(
                "Edit thresholds — \u{2191}/\u{2193}/j/k move • Space toggles selected metric • Enter edit • Esc back",
            )
            .style(Style::default().fg(Color::Cyan));
            f.render_widget(title, chunks[0]);

            let total_items = keys.len() + 1;
            let mut list_items = Vec::with_capacity(total_items);
            for idx in 0..total_items {
                let (mut item, base_style) = if idx < keys.len() {
                    let key = &keys[idx];
                    let thr = thresholds.get(key).unwrap();
                    let display_name = FILTERABLE_METRICS
                        .iter()
                        .find(|(known, _)| known == &key.as_str())
                        .map(|(_, label)| *label)
                        .unwrap_or_else(|| key.as_str());
                    let label = format!(
                        "{:<16} [{:>6.2}, {:>6.2}]  {}",
                        display_name,
                        thr.lower,
                        thr.upper,
                        if thr.valid { "ON" } else { "OFF" }
                    );
                    let base_style = if thr.valid {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    (ListItem::new(label), base_style)
                } else {
                    (ListItem::new("Back"), Style::default())
                };

                let final_style = if idx == selected {
                    base_style.add_modifier(Modifier::REVERSED)
                } else {
                    base_style
                };
                item = item.style(final_style);
                list_items.push(item);
            }

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title("Thresholds"));
            f.render_widget(list, chunks[1]);

            f.render_widget(
                Paragraph::new("Enter edit • Space toggles ON/OFF • Esc back")
                    .style(Style::default().fg(Color::Gray)),
                chunks[2],
            );

            if let Mode::Edit {
                name,
                lower,
                upper,
                valid,
                field,
                orig_lower,
                orig_upper,
            } = &mode
            {
                let area = centered_rect(60, 40, size);
                f.render_widget(Clear, area);
                let display_title = FILTERABLE_METRICS
                    .iter()
                    .find(|(known, _)| known == &name.as_str())
                    .map(|(_, label)| *label)
                    .unwrap_or(name.as_str());
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Edit '{}'", display_title));
                f.render_widget(block.clone(), area);
                let inner = block.inner(area);
                let v = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(inner);
                let l_style = if *field == 0 {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                let u_style = if *field == 1 {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                let status_style = if *valid {
                    if *field == 2 {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    }
                } else {
                    let base = Style::default().fg(Color::DarkGray);
                    if *field == 2 {
                        base.add_modifier(Modifier::REVERSED)
                    } else {
                        base
                    }
                };
                let lower_text = if lower.is_empty() {
                    format!("{:.2}", orig_lower)
                } else {
                    lower.clone()
                };
                let upper_text = if upper.is_empty() {
                    format!("{:.2}", orig_upper)
                } else {
                    upper.clone()
                };
                f.render_widget(
                    Paragraph::new(format!("Lower: {}", lower_text)).style(l_style),
                    v[0],
                );
                f.render_widget(
                    Paragraph::new(format!("Upper: {}", upper_text)).style(u_style),
                    v[1],
                );
                f.render_widget(
                    Paragraph::new(format!(
                        "Status: {} (Space toggles)",
                        if *valid { "ON" } else { "OFF" }
                    ))
                    .style(status_style),
                    v[2],
                );
                f.render_widget(
                    Paragraph::new(
                        "Enter save • Space toggle • Esc cancel • Tab/↑/↓/j/k switch",
                    )
                    .style(Style::default().fg(Color::Gray)),
                    v[3],
                );
            }
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match (&mode, k.code) {
                    (Mode::List, KeyCode::Up) | (Mode::List, KeyCode::Char('k')) => {
                        if selected == 0 {
                            selected = keys.len();
                        } else {
                            selected -= 1;
                        }
                    }
                    (Mode::List, KeyCode::Down) | (Mode::List, KeyCode::Char('j')) => {
                        selected = (selected + 1) % (keys.len() + 1);
                    }
                    (Mode::List, KeyCode::Enter) => {
                        if selected < keys.len() {
                            let name = keys[selected].clone();
                            if let Some(t) = thresholds.get(&name) {
                                mode = Mode::Edit {
                                    name,
                                    lower: String::new(),
                                    upper: String::new(),
                                    valid: t.valid,
                                    field: 0,
                                    orig_lower: t.lower,
                                    orig_upper: t.upper,
                                };
                            }
                        } else {
                            guard.restore()?;
                            return Ok(());
                        }
                    }
                    (Mode::List, KeyCode::Char(' '))
                    | (Mode::List, KeyCode::Char('t'))
                    | (Mode::List, KeyCode::Char('v')) => {
                        if selected < keys.len() {
                            let key_name = keys[selected].clone();
                            if let Some(thr) = thresholds.get_mut(&key_name) {
                                thr.valid = !thr.valid;
                            }
                            sort_metric_keys(&mut keys, thresholds);
                            if let Some(pos) = keys.iter().position(|k| k == &key_name) {
                                selected = pos;
                            }
                        }
                    }
                    (Mode::List, KeyCode::Esc) => {
                        guard.restore()?;
                        return Ok(());
                    }
                    (Mode::List, KeyCode::Char('c'))
                        if k.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        guard.restore()?;
                        return Ok(());
                    }
                    (
                        Mode::Edit {
                            name,
                            lower,
                            upper,
                            valid,
                            field,
                            orig_lower,
                            orig_upper,
                        },
                        key,
                    ) => {
                        let nm = name.clone();
                        let mut lo = lower.clone();
                        let mut up = upper.clone();
                        let mut is_valid = *valid;
                        let mut fld = *field;
                        let total_fields = 3usize;
                        match key {
                            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                                // Cycle focus through lower/upper/status fields.
                                fld = (fld + 1) % total_fields;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                fld = (fld + total_fields - 1) % total_fields;
                            }
                            KeyCode::Backspace => {
                                if fld == 0 {
                                    lo.pop();
                                } else if fld == 1 {
                                    up.pop();
                                }
                            }
                            KeyCode::Char(c)
                                if (c.is_ascii_digit() || c == '.' || c == '-') && fld <= 1 =>
                            {
                                if fld == 0 {
                                    lo.push(c);
                                } else {
                                    up.push(c);
                                }
                            }
                            KeyCode::Char(' ') | KeyCode::Char('t') | KeyCode::Char('v') => {
                                if fld == 2 {
                                    is_valid = !is_valid;
                                }
                            }
                            KeyCode::Enter => {
                                // Parse the buffered input, falling back to the original values when parsing fails.
                                let lo_v = if lo.is_empty() {
                                    *orig_lower
                                } else {
                                    lo.parse::<f64>().unwrap_or(*orig_lower)
                                };
                                let up_v = if up.is_empty() {
                                    *orig_upper
                                } else {
                                    up.parse::<f64>().unwrap_or(*orig_upper)
                                };
                                let (a, b) = if lo_v <= up_v {
                                    (lo_v, up_v)
                                } else {
                                    (up_v, lo_v)
                                };
                                thresholds.insert(
                                    nm.clone(),
                                    Threshold {
                                        lower: a,
                                        upper: b,
                                        valid: is_valid,
                                    },
                                );
                                sort_metric_keys(&mut keys, thresholds);
                                if let Some(pos) = keys.iter().position(|k| k == &nm) {
                                    selected = pos;
                                }
                                mode = Mode::List;
                                continue;
                            }
                            KeyCode::Esc => {
                                sort_metric_keys(&mut keys, thresholds);
                                if let Some(pos) = keys.iter().position(|k| k == &nm) {
                                    selected = pos;
                                }
                                mode = Mode::List;
                                continue;
                            }
                            _ => {}
                        }
                        mode = Mode::Edit {
                            name: nm,
                            lower: lo,
                            upper: up,
                            valid: is_valid,
                            field: fld,
                            orig_lower: *orig_lower,
                            orig_upper: *orig_upper,
                        };
                    }
                    _ => {}
                }
            }
        }
    }
}
