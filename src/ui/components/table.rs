use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Row, Table},
};

use ratatui::prelude::Stylize;

pub fn build_table<'a>(
    rows: Vec<Row<'a>>,
    header: Row<'a>,
    widths: Vec<Constraint>,
    title: impl Into<String>,
) -> Table<'a> {
    Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title.into()))
        .column_spacing(2)
}

pub fn highlight_row<'a>(row: Row<'a>) -> Row<'a> {
    row.reversed()
}
