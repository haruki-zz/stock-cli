use crate::error::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::Stylize;
use ratatui::text::Line as TextLine;
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;
use std::time::Duration;

use crate::ui::styles::{header_text, secondary_line, selection_style, ACCENT};
use crate::{
    config::Threshold,
    records::{ensure_metric_thresholds, FILTERABLE_METRICS},
    ui::{
        components::utils::{centered_rect, split_vertical},
        TerminalGuard, UiRoute,
    },
};

pub fn run_thresholds_editor(
    thresholds: &mut HashMap<String, Threshold>,
    mut save_callback: Option<&mut dyn FnMut(&str, &HashMap<String, Threshold>) -> Result<String>>,
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
        Save {
            buffer: String,
            error: Option<String>,
        },
    }
    let mut mode = Mode::List;
    enum SaveStatus {
        Info(String),
        Error(String),
    }
    let mut save_status: Option<SaveStatus> = None;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let chunks = split_vertical(
                size,
                &[
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ],
            );

            let title = Paragraph::new(header_text(
                "Edit thresholds — \u{2191}/\u{2193}/j/k move • Space toggles selected metric • Enter edit • Esc back",
            ));
            f.render_widget(title, chunks[0]);

            let save_hint: Line = vec![
                "Press ".into(),
                "S".bold().fg(ACCENT),
                " or ".into(),
                "Ctrl+S".bold().fg(ACCENT),
                " to save current filters as a preset".into(),
            ]
            .into();
            let hint = Paragraph::new(save_hint).alignment(Alignment::Center);
            f.render_widget(hint, chunks[1]);

            let total_items = keys.len() + 1;
            let mut list_items = Vec::with_capacity(total_items);
            for idx in 0..total_items {
                let mut item = if idx < keys.len() {
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
                    let line = if thr.valid {
                        TextLine::from(label)
                    } else {
                        TextLine::from(label).dim()
                    };
                    ListItem::new(line)
                } else {
                    ListItem::new(TextLine::from("Back"))
                };

                if idx == selected {
                    item = item.style(selection_style());
                }
                list_items.push(item);
            }

            let list = List::new(list_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(UiRoute::Thresholds.title()),
            );
            f.render_widget(list, chunks[2]);

            let footer = match &save_status {
                Some(SaveStatus::Info(message)) => Paragraph::new(secondary_line(message)),
                Some(SaveStatus::Error(message)) => {
                    Paragraph::new(message.clone().red())
                }
                None => {
                    let base = if save_callback.is_some() {
                        "Enter edit • Space toggles ON/OFF • s save preset • Esc back"
                    } else {
                        "Enter edit • Space toggles ON/OFF • Esc back"
                    };
                    Paragraph::new(secondary_line(base))
                }
            };
            f.render_widget(footer, chunks[3]);

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
                let v = split_vertical(
                    inner,
                    &[
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ],
                );
                let mut l_style = Style::default();
                if *field == 0 {
                    l_style = l_style.patch(selection_style());
                }
                let mut u_style = Style::default();
                if *field == 1 {
                    u_style = u_style.patch(selection_style());
                }
                let mut status_style = if *valid {
                    Style::default()
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };
                if *field == 2 {
                    status_style = status_style.patch(selection_style());
                }
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
                    Paragraph::new(secondary_line(
                        "Enter save • Space toggle • Esc cancel • Tab/↑/↓/j/k switch",
                    )),
                    v[3],
                );
            }

            if let Mode::Save { buffer, error } = &mode {
                let area = centered_rect(60, 35, size);
                f.render_widget(Clear, area);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title("Save filters as preset");
                f.render_widget(block.clone(), area);
                let inner = block.inner(area);
                let sections = split_vertical(
                    inner,
                    &[
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ],
                );

                let instructions = Paragraph::new(secondary_line(
                    "Allowed: letters, numbers, space, '-' and '_'",
                ));
                f.render_widget(instructions, sections[0]);

                let mut display = buffer.clone();
                display.push('_');
                let input = Paragraph::new(display)
                    .style(Style::default().fg(ACCENT))
                    .block(Block::default().borders(Borders::ALL).title("Preset name"));
                f.render_widget(input, sections[1]);

                let message = if let Some(msg) = error {
                    Paragraph::new(msg.clone().red())
                } else {
                    Paragraph::new(secondary_line(
                        "Enter save • Esc cancel • Backspace delete",
                    ))
                };
                f.render_widget(message, sections[2]);
            }
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match (&mode, k.code) {
                    (Mode::List, KeyCode::Up) | (Mode::List, KeyCode::Char('k')) => {
                        save_status = None;
                        if selected == 0 {
                            selected = keys.len();
                        } else {
                            selected -= 1;
                        }
                    }
                    (Mode::List, KeyCode::Down) | (Mode::List, KeyCode::Char('j')) => {
                        save_status = None;
                        selected = (selected + 1) % (keys.len() + 1);
                    }
                    (Mode::List, KeyCode::Enter) => {
                        save_status = None;
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
                        save_status = None;
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
                    (Mode::List, KeyCode::Char('s')) | (Mode::List, KeyCode::Char('S')) => {
                        if save_callback.is_some() {
                            save_status = None;
                            mode = Mode::Save {
                                buffer: String::new(),
                                error: None,
                            };
                        } else {
                            save_status = Some(SaveStatus::Error(
                                "Saving presets is unavailable in this context.".to_string(),
                            ));
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
                        save_status = None;
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
                    (Mode::Save { buffer, error }, key) => {
                        let mut buf = buffer.clone();
                        let mut err = error.clone();
                        match key {
                            KeyCode::Backspace => {
                                buf.pop();
                                err = None;
                            }
                            KeyCode::Char(ch) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                                if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_') {
                                    buf.push(ch);
                                    err = None;
                                } else {
                                    err = Some("Unsupported character".to_string());
                                }
                            }
                            KeyCode::Enter => {
                                let trimmed = buf.trim();
                                if trimmed.is_empty() {
                                    err = Some("Name cannot be empty".to_string());
                                } else if let Some(cb) = save_callback.as_deref_mut() {
                                    match cb(trimmed, thresholds) {
                                        Ok(saved_name) => {
                                            save_status = Some(SaveStatus::Info(format!(
                                                "Saved filters as '{}'",
                                                saved_name
                                            )));
                                            mode = Mode::List;
                                            continue;
                                        }
                                        Err(callback_err) => {
                                            err = Some(callback_err.to_string());
                                        }
                                    }
                                } else {
                                    err = Some(
                                        "Saving presets is unavailable in this context."
                                            .to_string(),
                                    );
                                }
                            }
                            KeyCode::Esc => {
                                mode = Mode::List;
                                save_status = None;
                                continue;
                            }
                            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                mode = Mode::List;
                                save_status = None;
                                continue;
                            }
                            _ => {}
                        }
                        mode = Mode::Save {
                            buffer: buf,
                            error: err,
                        };
                    }
                    _ => {}
                }
            }
        }
    }
}
