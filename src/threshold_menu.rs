use anyhow::Result;
use std::collections::HashMap;

use crate::config::Threshold;
use crate::ui::navigate_list;

pub fn display_thresholds(thresholds: &HashMap<String, Threshold>) {
    use unicode_width::UnicodeWidthStr;
    use std::io::{self, Write};
    let mut out = io::stdout();
    if thresholds.is_empty() {
        let _ = write!(out, "  (no thresholds)\r\n");
        let _ = out.flush();
        return;
    }

    let mut items: Vec<_> = thresholds
        .iter()
        .filter(|(_, t)| t.valid)
        .map(|(k, t)| (k.as_str(), t.lower, t.upper))
        .collect();
    items.sort_by(|a, b| a.0.cmp(b.0));

    let name_width = items
        .iter()
        .map(|(k, _, _)| UnicodeWidthStr::width(*k))
        .max()
        .unwrap_or(4)
        .max("Metric".len());

    let header = format!("{:<name_w$} | Lower  | Upper  ", "Metric", name_w = name_width);
    let _ = write!(out, "{}\r\n", header);
    let _ = write!(out, "{}\r\n", "-".repeat(header.len()));
    for (k, lo, up) in items {
        let _ = write!(out, "{:<name_w$} | {:>6.2} | {:>6.2}\r\n", k, lo, up, name_w = name_width);
    }
    let _ = write!(out, "\r\n");
    let _ = out.flush();
}

pub fn set_thresholds_interactively(
    thresholds: &mut HashMap<String, Threshold>,
    top_row: u16,
) -> Result<()> {
    use crossterm::{
        style::{Attribute, SetAttribute},
        terminal::{self, ClearType},
        QueueableCommand,
    };
    use std::io::{stdout, Write};

    let mut stdout = stdout();
    // Raw mode is expected to be enabled by caller (shared screen)

    let mut keys: Vec<String> = thresholds.keys().cloned().collect();
    keys.sort();

    let mut selected = 0usize;
    loop {
        // Build a renderer that draws current list and options
        let render = |sel: usize| -> Result<()> {
            stdout.queue(crossterm::cursor::MoveTo(0, top_row))?;
            stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;
            stdout.write_all(b"Set Thresholds (Use Up/Down, Enter to edit, Esc to exit)\r\n\r\n")?;

            for (i, k) in keys.iter().enumerate() {
                let thr = thresholds.get(k).unwrap();
                let is_sel = i == sel;
                stdout.write_all(if is_sel { "► ".as_bytes() } else { "  ".as_bytes() })?;
                if is_sel { stdout.queue(SetAttribute(Attribute::Reverse))?; }
                stdout.write_all(
                    format!("{:<12} : [{:.2}, {:.2}]", k, thr.lower, thr.upper).as_bytes(),
                )?;
                if is_sel { stdout.queue(SetAttribute(Attribute::Reset))?; }
                stdout.write_all("\r\n".as_bytes())?;
            }

            let is_add = sel == keys.len();
            stdout.write_all(if is_add { "► ".as_bytes() } else { "  ".as_bytes() })?;
            if is_add { stdout.queue(SetAttribute(Attribute::Reverse))?; }
            stdout.write_all("Add new metric".as_bytes())?;
            if is_add { stdout.queue(SetAttribute(Attribute::Reset))?; }
            stdout.write_all("\r\n".as_bytes())?;

            let is_done = sel == keys.len() + 1;
            stdout.write_all(if is_done { "► ".as_bytes() } else { "  ".as_bytes() })?;
            if is_done { stdout.queue(SetAttribute(Attribute::Reverse))?; }
            stdout.write_all("Done".as_bytes())?;
            if is_done { stdout.queue(SetAttribute(Attribute::Reset))?; }
            stdout.write_all("\r\n".as_bytes())?;

            stdout.flush()?;
            Ok(())
        };

        let total = || keys.len() + 2; // include Add and Done
        match navigate_list(selected, total, render)? {
            None => break,        // Esc
            Some(sel) => {
                selected = sel;   // remember last selection
                terminal::disable_raw_mode()?; // for stdin line input
                if sel < keys.len() {
                    let name = keys[sel].clone();
                    if let Some(existing) = thresholds.get(&name).cloned() {
                        println!(
                            "\nEditing '{}' (current [{:.2}, {:.2}])",
                            name, existing.lower, existing.upper
                        );
                        let lower = prompt_f64("  Lower bound", existing.lower)?;
                        let upper = prompt_f64("  Upper bound", existing.upper)?;
                        let (lo, up) = if lower <= upper { (lower, upper) } else { (upper, lower) };
                        thresholds.insert(name, Threshold { lower: lo, upper: up, valid: true });
                    }
                } else if sel == keys.len() {
                    println!("\nEnter new metric name: ");
                    let name = read_line_trimmed()?;
                    if !name.is_empty() {
                        let lower = prompt_f64("  Lower bound", 0.0)?;
                        let upper = prompt_f64("  Upper bound", lower)?;
                        let (lo, up) = if lower <= upper { (lower, upper) } else { (upper, lower) };
                        thresholds.insert(name.clone(), Threshold { lower: lo, upper: up, valid: true });
                        if !keys.contains(&name) {
                            keys.push(name);
                            keys.sort();
                        }
                    } else {
                        println!("Metric name cannot be empty. Press Enter to continue...");
                        let _ = read_line_trimmed();
                    }
                } else {
                    // Done
                    break;
                }
                terminal::enable_raw_mode()?;
            }
        }
    }

    // Do not disable raw mode here; caller manages raw state
    Ok(())
}

fn read_line_trimmed() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_f64(label: &str, default: f64) -> Result<f64> {
    use std::io::Write;
    loop {
        print!("{} (default {:.4}): ", label, default);
        std::io::stdout().flush()?;
        let s = read_line_trimmed()?;
        if s.is_empty() {
            return Ok(default);
        }
        match s.parse::<f64>() {
            Ok(v) => return Ok(v),
            Err(_) => println!("Invalid number, please try again."),
        }
    }
}
