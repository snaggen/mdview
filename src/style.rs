//! Markdown styling derived from the active COSMIC theme.

use cosmic::iced::widget::{container, text_editor};
use cosmic::iced::{Background, Color, border, padding};
use cosmic::widget::markdown;

/// Base text size of the rendered document. Matches COSMIC's body text size;
/// heading sizes are derived from it by [`markdown::Settings::with_text_size`].
const TEXT_SIZE: f32 = 14.0;

/// Rendering settings for the markdown widget, using the theme's fonts and colours.
pub fn markdown_settings(theme: &cosmic::Theme) -> markdown::Settings {
    markdown::Settings {
        // The default (75% of the text size) is too small to read comfortably.
        code_size: (TEXT_SIZE * 0.9).into(),
        ..markdown::Settings::with_text_size(TEXT_SIZE, markdown_style(theme))
    }
}

/// Background of code blocks: the syntax theme's own background if it has one,
/// otherwise the COSMIC component colour, with the theme's corner radius.
pub fn code_block(theme: &cosmic::Theme, background: Option<Color>) -> container::Style {
    let cosmic = theme.cosmic();
    let background = background.unwrap_or_else(|| Color::from(cosmic.bg_component_color()));
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(Color::from(cosmic.on_bg_component_color())),
        border: border::rounded(cosmic.corner_radii.radius_s),
        ..container::Style::default()
    }
}

/// The markdown widget's default style hard-codes a dark inline-code background,
/// so build one from the COSMIC theme instead.
fn markdown_style(theme: &cosmic::Theme) -> markdown::Style {
    let cosmic = theme.cosmic();
    let space = f32::from(cosmic.spacing.space_xxxs);
    markdown::Style {
        font: cosmic::font::default(),
        inline_code_highlight: markdown::Highlight {
            background: Background::Color(Color::from(cosmic.bg_component_color())),
            border: border::rounded(cosmic.corner_radii.radius_xs),
        },
        inline_code_padding: padding::left(space).right(space),
        inline_code_color: Color::from(cosmic.on_bg_component_color()),
        inline_code_font: cosmic::font::mono(),
        code_block_font: cosmic::font::mono(),
        link_color: Color::from(cosmic.accent_text_color()),
    }
}

/// The source view is a plain surface: no field border or background of its own,
/// so it reads as the page rather than an input field.
pub fn source_editor(theme: &cosmic::Theme, _status: text_editor::Status) -> text_editor::Style {
    let cosmic = theme.cosmic();
    let value = Color::from(cosmic.on_bg_color());
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: border::Border::default(),
        placeholder: Color { a: 0.7, ..value },
        value,
        selection: Color {
            a: 0.4,
            ..Color::from(cosmic.accent_color())
        },
    }
}

/// Background of the content area: the theme's window background at the opacity
/// chosen in the settings. Bottom corners follow the window's rounded corners.
pub fn content(theme: &cosmic::Theme, alpha: f32, radius: [f32; 4]) -> container::Style {
    let cosmic = theme.cosmic();
    let background = Color::from(cosmic.bg_color());
    container::Style {
        background: Some(Background::Color(Color {
            a: alpha,
            ..background
        })),
        text_color: Some(Color::from(cosmic.on_bg_color())),
        border: border::rounded([0.0, 0.0, radius[2], radius[3]]),
        ..container::Style::default()
    }
}

/// Corner radius of the window, matching what libcosmic uses for its frame.
pub fn window_radius(theme: &cosmic::Theme, square: bool) -> [f32; 4] {
    let cosmic = theme.cosmic();
    if square {
        cosmic.radius_0()
    } else {
        // libcosmic enlarges the radius of larger corners to visually align content
        // with the window frame; the 1px frame border is subtracted.
        cosmic
            .radius_s()
            .map(|r| if r < 4.0 { r } else { r + 4.0 } - 1.0)
            .map(|r| r.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_window_has_no_radius() {
        let theme = cosmic::Theme::dark();
        assert_eq!(window_radius(&theme, true), theme.cosmic().radius_0());
    }

    #[test]
    fn content_uses_requested_alpha() {
        let theme = cosmic::Theme::dark();
        let style = content(&theme, 0.5, [0.0; 4]);
        let Some(Background::Color(color)) = style.background else {
            panic!("expected a colour background");
        };
        assert_eq!(color.a, 0.5);
    }
}
