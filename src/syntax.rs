use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub(crate) fn highlight_code_line(
    source: &str,
    language: &str,
    in_block_comment: &mut bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut index = 0;
    let base = Style::default().fg(Color::Green);
    let comment = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC);
    let string = Style::default().fg(Color::Yellow);
    let keyword = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let number = Style::default().fg(Color::Magenta);
    let bytes = source.as_bytes();

    while index < bytes.len() {
        let remaining = &source[index..];
        if *in_block_comment {
            if let Some(end) = remaining.find("*/") {
                let end = index + end + 2;
                spans.push(Span::styled(source[index..end].to_owned(), comment));
                index = end;
                *in_block_comment = false;
                continue;
            }
            spans.push(Span::styled(remaining.to_owned(), comment));
            break;
        }

        if remaining.starts_with("/*") {
            *in_block_comment = true;
            continue;
        }
        if remaining.starts_with("//")
            || (matches!(language, "python" | "py" | "bash" | "sh" | "zsh")
                && remaining.starts_with('#'))
        {
            spans.push(Span::styled(remaining.to_owned(), comment));
            break;
        }

        let first = bytes[index];
        if matches!(first, b'\'' | b'"' | b'`') {
            let quote = first;
            let mut end = index + 1;
            while end < bytes.len() {
                if bytes[end] == b'\\' {
                    end = (end + 2).min(bytes.len());
                } else if bytes[end] == quote {
                    end += 1;
                    break;
                } else {
                    end += 1;
                }
            }
            spans.push(Span::styled(source[index..end].to_owned(), string));
            index = end;
            continue;
        }

        if first.is_ascii_digit() {
            let mut end = index + 1;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'.' | b'_'))
            {
                end += 1;
            }
            spans.push(Span::styled(source[index..end].to_owned(), number));
            index = end;
            continue;
        }

        if first.is_ascii_alphabetic() || first == b'_' {
            let mut end = index + 1;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let word = &source[index..end];
            let style = if is_code_keyword(word, language) {
                keyword
            } else {
                base
            };
            spans.push(Span::styled(word.to_owned(), style));
            index = end;
            continue;
        }

        let character = remaining
            .chars()
            .next()
            .expect("index is on a character boundary");
        spans.push(Span::styled(character.to_string(), base));
        index += character.len_utf8();
    }

    spans
}

fn is_code_keyword(word: &str, language: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "as",
        "async",
        "await",
        "break",
        "class",
        "const",
        "continue",
        "def",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "fn",
        "for",
        "from",
        "function",
        "if",
        "impl",
        "import",
        "in",
        "interface",
        "let",
        "match",
        "mod",
        "mut",
        "new",
        "null",
        "pub",
        "raise",
        "return",
        "self",
        "static",
        "struct",
        "super",
        "switch",
        "this",
        "true",
        "try",
        "type",
        "use",
        "var",
        "while",
        "with",
        "None",
        "True",
        "False",
    ];
    let language_supported = matches!(
        language,
        "rust"
            | "rs"
            | "javascript"
            | "js"
            | "typescript"
            | "ts"
            | "python"
            | "py"
            | "c"
            | "h"
            | "cpp"
            | "c++"
            | "bash"
            | "sh"
            | "json"
    );
    language_supported && KEYWORDS.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_keywords_and_strings() {
        let spans =
            highlight_code_line(r#"fn main() { let answer = "hello"; }"#, "rust", &mut false);
        assert!(spans.iter().any(|span| {
            span.content == "fn" && span.style.add_modifier.contains(Modifier::BOLD)
        }));
        assert!(
            spans.iter().any(|span| {
                span.content == r#""hello""# && span.style.fg == Some(Color::Yellow)
            })
        );
    }
}
