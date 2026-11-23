use ratatui::style::{Color, Modifier, Style};

use crate::theme;

pub(crate) struct Tag {
    pub(crate) name: String,
    pub(crate) closing: bool,
    pub(crate) self_closing: bool,
    pub(crate) attributes: String,
}

pub(crate) fn parse_tag(source: &str) -> Option<Tag> {
    let body = source.strip_prefix('<')?.strip_suffix('>')?.trim();
    if body.starts_with('!') || body.starts_with('?') {
        return None;
    }

    let closing = body.starts_with('/');
    let body = body.strip_prefix('/').unwrap_or(body).trim();
    let self_closing = body.ends_with('/');
    let body = body.strip_suffix('/').unwrap_or(body).trim_end();
    let name_end = body
        .find(|character: char| character.is_whitespace())
        .unwrap_or(body.len());
    let name = body[..name_end].to_ascii_lowercase();
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return None;
    }

    Some(Tag {
        name,
        closing,
        self_closing,
        attributes: body[name_end..].to_owned(),
    })
}

pub(crate) fn apply_tag_style(name: &str, attributes: &str, style: Style) -> Style {
    match name {
        "b" | "strong" => style.add_modifier(Modifier::BOLD),
        "i" | "em" => style.add_modifier(Modifier::ITALIC),
        "u" | "ins" => style.add_modifier(Modifier::UNDERLINED),
        "s" | "strike" | "del" => style.add_modifier(Modifier::CROSSED_OUT),
        "code" | "tt" => style.fg(theme::CODE).bg(theme::CODE_BACKGROUND),
        "small" => style.add_modifier(Modifier::DIM),
        "big" => style.add_modifier(Modifier::BOLD),
        "sup" | "sub" => style.add_modifier(Modifier::DIM),
        "a" => style.fg(theme::LINK).add_modifier(Modifier::UNDERLINED),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            style.fg(theme::PRIMARY).add_modifier(Modifier::BOLD)
        }
        "font" => attribute_value(attributes, "color")
            .and_then(parse_color)
            .map(|color| style.fg(color))
            .unwrap_or(style),
        "span" => {
            let Some(css) = attribute_value(attributes, "style") else {
                return style;
            };
            css.split(';').fold(style, |style, rule| {
                let Some((property, value)) = rule.split_once(':') else {
                    return style;
                };
                match property.trim().to_ascii_lowercase().as_str() {
                    "color" => parse_color(value.trim())
                        .map(|color| style.fg(color))
                        .unwrap_or(style),
                    "font-weight" if value.trim() == "bold" => style.add_modifier(Modifier::BOLD),
                    "font-style" if value.trim() == "italic" => {
                        style.add_modifier(Modifier::ITALIC)
                    }
                    "text-decoration" if value.trim().contains("underline") => {
                        style.add_modifier(Modifier::UNDERLINED)
                    }
                    _ => style,
                }
            })
        }
        _ => style,
    }
}

pub(crate) fn attribute_value<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = attributes.trim();
    while !rest.is_empty() {
        let equals = rest.find('=')?;
        let key = rest[..equals].trim();
        let value = rest[equals + 1..].trim_start();
        let (parsed, tail) = if let Some(quoted) = value.strip_prefix('"') {
            let end = quoted.find('"')?;
            (&quoted[..end], &quoted[end + 1..])
        } else if let Some(quoted) = value.strip_prefix('\'') {
            let end = quoted.find('\'')?;
            (&quoted[..end], &quoted[end + 1..])
        } else {
            let end = value.find(char::is_whitespace).unwrap_or(value.len());
            (&value[..end], &value[end..])
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(parsed);
        }
        rest = tail.trim_start();
    }
    None
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "black" => Some(Color::Black),
        "white" => Some(Color::White),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        "cyan" | "aqua" => Some(Color::Cyan),
        "magenta" | "fuchsia" => Some(Color::Magenta),
        "gray" | "grey" => Some(Color::Gray),
        "orange" => Some(Color::Rgb(255, 165, 0)),
        _ if value.starts_with('#') && value.len() == 7 => {
            let red = u8::from_str_radix(&value[1..3], 16).ok()?;
            let green = u8::from_str_radix(&value[3..5], 16).ok()?;
            let blue = u8::from_str_radix(&value[5..7], 16).ok()?;
            Some(Color::Rgb(red, green, blue))
        }
        _ => None,
    }
}

pub(crate) fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_and_applies_basic_styles() {
        let tag = parse_tag(r#"<span style="color:green;font-weight:bold">"#).unwrap();
        assert_eq!(tag.name, "span");
        let style = apply_tag_style(&tag.name, &tag.attributes, Style::default());
        assert_eq!(style.fg, Some(Color::Green));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }
}
