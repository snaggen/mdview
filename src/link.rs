//! Resolving links and image references in a document relative to its location.

use std::path::{Path, PathBuf};

use url::Url;

/// Where a link in the document points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// A web page or other URL, handed to the desktop's default handler.
    External(String),
    /// A local Markdown file, opened in this viewer.
    Document(PathBuf),
    /// Any other local file, handed to the desktop's default handler.
    File(PathBuf),
    /// A link to a heading in the same document.
    Anchor,
}

/// Resolves `target` (as written in the Markdown) against the directory of the
/// document it appears in.
///
/// Returns `None` for links that cannot be parsed.
pub fn resolve(target: &str, base_dir: &Path) -> Option<Link> {
    if target.starts_with('#') {
        return Some(Link::Anchor);
    }
    let mut url = join(target, base_dir)?;
    if url.scheme() != "file" {
        return Some(Link::External(url.into()));
    }
    url.set_fragment(None);
    let path = url.to_file_path().ok()?;
    if is_markdown(&path) {
        Some(Link::Document(path))
    } else {
        Some(Link::File(path))
    }
}

/// The local file an image reference points to, or `None` for remote images,
/// which are not fetched.
pub fn local_file(target: &str, base_dir: &Path) -> Option<PathBuf> {
    let url = join(target, base_dir)?;
    if url.scheme() == "file" {
        url.to_file_path().ok()
    } else {
        None
    }
}

/// Parses `target` as an absolute URL, or as a reference relative to `base_dir`.
/// Going through `Url` takes care of percent-encoding (`my%20file.md`) and `..`.
fn join(target: &str, base_dir: &Path) -> Option<Url> {
    let base = Url::from_directory_path(base_dir).ok()?;
    base.join(target).ok()
}

/// Whether `path` has a Markdown file extension.
pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["md", "markdown", "mdown", "mkd"]
                .iter()
                .any(|md| ext.eq_ignore_ascii_case(md))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PathBuf {
        PathBuf::from("/home/user/docs")
    }

    #[test]
    fn web_link_is_external() {
        assert_eq!(
            resolve("https://example.com/a?b#c", &base()),
            Some(Link::External("https://example.com/a?b#c".into()))
        );
    }

    #[test]
    fn mailto_is_external() {
        assert_eq!(
            resolve("mailto:a@example.com", &base()),
            Some(Link::External("mailto:a@example.com".into()))
        );
    }

    #[test]
    fn relative_markdown_link_opens_in_viewer() {
        assert_eq!(
            resolve("../README.md#install", &base()),
            Some(Link::Document(PathBuf::from("/home/user/README.md")))
        );
    }

    #[test]
    fn percent_encoded_path_is_decoded() {
        assert_eq!(
            resolve("my%20notes.MD", &base()),
            Some(Link::Document(PathBuf::from("/home/user/docs/my notes.MD")))
        );
    }

    #[test]
    fn absolute_file_url_is_local() {
        assert_eq!(
            resolve("file:///tmp/report.pdf", &base()),
            Some(Link::File(PathBuf::from("/tmp/report.pdf")))
        );
    }

    #[test]
    fn fragment_only_is_anchor() {
        assert_eq!(resolve("#usage", &base()), Some(Link::Anchor));
    }

    #[test]
    fn local_image_is_resolved() {
        assert_eq!(
            local_file("img/logo.png", &base()),
            Some(PathBuf::from("/home/user/docs/img/logo.png"))
        );
    }

    #[test]
    fn remote_image_is_skipped() {
        assert_eq!(local_file("https://example.com/logo.png", &base()), None);
    }
}
