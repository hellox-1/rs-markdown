use std::path::Path;

use pulldown_cmark::{
    CodeBlockKind, Event as MarkdownEvent, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::html::{apply_tag_style, attribute_value, decode_entities, parse_tag};
use crate::{images::ImageCache, syntax::highlight_code_line, table::TableBuilder, theme};

#[derive(Default)]
struct ListState {
    next_number: Option<u64>,
}

#[derive(Default)]
struct ImageBuilder {
    source: String,
    alt: String,
    size: Option<(u32, Option<u32>)>,
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
    headings: Vec<(usize, String)>,
    toc_inserted: bool,
    html_styles: Vec<(String, Style)>,
    ignored_html: Option<String>,
    code_language: Option<String>,
    in_block_comment: bool,
    mermaid_source: Option<String>,
    width: usize,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(
        width: usize,
        base_dir: &'a Path,
        image_cache: &'a mut ImageCache,
        headings: Vec<(usize, String)>,
    ) -> Self {
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
            headings,
            toc_inserted: false,
            html_styles: Vec::new(),
            ignored_html: None,
            code_language: None,
            in_block_comment: false,
            mermaid_source: None,
            width: width.max(12),
        }
    }

    fn render(mut self, markdown: &str) -> Vec<Line<'static>> {
        let options = Options::ENABLE_GFM
            | Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_MATH;
        let markdown = normalize_wiki_tokens(markdown);

        for event in Parser::new_ext(&markdown, options) {
            match event {
                MarkdownEvent::Start(tag) => self.start_tag(tag),
                MarkdownEvent::End(tag) => self.end_tag(tag),
                MarkdownEvent::Text(text) => self.push_text(&text),
                MarkdownEvent::Html(text) | MarkdownEvent::InlineHtml(text) => {
                    self.push_html(&text);
                }
                MarkdownEvent::Code(code) => self.push_code(&code),
                MarkdownEvent::InlineMath(math) => self.push_text(&math),
                MarkdownEvent::DisplayMath(math) => {
                    self.ensure_block_gap();
                    self.push_styled_text(&math, Style::default().fg(theme::HIGHLIGHT));
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
                        Style::default().fg(theme::BORDER),
                    )));
                    self.add_blank_line();
                }
                MarkdownEvent::TaskListMarker(checked) => {
                    let marker = if checked { "[x] " } else { "[ ] " };
                    self.push_styled_text(
                        marker,
                        Style::default()
                            .fg(theme::ACCENT)
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
                    1 => theme::PRIMARY,
                    2 => theme::ACCENT,
                    _ => theme::HIGHLIGHT,
                };
                self.style = self.style.fg(color).add_modifier(Modifier::BOLD);
            }
            Tag::BlockQuote(_) => {
                if self.quote_depth == 0 && self.lists.is_empty() {
                    self.ensure_block_gap();
                }
                self.quote_depth += 1;
                self.style = self.style.fg(theme::TEXT).add_modifier(Modifier::ITALIC);
                self.add_quote_prefix();
            }
            Tag::CodeBlock(kind) => {
                self.ensure_block_gap();
                self.style = self.style.fg(theme::CODE);
                if let CodeBlockKind::Fenced(language) = kind
                    && !language.is_empty()
                {
                    let language = language
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    self.code_language = Some(language.clone());
                    if language == "mermaid" {
                        self.mermaid_source = Some(String::new());
                    } else {
                        self.push_styled_text(
                            &format!(" {language} "),
                            Style::default().fg(theme::MUTED),
                        );
                        self.finish_line();
                    }
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
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                );
            }
            Tag::Emphasis => self.style = self.style.add_modifier(Modifier::ITALIC),
            Tag::Strong => self.style = self.style.add_modifier(Modifier::BOLD),
            Tag::Strikethrough => self.style = self.style.add_modifier(Modifier::CROSSED_OUT),
            Tag::Link { .. } => {
                self.style = self
                    .style
                    .fg(theme::LINK)
                    .add_modifier(Modifier::UNDERLINED);
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                let destination = format!("{dest_url} {title}");
                let (source, size) = parse_image_size(destination.trim());
                self.image = Some(ImageBuilder {
                    source,
                    alt: String::new(),
                    size,
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
                self.finish_mermaid();
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
            cell.push_str(&replace_shortcodes(text));
            return;
        }

        if let Some(language) = self.code_language.clone() {
            if language == "mermaid" {
                if let Some(source) = self.mermaid_source.as_mut() {
                    source.push_str(text);
                }
                return;
            }
            let code_lines = text.split_terminator('\n').collect::<Vec<_>>();
            for (index, line) in code_lines.iter().enumerate() {
                if language == "suggestion" {
                    let style = match line.as_bytes().first() {
                        Some(b'+') => Style::default().fg(theme::STRING),
                        Some(b'-') => Style::default().fg(theme::ERROR),
                        _ => Style::default().fg(theme::MUTED),
                    };
                    self.push_styled_text(line, style);
                } else {
                    self.current.extend(highlight_code_line(
                        line,
                        &language,
                        &mut self.in_block_comment,
                    ));
                }
                if index + 1 < code_lines.len() {
                    self.finish_line();
                }
            }
            return;
        }

        let text = replace_shortcodes(text);
        for (index, piece) in text.split("RSMARKDOWNWIKITOC").enumerate() {
            if index > 0 {
                self.render_toc();
            }
            for (index, piece) in piece.split("RSMARKDOWNWIKITOSP").enumerate() {
                if index > 0 {
                    self.ensure_block_gap();
                    self.push_styled_text(
                        "Child pages are available in Azure DevOps Wiki.",
                        Style::default().fg(theme::MUTED),
                    );
                    self.finish_line();
                    self.add_blank_line();
                }
                self.push_text_lines(piece);
            }
        }
    }

    fn push_text_lines(&mut self, text: &str) {
        let mut pieces = text.split('\n').peekable();
        while let Some(piece) = pieces.next() {
            self.push_styled_text(piece, self.style);
            if pieces.peek().is_some() {
                self.finish_line();
                self.add_quote_prefix();
            }
        }
    }

    fn push_html(&mut self, html: &str) {
        let mut remaining = html;
        while !remaining.is_empty() {
            if let Some(ignored) = self.ignored_html.as_deref() {
                let lower = remaining.to_ascii_lowercase();
                let closing = format!("</{ignored}");
                if let Some(index) = lower.find(&closing) {
                    remaining = &remaining[index..];
                    self.ignored_html = None;
                } else {
                    return;
                }
            }

            let Some(start) = remaining.find('<') else {
                self.push_text(&decode_entities(remaining));
                break;
            };
            if start > 0 {
                self.push_text(&decode_entities(&remaining[..start]));
                remaining = &remaining[start..];
            }

            if remaining.starts_with("<!--") {
                if let Some(end) = remaining.find("-->") {
                    remaining = &remaining[end + 3..];
                    continue;
                }
                break;
            }

            let Some(end) = remaining.find('>') else {
                self.push_text("<");
                remaining = &remaining[1..];
                continue;
            };
            let token = &remaining[..=end];
            if self
                .current
                .last()
                .is_some_and(|span| span.content.ends_with('@'))
                && let Some(identity) = token
                    .strip_prefix('<')
                    .and_then(|token| token.strip_suffix('>'))
                && identity.len() == 36
                && identity
                    .chars()
                    .all(|character| character.is_ascii_hexdigit() || character == '-')
            {
                self.push_styled_text(
                    identity,
                    Style::default()
                        .fg(theme::LINK)
                        .add_modifier(Modifier::BOLD),
                );
                remaining = &remaining[end + 1..];
                continue;
            }
            let Some(tag) = parse_tag(token) else {
                self.push_text("<");
                remaining = &remaining[1..];
                continue;
            };
            remaining = &remaining[end + 1..];

            if !tag.closing
                && self
                    .current
                    .last()
                    .is_some_and(|span| span.content.ends_with('@'))
            {
                self.push_styled_text(
                    &tag.name,
                    Style::default()
                        .fg(theme::LINK)
                        .add_modifier(Modifier::BOLD),
                );
                continue;
            }

            if tag.name == "script" || tag.name == "style" || tag.name == "iframe" {
                if !tag.closing {
                    if tag.name == "iframe"
                        && let Some(src) = attribute_value(&tag.attributes, "src")
                    {
                        self.push_styled_text(
                            &format!("[embedded video: {src}]"),
                            Style::default().fg(theme::LINK),
                        );
                    }
                    if !tag.self_closing {
                        self.ignored_html = Some(tag.name);
                    }
                }
                continue;
            }
            if tag.name == "video" && !tag.closing {
                if let Some(src) = attribute_value(&tag.attributes, "src") {
                    self.push_styled_text(
                        &format!("[video: {src}]"),
                        Style::default().fg(theme::LINK),
                    );
                }
                if !tag.self_closing {
                    self.ignored_html = Some(tag.name);
                }
                continue;
            }
            if tag.name == "img" && !tag.closing {
                self.image = Some(ImageBuilder {
                    source: attribute_value(&tag.attributes, "src")
                        .unwrap_or_default()
                        .to_owned(),
                    alt: attribute_value(&tag.attributes, "alt")
                        .unwrap_or("image")
                        .to_owned(),
                    size: None,
                });
                self.finish_image();
                continue;
            }
            if tag.name == "br" {
                self.finish_line();
                self.add_quote_prefix();
                continue;
            }
            if matches!(tag.name.as_str(), "p" | "div") {
                if tag.closing {
                    self.finish_line();
                    self.add_blank_line();
                } else if !self.current.is_empty() {
                    self.finish_line();
                }
                continue;
            }
            if tag.name == "details" && !tag.closing {
                self.push_styled_text("▸ ", Style::default().fg(theme::ACCENT));
            }
            if matches!(tag.name.as_str(), "sup" | "sub") && !tag.closing {
                self.push_styled_text(
                    if tag.name == "sup" { "^" } else { "_" },
                    Style::default().fg(theme::MUTED),
                );
            }

            if tag.closing {
                if let Some(index) = self
                    .html_styles
                    .iter()
                    .rposition(|(name, _)| name == &tag.name)
                {
                    self.style = self.html_styles[index].1;
                    self.html_styles.truncate(index);
                }
            } else if !tag.self_closing {
                self.html_styles.push((tag.name.clone(), self.style));
                self.style = apply_tag_style(&tag.name, &tag.attributes, self.style);
            }
        }
    }

    fn render_toc(&mut self) {
        if self.toc_inserted {
            return;
        }
        self.toc_inserted = true;
        self.ensure_block_gap();
        self.push_styled_text("Contents", Style::default().add_modifier(Modifier::BOLD));
        self.finish_line();
        for (level, heading) in self.headings.clone() {
            let indent = "  ".repeat(level.saturating_sub(1));
            self.push_styled_text(
                &format!("{indent}• {heading}"),
                Style::default().fg(theme::LINK),
            );
            self.finish_line();
        }
        self.add_blank_line();
    }

    fn push_code(&mut self, code: &str) {
        if let Some(image) = self.image.as_mut() {
            image.alt.push_str(code);
        } else {
            let style = self.style.fg(theme::CODE).bg(theme::CODE_BACKGROUND);
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
            .render_sized(&image.source, self.base_dir, self.width, image.size)
        {
            Ok(lines) => {
                self.lines.extend(lines);
                if !image.alt.is_empty() {
                    self.lines.push(Line::from(Span::styled(
                        format!("  {}", image.alt),
                        Style::default()
                            .fg(theme::MUTED)
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
                Style::default().fg(theme::ERROR),
            ))),
        }
        self.add_blank_line();
    }

    fn finish_mermaid(&mut self) {
        let Some(source) = self.mermaid_source.take() else {
            return;
        };
        match mermaid_text::render_with_width(&source, Some(self.width)) {
            Ok(diagram) => {
                for line in diagram.lines() {
                    self.push_styled_text(line, Style::default().fg(theme::HIGHLIGHT));
                    self.finish_line();
                }
            }
            Err(error) => {
                self.push_styled_text(
                    &format!("Mermaid diagram could not render: {error}"),
                    Style::default().fg(theme::ERROR),
                );
                self.finish_line();
                for line in source.lines() {
                    self.push_styled_text(line, Style::default().fg(theme::MUTED));
                    self.finish_line();
                }
            }
        }
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
                    .map(|_| Span::styled("│ ", Style::default().fg(theme::QUOTE))),
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
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))];
    }
    let headings = collect_headings(markdown);
    let lines = MarkdownRenderer::new(width, base_dir, image_cache, headings).render(markdown);
    if lines.is_empty() {
        vec![Line::from(Span::styled(
            "This file is empty.",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        lines
    }
}

fn parse_image_size(destination: &str) -> (String, Option<(u32, Option<u32>)>) {
    let Some((source, dimensions)) = destination.rsplit_once(" =") else {
        return (destination.to_owned(), None);
    };
    let Some((width, height)) = dimensions.split_once('x') else {
        return (destination.to_owned(), None);
    };
    let Ok(width) = width.parse::<u32>() else {
        return (destination.to_owned(), None);
    };
    let height = if height.is_empty() {
        None
    } else if let Ok(height) = height.parse::<u32>() {
        Some(height)
    } else {
        return (destination.to_owned(), None);
    };
    (source.to_owned(), Some((width, height)))
}

fn collect_headings(markdown: &str) -> Vec<(usize, String)> {
    let mut headings = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for event in Parser::new_ext(markdown, Options::ENABLE_GFM) {
        match event {
            MarkdownEvent::Start(Tag::Heading { level, .. }) => {
                current = Some((heading_number(level) as usize, String::new()));
            }
            MarkdownEvent::End(TagEnd::Heading(_)) => {
                if let Some((level, text)) = current.take() {
                    headings.push((level, text));
                }
            }
            MarkdownEvent::Text(text) | MarkdownEvent::Code(text) => {
                if let Some((_, heading)) = current.as_mut() {
                    heading.push_str(&text);
                }
            }
            _ => {}
        }
    }
    headings
}

fn replace_shortcodes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let remaining = &input[index..];
        if remaining.starts_with("@<")
            && let Some(end) = remaining[2..].find('>')
        {
            let identity = &remaining[2..end + 2];
            if !identity.is_empty() && !identity.chars().any(char::is_whitespace) {
                output.push('@');
                output.push_str(identity);
                index += end + 3;
                continue;
            }
        }
        if remaining.starts_with(':')
            && let Some(end) = remaining[1..].find(':')
        {
            let shortcode = &remaining[1..end + 1];
            if !shortcode.is_empty()
                && shortcode.len() <= 40
                && !shortcode.chars().any(char::is_whitespace)
                && let Some(emoji) = emojis::get_by_shortcode(shortcode)
            {
                output.push_str(emoji.as_str());
                index += end + 2;
                continue;
            }
        }
        let character = remaining
            .chars()
            .next()
            .expect("index is on a character boundary");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn normalize_wiki_tokens(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut fence: Option<(char, usize)> = None;
    let mut mermaid_directive = false;
    let mut inline_code = None;

    for line in markdown.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = content.trim_start();
        if let Some((marker, minimum)) = fence {
            if mermaid_directive && trimmed == ":::" {
                output.push_str("```");
                fence = None;
                mermaid_directive = false;
            } else {
                output.push_str(content);
                if is_closing_fence(trimmed, marker, minimum) {
                    fence = None;
                }
            }
            if fence.is_none() && mermaid_directive {
                fence = None;
                mermaid_directive = false;
            }
        } else if trimmed.to_ascii_lowercase().starts_with(":::mermaid")
            || trimmed.to_ascii_lowercase().starts_with("::: mermaid")
        {
            output.push_str("```mermaid");
            fence = Some(('`', 3));
            mermaid_directive = true;
        } else if let Some((marker, length)) = opening_fence(trimmed) {
            fence = Some((marker, length));
            output.push_str(content);
        } else {
            output.push_str(&replace_wiki_tokens_in_line(content, &mut inline_code));
        }
        if line.ends_with('\n') {
            output.push('\n');
        }
    }

    output
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn is_closing_fence(line: &str, marker: char, minimum: usize) -> bool {
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    length >= minimum && line[length..].trim().is_empty()
}

fn replace_wiki_tokens_in_line(line: &str, inline_code: &mut Option<usize>) -> String {
    let mut output = String::with_capacity(line.len());
    let mut index = 0;
    while index < line.len() {
        let remaining = &line[index..];
        if remaining.starts_with('`') {
            let run = remaining
                .chars()
                .take_while(|character| *character == '`')
                .count();
            if *inline_code == Some(run) {
                *inline_code = None;
            } else if inline_code.is_none() {
                *inline_code = Some(run);
            }
            output.push_str(&remaining[..run]);
            index += run;
        } else if inline_code.is_none()
            && remaining.starts_with("@<")
            && let Some(end) = remaining[2..].find('>')
        {
            let identity = &remaining[2..end + 2];
            if identity.len() == 36
                && identity
                    .chars()
                    .all(|character| character.is_ascii_hexdigit() || character == '-')
            {
                output.push('@');
                output.push_str(identity);
                index += end + 3;
            } else {
                output.push('@');
                output.push('<');
                index += 2;
            }
        } else if inline_code.is_none() && remaining.starts_with("[[_TOC_]]") {
            output.push_str("RSMARKDOWNWIKITOC");
            index += "[[_TOC_]]".len();
        } else if inline_code.is_none() && remaining.starts_with("[[_TOSP_]]") {
            output.push_str("RSMARKDOWNWIKITOSP");
            index += "[[_TOSP_]]".len();
        } else {
            let character = remaining
                .chars()
                .next()
                .expect("index is on a character boundary");
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
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

    #[test]
    fn expands_emoji_shortcodes() {
        let lines = render("The team is :smile: :+1:");
        let text = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(text.contains("😄"));
        assert!(text.contains("👍"));
    }

    #[test]
    fn generates_a_wiki_table_of_contents() {
        let lines = render("[[_TOC_]]\n\n# First heading\n## Second heading");
        let text = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Contents"), "{text:?}");
        assert!(text.contains("First heading"));
        assert!(text.contains("  • Second heading"));
        assert!(!text.contains("[[_TOC_]]"));
    }

    #[test]
    fn leaves_wiki_tokens_in_code_untouched() {
        let lines = render("`[[_TOC_]]`\n\n```\n[[_TOC_]]\n```");
        let text = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(text.contains("[[_TOC_]]"));
        assert!(!text.contains("Contents"));
    }

    #[test]
    fn renders_mermaid_flowcharts_in_the_terminal() {
        for markdown in [
            "```mermaid\ngraph LR; A[Build] --> B[Deploy]\n```",
            "::: mermaid\ngraph LR; A[Build] --> B[Deploy]\n:::",
        ] {
            let text = render(markdown)
                .iter()
                .map(line_text)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("Build"), "{text}");
            assert!(text.contains("Deploy"), "{text}");
            assert!(!text.contains("Mermaid diagram could not render"), "{text}");
        }
    }

    #[test]
    fn displays_azure_devops_mentions() {
        assert_eq!(
            replace_shortcodes("@<123e4567-e89b-12d3-a456-426614174000>"),
            "@123e4567-e89b-12d3-a456-426614174000"
        );
        let text = render("@<123e4567-e89b-12d3-a456-426614174000>")
            .iter()
            .map(line_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("@123e4567-e89b-12d3-a456-426614174000"),
            "{text}"
        );
    }

    #[test]
    fn renders_common_html_formatting_without_markup() {
        let lines = render("<p><u>underlined</u> and <del>removed</del><br/>next</p>");
        let text = lines.iter().map(line_text).collect::<Vec<_>>();
        let underlined = lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content == "underlined")
            .unwrap();
        let removed = lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content == "removed")
            .unwrap();
        assert!(underlined.style.add_modifier.contains(Modifier::UNDERLINED));
        assert!(removed.style.add_modifier.contains(Modifier::CROSSED_OUT));
        assert!(text.iter().any(|line| line == "next"));
        assert!(!text.iter().any(|line| line.contains("<u>")));
    }

    #[test]
    fn parses_azure_image_dimensions() {
        assert_eq!(
            parse_image_size("./diagram.png =500x250"),
            ("./diagram.png".to_owned(), Some((500, Some(250))))
        );
        assert_eq!(
            parse_image_size("./diagram.png =320x"),
            ("./diagram.png".to_owned(), Some((320, None)))
        );
    }
}
