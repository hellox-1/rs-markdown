use std::{
    io::{self, Stdout},
    path::PathBuf,
};

use crate::theme;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    images::ImageCache,
    markdown::{line_count, line_text, render_markdown},
};

struct App {
    title: String,
    source: String,
    base_dir: PathBuf,
    lines: Vec<Line<'static>>,
    image_cache: ImageCache,
    width: usize,
    scroll: u16,
    search_query: Option<String>,
    search_input: Option<String>,
    current_match: Option<SearchMatch>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SearchMatch {
    line: usize,
    start: usize,
}

pub(crate) fn run(
    title: String,
    source: String,
    base_dir: PathBuf,
    lines: Vec<Line<'static>>,
    image_cache: ImageCache,
    width: usize,
) -> Result<()> {
    let mut app = App {
        title,
        source,
        base_dir,
        lines,
        image_cache,
        width,
        scroll: 0,
        search_query: None,
        search_input: None,
        current_match: None,
    };

    enable_raw_mode().context("could not enable terminal raw mode")?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error).context("could not enter alternate screen");
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error).context("could not initialize terminal");
        }
    };

    let result = event_loop(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);
    result?;
    restore_result
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("could not disable terminal raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("could not leave alternate screen")?;
    terminal.show_cursor().context("could not restore cursor")
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(input) = app.search_input.as_mut() {
                    match key.code {
                        KeyCode::Esc => app.search_input = None,
                        KeyCode::Enter => {
                            search_step(app, true);
                        }
                        KeyCode::Backspace => {
                            input.pop();
                            update_search_preview(app);
                        }
                        KeyCode::Char(character)
                            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            input.push(character);
                            update_search_preview(app);
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('/') => app.search_input = Some(String::new()),
                    KeyCode::Char('n') => search_step(app, true),
                    KeyCode::Char('N') => search_step(app, false),
                    KeyCode::Enter => search_step(app, true),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll = app.scroll.saturating_add(1),
                    KeyCode::PageUp => {
                        let page = terminal.size()?.height.saturating_sub(3).max(1);
                        app.scroll = app.scroll.saturating_sub(page);
                    }
                    KeyCode::PageDown => {
                        let page = terminal.size()?.height.saturating_sub(3).max(1);
                        app.scroll = app.scroll.saturating_add(page);
                    }
                    KeyCode::Home => app.scroll = 0,
                    KeyCode::End => app.scroll = u16::MAX,
                    _ => {}
                }
            }
            Event::Resize(width, _) => {
                app.width = usize::from(width.saturating_sub(4));
                app.lines =
                    render_markdown(&app.source, app.width, &app.base_dir, &mut app.image_cache);
            }
            _ => {}
        }
    }
}

fn update_search_preview(app: &mut App) {
    app.search_query = app
        .search_input
        .as_deref()
        .filter(|query| !query.is_empty())
        .map(str::to_owned);
    app.current_match = None;
}

fn search_matches(app: &App) -> Vec<SearchMatch> {
    let Some(query) = app.search_query.as_deref() else {
        return Vec::new();
    };
    if query.is_empty() {
        return Vec::new();
    }
    app.lines
        .iter()
        .enumerate()
        .flat_map(|(line, rendered)| {
            let text = line_text(rendered);
            search_ranges(&text, query)
                .into_iter()
                .map(move |matched| SearchMatch {
                    line,
                    start: matched.start,
                })
        })
        .collect()
}

fn search_ranges(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut folded = String::new();
    let mut original_start = Vec::new();
    let mut original_end = Vec::new();
    for (start, character) in text.char_indices() {
        let end = start + character.len_utf8();
        let lower = character.to_lowercase().to_string();
        folded.push_str(&lower);
        original_start.extend(std::iter::repeat_n(start, lower.len()));
        original_end.extend(std::iter::repeat_n(end, lower.len()));
    }

    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    folded
        .match_indices(&query)
        .map(|(start, matched)| {
            let end = start + matched.len();
            original_start[start]..original_end[end - 1]
        })
        .collect()
}

fn highlight_search_line(
    line: &Line<'_>,
    query: &str,
    active_match: Option<SearchMatch>,
    line_index: usize,
) -> Line<'static> {
    let text = line_text(line);
    let matches = search_ranges(&text, query);
    if matches.is_empty() {
        return Line::from(
            line.spans
                .iter()
                .map(|span| Span::styled(span.content.to_string(), span.style))
                .collect::<Vec<_>>(),
        );
    }

    let mut output = Vec::new();
    let mut offset = 0;
    for span in &line.spans {
        let content = span.content.as_ref();
        let span_end = offset + content.len();
        let mut boundaries = vec![offset, span_end];
        for matched in &matches {
            if matched.start < span_end && matched.end > offset {
                boundaries.push(matched.start.max(offset));
                boundaries.push(matched.end.min(span_end));
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        for pair in boundaries.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if start == end {
                continue;
            }
            let local_start = start - offset;
            let local_end = end - offset;
            let content = &content[local_start..local_end];
            let highlighted = matches
                .iter()
                .any(|matched| matched.start < end && matched.end > start);
            let style = if highlighted {
                let style = span
                    .style
                    .fg(crate::theme::SEARCH_TEXT)
                    .bg(crate::theme::SEARCH_BACKGROUND);
                if active_match.is_some_and(|active| {
                    active.line == line_index && active.start >= start && active.start < end
                }) {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                }
            } else {
                span.style
            };
            output.push(Span::styled(content.to_owned(), style));
        }
        offset = span_end;
    }
    Line::from(output)
}

fn search_step(app: &mut App, forward: bool) {
    let matches = search_matches(app);
    if matches.is_empty() {
        app.current_match = None;
        return;
    }
    let next = match (forward, app.current_match) {
        (true, Some(current)) => matches
            .iter()
            .copied()
            .find(|matched| *matched > current)
            .unwrap_or(matches[0]),
        (false, Some(current)) => matches
            .iter()
            .copied()
            .rev()
            .find(|matched| *matched < current)
            .unwrap_or(*matches.last().expect("matches is not empty")),
        (true, None) => matches[0],
        (false, None) => *matches.last().expect("matches is not empty"),
    };
    app.current_match = Some(next);
    app.scroll =
        line_count(&app.lines[..next.line], app.width as u16).min(u16::MAX as usize) as u16;
}

fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let header = Line::from(vec![
        Span::styled(" rmd ", Style::default().fg(theme::PRIMARY).bold()),
        Span::styled(&app.title, Style::default().add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(header, areas[0]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let content_area = block.inner(areas[1]);
    let lines = if let Some(query) = app
        .search_query
        .as_deref()
        .filter(|query| !query.is_empty())
    {
        app.lines
            .iter()
            .enumerate()
            .map(|(index, line)| highlight_search_line(line, query, app.current_match, index))
            .collect::<Vec<_>>()
    } else {
        app.lines.clone()
    };
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(block);
    let total_lines = line_count(&app.lines, content_area.width);
    let max_scroll = total_lines.saturating_sub(content_area.height as usize);
    app.scroll = app.scroll.min(max_scroll.min(u16::MAX as usize) as u16);
    frame.render_widget(paragraph.scroll((app.scroll, 0)), areas[1]);

    let footer_text = if let Some(input) = app.search_input.as_deref() {
        format!(" /{input}  Enter: next match   Esc: finish ")
    } else if let Some(query) = app.search_query.as_deref() {
        let matches = search_matches(app);
        let current = app
            .current_match
            .and_then(|current| matches.iter().position(|matched| *matched == current))
            .map(|index| index + 1)
            .unwrap_or(0);
        format!(
            " /{query}  {current}/{} matches   n/N: next/previous   q: quit ",
            matches.len()
        )
    } else {
        " ↑/↓ or j/k: scroll   PgUp/PgDn: page   Home/End: jump   /: search   q: quit ".to_owned()
    };
    frame.render_widget(
        Line::from(Span::styled(footer_text, Style::default().fg(theme::MUTED))),
        areas[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_wraps_forward_and_backward() {
        let lines = render_markdown(
            "alpha\n\nbeta\n\nalpha",
            76,
            std::path::Path::new("."),
            &mut ImageCache::default(),
        );
        let mut app = App {
            title: String::new(),
            source: String::new(),
            base_dir: PathBuf::from("."),
            lines,
            image_cache: ImageCache::default(),
            width: 76,
            scroll: 0,
            search_query: Some("alpha".to_owned()),
            search_input: None,
            current_match: None,
        };
        search_step(&mut app, true);
        assert_eq!(app.current_match, Some(SearchMatch { line: 0, start: 0 }));
        search_step(&mut app, true);
        assert!(app.current_match.unwrap().line > 0);
        search_step(&mut app, false);
        assert_eq!(app.current_match, Some(SearchMatch { line: 0, start: 0 }));
    }

    #[test]
    fn highlights_only_the_case_insensitive_match() {
        let line = Line::from(vec![
            Span::raw("Read "),
            Span::styled("Rust", Style::default().bold()),
            Span::raw(" Markdown"),
        ]);
        let highlighted = highlight_search_line(&line, "rust", None, 0);
        assert_eq!(
            highlighted
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "Read Rust Markdown"
        );
        assert_eq!(highlighted.spans[0].content, "Read ");
        assert_eq!(highlighted.spans[1].content, "Rust");
        assert_eq!(
            highlighted.spans[1].style.bg,
            Some(crate::theme::SEARCH_BACKGROUND)
        );
        assert_eq!(highlighted.spans[2].content, " Markdown");
    }

    #[test]
    fn search_ranges_handle_unicode_case_folding() {
        let matches = search_ranges("Straße", "STRAẞE");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], 0..7);
    }

    #[test]
    fn enter_cycles_through_repeated_matches_on_one_line() {
        let lines = render_markdown(
            "alpha alpha",
            76,
            std::path::Path::new("."),
            &mut ImageCache::default(),
        );
        let mut app = App {
            title: String::new(),
            source: String::new(),
            base_dir: PathBuf::from("."),
            lines,
            image_cache: ImageCache::default(),
            width: 76,
            scroll: 0,
            search_query: Some("alpha".to_owned()),
            search_input: Some("alpha".to_owned()),
            current_match: None,
        };
        search_step(&mut app, true);
        assert_eq!(app.current_match, Some(SearchMatch { line: 0, start: 0 }));
        search_step(&mut app, true);
        assert_eq!(app.current_match, Some(SearchMatch { line: 0, start: 6 }));
        search_step(&mut app, true);
        assert_eq!(app.current_match, Some(SearchMatch { line: 0, start: 0 }));
    }
}
