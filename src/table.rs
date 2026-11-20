use pulldown_cmark::Alignment;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

pub(crate) struct TableBuilder {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    pub(crate) row: Vec<String>,
    pub(crate) cell: Option<String>,
}

impl TableBuilder {
    pub(crate) fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            alignments,
            rows: Vec::new(),
            row: Vec::new(),
            cell: None,
        }
    }

    pub(crate) fn finish_row(&mut self) {
        if !self.row.is_empty() {
            self.rows.push(std::mem::take(&mut self.row));
        }
    }

    pub(crate) fn render(self, available_width: usize) -> Vec<Line<'static>> {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or_default();
        if columns == 0 {
            return Vec::new();
        }

        let mut widths = vec![3usize; columns];
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                widths[index] = widths[index].max(UnicodeWidthStr::width(cell.as_str()));
            }
        }

        let cell_budget = available_width
            .saturating_sub(columns * 3 + 1)
            .max(columns * 3);
        while widths.iter().sum::<usize>() > cell_budget {
            let Some((index, _)) = widths
                .iter()
                .enumerate()
                .filter(|(_, width)| **width > 3)
                .max_by_key(|(_, width)| **width)
            else {
                break;
            };
            widths[index] -= 1;
        }

        let mut output = Vec::new();
        for (row_index, row) in self.rows.iter().enumerate() {
            let mut cells = Vec::with_capacity(columns);
            for (column, width) in widths.iter().enumerate() {
                let cell = row.get(column).map(String::as_str).unwrap_or("");
                let cell = truncate_cell(cell, *width);
                let content_width = UnicodeWidthStr::width(cell.as_str());
                let padding = width.saturating_sub(content_width);
                let alignment = self
                    .alignments
                    .get(column)
                    .copied()
                    .unwrap_or(Alignment::Left);
                let (left_padding, right_padding) = match alignment {
                    Alignment::Right => (padding, 0),
                    Alignment::Center => (padding / 2, padding - padding / 2),
                    _ => (0, padding),
                };
                cells.push(format!(
                    " {}{}{} ",
                    " ".repeat(left_padding),
                    cell,
                    " ".repeat(right_padding)
                ));
            }
            let style = if row_index == 0 {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            output.push(Line::from(Span::styled(
                format!("│{}│", cells.join("│")),
                style,
            )));
            if row_index == 0 {
                output.push(Line::from(Span::styled(
                    format!(
                        "├{}┤",
                        widths
                            .iter()
                            .map(|width| "─".repeat(width + 2))
                            .collect::<Vec<_>>()
                            .join("┼")
                    ),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        output
    }
}

fn truncate_cell(cell: &str, width: usize) -> String {
    if UnicodeWidthStr::width(cell) <= width {
        return cell.to_owned();
    }
    let target = width.saturating_sub(1);
    let mut output = String::new();
    let mut used = 0;
    for character in cell.chars() {
        let char_width = UnicodeWidthStr::width(character.encode_utf8(&mut [0; 4]));
        if used + char_width > target {
            break;
        }
        output.push(character);
        used += char_width;
    }
    if width > 0 {
        output.push('…');
    }
    output
}
