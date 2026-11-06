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
    /// Watch a vault's `.obsidian` subtree for CSS changes.
    ///
    /// `.obsidian` is watched recursively rather than `themes/` and
    /// `snippets/` being watched one by one, because those two may not exist
    /// when the window opens: a non-recursive registration made at startup
    /// never covers a directory created afterwards, so the first theme a user
    /// installed would sit there until a restart. A recursive watch follows
    /// directories as they appear and needs no re-registration.
    ///
    /// The vault root is watched too, non-recursively: `.obsidian` itself can
    /// be created later (a folder of notes that was never opened in Obsidian
    /// has none), and nothing below it is visible to a watcher that started
    /// before it existed. Root stays non-recursive so saving a note does not
    /// fire a CSS reload.
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
                        // * Only a `.css` file under `.obsidian` matters. The
                        // * recursive watch also reports `workspace.json`
                        // * churn and plugin folders, and rebuilding the theme
                        // * for those would be work with no visible effect.
                        let ours = event.paths.iter().any(|p| {
                            p.extension().is_some_and(|e| e.eq_ignore_ascii_case("css"))
                                && p.components().any(|c| c.as_os_str() == ".obsidian")
                                && !is_symlink(p)
                        });
                        if ours {
                            let _ = tx.lock().unwrap().send(());
                        }
                    }
                }
            },
            notify::Config::default(),
        )?;

        if vault_path.is_dir() {
            watcher.watch(vault_path, RecursiveMode::NonRecursive)?;
        }
        let obsidian = vault_path.join(".obsidian");
        if obsidian.is_dir() {
            watcher.watch(&obsidian, RecursiveMode::Recursive)?;
        }

        Ok((Self { _watcher: watcher }, rx))
    }
}

/// A watcher event that names a symlink points at something the vault did not
/// place there, so it is refused: a link planted under `.obsidian` must not
/// make Silica reload CSS from outside the vault.
fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A theme installed after the watcher started still reloads: the folder
    /// is created late and the recursive watch is what picks it up.
    #[test]
    fn a_theme_folder_created_after_startup_is_seen() {
        let vault = tempfile::tempdir().expect("a temp vault");
        std::fs::create_dir_all(vault.path().join(".obsidian")).expect("config dir");
        let (_watcher, rx) = CssWatcher::new(vault.path()).expect("watcher");

        // * Created only now, which is the case a per-directory registration
        // * at startup would have missed.
        let themes = vault.path().join(".obsidian/themes");
        std::fs::create_dir_all(&themes).expect("themes");
        // * inotify registers a newly created subdirectory of a recursive watch
        // * asynchronously. A file written in the same instant lands before the
        // * registration exists and is never reported, so give the watch the
        // * moment it needs — the assertion below is about the file event, not
        // * about how fast the kernel got there.
        std::thread::sleep(std::time::Duration::from_millis(300));

        std::fs::write(themes.join("late.css"), ":root { --x: #123456; }").expect("theme");
        let first = rx.recv_timeout(std::time::Duration::from_secs(5));
        assert!(
            first.is_ok(),
            "a theme written into a late folder must trigger a reload (got {first:?})"
        );
    }
}
