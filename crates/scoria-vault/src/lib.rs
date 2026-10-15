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
    fn rejects_non_directories() {
        let dir = sample_vault();
        assert!(matches!(
            Vault::open(dir.path().join("Note.md")),
            Err(VaultError::NotADirectory(_))
        ));
    }
}
