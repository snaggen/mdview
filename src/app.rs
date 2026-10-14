//! The COSMIC application: state, messages, update and view.

use std::path::PathBuf;

use cosmic::app::{Core, Task};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, button, icon, markdown, tooltip};
use cosmic::{Application, ApplicationExt, Element, action, theme};

use crate::document::{self, Document};
use crate::fl;
use crate::style;
use crate::watch;

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

impl State {
    /// The file being shown, if any.
    fn path(&self) -> Option<&PathBuf> {
        match self {
            State::Empty => None,
            State::Loaded(doc) => Some(&doc.path),
            State::Failed { path, .. } => Some(path),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenDialog,
    Open(PathBuf),
    /// A read finished. `request` identifies the read, so results that arrive out
    /// of order (e.g. a reload finishing after another file was opened) are dropped.
    Loaded {
        request: u64,
        path: PathBuf,
        result: Result<String, String>,
    },
    FileChanged,
    LinkClicked(markdown::Uri),
}

pub struct App {
    core: Core,
    state: State,
    /// Id of the most recent read; see [`Message::Loaded`].
    request: u64,
    /// Set when re-reading the open document failed (e.g. it was deleted). The last
    /// good content stays visible and the next change on disk retries.
    reload_error: Option<String>,
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
            request: 0,
            reload_error: None,
        };
        let title = app.update_title();
        let open = match flags.path {
            Some(path) => app.open(path),
            None => Task::none(),
        };
        (app, Task::batch([title, open]))
    }

    fn header_end(&self) -> Vec<Element<'_, Message>> {
        let mut items = Vec::new();
        if let Some(error) = &self.reload_error {
            items.push(warning_indicator(fl!(
                "reload-failed",
                error = error.as_str()
            )));
        }
        items.push(icon_button(
            "document-open-symbolic",
            fl!("open-file"),
            Message::OpenDialog,
        ));
        items
    }

    fn subscription(&self) -> Subscription<Message> {
        match self.state.path() {
            Some(path) => watch::subscription(path.clone()).map(|()| Message::FileChanged),
            None => Subscription::none(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenDialog => return open_dialog(),
            Message::Open(path) => return self.open(path),
            Message::Loaded {
                request,
                path,
                result,
            } => {
                if request == self.request {
                    self.loaded(path, result);
                    return self.update_title();
                }
            }
            Message::FileChanged => {
                if let Some(path) = self.state.path() {
                    return self.read(path.clone());
                }
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
    /// Opens `path`, replacing the current document once it has been read.
    fn open(&mut self, path: PathBuf) -> Task<Message> {
        self.read(path)
    }

    /// Starts reading `path` in the background.
    fn read(&mut self, path: PathBuf) -> Task<Message> {
        self.request += 1;
        let request = self.request;
        Task::perform(document::read(path), move |(path, result)| {
            action::app(Message::Loaded {
                request,
                path,
                result,
            })
        })
    }

    /// Applies a finished read.
    ///
    /// A failed re-read of the open document keeps its last content, since editors
    /// briefly remove or truncate files while saving.
    fn loaded(&mut self, path: PathBuf, result: Result<String, String>) {
        let is_reload = matches!(&self.state, State::Loaded(doc) if doc.path == path);
        match result {
            Ok(source) => {
                self.state = State::Loaded(Document::new(path, source));
                self.reload_error = None;
            }
            Err(error) if is_reload => self.reload_error = Some(error),
            Err(error) => {
                self.state = State::Failed { path, error };
                self.reload_error = None;
            }
        }
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

/// A warning icon in the header bar, explaining itself in a tooltip.
fn warning_indicator<'a>(message: String) -> Element<'a, Message> {
    tooltip(
        icon::from_name("dialog-warning-symbolic").size(16),
        widget::text::body(message),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App {
            core: Core::default(),
            state: State::Empty,
            request: 0,
            reload_error: None,
        }
    }

    fn path() -> PathBuf {
        PathBuf::from("/docs/README.md")
    }

    #[test]
    fn successful_read_shows_document() {
        let mut app = app();
        app.loaded(path(), Ok("# Hello".into()));
        assert!(matches!(&app.state, State::Loaded(doc) if doc.path == path()));
        assert!(app.reload_error.is_none());
    }

    #[test]
    fn failed_first_read_shows_error() {
        let mut app = app();
        app.loaded(path(), Err("not found".into()));
        assert!(matches!(&app.state, State::Failed { error, .. } if error == "not found"));
    }

    #[test]
    fn failed_reload_keeps_last_content() {
        let mut app = app();
        app.loaded(path(), Ok("# Hello".into()));
        app.loaded(path(), Err("removed".into()));
        assert!(matches!(app.state, State::Loaded(_)));
        assert_eq!(app.reload_error.as_deref(), Some("removed"));

        app.loaded(path(), Ok("# Back".into()));
        assert!(app.reload_error.is_none());
    }

    #[test]
    fn failed_read_of_other_file_replaces_document() {
        let mut app = app();
        app.loaded(path(), Ok("# Hello".into()));
        app.loaded(PathBuf::from("/docs/other.md"), Err("not found".into()));
        assert!(matches!(app.state, State::Failed { .. }));
    }

    #[test]
    fn stale_read_is_ignored() {
        let mut app = app();
        let _ = app.read(path());
        let stale = app.request;
        let _ = app.read(PathBuf::from("/docs/other.md"));
        let _ = app.update(Message::Loaded {
            request: stale,
            path: path(),
            result: Ok("# Old".into()),
        });
        assert!(matches!(app.state, State::Empty));
    }
}
