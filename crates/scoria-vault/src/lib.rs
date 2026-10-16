//! A vault is a plain folder. This crate opens one, enumerates its files, and
//! reads `.obsidian` configuration without ever owning data outside the folder.

use _cache::LinkResolver;
use _formats::config::{
    AppConfig, AppearanceConfig, CommunityPlugins, CorePlugins, Hotkeys, PropertyTypes,
    DEFAULT_CONFIG_DIR,
};
use _formats::FileKind;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("vault path is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// One file in the vault. `path` is vault-relative with `/` separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultFile {
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
}

/// What [`Vault::refresh_path`] found; paths are vault-relative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    Added(String),
    Modified(String),
    Removed(String),
}

impl FileChange {
    pub fn path(&self) -> &str {
        match self {
            FileChange::Added(p) | FileChange::Modified(p) | FileChange::Removed(p) => p,
        }
    }
}

#[derive(Debug)]
pub struct Vault {
    root: PathBuf,
    config_dir: String,
    files: Vec<VaultFile>,
}

impl Vault {
    /// Opens a vault with the default `.obsidian` config directory.
    pub fn open(root: impl AsRef<Path>) -> Result<Vault, VaultError> {
        Vault::open_with_config_dir(root, DEFAULT_CONFIG_DIR)
    }

    pub fn open_with_config_dir(
        root: impl AsRef<Path>,
        config_dir: &str,
    ) -> Result<Vault, VaultError> {
        let root = root.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(VaultError::NotADirectory(root));
        }
        let mut vault = Vault {
            root,
            config_dir: config_dir.to_string(),
            files: Vec::new(),
        };
        vault.rescan()?;
        Ok(vault)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn config_dir(&self) -> &str {
        &self.config_dir
    }

    pub fn files(&self) -> &[VaultFile] {
        &self.files
    }

    pub fn markdown_files(&self) -> impl Iterator<Item = &VaultFile> {
        self.files.iter().filter(|f| f.kind == FileKind::Markdown)
    }

    /// Absolute path for a vault-relative path.
    pub fn absolute(&self, rel: &str) -> PathBuf {
        self.root
            .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    /// Re-enumerates the folder. Skips the config directory, `.trash`, and any
    /// dot-prefixed directory; keeps dot-files out of the file list as Obsidian does.
    pub fn rescan(&mut self) -> Result<(), VaultError> {
        let mut files = Vec::new();
        let walker = walkdir::WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(e.depth() > 0 && name.starts_with('.'))
            });
        for entry in walker {
            let entry = entry.map_err(|e| VaultError::Io {
                path: e.path().map(Path::to_path_buf).unwrap_or_default(),
                source: e
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk")),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&self.root)
                .expect("entry under root")
                .to_string_lossy()
                .replace('\\', "/");
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(VaultFile {
                kind: FileKind::from_path(&rel),
                path: rel,
                size,
            });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        self.files = files;
        Ok(())
    }

    /// Re-checks one path after a filesystem event and updates the file
    /// list: a file that exists is added or refreshed, a folder that exists
    /// is walked, and anything missing is removed together with everything
    /// under it. Paths inside dot-folders (the config directory, `.trash`)
    /// are ignored.
    pub fn refresh_path(&mut self, path: &Path) -> Vec<FileChange> {
        let Ok(rel_path) = path.strip_prefix(&self.root) else {
            return Vec::new();
        };
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            return Vec::new();
        }
        if rel.split('/').any(|part| part.starts_with('.')) {
            return Vec::new();
        }
        let mut changes = Vec::new();
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_file() => {
                let file = VaultFile {
                    kind: FileKind::from_path(&rel),
                    path: rel.clone(),
                    size: meta.len(),
                };
                match self.files.binary_search_by(|f| f.path.cmp(&rel)) {
                    Ok(ix) => {
                        self.files[ix] = file;
                        changes.push(FileChange::Modified(rel));
                    }
                    Err(ix) => {
                        self.files.insert(ix, file);
                        changes.push(FileChange::Added(rel));
                    }
                }
            }
            Ok(meta) if meta.is_dir() => {
                let prefix = format!("{rel}/");
                let before: Vec<String> = self
                    .files
                    .iter()
                    .filter(|f| f.path.starts_with(&prefix))
                    .map(|f| f.path.clone())
                    .collect();
                let mut seen = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .follow_links(false)
                    .into_iter()
                    .filter_entry(|e| {
                        !(e.depth() > 0 && e.file_name().to_string_lossy().starts_with('.'))
                    })
                    .flatten()
                {
                    if entry.file_type().is_file() {
                        seen.push(entry.path().to_path_buf());
                    }
                }
                for file in seen {
                    changes.extend(self.refresh_path(&file));
                }
                for old in before {
                    if !self.root.join(&old).is_file() {
                        changes.extend(self.refresh_path(&self.root.join(&old)));
                    }
                }
            }
            _ => {
                let prefix = format!("{rel}/");
                let removed: Vec<String> = self
                    .files
                    .iter()
                    .filter(|f| f.path == rel || f.path.starts_with(&prefix))
                    .map(|f| f.path.clone())
                    .collect();
                self.files
                    .retain(|f| !(f.path == rel || f.path.starts_with(&prefix)));
                changes.extend(removed.into_iter().map(FileChange::Removed));
            }
        }
        changes
    }

    /// Builds a link resolver over every file currently known.
    pub fn link_resolver(&self) -> LinkResolver {
        LinkResolver::new(self.files.iter().map(|f| f.path.as_str()))
    }

    /// Reads `<config_dir>/<name>` as JSON. A missing file is `Ok(None)`;
    /// Obsidian treats every config file as optional.
    pub fn read_config<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, VaultError> {
        let path = self.root.join(&self.config_dir).join(name);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(VaultError::Io { path, source }),
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|source| VaultError::Json { path, source })
    }

    pub fn app_config(&self) -> Result<AppConfig, VaultError> {
        Ok(self.read_config("app.json")?.unwrap_or_default())
    }

    pub fn appearance_config(&self) -> Result<AppearanceConfig, VaultError> {
        Ok(self.read_config("appearance.json")?.unwrap_or_default())
    }

    pub fn hotkeys(&self) -> Result<Hotkeys, VaultError> {
        Ok(self.read_config("hotkeys.json")?.unwrap_or_default())
    }

    pub fn core_plugins(&self) -> Result<Option<CorePlugins>, VaultError> {
        self.read_config("core-plugins.json")
    }

    pub fn community_plugins(&self) -> Result<CommunityPlugins, VaultError> {
        Ok(self
            .read_config("community-plugins.json")?
            .unwrap_or_default())
    }

    pub fn property_types(&self) -> Result<PropertyTypes, VaultError> {
        Ok(self.read_config("types.json")?.unwrap_or_default())
    }

    /// The raw workspace layout; typed modelling comes with the workspace milestone.
    pub fn workspace_json(&self) -> Result<Option<serde_json::Value>, VaultError> {
        self.read_config("workspace.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_vault() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join(".obsidian/plugins/x")).unwrap();
        fs::create_dir_all(r.join(".trash")).unwrap();
        fs::create_dir_all(r.join("Projects")).unwrap();
        fs::write(r.join("Note.md"), "# Note\n[[Plan]]").unwrap();
        fs::write(r.join("Projects/Plan.md"), "plan").unwrap();
        fs::write(r.join("Board.canvas"), "{}").unwrap();
        fs::write(r.join("pic.PNG"), "").unwrap();
        fs::write(r.join(".hidden.md"), "").unwrap();
        fs::write(r.join(".trash/old.md"), "").unwrap();
        fs::write(r.join(".obsidian/app.json"), r#"{"vimMode":true}"#).unwrap();
        fs::write(
            r.join(".obsidian/core-plugins.json"),
            r#"{"file-explorer":true,"graph":true,"canvas":false}"#,
        )
        .unwrap();
        fs::write(
            r.join(".obsidian/community-plugins.json"),
            r#"["dataview"]"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn scans_files_and_skips_dot_entries() {
        let dir = sample_vault();
        let vault = Vault::open(dir.path()).unwrap();
        let paths: Vec<&str> = vault.files().iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Board.canvas", "Note.md", "Projects/Plan.md", "pic.PNG"]
        );
        assert_eq!(vault.markdown_files().count(), 2);
        assert_eq!(vault.files()[3].kind, FileKind::Image);
    }

    #[test]
    fn reads_config_and_resolves_links() {
        let dir = sample_vault();
        let vault = Vault::open(dir.path()).unwrap();
        assert_eq!(vault.app_config().unwrap().vim_mode, Some(true));
        assert!(vault.core_plugins().unwrap().unwrap().is_enabled("graph"));
        assert_eq!(vault.community_plugins().unwrap(), vec!["dataview"]);
        assert!(vault.hotkeys().unwrap().is_empty());
        assert!(vault.workspace_json().unwrap().is_none());
        let r = vault.link_resolver();
        assert_eq!(r.resolve("plan", "Note.md"), Some("Projects/Plan.md"));
    }

    #[test]
    fn refresh_path_tracks_files_folders_and_removals() {
        let dir = sample_vault();
        let mut vault = Vault::open(dir.path()).unwrap();
        let r = dir.path();
        fs::write(r.join("Projects/New.md"), "x").unwrap();
        assert_eq!(
            vault.refresh_path(&r.join("Projects/New.md")),
            vec![FileChange::Added("Projects/New.md".into())]
        );
        assert_eq!(
            vault.refresh_path(&r.join("Projects/New.md")),
            vec![FileChange::Modified("Projects/New.md".into())]
        );
        fs::create_dir_all(r.join("Moved/in")).unwrap();
        fs::write(r.join("Moved/in/a.md"), "a").unwrap();
        fs::write(r.join("Moved/.hidden.md"), "a").unwrap();
        assert_eq!(
            vault.refresh_path(&r.join("Moved")),
            vec![FileChange::Added("Moved/in/a.md".into())]
        );
        fs::remove_dir_all(r.join("Projects")).unwrap();
        let mut removed = vault.refresh_path(&r.join("Projects"));
        removed.sort_by(|a, b| a.path().cmp(b.path()));
        assert_eq!(
            removed,
            vec![
                FileChange::Removed("Projects/New.md".into()),
                FileChange::Removed("Projects/Plan.md".into())
            ]
        );
        assert!(vault.refresh_path(&r.join(".obsidian/app.json")).is_empty());
        assert!(vault.refresh_path(&r.join(".trash/x.md")).is_empty());
        assert!(vault.refresh_path(Path::new("/elsewhere/x.md")).is_empty());
        let paths: Vec<&str> = vault.files().iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Board.canvas", "Moved/in/a.md", "Note.md", "pic.PNG"]
        );
    }

    #[test]
    fn rejects_non_directories() {
        let dir = sample_vault();
        assert!(matches!(
            Vault::open(dir.path().join("Note.md")),
            Err(VaultError::NotADirectory(_))
        ));
    }
}
