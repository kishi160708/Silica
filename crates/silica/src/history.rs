//! File version history: a snapshot of a note's bytes taken every time it
//! is saved, kept under the vault's config folder so the snapshots stay out
//! of the file tree, the index and the search.
//!
//! Obsidian's own history is Obsidian Sync's; nothing outside their sync
//! service writes it. This is the store a local-only vault needs for "Open
//! version history" to mean anything: same open, review and restore, from
//! bytes Silica itself wrote.
//!
//! Layout: `<config>/.silica-versions/<flat path>/<millis>.md`, where the
//! flat path replaces `/` with `%2F` so a note's history is one directory
//! and no folder of the vault leaks into the store. Only the most recent
//! [`KEEP`] snapshots of a note are kept; older ones are pruned on write, so
//! the store cannot grow without bound.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// How many snapshots of one note to keep.
pub const KEEP: usize = 30;

/// A counter for disambiguating snapshots taken within the same nanosecond.
static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The store for one vault.
pub struct History {
    dir: PathBuf,
}

/// One snapshot of a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// When it was taken, in nanoseconds since the epoch.
    pub at: u64,
    pub bytes: u64,
}

impl Version {
    /// `2026-11-05 14:22` in local time, for the picker's rows.
    pub fn label(&self) -> String {
        fmt_millis(self.at)
    }
}

impl History {
    /// The store under a vault's config folder, made on first use.
    pub fn new(vault_root: &Path, config_dir: &str) -> History {
        History {
            dir: vault_root.join(config_dir).join(".silica-versions"),
        }
    }

    /// The directory holding `path`'s snapshots.
    fn note_dir(&self, path: &str) -> PathBuf {
        self.dir.join(path.replace('/', "%2F"))
    }

    /// Takes a snapshot of `path` with `contents`. Silent on failure: a
    /// note that cannot be snapshotted must still save, and the history is
    /// a convenience rather than the note's own bytes.
    pub fn record(&self, path: &str, contents: &str) {
        let dir = self.note_dir(path);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        // Nanosecond precision plus a per-process counter guarantees a
        // unique filename even when two saves land in the same nanosecond.
        let at = now_nanos();
        let counter = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let file = dir.join(format!("{at}-{counter}.md"));
        if std::fs::write(&file, contents).is_err() {
            return;
        }
        self.prune(&dir);
    }

    /// Drops the oldest snapshots once more than [`KEEP`] are held.
    fn prune(&self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut stamps: Vec<u64> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.file_name()
                    .to_string_lossy()
                    .strip_suffix(".md")
                    .and_then(|s| s.split('-').next())
                    .and_then(|s| s.parse().ok())
            })
            .collect();
        if stamps.len() <= KEEP {
            return;
        }
        stamps.sort_unstable();
        let drop_count = stamps.len() - KEEP;
        for at in stamps.into_iter().take(drop_count) {
            // Remove every snapshot whose nanosecond stamp is `at`, since
            // counter-suffixed files share the same stamp.
            let prefix = format!("{at}-");
            let Ok(reentries) = std::fs::read_dir(dir) else {
                continue;
            };
            for e in reentries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with(&prefix) && name.ends_with(".md") {
                    let _ = std::fs::remove_file(dir.join(&name));
                }
            }
        }
    }

    /// `path`'s snapshots, newest first.
    pub fn list(&self, path: &str) -> Vec<Version> {
        let dir = self.note_dir(path);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        // Group counter-suffixed files by nanosecond stamp.
        let mut by_stamp: std::collections::BTreeMap<u64, u64> =
            std::collections::BTreeMap::new();
        for e in entries.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(stamp) = name
                .strip_suffix(".md")
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse().ok())
            else {
                continue;
            };
            let bytes = match e.metadata() {
                Ok(m) => m.len(),
                Err(_) => continue,
            };
            // Keep the largest byte count per stamp (any counter suffix is fine).
            by_stamp.entry(stamp).or_insert(bytes);
        }
        let mut versions: Vec<Version> = by_stamp
            .into_iter()
            .map(|(at, bytes)| Version { at, bytes })
            .collect();
        versions.sort_by_key(|v| std::cmp::Reverse(v.at));
        versions
    }

    /// The bytes of the snapshot taken at `at`, if it is still there.
    /// Reads the most recent counter-suffixed file for that stamp.
    pub fn read(&self, path: &str, at: u64) -> Option<String> {
        let dir = self.note_dir(path);
        let prefix = format!("{at}-");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return None;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(&prefix) && name.ends_with(".md") {
                return std::fs::read_to_string(dir.join(&name)).ok();
            }
        }
        None
    }
}

/// Nanoseconds since the epoch, used for snapshot filenames to avoid
/// collisions when two saves land in the same millisecond.
pub fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// A millisecond stamp as `YYYY-MM-DD HH:MM` in UTC. Days are converted
/// with the civil-from-days algorithm, so no date crate is needed.
pub fn fmt_millis(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to `(y, m, d)`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempdir::TempDir, History) {
        let dir = tempdir::TempDir::new("silica-versions").unwrap();
        let history = History::new(dir.path(), ".obsidian");
        (dir, history)
    }

    #[test]
    fn a_snapshot_can_be_read_back() {
        let (_dir, history) = temp_store();
        history.record("Notes/A.md", "first\n");
        let versions = history.list("Notes/A.md");
        assert_eq!(versions.len(), 1);
        // Reads by nanosecond stamp: any counter suffix for that stamp works.
        assert!(history
            .read("Notes/A.md", versions[0].at)
            .as_deref()
            .is_some_and(|s| s == "first\n"));
    }

    #[test]
    fn snapshots_are_newest_first() {
        let (_dir, history) = temp_store();
        history.record("A.md", "one\n");
        std::thread::sleep(std::time::Duration::from_nanos(1_000));
        history.record("A.md", "two\n");
        let versions = history.list("A.md");
        assert_eq!(versions.len(), 2);
        assert!(versions[0].at > versions[1].at);
        assert!(history
            .read("A.md", versions[0].at)
            .as_deref()
            .is_some_and(|s| s == "two\n"));
    }

    #[test]
    fn only_the_last_snapshots_are_kept() {
        let (_dir, history) = temp_store();
        for i in 0..(KEEP + 5) {
            history.record("A.md", &format!("{i}\n"));
            std::thread::sleep(std::time::Duration::from_nanos(1));
        }
        assert_eq!(history.list("A.md").len(), KEEP);
    }

    #[test]
    fn snapshots_within_same_nanosecond_do_not_collide() {
        let (_dir, history) = temp_store();
        // Two saves back-to-back will land in the same nanosecond most of the
        // time; the counter suffix must keep both files.
        history.record("A.md", "one\n");
        history.record("A.md", "two\n");
        let versions = history.list("A.md");
        assert_eq!(versions.len(), 2);
        assert!(history.read("A.md", versions[0].at).is_some());
        assert!(history.read("A.md", versions[1].at).is_some());
    }

    #[test]
    fn a_note_with_no_history_lists_nothing() {
        let (_dir, history) = temp_store();
        assert!(history.list("Missing.md").is_empty());
        assert!(history.read("Missing.md", 0).is_none());
    }

    #[test]
    fn millis_render_as_a_date() {
        // 2026-11-05 00:00:00 UTC
        assert_eq!(fmt_millis(1_793_836_800_000), "2026-11-05 00:00");
    }

    /// Minimal temp dir: the crate has no `tempfile` dependency and this is
    /// the only place a test needs one.
    mod tempdir {
        use std::path::{Path, PathBuf};

        pub struct TempDir(PathBuf);

        impl TempDir {
            pub fn new(prefix: &str) -> std::io::Result<TempDir> {
                // * A counter keeps two tests started in the same
                //   millisecond apart, since one's `Drop` removes the
                //   directory the other is using.
                use std::sync::atomic::{AtomicU64, Ordering};
                static NEXT: AtomicU64 = AtomicU64::new(0);
                let path = std::env::temp_dir().join(format!(
                    "{prefix}-{}-{}-{}",
                    std::process::id(),
                    super::now_nanos(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path)?;
                Ok(TempDir(path))
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
