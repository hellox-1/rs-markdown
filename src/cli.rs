use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command, value_parser};
use crossterm::{
    queue,
    style::{
        Attribute, Color as CrosstermColor, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal,
};
use ratatui::{
    style::{Color, Modifier},
    text::Line,
};

use crate::{images::ImageCache, markdown::render_markdown, viewer};

pub fn run() -> Result<()> {
    let matches = Command::new("rmd")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Read Markdown files in your terminal")
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Markdown file to view")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("static")
                .long("static")
                .short('s')
                .help("Render to stdout without opening the interactive viewer")
                .action(ArgAction::SetTrue),
        )
        .get_matches();
    let file = matches
        .get_one::<PathBuf>("file")
        .expect("the FILE argument is required");
    let markdown =
        fs::read_to_string(file).with_context(|| format!("could not read {}", file.display()))?;
    let base_dir = file.parent().unwrap_or_else(|| std::path::Path::new("."));
    let width = terminal::size()
        .map(|(width, _)| usize::from(width.saturating_sub(4)))
        .unwrap_or(76);
    let mut image_cache = ImageCache::default();
    let lines = render_markdown(&markdown, width, base_dir, &mut image_cache);

    if matches.get_flag("static") {
        print_static(&lines)?;
        return Ok(());
    }

    let title = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Markdown");
    viewer::run(
        title.to_owned(),
        markdown,
        base_dir.to_owned(),
        lines,
        image_cache,
        width,
    )
}

fn print_static(lines: &[Line<'_>]) -> Result<()> {
    let mut stdout = io::stdout().lock();
    for line in lines {
        for span in &line.spans {
            if let Some(color) = span.style.fg.and_then(to_terminal_color) {
                queue!(stdout, SetForegroundColor(color))?;
            }
            if let Some(color) = span.style.bg.and_then(to_terminal_color) {
                queue!(stdout, SetBackgroundColor(color))?;
            }
            for (modifier, attribute) in [
                (Modifier::BOLD, Attribute::Bold),
                (Modifier::ITALIC, Attribute::Italic),
                (Modifier::UNDERLINED, Attribute::Underlined),
                (Modifier::CROSSED_OUT, Attribute::CrossedOut),
            ] {
                if span.style.add_modifier.contains(modifier) {
                    queue!(stdout, SetAttribute(attribute))?;
                }
            }
            write!(stdout, "{}", span.content)?;
            queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        }
        writeln!(stdout)?;
    }
    Ok(())
}

fn to_terminal_color(color: Color) -> Option<CrosstermColor> {
    Some(match color {
        Color::Black => CrosstermColor::Black,
        Color::Red => CrosstermColor::Red,
        Color::Green => CrosstermColor::Green,
        Color::Yellow => CrosstermColor::Yellow,
        Color::Blue => CrosstermColor::Blue,
        Color::Magenta => CrosstermColor::Magenta,
        Color::Cyan => CrosstermColor::Cyan,
        Color::Gray => CrosstermColor::Grey,
        Color::DarkGray => CrosstermColor::DarkGrey,
        Color::LightRed => CrosstermColor::DarkRed,
        Color::LightGreen => CrosstermColor::DarkGreen,
        Color::LightYellow => CrosstermColor::DarkYellow,
        Color::LightBlue => CrosstermColor::DarkBlue,
        Color::LightMagenta => CrosstermColor::DarkMagenta,
        Color::LightCyan => CrosstermColor::DarkCyan,
        Color::White => CrosstermColor::White,
        Color::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
        _ => return None,
    })
}
