use anyhow::Result;
use crossterm::{cursor, style::{Attribute, Print, SetAttribute}, terminal::{self, ClearType}, QueueableCommand};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub struct SelectItem {
    pub label: String,
    pub description: String,
}

/// Render a vertical selectable list at `top_row`.
/// - Arrow is not reversed; the content (label) is reversed when selected.
/// - Labels are padded to the max label width to align descriptions.
pub fn render_select_list(top_row: u16, items: &[SelectItem], selected: usize) -> Result<()> {
    let mut out = std::io::stdout();

    let max_label_width = items
        .iter()
        .map(|it| UnicodeWidthStr::width(it.label.as_str()))
        .max()
        .unwrap_or(0);

    let (cols, _) = terminal::size().unwrap_or((80, 24));
    let term_cols = cols as usize;

    for (i, it) in items.iter().enumerate() {
        let y = top_row + i as u16;
        out.queue(cursor::MoveTo(0, y))?;
        out.queue(terminal::Clear(ClearType::CurrentLine))?;

        // Arrow (never reversed)
        let arrow = if i == selected { "► " } else { "  " };
        out.queue(Print(arrow))?;

        // Content (reversed when selected)
        let is_sel = i == selected;
        if is_sel { out.queue(SetAttribute(Attribute::Reverse))?; }
        let label_pad = max_label_width.saturating_sub(UnicodeWidthStr::width(it.label.as_str()));
        let label_render = format!("{}{}", it.label, " ".repeat(label_pad));
        out.queue(Print(label_render))?;
        if is_sel { out.queue(SetAttribute(Attribute::Reset))?; }

        // Description
        let desc = format!("   - {}", it.description);
        out.queue(Print(&desc))?;

        // Fill remainder to avoid artifacts
        let used = UnicodeWidthStr::width(arrow) + max_label_width + UnicodeWidthStr::width("   - ") + UnicodeWidthStr::width(it.description.as_str());
        let rem = term_cols.saturating_sub(used);
        if rem > 0 { out.queue(Print(" ".repeat(rem)))?; }
    }

    out.flush()?;
    Ok(())
}

