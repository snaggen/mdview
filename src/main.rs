//! mdview: a simple Markdown viewer for the COSMIC desktop.

mod app;
mod document;
mod link;
mod localize;
mod style;
mod viewer;
mod watch;

use std::path::PathBuf;

use cosmic::iced::Size;

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    localize::localize();

    // `mdview /path/to/file.md`: the first argument is the file to open. It is made
    // absolute so relative links and file watching do not depend on the working directory.
    let path = std::env::args_os()
        .nth(1)
        .map(|arg| std::path::absolute(&arg).unwrap_or_else(|_| PathBuf::from(arg)));
    let flags = app::Flags { path };
    let settings = cosmic::app::Settings::default().size(Size::new(900.0, 1000.0));
    cosmic::app::run::<app::App>(settings, flags)
}
