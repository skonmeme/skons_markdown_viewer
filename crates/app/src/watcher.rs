//! File system notifications for open documents (STANDARD §4.4).

use std::path::Path;

use dispatch2::DispatchQueue;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Watches the directory that contains a document. Watching the directory
/// instead of the file keeps working when editors save by replacing the file.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Calls `on_event(token)` on the main thread for every change in the
    /// directory of `path`. The callback decides whether the file changed.
    pub fn new(path: &Path, token: u64, on_event: fn(u64)) -> Option<Self> {
        let directory = path.parent()?;
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    DispatchQueue::main().exec_async(move || on_event(token));
                }
            })
            .ok()?;
        watcher.watch(directory, RecursiveMode::NonRecursive).ok()?;
        Some(Self { _watcher: watcher })
    }
}
