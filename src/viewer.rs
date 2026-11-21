use std::{
    io::{self, Stdout},
    path::PathBuf,
};

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
    style::{Color, Modifier, Style},
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
    current_match: Option<usize>,
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
                            app.search_query = app.search_input.take();
                            app.current_match = None;
                            search_step(app, true);
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(character)
                            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            input.push(character);
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

fn search_matches(app: &App) -> Vec<usize> {
    let Some(query) = app.search_query.as_deref() else {
        return Vec::new();
    };
    if query.is_empty() {
        return Vec::new();
    }
    let query = query.to_lowercase();
    app.lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line_text(line)
                .to_lowercase()
                .contains(&query)
                .then_some(index)
        })
        .collect()
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
            .find(|index| *index > current)
            .unwrap_or(matches[0]),
        (false, Some(current)) => matches
            .iter()
            .copied()
            .rev()
            .find(|index| *index < current)
            .unwrap_or(*matches.last().expect("matches is not empty")),
        (true, None) => matches[0],
        (false, None) => *matches.last().expect("matches is not empty"),
    };
    app.current_match = Some(next);
    app.scroll = line_count(&app.lines[..next], app.width as u16).min(u16::MAX as usize) as u16;
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
        Span::styled(" rmd ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(&app.title, Style::default().add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(header, areas[0]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let content_area = block.inner(areas[1]);
    let lines = if let Some(query) = app
        .search_query
        .as_deref()
        .filter(|query| !query.is_empty())
    {
        app.lines
            .iter()
            .map(|line| {
                let is_match = line_text(line)
                    .to_lowercase()
                    .contains(&query.to_lowercase());
                if is_match {
                    let mut line = line.clone();
                    for span in &mut line.spans {
                        span.style = span.style.bg(Color::DarkGray);
                    }
                    line
                } else {
                    line.clone()
                }
            })
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
        format!(" /{input}  Enter: search   Esc: cancel ")
    } else if let Some(query) = app.search_query.as_deref() {
        let matches = search_matches(app);
        let current = app
            .current_match
            .and_then(|index| matches.iter().position(|matched| *matched == index))
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
        Line::from(Span::styled(
            footer_text,
            Style::default().fg(Color::DarkGray),
        )),
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
        assert_eq!(app.current_match, Some(0));
        search_step(&mut app, true);
        assert!(app.current_match.unwrap() > 0);
        search_step(&mut app, false);
        assert_eq!(app.current_match, Some(0));
    }
}
