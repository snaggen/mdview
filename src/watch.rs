//! Live reload: watches the open file and reports when it changes on disk.

use std::path::{Path, PathBuf};
use std::time::Duration;

use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::futures::{SinkExt, Stream, StreamExt};
use cosmic::iced::{Subscription, stream};
use notify_debouncer_full::notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, new_debouncer};

/// Editors write files in bursts; changes within this window are reported once.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Emits `()` every time `path` is written, replaced, created or removed.
///
/// The subscription is keyed on the path, so opening another file replaces the watcher.
pub fn subscription(path: PathBuf) -> Subscription<()> {
    Subscription::run_with(path, |path| watch(path.clone()))
}

/// Watches the file's parent directory rather than the file itself. Many editors save
/// by writing a temporary file and renaming it over the original, which a watch on the
/// original inode would stop seeing after the first save.
fn watch(path: PathBuf) -> impl Stream<Item = ()> {
    stream::channel(1, async move |mut output| {
        let Some(dir) = path.parent().map(Path::to_path_buf) else {
            return;
        };
        let (tx, mut rx) = mpsc::unbounded();
        let debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            // The receiver is gone once the subscription is dropped; nothing to do then.
            let _ = tx.unbounded_send(result);
        });
        let mut debouncer = match debouncer {
            Ok(debouncer) => debouncer,
            Err(error) => {
                log::warn!("cannot watch {}: {error}", path.display());
                return;
            }
        };
        if let Err(error) = debouncer.watch(&dir, RecursiveMode::NonRecursive) {
            log::warn!("cannot watch {}: {error}", dir.display());
            return;
        }

        while let Some(result) = rx.next().await {
            match result {
                Ok(events) if events.iter().any(|e| affects(&e.kind, &e.paths, &path)) => {
                    if output.send(()).await.is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(errors) => log::warn!("file watch errors: {errors:?}"),
            }
        }
    })
}

/// Whether an event in the watched directory concerns the open file.
fn affects(kind: &EventKind, paths: &[PathBuf], file: &Path) -> bool {
    let relevant = matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    relevant && paths.iter().any(|p| p == file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_full::notify::event::{
        AccessKind, CreateKind, DataChange, ModifyKind, RenameMode,
    };

    fn file() -> PathBuf {
        PathBuf::from("/docs/README.md")
    }

    #[test]
    fn write_to_file_is_relevant() {
        let kind = EventKind::Modify(ModifyKind::Data(DataChange::Content));
        assert!(affects(&kind, &[file()], &file()));
    }

    #[test]
    fn rename_over_file_is_relevant() {
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        let paths = [PathBuf::from("/docs/.README.md.swp"), file()];
        assert!(affects(&kind, &paths, &file()));
    }

    #[test]
    fn other_file_in_directory_is_ignored() {
        let kind = EventKind::Create(CreateKind::File);
        assert!(!affects(&kind, &[PathBuf::from("/docs/other.md")], &file()));
    }

    #[test]
    fn access_is_ignored() {
        let kind = EventKind::Access(AccessKind::Read);
        assert!(!affects(&kind, &[file()], &file()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reports_change_on_disk() {
        let dir = std::env::temp_dir().join(format!("mdview-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.md");
        std::fs::write(&path, "one").unwrap();

        // The watcher is registered when the stream is first polled, so poll it in
        // the background and give it time to start before changing the file.
        let mut events = Box::pin(watch(path.clone()));
        let next = tokio::spawn(async move { events.next().await });
        tokio::time::sleep(Duration::from_millis(200)).await;
        std::fs::write(&path, "two").unwrap();

        let event = tokio::time::timeout(Duration::from_secs(5), next).await;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(event.unwrap().unwrap(), Some(()));
    }
}
