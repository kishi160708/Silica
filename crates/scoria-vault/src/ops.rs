//! File operations on the vault folder: new notes and folders, rename and
//! move, and deletion to the trash. Each operation changes the disk first and
//! then the file list, so the result is what a filesystem event would have
//! produced; link rewriting lives in `-cache` and is driven by the app.
//!
//! Behaviors observed in Obsidian 1.14.4: a new note is `Untitled.md`, then
//! `Untitled 1.md`, `Untitled 2.md`; a new folder `Untitled`, `Untitled 1`
//! (files and folders count separately); creating over an existing path
//! fails with "File already exists."; renaming onto an existing path fails
//! with "Destination file already exists!" while a case-only rename of the
//! same file succeeds; a note created from a link goes to `newFileLocation`
//! (`root`, `current`, or the `newFileFolderPath` folder, falling back to
//! the root when that folder does not exist), with subfolders created for a
//! `folder/Name` link; a file trashed locally lands flat in `.trash/`, a
//! folder with its contents, and a name already there gets ` 2`, ` 3`, ...

use crate::{FileChange, Vault};
use _formats::validate_file_name;
use std::path::{Path, PathBuf};

/// `trashOption` in `app.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrashOption {
    /// The operating system's trash (Obsidian's default).
    #[default]
    System,
    /// The vault's own `.trash` folder.
    Local,
    /// Delete permanently.
    None,
}

impl TrashOption {
    pub fn from_config(value: Option<&str>) -> TrashOption {
        match value {
            Some("local") => TrashOption::Local,
            Some("none") => TrashOption::None,
            _ => TrashOption::System,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("File already exists.")]
    Exists,
    #[error("Destination file already exists!")]
    DestinationExists,
    #[error("{0}")]
    InvalidName(&'static str),
    #[error("{0} does not exist")]
    NotFound(String),
    #[error("{action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

fn io<'p>(action: &'static str, path: &'p Path) -> impl FnOnce(std::io::Error) -> OpError + 'p {
    move |source| OpError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// The folder part of a vault path (`""` at the root).
pub fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// Checks every segment of a new path the way Obsidian's rename does.
fn validate_path(path: &str) -> Result<(), OpError> {
    if path.is_empty() {
        return Err(OpError::InvalidName("File name cannot be empty."));
    }
    for segment in path.split('/') {
        validate_file_name(segment).map_err(OpError::InvalidName)?;
    }
    Ok(())
}

impl Vault {
    /// Whether anything exists at this vault path on disk.
    pub fn exists(&self, rel: &str) -> bool {
        !rel.is_empty() && self.absolute(rel).exists()
    }

    /// The first free name among `stem`, `stem 1`, `stem 2`, ... in `folder`,
    /// with `.ext` appended when `ext` is not empty (Obsidian's
    /// `getAvailablePath`).
    pub fn available_path(&self, folder: &str, stem: &str, ext: &str) -> String {
        self.numbered_path(folder, stem, ext, 1)
    }

    fn numbered_path(&self, folder: &str, stem: &str, ext: &str, first: usize) -> String {
        let with_ext = |name: &str| {
            if ext.is_empty() {
                join(folder, name)
            } else {
                join(folder, &format!("{name}.{ext}"))
            }
        };
        let plain = with_ext(stem);
        if !self.exists(&plain) {
            return plain;
        }
        let mut n = first;
        loop {
            let candidate = with_ext(&format!("{stem} {n}"));
            if !self.exists(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// The folder a new note goes into: `newFileLocation` is `root`,
    /// `current` (the open note's folder) or `folder` (`newFileFolderPath`,
    /// falling back to the root when that folder is missing).
    pub fn new_note_folder(
        &self,
        location: Option<&str>,
        folder_path: Option<&str>,
        current: Option<&str>,
    ) -> String {
        match location {
            Some("current") => current.map(folder_of).unwrap_or("").to_string(),
            Some("folder") => {
                let folder = folder_path
                    .unwrap_or("")
                    .trim_matches('/')
                    .replace('\\', "/");
                if !folder.is_empty() && self.absolute(&folder).is_dir() {
                    folder
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }

    /// The path a note created for the link text `name` (without `.md`) gets:
    /// a name with a folder in it is taken from the vault root, anything
    /// else goes into the new-note folder.
    pub fn new_note_path(&self, name: &str, new_note_folder: &str) -> String {
        let name = name.trim().trim_matches('/');
        let name = name.strip_suffix(".md").unwrap_or(name);
        if name.contains('/') {
            format!("{name}.md")
        } else {
            join(new_note_folder, &format!("{name}.md"))
        }
    }

    /// Creates a markdown note (and its folders) with `text`. Fails when the
    /// path exists.
    pub fn create_note(&mut self, path: &str, text: &str) -> Result<Vec<FileChange>, OpError> {
        validate_path(path)?;
        if self.exists(path) {
            return Err(OpError::Exists);
        }
        let abs = self.absolute(path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(io("create folder", parent))?;
        }
        std::fs::write(&abs, text).map_err(io("create", &abs))?;
        Ok(self.refresh_path(&abs))
    }

    /// Creates an empty folder (and its parents).
    pub fn create_folder(&mut self, path: &str) -> Result<(), OpError> {
        validate_path(path)?;
        if self.exists(path) {
            return Err(OpError::Exists);
        }
        let abs = self.absolute(path);
        std::fs::create_dir_all(&abs).map_err(io("create folder", &abs))?;
        self.refresh_path(&abs);
        Ok(())
    }

    /// Files under `path` when it is a folder, or `path` itself when it is a
    /// file in the list.
    fn files_under(&self, path: &str) -> Vec<String> {
        let prefix = format!("{path}/");
        self.files()
            .iter()
            .filter(|f| f.path == path || f.path.starts_with(&prefix))
            .map(|f| f.path.clone())
            .collect()
    }

    /// The files `rename(from, to)` would move, as `(old path, new path)`,
    /// without touching anything.
    pub fn rename_plan(&self, from: &str, to: &str) -> Vec<(String, String)> {
        let from = from.trim_matches('/');
        let to = to.trim_matches('/');
        self.files_under(from)
            .into_iter()
            .map(|old| {
                let new = if old == from {
                    to.to_string()
                } else {
                    format!("{to}{}", &old[from.len()..])
                };
                (old, new)
            })
            .collect()
    }

    /// Renames or moves a file or folder. Returns every file that moved as
    /// `(old path, new path)`, so links can be rewritten.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<Vec<(String, String)>, OpError> {
        let from = from.trim_matches('/');
        let to = to.trim_matches('/');
        validate_path(to)?;
        let from_abs = self.absolute(from);
        if !from_abs.exists() {
            return Err(OpError::NotFound(from.to_string()));
        }
        if from == to {
            return Ok(Vec::new());
        }
        let to_abs = self.absolute(to);
        // A case-only rename of the same entry is allowed; anything else
        // already at the destination is not.
        let same_entry = from.eq_ignore_ascii_case(to)
            && std::fs::canonicalize(&from_abs).ok() == std::fs::canonicalize(&to_abs).ok();
        if to_abs.exists() && !same_entry {
            return Err(OpError::DestinationExists);
        }
        if to.starts_with(&format!("{from}/")) {
            return Err(OpError::InvalidName("Cannot move a folder into itself."));
        }
        if let Some(parent) = to_abs.parent() {
            std::fs::create_dir_all(parent).map_err(io("create folder", parent))?;
        }
        let moved = self.rename_plan(from, to);
        std::fs::rename(&from_abs, &to_abs).map_err(io("rename", &from_abs))?;
        self.refresh_path(&from_abs);
        self.refresh_path(&to_abs);
        Ok(moved)
    }

    /// Deletes a file or folder per `option`. Returns the vault paths of the
    /// files that are gone.
    pub fn trash(&mut self, path: &str, option: TrashOption) -> Result<Vec<String>, OpError> {
        let path = path.trim_matches('/');
        let abs = self.absolute(path);
        if !abs.exists() {
            return Err(OpError::NotFound(path.to_string()));
        }
        let removed = self.files_under(path);
        match option {
            TrashOption::None => {
                if abs.is_dir() {
                    std::fs::remove_dir_all(&abs).map_err(io("delete", &abs))?;
                } else {
                    std::fs::remove_file(&abs).map_err(io("delete", &abs))?;
                }
            }
            TrashOption::Local => self.trash_locally(path)?,
            TrashOption::System => {
                if let Err(err) = system_trash(&abs) {
                    // No system trash here (another platform, or a read-only
                    // data directory): the vault's own trash is the fallback.
                    if err.kind() == std::io::ErrorKind::Unsupported {
                        self.trash_locally(path)?;
                    } else {
                        return Err(OpError::Io {
                            action: "move to the system trash",
                            path: abs,
                            source: err,
                        });
                    }
                }
            }
        }
        self.refresh_path(&abs);
        Ok(removed)
    }

    fn trash_locally(&self, path: &str) -> Result<(), OpError> {
        let abs = self.absolute(path);
        let trash = self.root().join(".trash");
        std::fs::create_dir_all(&trash).map_err(io("create folder", &trash))?;
        let name = path.rsplit('/').next().unwrap_or(path);
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() && abs.is_file() => (s, e),
            _ => (name, ""),
        };
        let target = self.numbered_path(".trash", stem, ext, 2);
        let target_abs = self.absolute(&target);
        std::fs::rename(&abs, &target_abs).map_err(io("move to .trash", &abs))
    }
}

/// Moves `abs` to the freedesktop.org trash (`$XDG_DATA_HOME/Trash`), writing
/// the `.trashinfo` record the file manager needs to restore it.
#[cfg(all(unix, not(target_os = "macos")))]
fn system_trash(abs: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, "no home directory"))?;
    let trash = data_home.join("Trash");
    let files = trash.join("files");
    let info = trash.join("info");
    std::fs::create_dir_all(&files)?;
    std::fs::create_dir_all(&info)?;
    let name = abs
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "no file name"))?
        .to_string_lossy()
        .into_owned();
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() && abs.is_file() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    let mut candidate = name.clone();
    let mut n = 2;
    while files.join(&candidate).exists() || info.join(format!("{candidate}.trashinfo")).exists() {
        candidate = format!("{stem} {n}{ext}");
        n += 1;
    }
    let record = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        url_encode_path(&abs.to_string_lossy()),
        utc_timestamp()
    );
    std::fs::write(info.join(format!("{candidate}.trashinfo")), record)?;
    let dest = files.join(&candidate);
    match std::fs::rename(abs, &dest) {
        Ok(()) => Ok(()),
        Err(err) if err.raw_os_error() == Some(libc_exdev()) => {
            copy_tree(abs, &dest)?;
            if abs.is_dir() {
                std::fs::remove_dir_all(abs)
            } else {
                std::fs::remove_file(abs)
            }
        }
        Err(err) => {
            let _ = std::fs::remove_file(info.join(format!("{candidate}.trashinfo")));
            Err(err)
        }
    }
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn system_trash(_abs: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "system trash is not implemented on this platform",
    ))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn libc_exdev() -> i32 {
    18
}

#[cfg(all(unix, not(target_os = "macos")))]
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

/// Percent-encodes a path for a `.trashinfo` record, keeping `/` and the
/// unreserved characters.
#[cfg(all(unix, not(target_os = "macos")))]
fn url_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `YYYY-MM-DDThh:mm:ss` for now, in UTC.
#[cfg(all(unix, not(target_os = "macos")))]
fn utc_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn vault() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("Sub/Deep")).unwrap();
        fs::create_dir_all(r.join("Inbox")).unwrap();
        fs::write(r.join("A.md"), "a").unwrap();
        fs::write(r.join("Untitled.md"), "").unwrap();
        fs::write(r.join("Sub/S.md"), "s").unwrap();
        fs::write(r.join("Sub/Deep/D.md"), "d").unwrap();
        fs::write(r.join("Sub/pic.png"), "p").unwrap();
        let vault = Vault::open(r).unwrap();
        (dir, vault)
    }

    fn paths(v: &Vault) -> Vec<&str> {
        v.files().iter().map(|f| f.path.as_str()).collect()
    }

    #[test]
    fn new_notes_and_folders_take_the_next_free_number() {
        let (_dir, mut v) = vault();
        assert_eq!(v.available_path("", "Untitled", "md"), "Untitled 1.md");
        v.create_note("Untitled 1.md", "").unwrap();
        assert_eq!(v.available_path("", "Untitled", "md"), "Untitled 2.md");
        assert_eq!(v.available_path("", "Untitled", ""), "Untitled");
        v.create_folder("Untitled").unwrap();
        assert_eq!(v.available_path("", "Untitled", ""), "Untitled 1");
        assert_eq!(v.available_path("Sub", "S", "md"), "Sub/S 1.md");
        assert!(matches!(v.create_note("A.md", "x"), Err(OpError::Exists)));
        assert!(matches!(v.create_folder("Sub"), Err(OpError::Exists)));
        assert!(matches!(
            v.create_note("bad:name.md", ""),
            Err(OpError::InvalidName(_))
        ));
        let changes = v.create_note("New/Nested/N.md", "n").unwrap();
        assert_eq!(changes, vec![FileChange::Added("New/Nested/N.md".into())]);
        assert!(paths(&v).contains(&"New/Nested/N.md"));
        assert!(v.folders().contains(&"Untitled".to_string()));
        assert!(v.folders().contains(&"New/Nested".to_string()));
    }

    #[test]
    fn new_note_location_follows_the_setting() {
        let (_dir, v) = vault();
        assert_eq!(v.new_note_folder(None, None, Some("Sub/S.md")), "");
        assert_eq!(
            v.new_note_folder(Some("current"), None, Some("Sub/S.md")),
            "Sub"
        );
        assert_eq!(v.new_note_folder(Some("current"), None, None), "");
        assert_eq!(
            v.new_note_folder(Some("folder"), Some("Inbox"), None),
            "Inbox"
        );
        assert_eq!(v.new_note_folder(Some("folder"), Some("/"), None), "");
        assert_eq!(
            v.new_note_folder(Some("folder"), Some("Missing/Folder"), None),
            "",
            "a missing folder falls back to the root"
        );
        assert_eq!(v.new_note_path("Missing", "Inbox"), "Inbox/Missing.md");
        assert_eq!(
            v.new_note_path("Deep/Also Missing", "Inbox"),
            "Deep/Also Missing.md"
        );
        assert_eq!(v.new_note_path("Note.md", ""), "Note.md");
    }

    #[test]
    fn rename_moves_files_and_folders_and_reports_every_file() {
        let (dir, mut v) = vault();
        assert_eq!(
            v.rename("A.md", "Renamed A.md").unwrap(),
            vec![("A.md".to_string(), "Renamed A.md".to_string())]
        );
        assert!(dir.path().join("Renamed A.md").is_file());
        assert!(!paths(&v).contains(&"A.md"));
        assert!(paths(&v).contains(&"Renamed A.md"));
        let moved = v.rename("Sub", "Other/Sub2").unwrap();
        assert_eq!(
            moved,
            vec![
                (
                    "Sub/Deep/D.md".to_string(),
                    "Other/Sub2/Deep/D.md".to_string()
                ),
                ("Sub/S.md".to_string(), "Other/Sub2/S.md".to_string()),
                ("Sub/pic.png".to_string(), "Other/Sub2/pic.png".to_string()),
            ]
        );
        assert_eq!(
            paths(&v),
            vec![
                "Other/Sub2/Deep/D.md",
                "Other/Sub2/S.md",
                "Other/Sub2/pic.png",
                "Renamed A.md",
                "Untitled.md"
            ]
        );
        assert_eq!(
            v.folders(),
            ["Inbox", "Other", "Other/Sub2", "Other/Sub2/Deep"]
        );
        assert!(matches!(
            v.rename("Untitled.md", "Renamed A.md"),
            Err(OpError::DestinationExists)
        ));
        assert!(matches!(
            v.rename("Other", "Other/Sub2/X"),
            Err(OpError::InvalidName(_))
        ));
        assert!(matches!(
            v.rename("Nope.md", "X.md"),
            Err(OpError::NotFound(_))
        ));
        // A case-only rename keeps the same file.
        v.rename("Renamed A.md", "renamed a.md").unwrap();
        assert!(paths(&v).contains(&"renamed a.md"));
        assert!(v.rename("renamed a.md", "renamed a.md").unwrap().is_empty());
    }

    #[test]
    fn trash_goes_to_the_local_folder_with_numbered_collisions() {
        let (dir, mut v) = vault();
        fs::create_dir_all(dir.path().join(".trash")).unwrap();
        fs::write(dir.path().join(".trash/A.md"), "old").unwrap();
        assert_eq!(v.trash("A.md", TrashOption::Local).unwrap(), vec!["A.md"]);
        assert!(dir.path().join(".trash/A 2.md").is_file());
        v.create_note("A.md", "again").unwrap();
        v.trash("A.md", TrashOption::Local).unwrap();
        assert!(dir.path().join(".trash/A 3.md").is_file());
        let removed = v.trash("Sub", TrashOption::Local).unwrap();
        assert_eq!(removed, vec!["Sub/Deep/D.md", "Sub/S.md", "Sub/pic.png"]);
        assert!(dir.path().join(".trash/Sub/Deep/D.md").is_file());
        assert_eq!(paths(&v), vec!["Untitled.md"]);
        assert_eq!(v.folders(), ["Inbox"]);
        v.trash("Untitled.md", TrashOption::None).unwrap();
        assert!(paths(&v).is_empty());
        assert!(matches!(
            v.trash("Untitled.md", TrashOption::None),
            Err(OpError::NotFound(_))
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn freedesktop_records_are_well_formed() {
        assert_eq!(
            url_encode_path("/home/me/My Vault/Ä.md"),
            "/home/me/My%20Vault/%C3%84.md"
        );
        let stamp = utc_timestamp();
        assert_eq!(stamp.len(), 19);
        assert_eq!(&stamp[10..11], "T");
        assert!(stamp.starts_with("20"));
    }
}
