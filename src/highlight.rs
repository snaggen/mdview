//! Syntax highlighting with syntect. The default themes are COSMIC Dark and COSMIC
//! Light, as in COSMIC Text Editor; GitHub, Gruvbox Material and Everforest can be
//! chosen in the settings. Each has a light and a dark variant, picked by the
//! desktop's mode.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Range;
use std::sync::LazyLock;

use cosmic::iced::Color;
use cosmic::iced::advanced::text::highlighter;
use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// A family of syntax themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SyntaxTheme {
    #[default]
    Cosmic,
    GitHub,
    GruvboxMaterial,
    Everforest,
}

impl SyntaxTheme {
    /// All themes, in the order shown in the settings.
    pub const ALL: [Self; 4] = [
        Self::Cosmic,
        Self::GitHub,
        Self::GruvboxMaterial,
        Self::Everforest,
    ];

    /// Display names, matching [`Self::ALL`]. They are product names, so not translated.
    pub const NAMES: [&'static str; 4] = ["COSMIC", "GitHub", "Gruvbox Material", "Everforest"];

    /// The bundled tmTheme for the light or dark variant.
    fn data(self, dark: bool) -> &'static str {
        match (self, dark) {
            (Self::Cosmic, true) => cosmic_syntax_theme::COSMIC_DARK_TM_THEME,
            (Self::Cosmic, false) => cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME,
            (Self::GitHub, true) => include_str!("../res/syntax-themes/github-dark.tmTheme"),
            (Self::GitHub, false) => include_str!("../res/syntax-themes/github-light.tmTheme"),
            (Self::GruvboxMaterial, true) => {
                include_str!("../res/syntax-themes/gruvbox-material-dark.tmTheme")
            }
            (Self::GruvboxMaterial, false) => {
                include_str!("../res/syntax-themes/gruvbox-material-light.tmTheme")
            }
            (Self::Everforest, true) => {
                include_str!("../res/syntax-themes/everforest-dark.tmTheme")
            }
            (Self::Everforest, false) => {
                include_str!("../res/syntax-themes/everforest-light.tmTheme")
            }
        }
    }
}

/// A theme variant: the family and whether the desktop is dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeKey {
    pub theme: SyntaxTheme,
    pub dark: bool,
}

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
    themes: HashMap<ThemeKey, Theme>,
}

impl Highlighter {
    fn theme(&self, key: ThemeKey) -> &Theme {
        // Every key is inserted below; the fallback only satisfies the type.
        static EMPTY: LazyLock<Theme> = LazyLock::new(Theme::default);
        self.themes.get(&key).unwrap_or(&EMPTY)
    }
}

/// Loading the syntax definitions takes a moment, so it happens once, on first use.
static HIGHLIGHTER: LazyLock<Highlighter> = LazyLock::new(|| {
    let themes = SyntaxTheme::ALL
        .into_iter()
        .flat_map(|theme| [true, false].map(|dark| ThemeKey { theme, dark }))
        .map(|key| (key, load_theme(key.theme.data(key.dark))))
        .collect();
    Highlighter {
        // The "newlines" variant matches `LinesWithEndings` below.
        syntaxes: two_face::syntax::extra_newlines(),
        themes,
    }
});

/// The background colour a theme was designed for, used behind code blocks so its
/// colours keep their intended contrast. `None` for the COSMIC themes, which are
/// made for the desktop's own surfaces.
pub fn background(key: ThemeKey) -> Option<Color> {
    if key.theme == SyntaxTheme::Cosmic {
        return None;
    }
    HIGHLIGHTER.theme(key).settings.background.map(to_color)
}

fn to_color(c: syntect::highlighting::Color) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, f32::from(c.a) / 255.0)
}

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
pub fn highlight(code: &str, language: Option<&str>, key: ThemeKey, default: Color) -> Vec<Line> {
    let highlighter = &*HIGHLIGHTER;
    let theme = highlighter.theme(key);
    let syntax = find_syntax(&highlighter.syntaxes, language);
    let mut lines = HighlightLines::new(syntax, theme);

    LinesWithEndings::from(code)
        .map(
            |line| match lines.highlight_line(line, &highlighter.syntaxes) {
                Ok(ranges) => ranges
                    .into_iter()
                    .map(|(style, text)| Span {
                        text: text.trim_end_matches(['\n', '\r']).to_owned(),
                        color: to_color(style.foreground),
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

/// Highlights the Markdown source shown in the source view's text editor.
///
/// The editor feeds lines one at a time, without line endings, and restarts from
/// [`current_line`](highlighter::Highlighter::current_line) after a change. The
/// source view is read-only, so changes only happen when the file is reloaded;
/// restarting from the top then is simple and cheap enough.
pub struct SourceHighlighter {
    lines: HighlightLines<'static>,
    key: ThemeKey,
    current: usize,
}

impl SourceHighlighter {
    fn start(key: ThemeKey) -> Self {
        let highlighter = &*HIGHLIGHTER;
        let syntax = find_syntax(&highlighter.syntaxes, Some("md"));
        Self {
            lines: HighlightLines::new(syntax, highlighter.theme(key)),
            key,
            current: 0,
        }
    }
}

impl highlighter::Highlighter for SourceHighlighter {
    type Settings = ThemeKey;
    type Highlight = Color;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Color)>;

    fn new(key: &ThemeKey) -> Self {
        Self::start(*key)
    }

    fn update(&mut self, key: &ThemeKey) {
        if *key != self.key {
            *self = Self::start(*key);
        }
    }

    fn change_line(&mut self, _line: usize) {
        *self = Self::start(self.key);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current += 1;
        // The syntax set expects line endings; ranges past the line are dropped.
        let with_ending = format!("{line}\n");
        let ranges = match self
            .lines
            .highlight_line(&with_ending, &HIGHLIGHTER.syntaxes)
        {
            Ok(ranges) => ranges,
            Err(error) => {
                log::debug!("highlighting failed: {error}");
                return Vec::new().into_iter();
            }
        };
        let mut start = 0;
        let mut highlights = Vec::with_capacity(ranges.len());
        for (style, text) in ranges {
            let end = (start + text.len()).min(line.len());
            if start < end {
                highlights.push((start..end, to_color(style.foreground)));
            }
            start += text.len();
        }
        highlights.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current
    }
}

/// Text editor format for a [`SourceHighlighter`] colour.
pub fn source_format(
    color: &Color,
    _theme: &cosmic::Theme,
) -> highlighter::Format<cosmic::iced::Font> {
    highlighter::Format {
        color: Some(*color),
        font: None,
    }
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

    const DARK: ThemeKey = ThemeKey {
        theme: SyntaxTheme::Cosmic,
        dark: true,
    };
    const LIGHT: ThemeKey = ThemeKey {
        theme: SyntaxTheme::Cosmic,
        dark: false,
    };

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|s| s.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn keeps_text_and_splits_lines() {
        let lines = highlight(RUST, Some("rust"), DARK, Color::WHITE);
        assert_eq!(text(&lines), ["fn main() {", "    let x = 1;", "}"]);
    }

    #[test]
    fn known_language_gets_several_colors() {
        let lines = highlight(RUST, Some("rs"), LIGHT, Color::BLACK);
        let colors: Vec<Color> = lines.iter().flatten().map(|s| s.color).collect();
        assert!(colors.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn dark_and_light_differ() {
        let dark = highlight(RUST, Some("rust"), DARK, Color::WHITE);
        let light = highlight(RUST, Some("rust"), LIGHT, Color::BLACK);
        assert_ne!(dark, light);
    }

    #[test]
    fn unknown_language_is_plain_text() {
        let lines = highlight("a b c", Some("no-such-language"), DARK, Color::WHITE);
        assert_eq!(text(&lines), ["a b c"]);
    }

    #[test]
    fn source_highlighter_covers_line_within_bounds() {
        use highlighter::Highlighter as _;

        let mut source = SourceHighlighter::new(&DARK);
        let line = "# Title with `code`";
        let ranges: Vec<_> = source.highlight_line(line).collect();
        assert!(!ranges.is_empty());
        assert!(ranges.iter().all(|(range, _)| range.end <= line.len()));
        assert_eq!(source.current_line(), 1);

        source.change_line(0);
        assert_eq!(source.current_line(), 0);
    }

    #[test]
    fn every_bundled_theme_loads_and_colors_code() {
        for theme in SyntaxTheme::ALL {
            for dark in [true, false] {
                let key = ThemeKey { theme, dark };
                let data = theme.data(dark);
                assert!(
                    ThemeSet::load_from_reader(&mut Cursor::new(data)).is_ok(),
                    "{key:?} does not parse"
                );
                let lines = highlight(RUST, Some("rust"), key, Color::WHITE);
                let colors: Vec<Color> = lines.iter().flatten().map(|s| s.color).collect();
                assert!(
                    colors.windows(2).any(|pair| pair[0] != pair[1]),
                    "{key:?} has no colours"
                );
            }
        }
    }

    #[test]
    fn only_non_cosmic_themes_bring_a_background() {
        assert_eq!(background(DARK), None);
        let github = ThemeKey {
            theme: SyntaxTheme::GitHub,
            dark: true,
        };
        assert_eq!(background(github), Some(Color::from_rgb8(0x0d, 0x11, 0x17)));
    }

    #[test]
    fn names_match_themes() {
        assert_eq!(SyntaxTheme::NAMES.len(), SyntaxTheme::ALL.len());
    }

    #[test]
    fn extended_syntaxes_are_available() {
        let set = &HIGHLIGHTER.syntaxes;
        assert_eq!(find_syntax(set, Some("toml")).name, "TOML");
    }
}
