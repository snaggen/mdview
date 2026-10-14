//! Syntax highlighting with syntect, using the COSMIC Dark and COSMIC Light themes
//! that COSMIC Text Editor uses by default.

use std::io::Cursor;
use std::sync::LazyLock;

use cosmic::iced::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// A run of text with one style.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
}

/// The spans of one line, without its line ending.
pub type Line = Vec<Span>;

struct Highlighter {
    syntaxes: SyntaxSet,
    dark: Theme,
    light: Theme,
}

/// Loading the syntax definitions takes a moment, so it happens once, on first use.
static HIGHLIGHTER: LazyLock<Highlighter> = LazyLock::new(|| Highlighter {
    // The "newlines" variant matches `LinesWithEndings` below.
    syntaxes: two_face::syntax::extra_newlines(),
    dark: load_theme(cosmic_syntax_theme::COSMIC_DARK_TM_THEME),
    light: load_theme(cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME),
});

/// Parses a bundled tmTheme. Falls back to an empty theme (plain text colours)
/// rather than failing, since this runs lazily while rendering.
fn load_theme(data: &str) -> Theme {
    ThemeSet::load_from_reader(&mut Cursor::new(data)).unwrap_or_else(|error| {
        log::error!("cannot load bundled syntax theme: {error}");
        Theme::default()
    })
}

/// Highlights `code` written in `language` (a code fence token such as `rust`,
/// `py` or `toml`). Unknown languages are returned as plain text, in `default`.
pub fn highlight(code: &str, language: Option<&str>, dark: bool, default: Color) -> Vec<Line> {
    let highlighter = &*HIGHLIGHTER;
    let theme = if dark {
        &highlighter.dark
    } else {
        &highlighter.light
    };
    let syntax = find_syntax(&highlighter.syntaxes, language);
    let mut lines = HighlightLines::new(syntax, theme);

    LinesWithEndings::from(code)
        .map(
            |line| match lines.highlight_line(line, &highlighter.syntaxes) {
                Ok(ranges) => ranges
                    .into_iter()
                    .map(|(style, text)| Span {
                        text: text.trim_end_matches(['\n', '\r']).to_owned(),
                        color: Color::from_rgba8(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            f32::from(style.foreground.a) / 255.0,
                        ),
                        bold: style.font_style.contains(FontStyle::BOLD),
                        italic: style.font_style.contains(FontStyle::ITALIC),
                    })
                    .filter(|span| !span.text.is_empty())
                    .collect(),
                // A syntax definition failing on some input should not lose the text.
                Err(error) => {
                    log::debug!("highlighting failed: {error}");
                    vec![plain(line, default)]
                }
            },
        )
        .collect()
}

/// The syntax for a code fence token, matched against syntax names and file
/// extensions, or plain text.
fn find_syntax<'a>(syntaxes: &'a SyntaxSet, language: Option<&str>) -> &'a SyntaxReference {
    language
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .and_then(|token| syntaxes.find_syntax_by_token(token))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

fn plain(line: &str, color: Color) -> Span {
    Span {
        text: line.trim_end_matches(['\n', '\r']).to_owned(),
        color,
        bold: false,
        italic: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = "fn main() {\n    let x = 1;\n}\n";

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|s| s.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn keeps_text_and_splits_lines() {
        let lines = highlight(RUST, Some("rust"), true, Color::WHITE);
        assert_eq!(text(&lines), ["fn main() {", "    let x = 1;", "}"]);
    }

    #[test]
    fn known_language_gets_several_colors() {
        let lines = highlight(RUST, Some("rs"), false, Color::BLACK);
        let colors: Vec<Color> = lines.iter().flatten().map(|s| s.color).collect();
        assert!(colors.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn dark_and_light_differ() {
        let dark = highlight(RUST, Some("rust"), true, Color::WHITE);
        let light = highlight(RUST, Some("rust"), false, Color::BLACK);
        assert_ne!(dark, light);
    }

    #[test]
    fn unknown_language_is_plain_text() {
        let lines = highlight("a b c", Some("no-such-language"), true, Color::WHITE);
        assert_eq!(text(&lines), ["a b c"]);
    }

    #[test]
    fn extended_syntaxes_are_available() {
        let set = &HIGHLIGHTER.syntaxes;
        assert_eq!(find_syntax(set, Some("toml")).name, "TOML");
    }
}
