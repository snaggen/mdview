//! Reading and parsing a Markdown document.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use cosmic::widget::markdown::{Bullet, Item};
use cosmic::widget::{image, markdown, svg};

use crate::link;
use crate::viewer::Image;

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

    /// The directory relative links and images are resolved against.
    pub fn base_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("/"))
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

/// A code block of the document: its fence language and its code.
pub type CodeBlock = (Option<String>, String);

/// All code blocks in `items`, including those nested in lists, quotes and tables.
pub fn code_blocks(items: &[Item]) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    collect_code_blocks(items, &mut blocks);
    blocks
}

fn collect_code_blocks(items: &[Item], blocks: &mut Vec<CodeBlock>) {
    for item in items {
        match item {
            Item::CodeBlock { language, code, .. } => blocks.push((language.clone(), code.clone())),
            Item::List { bullets, .. } => {
                for bullet in bullets {
                    let (Bullet::Point { items } | Bullet::Task { items, .. }) = bullet;
                    collect_code_blocks(items, blocks);
                }
            }
            Item::Quote(items) => collect_code_blocks(items, blocks),
            Item::Table { columns, rows } => {
                for column in columns {
                    collect_code_blocks(&column.header, blocks);
                }
                for cell in rows.iter().flat_map(|row| &row.cells) {
                    collect_code_blocks(cell, blocks);
                }
            }
            Item::Heading(..) | Item::Paragraph(_) | Item::Image { .. } | Item::Rule => {}
        }
    }
}

/// Key under which the highlighted lines of a code block are cached.
pub fn code_key(language: Option<&str>, code: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    (language, code).hash(&mut hasher);
    hasher.finish()
}

/// Reads the local images referenced by a document. Remote images and files that
/// cannot be read are left out, so they show their alt text.
pub async fn load_images(
    base_dir: PathBuf,
    urls: Vec<markdown::Uri>,
) -> HashMap<markdown::Uri, Image> {
    let mut images = HashMap::new();
    for url in urls {
        let Some(path) = link::local_file(&url, &base_dir) else {
            continue;
        };
        match tokio::fs::read(&path).await {
            Ok(bytes) => {
                images.insert(url, decode(&path, bytes));
            }
            Err(error) => log::debug!("cannot read image {}: {error}", path.display()),
        }
    }
    images
}

/// Wraps image bytes in the handle type matching the file. Decoding itself
/// happens in the renderer.
fn decode(path: &Path, bytes: Vec<u8>) -> Image {
    let is_svg = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
    if is_svg {
        Image::Svg(svg::Handle::from_memory(bytes))
    } else {
        Image::Raster(image::Handle::from_bytes(bytes))
    }
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

    #[tokio::test]
    async fn load_images_skips_remote_and_missing() {
        let dir = std::env::temp_dir().join(format!("mdview-img-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.svg"), "<svg/>").unwrap();
        std::fs::write(dir.join("b.png"), [0u8; 4]).unwrap();

        let urls = ["a.svg", "b.png", "missing.png", "https://example.com/c.png"];
        let images = load_images(dir.clone(), urls.map(String::from).to_vec()).await;
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(matches!(images.get("a.svg"), Some(Image::Svg(_))));
        assert!(matches!(images.get("b.png"), Some(Image::Raster(_))));
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn code_blocks_are_found_when_nested() {
        let source = "```rust\nfn a() {}\n```\n\n- item\n\n  ```\n  plain\n  ```\n\n> ```py\n> x = 1\n> ```\n";
        let doc = Document::new(PathBuf::from("a.md"), source.into());
        let blocks = code_blocks(doc.content.items());
        let languages: Vec<_> = blocks.iter().map(|(lang, _)| lang.as_deref()).collect();
        assert_eq!(languages, [Some("rust"), None, Some("py")]);
    }

    #[test]
    fn code_key_depends_on_language_and_code() {
        assert_eq!(code_key(Some("rs"), "x"), code_key(Some("rs"), "x"));
        assert_ne!(code_key(Some("rs"), "x"), code_key(None, "x"));
        assert_ne!(code_key(Some("rs"), "x"), code_key(Some("rs"), "y"));
    }

    #[test]
    fn document_parses_source() {
        let doc = Document::new(PathBuf::from("a.md"), "# Title\n\nText".into());
        assert_eq!(doc.content.items().len(), 2);
        assert_eq!(doc.title(), "a.md");
        assert_eq!(doc.base_dir(), Path::new(""));
    }
}
