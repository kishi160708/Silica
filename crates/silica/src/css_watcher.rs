//! Watch a vault's `.obsidian/themes/` and `.obsidian/snippets/` directories
//! for changes and signal the main thread so community CSS themes and
//! snippets can be reloaded without restarting.
//!
//! Uses [`notify`] on a background thread and a [`std::sync::mpsc`] channel
//! that the render loop drains non-blockingly.

use std::path::Path;
use std::sync::mpsc;

use notify::RecommendedWatcher;
use notify::{RecursiveMode, Watcher as _};

/// Watches a vault's CSS theme and snippet directories for changes.
///
/// Returns the watcher (which must be kept alive) and a receiver that
/// yields `()` whenever a watched file is created, modified or removed.
pub struct CssWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl CssWatcher {
    /// Watch `vault_path/.obsidian/themes/` and
    /// `vault_path/.obsidian/snippets/` (only when they exist).
    pub fn new(vault_path: &Path) -> anyhow::Result<(Self, mpsc::Receiver<()>)> {
        let (tx, rx) = mpsc::channel::<()>();
        let tx = std::sync::Mutex::new(tx);

        let mut watcher = RecommendedWatcher::new(
            move |event: Result<notify::Event, notify::Error>| {
                if let Ok(event) = event {
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_)
                            | notify::EventKind::Create(_)
                            | notify::EventKind::Remove(_)
                    ) {
                        let _ = tx.lock().unwrap().send(());
                    }
                }
            },
            notify::Config::default(),
        )?;

        // * `.obsidian` itself is watched, not only the two folders inside it:
        // * a vault that has no `themes/` yet gets no events from watching a
        // * directory that does not exist, so creating one later would never
        // * reload. Watching the parent covers the folder appearing; the two
        // * are still watched directly so a file edited inside one is caught
        // * even where the parent event is only a directory write.
        let obsidian = vault_path.join(".obsidian");
        let themes_dir = obsidian.join("themes");
        let snippets_dir = obsidian.join("snippets");
        if obsidian.is_dir() {
            watcher.watch(&obsidian, RecursiveMode::NonRecursive)?;
        }
        if themes_dir.is_dir() {
            watcher.watch(&themes_dir, RecursiveMode::NonRecursive)?;
        }
        if snippets_dir.is_dir() {
            watcher.watch(&snippets_dir, RecursiveMode::NonRecursive)?;
        }

        Ok((Self { _watcher: watcher }, rx))
    }
}
