//! The COSMIC application: state, messages, update and view.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::app::{Core, Task};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::{Alignment, Color, Length, Subscription};
use cosmic::widget::{self, button, icon, markdown, tooltip};
use cosmic::{Application, ApplicationExt, Element, action, theme};

use crate::document::{self, CodeBlock, Document};
use crate::fl;
use crate::highlight;
use crate::link::{self, Link};
use crate::style;
use crate::viewer::{DocViewer, Image};
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
    /// Images of the document read by request `request` have been loaded.
    ImagesLoaded {
        request: u64,
        images: HashMap<markdown::Uri, Image>,
    },
    /// Code blocks of the document read by request `request` have been highlighted
    /// for a dark (`dark == true`) or light theme.
    Highlighted {
        request: u64,
        dark: bool,
        highlights: HashMap<u64, Vec<highlight::Line>>,
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
    /// Local images of the open document, keyed by the URL written in the Markdown.
    images: HashMap<markdown::Uri, Image>,
    /// Highlighted code blocks of the open document; see [`DocViewer::highlights`].
    highlights: HashMap<u64, Vec<highlight::Line>>,
    /// Whether the desktop uses a dark theme, which selects the syntax theme.
    dark: bool,
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
            images: HashMap::new(),
            highlights: HashMap::new(),
            dark: theme::active().cosmic().is_dark,
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
                    return Task::batch([
                        self.update_title(),
                        self.load_images(),
                        self.highlight_code(),
                    ]);
                }
            }
            Message::ImagesLoaded { request, images } => {
                if request == self.request {
                    self.images = images;
                }
            }
            Message::Highlighted {
                request,
                dark,
                highlights,
            } => {
                if request == self.request && dark == self.dark {
                    self.highlights = highlights;
                }
            }
            Message::FileChanged => {
                if let Some(path) = self.state.path() {
                    return self.read(path.clone());
                }
            }
            Message::LinkClicked(url) => return self.follow_link(&url),
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
            State::Loaded(doc) => self.rendered(doc),
        }
    }

    fn system_theme_update(
        &mut self,
        _keys: &[&'static str],
        new_theme: &cosmic::cosmic_theme::Theme,
    ) -> Task<Message> {
        self.set_dark(new_theme.is_dark)
    }

    fn system_theme_mode_update(
        &mut self,
        _keys: &[&'static str],
        new_theme: &cosmic::cosmic_theme::ThemeMode,
    ) -> Task<Message> {
        self.set_dark(new_theme.is_dark)
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

    /// Starts loading the images of the open document. Images are re-read on every
    /// reload, so edited images show up together with the edited text.
    fn load_images(&self) -> Task<Message> {
        let State::Loaded(doc) = &self.state else {
            return Task::none();
        };
        let request = self.request;
        let urls = doc.content.images().iter().cloned().collect();
        Task::perform(
            document::load_images(doc.base_dir().to_path_buf(), urls),
            move |images| action::app(Message::ImagesLoaded { request, images }),
        )
    }

    /// Starts highlighting the code blocks of the open document, off the UI thread.
    /// Until it finishes, code blocks are shown without colours.
    fn highlight_code(&self) -> Task<Message> {
        let State::Loaded(doc) = &self.state else {
            return Task::none();
        };
        let request = self.request;
        let dark = self.dark;
        let blocks = document::code_blocks(doc.content.items());
        let plain = Color::from(theme::active().cosmic().on_bg_component_color());
        let work = tokio::task::spawn_blocking(move || highlight_blocks(&blocks, dark, plain));
        Task::perform(work, move |result| {
            let highlights = result.unwrap_or_else(|error| {
                log::warn!("highlighting failed: {error}");
                HashMap::new()
            });
            action::app(Message::Highlighted {
                request,
                dark,
                highlights,
            })
        })
    }

    /// Re-highlights code when the desktop switches between light and dark.
    fn set_dark(&mut self, dark: bool) -> Task<Message> {
        if dark == self.dark {
            return Task::none();
        }
        self.dark = dark;
        self.highlight_code()
    }

    /// The rendered document in a scrollable, centered column.
    fn rendered<'a>(&'a self, doc: &'a Document) -> Element<'a, Message> {
        let theme = theme::active();
        let spacing = theme.cosmic().spacing;
        let viewer = DocViewer {
            images: &self.images,
            highlights: &self.highlights,
        };
        let content = markdown::view_with(
            doc.content.items(),
            style::markdown_settings(&theme),
            &viewer,
        );

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

    /// Opens a clicked link: Markdown files in this window, everything else with the
    /// desktop's default application.
    fn follow_link(&mut self, url: &str) -> Task<Message> {
        let State::Loaded(doc) = &self.state else {
            return Task::none();
        };
        match link::resolve(url, doc.base_dir()) {
            Some(Link::Document(path)) => return self.open(path),
            Some(Link::External(url)) => open_externally(url),
            Some(Link::File(path)) => open_externally(path.into_os_string()),
            // Scrolling to headings is not supported.
            Some(Link::Anchor) => {}
            None => log::warn!("cannot resolve link {url}"),
        }
        Task::none()
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

/// Highlights each code block, keyed by [`document::code_key`].
fn highlight_blocks(
    blocks: &[CodeBlock],
    dark: bool,
    plain: Color,
) -> HashMap<u64, Vec<highlight::Line>> {
    blocks
        .iter()
        .map(|(language, code)| {
            let language = language.as_deref();
            let lines = highlight::highlight(code, language, dark, plain);
            (document::code_key(language, code), lines)
        })
        .collect()
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

/// Hands a URL or path to the desktop's default handler, without blocking the UI.
fn open_externally(target: impl Into<std::ffi::OsString>) {
    let target = target.into();
    if let Err(error) = open::that_detached(&target) {
        log::warn!("cannot open {}: {error}", target.to_string_lossy());
    }
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
            images: HashMap::new(),
            highlights: HashMap::new(),
            dark: true,
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
    fn highlights_for_other_theme_are_ignored() {
        let mut app = app();
        app.loaded(path(), Ok("```rust\nfn a() {}\n```".into()));
        let highlights = HashMap::from([(1, Vec::new())]);
        let _ = app.update(Message::Highlighted {
            request: app.request,
            dark: false,
            highlights: highlights.clone(),
        });
        assert!(app.highlights.is_empty());

        let _ = app.update(Message::Highlighted {
            request: app.request,
            dark: true,
            highlights,
        });
        assert_eq!(app.highlights.len(), 1);
    }

    #[test]
    fn highlight_blocks_keys_by_language_and_code() {
        let blocks = vec![(Some("rust".to_owned()), "fn a() {}".to_owned())];
        let highlights = highlight_blocks(&blocks, true, Color::WHITE);
        let key = document::code_key(Some("rust"), "fn a() {}");
        assert_eq!(highlights[&key].len(), 1);
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
