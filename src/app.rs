//! The COSMIC application: state, messages, update and view.

use std::path::PathBuf;

use cosmic::app::{Core, Task};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, button, icon, markdown, tooltip};
use cosmic::{Application, ApplicationExt, Element, action, theme};

use crate::document::{self, Document};
use crate::fl;
use crate::style;

/// Maximum width of the document column, so lines stay readable in wide windows.
const READABLE_WIDTH: f32 = 860.0;

/// Size of the icon shown in the empty and error states.
const STATUS_ICON_SIZE: u16 = 64;

/// Command line input.
pub struct Flags {
    pub path: Option<PathBuf>,
}

/// What the content area currently shows.
enum State {
    Empty,
    Loaded(Document),
    Failed { path: PathBuf, error: String },
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenDialog,
    Open(PathBuf),
    Loaded(PathBuf, Result<String, String>),
    LinkClicked(markdown::Uri),
}

pub struct App {
    core: Core,
    state: State,
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = "io.github.snaggen.MdView";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Flags) -> (Self, Task<Message>) {
        let mut app = Self {
            core,
            state: State::Empty,
        };
        let title = app.update_title();
        let open = match flags.path {
            Some(path) => app.open(path),
            None => Task::none(),
        };
        (app, Task::batch([title, open]))
    }

    fn header_end(&self) -> Vec<Element<'_, Message>> {
        vec![icon_button(
            "document-open-symbolic",
            fl!("open-file"),
            Message::OpenDialog,
        )]
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenDialog => return open_dialog(),
            Message::Open(path) => return self.open(path),
            Message::Loaded(path, result) => {
                self.state = match result {
                    Ok(source) => State::Loaded(Document::new(path, source)),
                    Err(error) => State::Failed { path, error },
                };
                return self.update_title();
            }
            Message::LinkClicked(url) => log::debug!("link clicked: {url}"),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Empty => status_page(
                "text-x-generic-symbolic",
                fl!("empty-title"),
                fl!("empty-body"),
            ),
            State::Failed { path, error } => status_page(
                "dialog-error-symbolic",
                fl!("load-failed", path = document::display_name(path)),
                error.clone(),
            ),
            State::Loaded(doc) => rendered(doc),
        }
    }
}

impl App {
    /// Starts reading `path` in the background.
    fn open(&mut self, path: PathBuf) -> Task<Message> {
        Task::perform(document::read(path), |(path, result)| {
            action::app(Message::Loaded(path, result))
        })
    }

    /// Sets header bar and window title from the open document.
    fn update_title(&mut self) -> Task<Message> {
        let title = match &self.state {
            State::Loaded(doc) => doc.title(),
            State::Failed { path, .. } => document::display_name(path),
            State::Empty => fl!("app-title"),
        };
        self.set_header_title(title.clone());
        match self.core.main_window_id() {
            Some(id) => self.set_window_title(title, id),
            None => Task::none(),
        }
    }
}

/// The rendered document in a scrollable, centered column.
fn rendered(doc: &Document) -> Element<'_, Message> {
    let theme = theme::active();
    let spacing = theme.cosmic().spacing;
    let content = markdown::view(doc.content.items(), style::markdown_settings(&theme))
        .map(Message::LinkClicked);

    let column = widget::container(content)
        .max_width(READABLE_WIDTH)
        .padding([spacing.space_m, spacing.space_l]);
    widget::scrollable(
        widget::container(column)
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .into()
}

/// A centered icon, title, explanation and an "Open" button, used for the
/// empty and error states.
fn status_page<'a>(icon_name: &'static str, title: String, body: String) -> Element<'a, Message> {
    let spacing = theme::active().cosmic().spacing;
    let column = widget::column::with_capacity(4)
        .push(icon::from_name(icon_name).size(STATUS_ICON_SIZE))
        .push(widget::text::title3(title))
        .push(widget::text::body(body))
        .push(button::suggested(fl!("open")).on_press(Message::OpenDialog))
        .spacing(spacing.space_s)
        .align_x(Alignment::Center);
    widget::container(column)
        .padding(spacing.space_l)
        .center(Length::Fill)
        .into()
}

/// An icon-only header button with a tooltip.
fn icon_button<'a>(
    icon_name: &'static str,
    label: String,
    message: Message,
) -> Element<'a, Message> {
    tooltip(
        button::icon(icon::from_name(icon_name)).on_press(message),
        widget::text::body(label),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Shows the portal's file chooser filtered on Markdown files.
fn open_dialog() -> Task<Message> {
    cosmic::task::future(async {
        let filter = FileFilter::new(&fl!("markdown-files"))
            .mimetype("text/markdown")
            .glob("*.md")
            .glob("*.markdown");
        let dialog = file_chooser::open::Dialog::new()
            .title(fl!("open-markdown-title"))
            .filter(filter);
        match dialog.open_file().await {
            Ok(response) => match response.url().to_file_path() {
                Ok(path) => action::app(Message::Open(path)),
                Err(()) => action::none(),
            },
            Err(file_chooser::Error::Cancelled) => action::none(),
            Err(error) => {
                log::warn!("file chooser failed: {error}");
                action::none()
            }
        }
    })
}
