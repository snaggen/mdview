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

/// Background of code blocks: the theme's component colour, with its corner radius.
pub fn code_block(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(Color::from(cosmic.bg_component_color()))),
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
