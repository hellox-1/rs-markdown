use std::path::Path;

use pulldown_cmark::{
    CodeBlockKind, Event as MarkdownEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::{images::ImageCache, syntax::highlight_code_line, table::TableBuilder};

#[derive(Default)]
struct ListState {
    next_number: Option<u64>,
}

#[derive(Default)]
struct ImageBuilder {
    source: String,
    alt: String,
}

struct MarkdownRenderer<'a> {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    style: Style,
    style_stack: Vec<Style>,
    lists: Vec<ListState>,
    quote_depth: usize,
    table: Option<TableBuilder>,
    image: Option<ImageBuilder>,
    base_dir: &'a Path,
    image_cache: &'a mut ImageCache,
    code_language: Option<String>,
    in_block_comment: bool,
    width: usize,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(width: usize, base_dir: &'a Path, image_cache: &'a mut ImageCache) -> Self {
        Self {
            lines: Vec::new(),
            current: Vec::new(),
            style: Style::default(),
            style_stack: Vec::new(),
            lists: Vec::new(),
            quote_depth: 0,
            table: None,
            image: None,
            base_dir,
            image_cache,
            code_language: None,
            in_block_comment: false,
            width: width.max(12),
        }
    }

    fn render(mut self, markdown: &str) -> Vec<Line<'static>> {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_MATH;

        for event in Parser::new_ext(markdown, options) {
            match event {
                MarkdownEvent::Start(tag) => self.start_tag(tag),
                MarkdownEvent::End(tag) => self.end_tag(tag),
                MarkdownEvent::Text(text) | MarkdownEvent::Html(text) => {
                    self.push_text(&text);
                }
                MarkdownEvent::InlineHtml(text) => self.push_text(&text),
                MarkdownEvent::Code(code) => self.push_code(&code),
                MarkdownEvent::InlineMath(math) => self.push_text(&math),
                MarkdownEvent::DisplayMath(math) => {
                    self.ensure_block_gap();
                    self.push_styled_text(&math, Style::default().fg(Color::Magenta));
                    self.finish_line();
                    self.add_blank_line();
                }
                MarkdownEvent::SoftBreak => self.push_text(" "),
                MarkdownEvent::HardBreak => {
                    self.finish_line();
                    self.add_quote_prefix();
                }
                MarkdownEvent::Rule => {
                    self.ensure_block_gap();
                    self.lines.push(Line::from(Span::styled(
                        "─".repeat(self.width.min(72)),
                        Style::default().fg(Color::DarkGray),
                    )));
                    self.add_blank_line();
                }
                MarkdownEvent::TaskListMarker(checked) => {
                    let marker = if checked { "[x] " } else { "[ ] " };
                    self.push_styled_text(
                        marker,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                }
                MarkdownEvent::FootnoteReference(name) => {
                    self.push_text(&format!("[^{name}]"));
                }
            }
        }

        self.finish_line();
        self.lines
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        self.style_stack.push(self.style);
        match tag {
            Tag::Paragraph => {
                if self.current.is_empty() {
                    self.ensure_block_gap();
                    self.add_quote_prefix();
                }
            }
            Tag::Heading { level, .. } => {
                self.ensure_block_gap();
                let color = match heading_number(level) {
                    1 => Color::Cyan,
                    2 => Color::Blue,
                    _ => Color::LightBlue,
                };
                self.style = self.style.fg(color).add_modifier(Modifier::BOLD);
            }
            Tag::BlockQuote(_) => {
                if self.quote_depth == 0 && self.lists.is_empty() {
                    self.ensure_block_gap();
                }
                self.quote_depth += 1;
                self.style = self.style.fg(Color::White).add_modifier(Modifier::ITALIC);
                self.add_quote_prefix();
            }
            Tag::CodeBlock(kind) => {
                self.ensure_block_gap();
                self.style = self.style.fg(Color::Green);
                if let CodeBlockKind::Fenced(language) = kind
                    && !language.is_empty()
                {
                    self.code_language = Some(
                        language
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                    );
                    self.push_styled_text(
                        &format!(" {language} "),
                        Style::default().fg(Color::DarkGray),
                    );
                    self.finish_line();
                }
            }
            Tag::List(start) => {
                if !self.current.is_empty() && self.quote_depth == 0 {
                    self.finish_line();
                }
                if self.lists.is_empty() && self.quote_depth == 0 {
                    self.ensure_block_gap();
                }
                self.lists.push(ListState { next_number: start });
            }
            Tag::Item => {
                if !self.current.is_empty() && self.quote_depth == 0 {
                    self.finish_line();
                }
                self.add_quote_prefix();
                let indent = self.lists.len().saturating_sub(1) * 2;
                self.push_text(&" ".repeat(indent));
                let marker = if let Some(list) = self.lists.last_mut() {
                    if let Some(number) = list.next_number.as_mut() {
                        let marker = format!("{number}. ");
                        *number = number.saturating_add(1);
                        marker
                    } else {
                        "• ".to_owned()
                    }
                } else {
                    "• ".to_owned()
                };
                self.push_styled_text(
                    &marker,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
            }
            Tag::Emphasis => self.style = self.style.add_modifier(Modifier::ITALIC),
            Tag::Strong => self.style = self.style.add_modifier(Modifier::BOLD),
            Tag::Strikethrough => self.style = self.style.add_modifier(Modifier::CROSSED_OUT),
            Tag::Link { .. } => {
                self.style = self
                    .style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED);
            }
            Tag::Image { dest_url, .. } => {
                self.image = Some(ImageBuilder {
                    source: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Tag::Table(alignments) => {
                self.ensure_block_gap();
                self.table = Some(TableBuilder::new(alignments));
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.finish_row();
                }
            }
            Tag::TableCell => {
                if let Some(table) = self.table.as_mut() {
                    table.cell = Some(String::new());
                }
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.finish_line();
                if self.lists.is_empty() && self.quote_depth == 0 {
                    self.add_blank_line();
                }
            }
            TagEnd::Heading(_) => {
                self.finish_line();
                self.add_blank_line();
            }
            TagEnd::CodeBlock => {
                self.code_language = None;
                self.in_block_comment = false;
                self.finish_line();
                self.add_blank_line();
            }
            TagEnd::BlockQuote(_) => {
                self.finish_line();
                self.quote_depth = self.quote_depth.saturating_sub(1);
                if self.quote_depth == 0 && self.lists.is_empty() {
                    self.add_blank_line();
                } else {
                    self.add_quote_prefix();
                }
            }
            TagEnd::Item => self.finish_line(),
            TagEnd::List(_) => {
                self.lists.pop();
                if self.lists.is_empty() && self.quote_depth == 0 {
                    self.add_blank_line();
                }
            }
            TagEnd::TableCell => {
                if let Some(table) = self.table.as_mut() {
                    table.row.push(table.cell.take().unwrap_or_default());
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.finish_row();
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.lines.extend(table.render(self.width));
                    self.add_blank_line();
                }
            }
            TagEnd::Image => self.finish_image(),
            _ => {}
        }
        self.style = self.style_stack.pop().unwrap_or_default();
    }

    fn push_text(&mut self, text: &str) {
        if let Some(image) = self.image.as_mut() {
            image.alt.push_str(text);
            return;
        }

        if let Some(table) = self.table.as_mut()
            && let Some(cell) = table.cell.as_mut()
        {
            cell.push_str(text);
            return;
        }

        if let Some(language) = self.code_language.clone() {
            let code_lines = text.split_terminator('\n').collect::<Vec<_>>();
            for (index, line) in code_lines.iter().enumerate() {
                self.current.extend(highlight_code_line(
                    line,
                    &language,
                    &mut self.in_block_comment,
                ));
                if index + 1 < code_lines.len() {
                    self.finish_line();
                }
            }
            return;
        }

        let mut pieces = text.split('\n').peekable();
        while let Some(piece) = pieces.next() {
            self.push_styled_text(piece, self.style);
            if pieces.peek().is_some() {
                self.finish_line();
                self.add_quote_prefix();
            }
        }
    }

    fn push_code(&mut self, code: &str) {
        if let Some(image) = self.image.as_mut() {
            image.alt.push_str(code);
        } else {
            let style = self.style.fg(Color::LightGreen).bg(Color::DarkGray);
            self.push_styled_text(code, style);
        }
    }

    fn finish_image(&mut self) {
        let Some(image) = self.image.take() else {
            return;
        };
        self.ensure_block_gap();
        match self
            .image_cache
            .render(&image.source, self.base_dir, self.width)
        {
            Ok(lines) => {
                self.lines.extend(lines);
                if !image.alt.is_empty() {
                    self.lines.push(Line::from(Span::styled(
                        format!("  {}", image.alt),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
            }
            Err(error) => self.lines.push(Line::from(Span::styled(
                format!(
                    "Image unavailable{}: {error}",
                    if image.alt.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", image.alt)
                    }
                ),
                Style::default().fg(Color::LightRed),
            ))),
        }
        self.add_blank_line();
    }

    fn push_styled_text(&mut self, text: &str, style: Style) {
        if !text.is_empty() {
            self.current.push(Span::styled(text.to_owned(), style));
        }
    }

    fn add_quote_prefix(&mut self) {
        if self.current.is_empty() && self.quote_depth > 0 {
            self.current.extend(
                (0..self.quote_depth)
                    .map(|_| Span::styled("│ ", Style::default().fg(Color::Yellow))),
            );
        }
    }

    fn ensure_block_gap(&mut self) {
        self.finish_line();
        if self.lists.is_empty() && self.quote_depth == 0 {
            self.add_blank_line();
        }
    }

    fn add_blank_line(&mut self) {
        if self.current.is_empty() && self.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.lines.push(Line::from(""));
        }
    }

    fn finish_line(&mut self) {
        if !self.current.is_empty() {
            self.lines
                .push(Line::from(std::mem::take(&mut self.current)));
        }
    }
}

fn heading_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub(crate) fn render_markdown(
    markdown: &str,
    width: usize,
    base_dir: &Path,
    image_cache: &mut ImageCache,
) -> Vec<Line<'static>> {
    if markdown.is_empty() {
        return vec![Line::from(Span::styled(
            "This file is empty.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))];
    }
    let lines = MarkdownRenderer::new(width, base_dir, image_cache).render(markdown);
    if lines.is_empty() {
        vec![Line::from(Span::styled(
            "This file is empty.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        lines
    }
}

pub(crate) fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

pub(crate) fn line_count(lines: &[Line<'_>], width: u16) -> usize {
    let width = usize::from(width.max(1));
    lines
        .iter()
        .map(|line| {
            let visible_width = line
                .spans
                .iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>();
            visible_width.max(1).div_ceil(width)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::style::Modifier;

    use super::*;

    fn render(markdown: &str) -> Vec<Line<'static>> {
        render_markdown(markdown, 76, Path::new("."), &mut ImageCache::default())
    }

    #[test]
    fn parses_nested_lists_and_task_markers() {
        let lines = render("- parent\n  - child\n- [x] done\n- [ ] todo");
        let text = lines.iter().map(line_text).collect::<Vec<_>>();
        assert!(text.iter().any(|line| line == "  • child"));
        assert!(text.iter().any(|line| line.contains("[x] done")));
        assert!(text.iter().any(|line| line.contains("[ ] todo")));
    }

    #[test]
    fn renders_tables_and_preserves_cell_text() {
        let lines = render("| Name | Value |\n| --- | ---: |\n| Rust | 42 |");
        let text = lines.iter().map(line_text).collect::<Vec<_>>();
        assert!(
            text.iter()
                .any(|line| line.contains("Name") && line.contains("Value"))
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Rust") && line.contains("42"))
        );
    }

    #[test]
    fn empty_documents_show_a_message() {
        let lines = render("");
        assert_eq!(line_text(&lines[0]), "This file is empty.");
    }

    #[test]
    fn parses_strikethrough_and_links() {
        let lines = render("~~removed~~ [Rust](https://rust-lang.org)");
        let spans = lines[0].spans.as_slice();
        assert!(spans.iter().any(|span| {
            span.content == "removed" && span.style.add_modifier.contains(Modifier::CROSSED_OUT)
        }));
        assert!(spans.iter().any(|span| span.content == "Rust"));
    }
}
