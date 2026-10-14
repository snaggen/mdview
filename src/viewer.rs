//! How markdown items are turned into widgets, where the defaults do not fit.

use std::collections::HashMap;

use cosmic::iced::alignment::Horizontal;
use cosmic::iced::font::{Style, Weight};
use cosmic::iced::widget::{rich_text, scrollable, span, table, text};
use cosmic::iced::{Font, Length};
use cosmic::widget::markdown::{self, Item, Settings};
use cosmic::widget::{self, image, svg};
use cosmic::{Element, Renderer, Theme};

use crate::app::Message;
use crate::document::code_key;
use crate::highlight;
use crate::style;

/// A decoded local image referenced by the document.
#[derive(Debug, Clone)]
pub enum Image {
    Raster(image::Handle),
    Svg(svg::Handle),
}

/// Column widths in a table are proportional to their longest text, capped so one
/// long cell cannot squeeze the other columns to nothing.
const MIN_COLUMN_WEIGHT: usize = 3;
const MAX_COLUMN_WEIGHT: usize = 40;

/// The [`markdown::Viewer`] used for the rendered document.
pub struct DocViewer<'a> {
    pub images: &'a HashMap<markdown::Uri, Image>,
    /// Highlighted code blocks, keyed by [`code_key`]. Blocks not highlighted yet are
    /// shown as plain text.
    pub highlights: &'a HashMap<u64, Vec<highlight::Line>>,
}

impl<'a> markdown::Viewer<'a, Message, Theme, Renderer> for DocViewer<'a> {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::LinkClicked(url)
    }

    /// Shows local images that have been loaded; anything else (remote images,
    /// missing files) shows its alt text.
    fn image(
        &self,
        settings: Settings,
        url: &'a markdown::Uri,
        _title: &'a str,
        alt: &markdown::Text,
    ) -> Element<'a, Message> {
        match self.images.get(url) {
            Some(Image::Raster(handle)) => widget::image(handle.clone()).into(),
            Some(Image::Svg(handle)) => widget::svg(handle.clone()).into(),
            None => widget::container(
                rich_text(alt.spans(settings.style)).on_link_click(Self::on_link_click),
            )
            .padding(settings.spacing.0)
            .class(cosmic::theme::Container::Card)
            .into(),
        }
    }

    /// Code blocks use the COSMIC syntax themes (the default one is always dark) and
    /// scroll sideways rather than wrapping.
    fn code_block(
        &self,
        settings: Settings,
        language: Option<&'a str>,
        code: &'a str,
        lines: &'a [markdown::Text],
    ) -> Element<'a, Message> {
        let font = settings.style.code_block_font;
        let lines: Vec<Element<'a, Message>> = match self.highlights.get(&code_key(language, code))
        {
            Some(highlighted) => highlighted
                .iter()
                .map(|line| code_line(line_spans(line, font), settings))
                .collect(),
            None => lines
                .iter()
                .map(|line| code_line(line.spans(settings.style).to_vec(), settings))
                .collect(),
        };

        widget::container(
            scrollable(
                widget::container(widget::column::with_children(lines)).padding(settings.code_size),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            )),
        )
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(style::code_block))
        .into()
    }

    /// Tables span the page width and wrap their cells, instead of the default
    /// horizontally scrolling table that grows as wide as its longest line.
    fn table(
        &self,
        settings: Settings,
        columns: &'a [markdown::Column],
        rows: &'a [markdown::Row],
    ) -> Element<'a, Message> {
        let weights = column_weights(&settings, columns, rows);
        let columns = columns
            .iter()
            .zip(weights)
            .enumerate()
            .map(|(i, (column, weight))| {
                table::column(
                    markdown::items(self, settings, &column.header),
                    move |row: &'a markdown::Row| match row.cells.get(i) {
                        Some(cells) => markdown::items(self, settings, cells),
                        None => widget::text("").into(),
                    },
                )
                .width(Length::FillPortion(weight))
                .align_x(alignment(column.alignment))
            });

        table(columns, rows)
            .width(Length::Fill)
            .padding_x(settings.spacing.0)
            .padding_y(settings.spacing.0 / 2.0)
            .separator_x(0)
            .into()
    }
}

/// One line of a code block.
fn code_line<'a>(
    spans: Vec<text::Span<'a, markdown::Uri>>,
    settings: Settings,
) -> Element<'a, Message> {
    // An empty line would have no height, collapsing blank lines in the code.
    let spans = if spans.is_empty() {
        vec![span(" ")]
    } else {
        spans
    };
    rich_text(spans)
        .font(settings.style.code_block_font)
        .size(settings.code_size)
        .into()
}

/// Turns highlighted spans into text spans in the code font.
fn line_spans(line: &highlight::Line, font: Font) -> Vec<text::Span<'_, markdown::Uri>> {
    line.iter()
        .map(|s| {
            let font = Font {
                weight: if s.bold { Weight::Bold } else { Weight::Normal },
                style: if s.italic {
                    Style::Italic
                } else {
                    Style::Normal
                },
                ..font
            };
            span(s.text.as_str()).color(s.color).font(font)
        })
        .collect()
}

/// Relative widths for the columns of a table, from the longest text in each.
fn column_weights(
    settings: &Settings,
    columns: &[markdown::Column],
    rows: &[markdown::Row],
) -> Vec<u16> {
    columns
        .iter()
        .enumerate()
        .map(|(i, column)| {
            let header = text_len(settings, &column.header);
            let cells = rows
                .iter()
                .filter_map(|row| row.cells.get(i))
                .map(|cell| text_len(settings, cell));
            let longest = cells.fold(header, usize::max);
            let weight = longest.clamp(MIN_COLUMN_WEIGHT, MAX_COLUMN_WEIGHT);
            u16::try_from(weight).unwrap_or(u16::MAX)
        })
        .collect()
}

/// Number of characters of text in `items`, as a measure of how wide they want to be.
fn text_len(settings: &Settings, items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Paragraph(text) | Item::Heading(_, text) => text
                .spans(settings.style)
                .iter()
                .map(|span| span.text.chars().count())
                .sum(),
            Item::CodeBlock { code, .. } => {
                code.lines().map(|l| l.chars().count()).max().unwrap_or(0)
            }
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn alignment(alignment: pulldown_cmark::Alignment) -> Horizontal {
    match alignment {
        pulldown_cmark::Alignment::Center => Horizontal::Center,
        pulldown_cmark::Alignment::Right => Horizontal::Right,
        pulldown_cmark::Alignment::None | pulldown_cmark::Alignment::Left => Horizontal::Left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weights(source: &str) -> Vec<u16> {
        let content = markdown::Content::parse(source);
        let settings = crate::style::markdown_settings(&Theme::dark());
        match content.items() {
            [Item::Table { columns, rows }] => column_weights(&settings, columns, rows),
            items => panic!("expected one table, got {items:?}"),
        }
    }

    #[test]
    fn columns_are_weighted_by_longest_text() {
        let table = "| # | Description |\n|---|---|\n| 1 | A somewhat longer cell |\n";
        assert_eq!(weights(table), vec![3, 22]);
    }

    #[test]
    fn long_cells_are_capped() {
        let long = "word ".repeat(50);
        let table = format!("| a | b |\n|---|---|\n| {long} | short |\n");
        assert_eq!(weights(&table), vec![40, 5]);
    }
}
