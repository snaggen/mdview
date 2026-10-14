//! Reading and parsing a Markdown document.

use std::path::{Path, PathBuf};

use cosmic::widget::markdown;

/// A Markdown file that has been read and parsed.
pub struct Document {
    pub path: PathBuf,
    pub content: markdown::Content,
}

impl Document {
    /// Parses `source`, read from `path`.
    pub fn new(path: PathBuf, source: String) -> Self {
        let content = markdown::Content::parse(&source);
        Self { path, content }
    }

    /// The name shown in the header bar and window title.
    pub fn title(&self) -> String {
        display_name(&self.path)
    }
}

/// The file name of `path`, or the whole path if it has none.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Reads the file at `path` as UTF-8.
///
/// The error is returned as a string, since application messages have to be `Clone`.
pub async fn read(path: PathBuf) -> (PathBuf, Result<String, String>) {
    let result = tokio::fs::read_to_string(&path)
        .await
        .map_err(|error| error.to_string());
    (path, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_uses_file_name() {
        assert_eq!(display_name(Path::new("/tmp/docs/README.md")), "README.md");
    }

    #[test]
    fn display_name_falls_back_to_path() {
        assert_eq!(display_name(Path::new("/")), "/");
    }

    #[tokio::test]
    async fn read_reports_missing_file() {
        let (_, result) = read(PathBuf::from("/nonexistent/file.md")).await;
        assert!(result.is_err());
    }

    #[test]
    fn document_parses_source() {
        let doc = Document::new(PathBuf::from("a.md"), "# Title\n\nText".into());
        assert_eq!(doc.content.items().len(), 2);
        assert_eq!(doc.title(), "a.md");
    }
}
