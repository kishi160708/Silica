//! The workspace shell: left dock (Files, Search), main area (reading view
//! or source editor), right dock (Backlinks, Outgoing links, Outline, Tags),
//! view header, status bar, and the quick switcher, command palette and
//! Settings overlays. Commands run by id ([`Workspace::run_command`]);
//! hotkeys come from the command registry, so `hotkeys.json` applies.
//!
//! The vault is indexed on a background thread when the window opens and
//! then kept current by a filesystem watcher: changed paths are re-read and
//! re-indexed in place, link edges re-resolved, and every pane refreshed.

use crate::chooser::{Chooser, ChooserEvent, Row as ChooserRow};
use crate::editor::{Editor, EditorEvent};
use crate::explorer::{display_name, kind_name, ExplorerEvent, FileExplorer, FileTree};
use crate::graph::{GraphEvent, GraphView};
use crate::mentions::{Finder, Name};
use crate::palette::{CommandPalette, PaletteCommand, PaletteEvent};
use crate::prompt::{Prompt, PromptEvent, PromptSpec};
use crate::reading::{self, LinkTarget, ReadingEvent, ReadingView, TaskMarker, TaskToggled};
use crate::search::{SearchEvent, SearchPane};
use crate::settings::{self, SettingChange, SettingsEvent, SettingsView};
use crate::switcher::{QuickSwitcher, SwitcherEvent};
use crate::theme::Theme;
use crate::watch;
use gpui::{
    actions, div, prelude::*, px, rgba, uniform_list, Action, App, Context, Entity, FocusHandle,
    Focusable, Global, KeyBinding, SharedString, Subscription, Task, UniformListScrollHandle,
    Window,
};
use rayon::prelude::*;
use _cache::{
    basename, frontmatter_aliases, index_markdown, Backlink, CachedMetadata, LinkFormat,
    MetadataCache,
};
use _commands::{keystroke, CommandRegistry};
use _formats::config::{
    self as config, AppConfig, AppearanceConfig, CommandPaletteConfig, CorePlugins,
    DailyNotesConfig, PluginManifest, PropertyTypes, SwitcherConfig, TemplatesConfig,
};
use _formats::properties::PropertiesInDocument;
use _formats::FileKind;
use _markdown::Subpath;
use _templates as templates;
use _text::table::TableCommand;
use _text::{LinePrefix, LinkSyntax};
use _vault::{FileChange, TrashOption, Vault};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant};

actions!(
    workspace,
    [ToggleMode, GoBack, GoForward, OpenSearch, OpenGraph]
);

/// Runs the command with this Obsidian id (bound from the registry's hotkeys).
#[derive(Debug, Clone, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct RunCommand {
    pub id: String,
}

/// The command registry with the vault's `hotkeys.json` applied.
pub struct Commands(pub Arc<CommandRegistry>);

impl Global for Commands {}

/// The local civil time, which is what Obsidian's dates are in.
fn now() -> templates::Stamp {
    let local = chrono::Local::now();
    let naive = local.naive_local();
    templates::Stamp {
        year: chrono::Datelike::year(&naive) as i64,
        month: chrono::Datelike::month(&naive),
        day: chrono::Datelike::day(&naive),
        hour: chrono::Timelike::hour(&naive),
        minute: chrono::Timelike::minute(&naive),
        second: chrono::Timelike::second(&naive),
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.is_empty())
}

/// Command ids the workspace can run today; the palette lists only these,
/// the way Obsidian hides commands whose check callback fails.
pub const RUNNABLE: &[&str] = &[
    "command-palette:open",
    "editor:save-file",
    "editor:toggle-source",
    "editor:open-search",
    "editor:open-search-replace",
    "editor:add-cursor-above",
    "editor:add-cursor-below",
    "editor:toggle-bold",
    "editor:toggle-italics",
    "editor:toggle-strikethrough",
    "editor:toggle-highlight",
    "editor:toggle-code",
    "editor:toggle-inline-math",
    "editor:toggle-comments",
    "editor:toggle-blockquote",
    "editor:toggle-bullet-list",
    "editor:toggle-numbered-list",
    "editor:toggle-checklist-status",
    "editor:set-heading-0",
    "editor:set-heading-1",
    "editor:set-heading-2",
    "editor:set-heading-3",
    "editor:set-heading-4",
    "editor:set-heading-5",
    "editor:set-heading-6",
    "editor:indent-list",
    "editor:unindent-list",
    "editor:delete-paragraph",
    "editor:swap-line-up",
    "editor:swap-line-down",
    "editor:insert-link",
    "editor:insert-wikilink",
    "editor:insert-embed",
    "editor:follow-link",
    "editor:open-link-in-new-leaf",
    "switcher:open",
    "global-search:open",
    "graph:open",
    "graph:open-local",
    "markdown:toggle-preview",
    "markdown:add-metadata-property",
    "markdown:clear-metadata-properties",
    "editor:toggle-fold-properties",
    "editor:insert-table",
    "insert-template",
    "insert-current-date",
    "insert-current-time",
    "daily-notes",
    "daily-notes:goto-prev",
    "daily-notes:goto-next",
    "editor:table-row-before",
    "editor:table-row-after",
    "editor:table-row-up",
    "editor:table-row-down",
    "editor:table-row-copy",
    "editor:table-row-delete",
    "editor:table-col-before",
    "editor:table-col-after",
    "editor:table-col-left",
    "editor:table-col-right",
    "editor:table-col-copy",
    "editor:table-col-delete",
    "editor:table-col-align-left",
    "editor:table-col-align-center",
    "editor:table-col-align-right",
    "app:go-back",
    "app:go-forward",
    "app:open-settings",
    "app:toggle-left-sidebar",
    "app:toggle-right-sidebar",
    "file-explorer:open",
    "file-explorer:reveal-active-file",
    "file-explorer:new-file",
    "file-explorer:new-file-in-current-tab",
    "file-explorer:new-file-in-new-pane",
    "file-explorer:new-folder",
    "file-explorer:move-file",
    "file-explorer:duplicate-file",
    "workspace:edit-file-title",
    "workspace:new-tab",
    "workspace:close",
    "workspace:close-others",
    "workspace:next-tab",
    "workspace:previous-tab",
    "workspace:goto-tab-1",
    "workspace:goto-tab-2",
    "workspace:goto-tab-3",
    "workspace:goto-tab-4",
    "workspace:goto-tab-5",
    "workspace:goto-tab-6",
    "workspace:goto-tab-7",
    "workspace:goto-tab-8",
    "workspace:goto-last-tab",
    "workspace:undo-close-pane",
    "app:delete-file",
    "backlink:open",
    "backlink:open-backlinks",
    "outgoing-links:open",
    "outgoing-links:open-for-current",
    "outline:open",
    "outline:open-for-current",
    "tag-pane:open",
];

/// Binds every runnable command's hotkeys in the Workspace context; call
/// once at startup with the registry that has `hotkeys.json` applied.
pub fn bind_keys(cx: &mut App, registry: &CommandRegistry) {
    let ctx = Some("Workspace");
    let bindings: Vec<KeyBinding> = RUNNABLE
        .iter()
        .filter_map(|id| registry.get(id))
        .flat_map(|c| {
            c.hotkeys.iter().filter_map(|h| {
                let keys = keystroke(h)?;
                Some(KeyBinding::new(&keys, RunCommand { id: c.id.clone() }, ctx))
            })
        })
        .collect();
    cx.bind_keys(bindings);
}

/// Everything derived from the vault folder: built once off the UI thread,
/// then updated in place by [`VaultState::apply`] as files change.
pub struct VaultState {
    pub root: PathBuf,
    vault: RwLock<Vault>,
    cache: RwLock<MetadataCache>,
    /// Note text as indexed, for search and mentions.
    texts: RwLock<HashMap<String, Arc<String>>>,
    /// The config files Settings writes, behind locks for the same reason
    /// `property_types` is: a control changes one while the vault is open.
    app: RwLock<AppConfig>,
    appearance: RwLock<AppearanceConfig>,
    core_plugins: RwLock<Option<CorePlugins>>,
    community_plugins: RwLock<Vec<String>>,
    switcher: RwLock<SwitcherConfig>,
    /// The Templates plugin's settings, for the two date commands and the
    /// template chooser.
    templates: RwLock<TemplatesConfig>,
    /// The Daily notes plugin's settings.
    daily_notes: RwLock<DailyNotesConfig>,
    pub plugin_manifests: Vec<PluginManifest>,
    pub palette: CommandPaletteConfig,
    /// `types.json`: the type assigned to each property name.
    property_types: RwLock<PropertyTypes>,
    pub index_ms: f64,
}

/// One re-read note: its path and, for markdown, the new record and text.
type Reindexed = (String, Option<(CachedMetadata, Arc<String>)>);

/// What a batch of filesystem events changed.
#[derive(Debug, Default)]
pub struct Changes {
    pub files: Vec<FileChange>,
    /// Notes whose text was re-indexed.
    pub notes: Vec<String>,
    /// Notes that no longer exist.
    pub removed_notes: Vec<String>,
}

impl Changes {
    fn structural(&self) -> bool {
        self.files
            .iter()
            .any(|c| !matches!(c, FileChange::Modified(_)))
    }
}

/// What a rename did: the files that moved and the notes whose links
/// were rewritten, on top of the index changes.
#[derive(Debug, Default)]
pub struct RenameOutcome {
    pub changes: Changes,
    pub renamed: Vec<(String, String)>,
    pub rewritten_files: usize,
    pub rewritten_links: usize,
    /// Notes whose links were left alone because the file on disk was not
    /// the text the rewrite was planned from.
    pub stale: Vec<String>,
}

impl VaultState {
    pub fn load(root: &PathBuf) -> Result<VaultState, String> {
        let started = Instant::now();
        let vault = Vault::open(root).map_err(|e| e.to_string())?;
        let app = vault.app_config().unwrap_or_default();
        let appearance = vault.appearance_config().unwrap_or_default();
        let core_plugins = vault.core_plugins().unwrap_or_default();
        let community_plugins = vault.community_plugins().unwrap_or_default();
        let plugin_manifests = vault.plugin_manifests();
        let palette = vault.command_palette_config().unwrap_or_default();
        let switcher = vault.switcher_config().unwrap_or_default();
        let templates = vault.templates_config().unwrap_or_default();
        let daily_notes = vault.daily_notes_config().unwrap_or_default();
        let property_types = vault.property_types().unwrap_or_default();
        let notes: Vec<String> = vault.markdown_files().map(|f| f.path.clone()).collect();
        let loaded: Vec<(String, CachedMetadata, Arc<String>)> = notes
            .par_iter()
            .map(|path| {
                let text = std::fs::read_to_string(vault.absolute(path)).unwrap_or_default();
                let meta = index_markdown(&text);
                (path.clone(), meta, Arc::new(text))
            })
            .collect();
        let mut texts = HashMap::with_capacity(loaded.len());
        let metas: Vec<(String, CachedMetadata)> = loaded
            .into_iter()
            .map(|(path, meta, text)| {
                texts.insert(path.clone(), text);
                (path, meta)
            })
            .collect();
        let cache = MetadataCache::new(vault.link_resolver(), metas);
        Ok(VaultState {
            root: vault.root().to_path_buf(),
            vault: RwLock::new(vault),
            cache: RwLock::new(cache),
            texts: RwLock::new(texts),
            app: RwLock::new(app),
            appearance: RwLock::new(appearance),
            core_plugins: RwLock::new(core_plugins),
            community_plugins: RwLock::new(community_plugins),
            switcher: RwLock::new(switcher),
            templates: RwLock::new(templates),
            daily_notes: RwLock::new(daily_notes),
            plugin_manifests,
            palette,
            property_types: RwLock::new(property_types),
            index_ms: started.elapsed().as_secs_f64() * 1000.0,
        })
    }

    /// The vault's property type assignments.
    pub fn property_types(&self) -> RwLockReadGuard<'_, PropertyTypes> {
        self.property_types.read().expect("property types lock")
    }

    pub fn app(&self) -> RwLockReadGuard<'_, AppConfig> {
        self.app.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn appearance(&self) -> RwLockReadGuard<'_, AppearanceConfig> {
        self.appearance.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn core_plugins(&self) -> RwLockReadGuard<'_, Option<CorePlugins>> {
        self.core_plugins.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn community_plugins(&self) -> RwLockReadGuard<'_, Vec<String>> {
        self.community_plugins
            .read()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn templates(&self) -> RwLockReadGuard<'_, TemplatesConfig> {
        self.templates.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn daily_notes(&self) -> RwLockReadGuard<'_, DailyNotesConfig> {
        self.daily_notes.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn switcher(&self) -> RwLockReadGuard<'_, SwitcherConfig> {
        self.switcher.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Writes a setting the Settings modal changed and keeps the config in
    /// hand in step. The file is written first: the vault is the source of
    /// truth, so a change that cannot reach disk does not count.
    pub fn apply_setting(&self, change: &SettingChange) -> Result<(), String> {
        match change {
            SettingChange::App(config) => {
                self.write_config("app.json", config)?;
                *self.app.write().unwrap_or_else(|e| e.into_inner()) = config.clone();
            }
            SettingChange::Appearance(config) => {
                self.write_config("appearance.json", config)?;
                *self.appearance.write().unwrap_or_else(|e| e.into_inner()) = config.clone();
            }
            SettingChange::Switcher(config) => {
                self.write_config("switcher.json", config)?;
                *self.switcher.write().unwrap_or_else(|e| e.into_inner()) = config.clone();
            }
            SettingChange::CorePlugins(config) => {
                self.write_config("core-plugins.json", config)?;
                *self.core_plugins.write().unwrap_or_else(|e| e.into_inner()) =
                    Some(config.clone());
            }
            SettingChange::CommunityPlugins(config) => {
                self.write_config("community-plugins.json", config)?;
                *self
                    .community_plugins
                    .write()
                    .unwrap_or_else(|e| e.into_inner()) = config.clone();
            }
            SettingChange::Hotkeys(overrides) => {
                // Whole, not merged: an entry taken out is a command back
                // on its defaults, and a merge would keep it.
                self.vault()
                    .replace_config("hotkeys.json", overrides)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn write_config<T: serde::Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        self.vault()
            .write_config(name, value)
            .map_err(|e| e.to_string())
    }

    /// Records `key`'s type in `types.json`, as Obsidian's type menu does.
    /// Returns the table as it now stands so the views can redraw from it.
    pub fn assign_property_type(
        &self,
        key: &str,
        ty: _formats::properties::PropertyType,
    ) -> Result<PropertyTypes, String> {
        let mut types = self.property_types.write().expect("property types lock");
        types.types.insert(key.to_string(), ty.as_str().to_string());
        self.vault()
            .write_config("types.json", &*types)
            .map_err(|e| e.to_string())?;
        Ok(types.clone())
    }

    pub fn vault(&self) -> RwLockReadGuard<'_, Vault> {
        self.vault.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn cache(&self) -> RwLockReadGuard<'_, MetadataCache> {
        self.cache.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn texts(&self) -> RwLockReadGuard<'_, HashMap<String, Arc<String>>> {
        self.texts.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn build_options(&self) -> reading::BuildOptions {
        reading::BuildOptions {
            strict_line_breaks: self.app().strict_line_breaks.unwrap_or(false),
        }
    }

    /// Applies filesystem events for `paths`: refreshes the file list,
    /// re-indexes changed notes, and re-resolves links when files came or went.
    pub fn apply(&self, paths: &[PathBuf]) -> Changes {
        let mut files = Vec::new();
        {
            let mut vault = self.vault.write().unwrap_or_else(|e| e.into_inner());
            for path in paths {
                files.extend(vault.refresh_path(path));
            }
        }
        self.absorb(files)
    }

    /// Re-reads and re-indexes the notes among `files` (already applied to
    /// the file list) and re-resolves links when files came or went.
    fn absorb(&self, files: Vec<FileChange>) -> Changes {
        let mut changes = Changes::default();
        // Keep one change per path, the last one seen.
        let mut last: HashMap<String, FileChange> = HashMap::new();
        for c in files {
            last.insert(c.path().to_string(), c);
        }
        let mut files: Vec<FileChange> = last.into_values().collect();
        files.sort_by(|a, b| a.path().cmp(b.path()));
        if files.is_empty() {
            return changes;
        }
        let reread: Vec<Reindexed> = files
            .par_iter()
            .map(|c| match c {
                FileChange::Added(p) | FileChange::Modified(p)
                    if FileKind::from_path(p) == FileKind::Markdown =>
                {
                    let text = std::fs::read_to_string(self.root.join(p)).unwrap_or_default();
                    let meta = index_markdown(&text);
                    (p.clone(), Some((meta, Arc::new(text))))
                }
                other => (other.path().to_string(), None),
            })
            .collect();
        {
            let mut cache = self.cache.write().unwrap_or_else(|e| e.into_inner());
            let mut texts = self.texts.write().unwrap_or_else(|e| e.into_inner());
            for (c, (path, indexed)) in files.iter().zip(reread) {
                match (c, indexed) {
                    (FileChange::Removed(_), _) => {
                        if texts.remove(&path).is_some() {
                            changes.removed_notes.push(path.clone());
                        }
                        cache.forget_path(&path);
                    }
                    (_, Some((meta, text))) => {
                        cache.insert_file(path.clone(), meta);
                        texts.insert(path.clone(), text);
                        changes.notes.push(path);
                    }
                    (_, None) => cache.add_path(&path),
                }
            }
            if files.iter().any(|c| !matches!(c, FileChange::Modified(_))) {
                cache.reresolve();
            }
        }
        changes.files = files;
        changes
    }

    pub fn link_format(&self) -> LinkFormat {
        LinkFormat::from_config(self.app().new_link_format.as_deref())
    }

    pub fn trash_option(&self) -> TrashOption {
        TrashOption::from_config(self.app().trash_option.as_deref())
    }

    /// `alwaysUpdateLinks`: on by default; off means ask before rewriting.
    pub fn update_links_automatically(&self) -> bool {
        self.app().always_update_links.unwrap_or(true)
    }

    /// `promptDelete`: on by default.
    pub fn confirm_delete(&self) -> bool {
        self.app().prompt_delete.unwrap_or(true)
    }

    /// Where a note created for the link text `name` goes, given the open
    /// note (`newFileLocation`, `newFileFolderPath`).
    pub fn new_note_path(&self, name: &str, current: Option<&str>) -> String {
        let vault = self.vault();
        let folder = vault.new_note_folder(
            self.app().new_file_location.as_deref(),
            self.app().new_file_folder_path.as_deref(),
            current,
        );
        vault.new_note_path(name, &folder)
    }

    /// Creates a note (and its folders) and indexes it.
    pub fn create_note(&self, path: &str, text: &str) -> Result<Changes, String> {
        let files = self
            .vault
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .create_note(path, text)
            .map_err(|e| e.to_string())?;
        Ok(self.absorb(files))
    }

    pub fn create_folder(&self, path: &str) -> Result<(), String> {
        self.vault
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .create_folder(path)
            .map_err(|e| e.to_string())
    }

    /// Copies a file next to itself as `Name 1.ext` and indexes the copy.
    pub fn duplicate(&self, path: &str) -> Result<(String, Changes), String> {
        let (target, from, to) = {
            let vault = self.vault();
            let (folder, name) = match path.rsplit_once('/') {
                Some((d, n)) => (d, n),
                None => ("", path),
            };
            let (stem, ext) = match name.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s, e),
                _ => (name, ""),
            };
            let target = vault.available_path(folder, stem, ext);
            (
                target.clone(),
                vault.absolute(path),
                vault.absolute(&target),
            )
        };
        std::fs::copy(&from, &to).map_err(|e| format!("Could not copy {path}: {e}"))?;
        let files = self
            .vault
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .refresh_path(&to);
        Ok((target, self.absorb(files)))
    }

    /// The rewrites a rename would need, without doing anything:
    /// `(files, links)`.
    pub fn rename_preview(&self, from: &str, to: &str) -> (usize, usize) {
        let plan = self.vault().rename_plan(from, to);
        let texts = self.texts();
        let rewrites = self
            .cache()
            .plan_rename_rewrites(&plan, self.link_format(), &|p| {
                texts.get(p).map(|t| t.to_string())
            });
        (
            rewrites.len(),
            rewrites.iter().map(|r| r.links).sum::<usize>(),
        )
    }

    /// Renames or moves a file or folder, rewrites the links to it in every
    /// note (when `update_links`), and brings the index up to date.
    /// Writes a block id into `path`, which is a note the user is not
    /// editing. The bytes on disk have to be the bytes the insertion was
    /// planned against: the vault is the source of truth, and a note that
    /// moved underneath is left alone rather than overwritten.
    ///
    /// The file is written and the watcher re-indexes it, which is the same
    /// path a save takes.
    pub fn write_block_id(
        &self,
        path: &str,
        before: &str,
        insertion: &_suggest::blocks::IdInsertion,
    ) -> Result<(), String> {
        let absolute = self.vault().absolute(path);
        let on_disk = std::fs::read_to_string(&absolute)
            .map_err(|e| format!("Could not read {path}: {e}"))?;
        if on_disk != before {
            return Err(format!("{path} changed on disk; no id was written"));
        }
        std::fs::write(&absolute, insertion.applied(&on_disk).as_bytes())
            .map_err(|e| format!("Could not write {path}: {e}"))
    }

    pub fn rename(
        &self,
        from: &str,
        to: &str,
        update_links: bool,
    ) -> Result<RenameOutcome, String> {
        let moved = self
            .vault
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .rename(from, to)
            .map_err(|e| e.to_string())?;
        let plan = if update_links {
            let texts = self.texts();
            self.cache()
                .plan_rename_rewrites(&moved, self.link_format(), &|p| {
                    texts.get(p).map(|t| t.to_string())
                })
        } else {
            Vec::new()
        };
        // Each rewrite was planned from the text the index holds, so the
        // file has to still be that text: another app may have saved it
        // since, and writing the plan would throw that away. A note that
        // moved underneath keeps its own bytes and its links are left for
        // the user, which loses nothing either way.
        let mut stale: Vec<String> = Vec::new();
        let texts = self.texts();
        for rw in &plan {
            let absolute = self.root.join(&rw.new_path);
            let planned_from = texts.get(&rw.old_path).map(|t| t.to_string());
            match (std::fs::read_to_string(&absolute), planned_from) {
                (Ok(on_disk), Some(was)) if on_disk != was => {
                    stale.push(rw.new_path.clone());
                    continue;
                }
                _ => {}
            }
            std::fs::write(&absolute, rw.text.as_bytes())
                .map_err(|e| format!("Could not update links in {}: {e}", rw.new_path))?;
        }
        drop(texts);
        let moved_from: HashMap<&str, &str> = moved
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let mut changes = Changes::default();
        let mut cache = self.cache.write().unwrap_or_else(|e| e.into_inner());
        let mut texts = self.texts.write().unwrap_or_else(|e| e.into_inner());
        for (old, new) in &moved {
            let old_text = texts.remove(old);
            cache.forget_path(old);
            changes.files.push(FileChange::Removed(old.clone()));
            if old_text.is_some() {
                changes.removed_notes.push(old.clone());
            }
            if FileKind::from_path(new) == FileKind::Markdown {
                let text = plan
                    .iter()
                    .find(|r| r.old_path == *old)
                    .map(|r| Arc::new(r.text.clone()))
                    .or(old_text)
                    .unwrap_or_else(|| {
                        Arc::new(std::fs::read_to_string(self.root.join(new)).unwrap_or_default())
                    });
                cache.insert_file(new.clone(), index_markdown(&text));
                texts.insert(new.clone(), text);
                changes.notes.push(new.clone());
            } else {
                cache.add_path(new);
            }
            changes.files.push(FileChange::Added(new.clone()));
        }
        for rw in plan
            .iter()
            .filter(|r| !moved_from.contains_key(r.old_path.as_str()))
        {
            let text = Arc::new(rw.text.clone());
            cache.insert_file(rw.new_path.clone(), index_markdown(&text));
            texts.insert(rw.new_path.clone(), text);
            changes
                .files
                .push(FileChange::Modified(rw.new_path.clone()));
            changes.notes.push(rw.new_path.clone());
        }
        cache.reresolve();
        Ok(RenameOutcome {
            changes,
            renamed: moved,
            rewritten_files: plan.len() - stale.len(),
            rewritten_links: plan
                .iter()
                .filter(|r| !stale.contains(&r.new_path))
                .map(|r| r.links)
                .sum(),
            stale,
        })
    }

    /// Deletes a file or folder per `trashOption` and drops it from the index.
    pub fn trash(&self, path: &str) -> Result<Changes, String> {
        let removed = self
            .vault
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .trash(path, self.trash_option())
            .map_err(|e| e.to_string())?;
        let mut changes = Changes::default();
        let mut cache = self.cache.write().unwrap_or_else(|e| e.into_inner());
        let mut texts = self.texts.write().unwrap_or_else(|e| e.into_inner());
        for p in removed {
            if texts.remove(&p).is_some() {
                changes.removed_notes.push(p.clone());
            }
            cache.forget_path(&p);
            changes.files.push(FileChange::Removed(p));
        }
        cache.reresolve();
        Ok(changes)
    }

    /// Note names and aliases, for unlinked mentions.
    fn names(&self) -> Vec<(String, String)> {
        let cache = self.cache();
        let mut out = Vec::new();
        for (path, meta) in cache.files() {
            let base = path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".md");
            out.push((base.to_string(), path.to_string()));
            for alias in frontmatter_aliases(meta) {
                out.push((alias, path.to_string()));
            }
        }
        out
    }
}

impl reading::EmbedSource for VaultState {
    fn resolve(&self, linktext: &str, source_path: &str) -> Option<String> {
        self.cache()
            .resolve(linktext, source_path)
            .map(str::to_string)
    }

    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.vault().absolute(path)).ok()
    }

    fn absolute(&self, path: &str) -> PathBuf {
        self.vault().absolute(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Reading,
    /// Editing with markers hidden and constructs rendered in place.
    LivePreview,
    /// Editing the raw text.
    Source,
    Graph,
}

impl ViewMode {
    /// Both editing modes show the editor.
    pub fn is_editing(self) -> bool {
        matches!(self, ViewMode::LivePreview | ViewMode::Source)
    }

    /// The name a mode goes by outside the app: the control socket's
    /// `set_mode` and the `mode` in its state dump.
    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Reading => "reading",
            ViewMode::LivePreview => "live",
            ViewMode::Source => "source",
            ViewMode::Graph => "graph",
        }
    }

    /// The mode one of those names stands for; `None` for anything else,
    /// so a bad request is answered rather than guessed at.
    pub fn parse(name: &str) -> Option<ViewMode> {
        match name {
            "reading" => Some(ViewMode::Reading),
            "live" => Some(ViewMode::LivePreview),
            "source" => Some(ViewMode::Source),
            "graph" => Some(ViewMode::Graph),
            _ => None,
        }
    }
}

/// What the main area paints. The mode asks for a view; the open file
/// decides whether that view has anything to show, and `Empty` is the
/// placeholder for when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainView {
    Reading,
    Editor,
    Graph,
    Empty,
}

impl MainView {
    /// The name the socket's `focus` and `state` use for this view.
    fn as_str(self) -> &'static str {
        match self {
            MainView::Reading => "reading",
            MainView::Editor => "editor",
            MainView::Graph => "graph",
            MainView::Empty => "nothing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Files,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Backlinks,
    Outgoing,
    Outline,
    Tags,
}

#[derive(Clone)]
struct OutlineRow {
    level: u8,
    text: SharedString,
    offset: usize,
}

/// A row of the Backlinks or Outgoing links pane.
#[derive(Clone)]
enum PaneRow {
    Header(SharedString),
    Item {
        /// File to open; `None` for an unresolved link.
        path: Option<String>,
        label: SharedString,
        detail: SharedString,
        /// Byte offset to scroll to after opening.
        offset: Option<usize>,
    },
}

/// The Outline pane's rows for one note's metadata, whether that metadata
/// came from the index or from the buffer being typed into.
fn outline_rows(meta: &CachedMetadata) -> Vec<OutlineRow> {
    meta.headings
        .iter()
        .map(|h| OutlineRow {
            level: h.level,
            text: Workspace::heading_text(&h.heading).into(),
            offset: h.position.start.offset,
        })
        .collect()
}

/// The context a Linked mentions row shows: the property a link was written
/// in, which is what Obsidian names for one, or else its line in the source.
fn backlink_detail(b: &Backlink<'_>, source_text: Option<&str>) -> String {
    match b.key {
        Some(key) => key.to_string(),
        None => {
            let line = b.link.position.start.line;
            let context = source_text
                .and_then(|t| t.lines().nth(line as usize))
                .map(str::trim)
                .unwrap_or_default();
            format!("{}: {}", line + 1, context)
        }
    }
}

/// Rows for the Outgoing links pane: every link, embed and property link of
/// `path`. A link written in a property is a link like any other, so it is
/// listed too, first and in file order, with its property as the context.
fn outgoing_link_rows(cache: &MetadataCache, path: &str) -> Vec<PaneRow> {
    let Some(meta) = cache.file(path) else {
        return Vec::new();
    };
    let properties = meta
        .frontmatter_links
        .iter()
        .map(|l| (l.link.as_str(), Some(l.key.as_str())));
    let body = meta
        .links
        .iter()
        .chain(meta.embeds.iter())
        .map(|l| (l.link.as_str(), None));
    properties
        .chain(body)
        .map(|(link, key)| match cache.resolve(link, path) {
            Some(target) => PaneRow::Item {
                path: Some(target.to_string()),
                label: display_name(target.rsplit('/').next().unwrap_or(target))
                    .to_string()
                    .into(),
                detail: key.unwrap_or(target).to_string().into(),
                offset: None,
            },
            None => PaneRow::Item {
                path: None,
                label: _markdown::parse_linktext(link)
                    .path
                    .to_string()
                    .into(),
                detail: "Not created yet".into(),
                offset: None,
            },
        })
        .collect()
}

#[derive(Clone)]
struct TagRow {
    name: SharedString,
    count: usize,
}

/// Mentions computed off the UI thread for one open note.
struct MentionRows {
    path: String,
    incoming: Vec<PaneRow>,
    outgoing: Vec<PaneRow>,
}

/// How long a transient message stays on screen. Obsidian reports what a
/// command did with a Notice that fades on its own after a few seconds;
/// the exact duration there has not been probed.
const NOTICE_LIFETIME: Duration = Duration::from_secs(5);

/// A message on screen, with the id its expiry timer names it by.
struct Notice {
    id: u64,
    text: SharedString,
}

/// The transient messages, oldest first. The status bar is for what stays
/// true — the open file, the index summary — so everything a command has
/// to say goes here instead and leaves again on its own.
#[derive(Default)]
struct Notices {
    live: Vec<Notice>,
    last_id: u64,
}

impl Notices {
    /// Shows `text` and returns the id to expire it by. A message already
    /// on screen is renewed rather than stacked: the autosave retries on a
    /// timer, so the same refusal arrives over and over.
    fn push(&mut self, text: SharedString) -> u64 {
        self.live.retain(|n| n.text != text);
        self.last_id += 1;
        self.live.push(Notice {
            id: self.last_id,
            text,
        });
        self.last_id
    }

    /// Drops the message with this id, if it is still the one on screen.
    fn expire(&mut self, id: u64) {
        self.live.retain(|n| n.id != id);
    }

    fn clear(&mut self) {
        self.live.clear();
    }

    /// The newest message, which is what the last command said.
    fn latest(&self) -> Option<&SharedString> {
        self.live.last().map(|n| &n.text)
    }

    fn texts(&self) -> Vec<String> {
        self.live.iter().map(|n| n.text.to_string()).collect()
    }
}

/// Which overlay is up, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    None,
    Switcher,
    Palette,
    Settings,
    Prompt,
    /// The template chooser.
    Chooser,
}

/// The file operation the open prompt will finish.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingOp {
    Rename(String),
    Move(String),
    Delete(String),
    NewFolder {
        parent: String,
    },
    /// `alwaysUpdateLinks` is off: asking whether to rewrite links.
    UpdateLinks {
        from: String,
        to: String,
    },
}

/// One tab of the main area: what it shows and where it has been. The
/// active tab's fields live on the workspace itself — `current`, `mode`,
/// `history`, `history_ix` — so every path that opens a note keeps working
/// unchanged; `tabs[active_tab]` is a snapshot of them, refreshed by
/// [`Workspace::stash_active_tab`] before anything reads it.
#[derive(Debug, Clone, PartialEq)]
struct Tab {
    path: Option<String>,
    mode: ViewMode,
    history: Vec<String>,
    history_ix: usize,
    /// The `workspace.json` leaf this tab was restored from, so a layout
    /// nothing changed is written back with the keys Obsidian gave it.
    leaf: Option<Value>,
}

impl Tab {
    fn empty(mode: ViewMode) -> Tab {
        Tab {
            path: None,
            mode,
            history: Vec::new(),
            history_ix: 0,
            leaf: None,
        }
    }

    /// The tab's title as the strip shows it: the note's name, or Obsidian's
    /// "New tab" for one with nothing in it.
    fn title(&self) -> SharedString {
        match (&self.path, self.mode) {
            (Some(p), _) => display_name(p.rsplit('/').next().unwrap_or(p))
                .to_string()
                .into(),
            (None, ViewMode::Graph) => "Graph view".into(),
            (None, _) => "New tab".into(),
        }
    }
}

/// How many closed tabs `workspace:undo-close-pane` can bring back.
const CLOSED_TABS_KEPT: usize = 20;

/// How many paths `workspace.json`'s `lastOpenFiles` keeps.
const LAST_OPEN_FILES_KEPT: usize = 10;

pub struct Workspace {
    focus_handle: FocusHandle,
    /// Set when something changed what the main area shows and the keyboard
    /// has to follow it; [`Workspace::render`] is where it is honoured.
    refocus_main: bool,
    state: Option<Arc<VaultState>>,
    vault_name: SharedString,
    loading: Option<SharedString>,
    explorer: Entity<FileExplorer>,
    search: Entity<SearchPane>,
    switcher: Entity<QuickSwitcher>,
    chooser: Entity<Chooser>,
    palette: Entity<CommandPalette>,
    settings: Entity<SettingsView>,
    prompt: Entity<Prompt>,
    pending: Option<PendingOp>,
    modal: Modal,
    /// Command ids run this session, most recent first.
    recent_commands: Vec<String>,
    left_visible: bool,
    right_visible: bool,
    left_tab: LeftTab,
    reading: Entity<ReadingView>,
    editor: Entity<Editor>,
    graph: Entity<GraphView>,
    /// The mode to return to when the graph closes.
    mode_before_graph: ViewMode,
    mode: ViewMode,
    current: Option<String>,
    /// The note whose file went away while its buffer was still on screen.
    /// The text stays — it may hold edits that exist nowhere else — but it
    /// is no longer backed by a file, so the main area says so and nothing
    /// writes it back.
    detached: Option<SharedString>,
    history: Vec<String>,
    history_ix: usize,
    /// The main area's tabs; the active one's fields are the live ones above.
    tabs: Vec<Tab>,
    active_tab: usize,
    /// Tabs closed this session with the index they had, oldest first.
    closed_tabs: Vec<(usize, Tab)>,
    /// `workspace.json` as it was read at load, `None` when the vault has
    /// none; what the quit write is merged into.
    layout: Option<Value>,
    /// A `workspace.json` was read, so the startup flags leave the first
    /// note unopened the way Obsidian does.
    layout_restored: bool,
    /// `lastOpenFiles`: the notes opened, most recent first.
    last_open_files: Vec<String>,
    /// What the window's title bar says, set only when it changes.
    window_title: SharedString,
    /// The next link followed opens in a new tab: a Ctrl+click, or
    /// `editor:open-link-in-new-leaf`. Taken by [`Workspace::follow_link`].
    link_in_new_tab: bool,
    right_tab: RightTab,
    outline: Vec<OutlineRow>,
    backlink_rows: Vec<PaneRow>,
    outgoing_rows: Vec<PaneRow>,
    /// Linked mentions, kept apart so mentions can be appended when they arrive.
    linked: Vec<PaneRow>,
    outgoing_links: Vec<PaneRow>,
    tags: Vec<TagRow>,
    tag_scroll: UniformListScrollHandle,
    /// Matcher over every note name and alias, rebuilt when files change.
    name_finder: Option<Arc<Finder>>,
    mention_generation: u64,
    /// What the status bar says about the vault and the open file: it is
    /// true until something makes it untrue, so nothing expires it.
    status: SharedString,
    notices: Notices,
    words: usize,
    chars: usize,
    _watcher: Option<watch::Watcher>,
    /// Pending autosave, two seconds after the last edit.
    save_task: Option<Task<()>>,
    /// Whether the startup flags have been applied. A client driving the
    /// control socket needs to know that the window has stopped arranging
    /// itself, not just that the vault finished indexing.
    started: bool,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workspace {
    pub fn new(root: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Workspace {
        let explorer = cx.new(FileExplorer::new);
        let search = cx.new(SearchPane::new);
        let switcher = cx.new(QuickSwitcher::new);
        let chooser = cx.new(Chooser::new);
        let palette = cx.new(CommandPalette::new);
        let settings = cx.new(SettingsView::new);
        let prompt = cx.new(Prompt::new);
        let reading = cx.new(ReadingView::new);
        let editor = cx.new(|cx| Editor::new(window, cx));
        let graph = cx.new(GraphView::new);
        let subscriptions = vec![
            cx.subscribe_in(
                &graph,
                window,
                |this, _, event: &GraphEvent, window, cx| match event {
                    GraphEvent::Open(path) => {
                        let back = this.mode_before_graph;
                        this.set_mode(back, window, cx);
                        this.open_file(path.clone(), None, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &explorer,
                window,
                |this, _, event: &ExplorerEvent, window, cx| match event {
                    ExplorerEvent::Open(path) => this.open_file(path.clone(), None, cx),
                    ExplorerEvent::OpenInNewTab(path) => {
                        this.open_in_new_tab(path.clone(), None, cx)
                    }
                    ExplorerEvent::NewNote { folder } => this.new_note(folder, window, cx),
                    ExplorerEvent::NewFolder { parent } => {
                        this.start_new_folder(parent, window, cx)
                    }
                    ExplorerEvent::Rename(path) => this.start_rename(path, window, cx),
                    ExplorerEvent::Move(path) => this.start_move(path, window, cx),
                    ExplorerEvent::Duplicate(path) => this.duplicate(path, cx),
                    ExplorerEvent::Delete(path) => this.start_delete(path, window, cx),
                },
            ),
            cx.subscribe_in(
                &prompt,
                window,
                |this, _, event: &PromptEvent, window, cx| match event {
                    PromptEvent::Submit(value) => this.prompt_submitted(value, window, cx),
                    PromptEvent::Secondary => this.prompt_secondary(window, cx),
                    PromptEvent::Cancel => {
                        this.pending = None;
                        this.close_modal(window, cx);
                    }
                },
            ),
            cx.subscribe(&search, |this, _, event: &SearchEvent, cx| match event {
                SearchEvent::Open(path, offset) => {
                    this.open_file(path.clone(), None, cx);
                    if let Some(offset) = *offset {
                        this.reveal_offset(offset, cx);
                    }
                }
            }),
            cx.subscribe_in(
                &switcher,
                window,
                |this, _, event: &SwitcherEvent, window, cx| match event {
                    SwitcherEvent::Open(path) => {
                        this.close_modal(window, cx);
                        this.open_file(path.clone(), None, cx);
                    }
                    SwitcherEvent::Create(name) => {
                        this.close_modal(window, cx);
                        this.create_from_link(name, window, cx);
                    }
                    SwitcherEvent::Close => this.close_modal(window, cx),
                },
            ),
            cx.subscribe_in(
                &palette,
                window,
                |this, _, event: &PaletteEvent, window, cx| match event {
                    PaletteEvent::Run(id) => {
                        this.close_modal(window, cx);
                        let id = id.clone();
                        this.run_command(&id, window, cx);
                    }
                    PaletteEvent::Close => this.close_modal(window, cx),
                },
            ),
            cx.subscribe_in(
                &settings,
                window,
                |this, _, event: &SettingsEvent, window, cx| match event {
                    SettingsEvent::Close => this.close_modal(window, cx),
                    SettingsEvent::Changed(change) => {
                        this.change_setting(change.as_ref().clone(), cx)
                    }
                },
            ),
            cx.subscribe(&reading, |this, _, event: &ReadingEvent, cx| match event {
                ReadingEvent::OpenLink(target) => this.follow_link(target.clone(), cx),
                ReadingEvent::OpenLinkInNewTab(target) => {
                    this.link_in_new_tab = true;
                    this.follow_link(target.clone(), cx);
                }
            }),
            cx.subscribe(&reading, |this, _, event: &TaskToggled, cx| {
                this.toggle_task(&event.path, event.marker, cx)
            }),
            cx.subscribe_in(
                &chooser,
                window,
                |this, _, event: &ChooserEvent, window, cx| match event {
                    ChooserEvent::Chosen(path) => this.insert_template(path.clone(), window, cx),
                    ChooserEvent::Close => {
                        this.modal = Modal::None;
                        // Whatever the main area paints, not the editor:
                        // its handle is outside the dispatch tree in
                        // reading view and would swallow every key.
                        this.focus_main(window, cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| match event {
                EditorEvent::Edited => {
                    this.schedule_save(cx);
                    this.refresh_outline_from_buffer(cx);
                }
                EditorEvent::OpenLink(target) => this.follow_link(target.clone(), cx),
                EditorEvent::PropertyTypeAssigned { key, ty } => {
                    this.assign_property_type(key.clone(), *ty, cx)
                }
                EditorEvent::Reported(message) => this.report(message.clone(), cx),
            }),
        ];
        // Closing the window must not lose edits waiting for the autosave.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            if let Some(ws) = this.upgrade() {
                ws.update(cx, |ws, cx| ws.on_quit(cx));
            }
            true
        });
        let mut ws = Workspace {
            focus_handle: cx.focus_handle(),
            refocus_main: false,
            state: None,
            vault_name: "No vault".into(),
            loading: None,
            explorer,
            search,
            switcher,
            chooser,
            palette,
            settings,
            prompt,
            pending: None,
            modal: Modal::None,
            recent_commands: Vec::new(),
            left_visible: true,
            right_visible: true,
            left_tab: LeftTab::Files,
            reading,
            editor,
            graph,
            mode_before_graph: ViewMode::Reading,
            mode: ViewMode::Reading,
            current: None,
            detached: None,
            history: Vec::new(),
            history_ix: 0,
            tabs: vec![Tab::empty(ViewMode::Reading)],
            active_tab: 0,
            closed_tabs: Vec::new(),
            layout: None,
            layout_restored: false,
            last_open_files: Vec::new(),
            window_title: SharedString::default(),
            link_in_new_tab: false,
            right_tab: RightTab::Backlinks,
            outline: Vec::new(),
            backlink_rows: Vec::new(),
            outgoing_rows: Vec::new(),
            linked: Vec::new(),
            outgoing_links: Vec::new(),
            tags: Vec::new(),
            tag_scroll: UniformListScrollHandle::new(),
            name_finder: None,
            mention_generation: 0,
            status: "Pass a vault folder as the first argument".into(),
            notices: Notices::default(),
            words: 0,
            chars: 0,
            _watcher: None,
            save_task: None,
            started: false,
            _subscriptions: subscriptions,
        };
        if let Some(root) = root {
            ws.load_vault(root, cx);
        }
        ws
    }

    fn load_vault(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.vault_name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .into();
        self.loading = Some(format!("Indexing {}…", root.display()).into());
        self.status = "Indexing vault".into();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { VaultState::load(&root) })
                .await;
            this.update(cx, |ws, cx| match result {
                Ok(state) => ws.set_state(state, cx),
                Err(err) => {
                    ws.loading = None;
                    ws.status = format!("Vault failed to open: {err}").into();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Records a property type in the vault's `types.json` and redraws the
    /// views from the new table.
    fn assign_property_type(
        &mut self,
        key: String,
        ty: _formats::properties::PropertyType,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.state.clone() else {
            return;
        };
        match state.assign_property_type(&key, ty) {
            Ok(types) => {
                self.reading
                    .update(cx, |r, _| r.set_property_types(types.clone()));
                self.editor.update(cx, |e, _| e.set_property_types(types));
                cx.notify();
            }
            Err(err) => {
                self.report(format!("types.json could not be written: {err}"), cx);
            }
        }
    }

    /// Pushes what `app.json` says into the open views: the parts of the
    /// editor and reading view that a setting changes while the vault is
    /// open. Called when the vault loads and again whenever Settings
    /// writes.
    fn apply_app_config(&mut self, state: &VaultState, cx: &mut Context<Self>) {
        let readable = state.app().readable_line_length.unwrap_or(true);
        let properties =
            PropertiesInDocument::from_config(state.app().properties_in_document.as_deref());
        self.reading.update(cx, |r, cx| {
            r.set_readable_line_length(readable);
            r.set_property_types(state.property_types().clone());
            r.set_properties_in_document(properties);
            cx.notify();
        });
        let (smart, use_tab, tab_size) = (
            state.app().smart_indent_list.unwrap_or(true),
            state.app().use_tab.unwrap_or(true),
            state.app().tab_size.unwrap_or(4),
        );
        let build_options = state.build_options();
        let link_options = _suggest::Options {
            format: state.link_format(),
            markdown_links: state.app().use_markdown_links.unwrap_or(false),
        };
        self.editor.update(cx, |e, cx| {
            e.set_link_options(link_options);
            e.set_readable_line_length(readable);
            e.set_indent_options(smart, use_tab, tab_size);
            e.set_property_types(state.property_types().clone());
            e.set_properties_in_document(properties);
            e.set_build_options(build_options, cx);
        });
    }

    /// Writes a setting the Settings modal changed, then applies it. The
    /// modal redraws from the vault's config either way, so a write that
    /// fails leaves the control as it was and says so in the status bar.
    fn change_setting(&mut self, change: SettingChange, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        if let Err(err) = state.apply_setting(&change) {
            self.report(format!("{} could not be written: {err}", change.file()), cx);
            return;
        }
        if let SettingChange::Hotkeys(overrides) = &change {
            // The registry is what the palette, the Hotkeys tab and the key
            // bindings read, so it is rebuilt from the file as written and
            // every binding made from it again.
            let mut registry = CommandRegistry::with_known_ids();
            registry.apply_overrides(overrides);
            let registry = Arc::new(registry);
            crate::bind_all_keys(cx, &registry);
            cx.set_global(Commands(registry));
        }
        self.apply_app_config(&state, cx);
        self.refresh_vault_views(cx);
        cx.notify();
    }

    fn set_state(&mut self, state: VaultState, cx: &mut Context<Self>) {
        self.loading = None;
        self.status = format!(
            "{} files, {} notes indexed in {:.0} ms",
            state.vault().files().len(),
            state.cache().len(),
            state.index_ms
        )
        .into();
        let state = Arc::new(state);
        self.state = Some(state.clone());
        let embed_source: Arc<dyn reading::EmbedSource + Send + Sync> = state.clone();
        let build_options = state.build_options();
        self.editor.update(cx, |e, cx| {
            e.set_vault(Some(embed_source), build_options, cx);
            e.set_suggest_vault(state.clone());
        });
        self.apply_app_config(&state, cx);
        self.refresh_vault_views(cx);
        self.start_watching(&state, cx);
        if let Some(current) = self.current.clone() {
            self.open_file(current, None, cx);
        } else {
            self.restore_layout(&state, cx);
        }
        cx.notify();
    }

    /// Opens today's daily note, creating it if it is not there, or steps
    /// to the previous or next one that exists.
    ///
    /// The configured folder has to exist already — Obsidian refuses rather
    /// than creating it — while folders the format itself names are made on
    /// the way. See
    /// [[Specification#Observed daily notes behavior, Obsidian 1.14.4]].
    fn open_daily(&mut self, step: Option<bool>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let (format, folder, template) = {
            let daily = state.daily_notes();
            (
                non_empty(daily.format.as_deref())
                    .unwrap_or(config::DAILY_NOTE_DEFAULT_FORMAT)
                    .to_string(),
                daily.folder.clone().unwrap_or_default(),
                daily.template.clone().unwrap_or_default(),
            )
        };
        let today = now();
        if let Some(forward) = step {
            // From the note in hand when it is a daily note, else from today.
            let from = self
                .current
                .as_deref()
                .and_then(|path| state.vault().daily_note_day(&format, &folder, path, &today))
                .unwrap_or(today);
            let next = state
                .vault()
                .step_daily_note(&format, &folder, &from, forward);
            match next {
                Some(path) => {
                    self.open_file(path, None, cx);
                    let mode = self.editing_mode();
                    self.set_mode(mode, window, cx);
                }
                None => self.report(
                    if forward {
                        "No next daily note"
                    } else {
                        "No previous daily note"
                    },
                    cx,
                ),
            }
            return;
        }
        let (path, exists) = match state.vault().daily_note(&format, &folder, &today) {
            Ok(found) => found,
            Err(err) => return self.report(err.to_string(), cx),
        };
        if exists {
            // An existing note is opened and not touched, bytes or times.
            self.open_file(path, None, cx);
            let mode = self.editing_mode();
            self.set_mode(mode, window, cx);
            return;
        }
        let text = self.daily_template_text(&state, &template, &path, cx);
        match state.create_note(&path, &text) {
            Ok(changes) => {
                self.after_changes(changes, cx);
                self.open_file(path.clone(), None, cx);
                let mode = self.editing_mode();
                self.set_mode(mode, window, cx);
                self.report(format!("Created {path}"), cx);
            }
            Err(err) => self.report(err, cx),
        }
    }

    /// The bytes a new daily note starts with: its template expanded, or
    /// nothing when none is set or it cannot be read.
    fn daily_template_text(
        &mut self,
        state: &Arc<VaultState>,
        template: &str,
        path: &str,
        cx: &mut Context<Self>,
    ) -> String {
        if template.is_empty() {
            return String::new();
        }
        let template_path = if template.ends_with(".md") {
            template.to_string()
        } else {
            format!("{template}.md")
        };
        let raw = match std::fs::read_to_string(state.vault().absolute(&template_path)) {
            Ok(text) => text,
            Err(err) => {
                self.report(format!("Could not read {template_path}: {err}"), cx);
                return String::new();
            }
        };
        let text = _text::FileText::parse(&raw).text;
        let title = basename(path).to_string();
        let templates = state.templates();
        let context = templates::Context {
            at: now(),
            date_format: non_empty(templates.date_format.as_deref())
                .unwrap_or(templates::DEFAULT_DATE_FORMAT),
            time_format: non_empty(templates.time_format.as_deref())
                .unwrap_or(templates::DEFAULT_TIME_FORMAT),
            title: &title,
        };
        templates::expand(&text, &context)
    }

    /// Obsidian's template chooser: the template folder's notes by
    /// basename, sorted, with its own placeholder. An unset or empty
    /// folder still opens the list, which then says why it is empty.
    fn open_template_chooser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.mode.is_editing() {
            return;
        }
        let Some(state) = self.state.clone() else {
            return;
        };
        let folder = state
            .templates()
            .folder
            .clone()
            .unwrap_or_default()
            .trim_matches('/')
            .to_string();
        let empty = if folder.is_empty() {
            "No template folder is set in Settings".to_string()
        } else {
            format!("No templates in {folder}")
        };
        let mut rows: Vec<ChooserRow> = Vec::new();
        if !folder.is_empty() {
            let prefix = format!("{folder}/");
            let vault = state.vault();
            for file in vault.files() {
                if file.kind != FileKind::Markdown || !file.path.starts_with(&prefix) {
                    continue;
                }
                rows.push(ChooserRow {
                    label: basename(&file.path).to_string(),
                    value: file.path.clone(),
                });
            }
        }
        rows.sort_by(|a, b| a.label.cmp(&b.label));
        self.close_editor_suggest(cx);
        self.modal = Modal::Chooser;
        self.chooser.update(cx, |c, cx| {
            c.open(rows, "Type name of a template...", empty, window, cx)
        });
        cx.notify();
    }

    /// Opens the chooser and picks `query` out of it, for `--template`.
    pub fn pick_template(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_template_chooser(window, cx);
        self.chooser.update(cx, |c, cx| c.pick(query, cx));
    }

    /// Reads the chosen template, expands its placeholders and writes it at
    /// the cursor.
    fn insert_template(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Modal::None;
        let Some(state) = self.state.clone() else {
            return;
        };
        let raw = match std::fs::read_to_string(state.vault().absolute(&path)) {
            Ok(text) => text,
            Err(err) => return self.report(format!("Could not read {path}: {err}"), cx),
        };
        // A template's own byte order mark and line endings are its file's,
        // not the target's: only its text is inserted.
        let text = _text::FileText::parse(&raw).text;
        let title = self
            .current
            .as_deref()
            .map(basename)
            .unwrap_or_default()
            .to_string();
        let expanded = {
            let templates = state.templates();
            let cx = templates::Context {
                at: now(),
                date_format: non_empty(templates.date_format.as_deref())
                    .unwrap_or(templates::DEFAULT_DATE_FORMAT),
                time_format: non_empty(templates.time_format.as_deref())
                    .unwrap_or(templates::DEFAULT_TIME_FORMAT),
                title: &title,
            };
            templates::expand(&text, &cx)
        };
        self.editor
            .update(cx, |e, cx| e.insert_at_cursors(&expanded, window, cx));
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }

    /// Writes the current date or time at the cursor, in the format
    /// `templates.json` gives — `{{date}}` and `{{time}}` without a
    /// template around them, which is what Obsidian's two commands are.
    fn insert_stamp(&mut self, date: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.mode.is_editing() {
            return;
        }
        let Some(state) = self.state.clone() else {
            return;
        };
        let templates = state.templates();
        let format = if date {
            non_empty(templates.date_format.as_deref()).unwrap_or(templates::DEFAULT_DATE_FORMAT)
        } else {
            non_empty(templates.time_format.as_deref()).unwrap_or(templates::DEFAULT_TIME_FORMAT)
        };
        let missing = templates::unsupported_tokens(format);
        if !missing.is_empty() {
            self.report(
                format!(
                    " cannot render {} in a date format",
                    missing.join(", ")
                ),
                cx,
            );
            return;
        }
        let text = templates::format(format, &now());
        drop(templates);
        self.editor
            .update(cx, |e, cx| e.insert_at_cursors(&text, window, cx));
    }

    /// Rebuilds everything that depends on the file list or the whole cache.
    fn refresh_vault_views(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let show_unsupported = state.app().show_unsupported_files.unwrap_or(false);
        let tree = {
            let vault = state.vault();
            FileTree::new(vault.files(), vault.folders(), show_unsupported)
        };
        self.explorer.update(cx, |e, cx| e.set_tree(tree, cx));
        self.tags = state
            .cache()
            .tags()
            .values()
            .map(|t| TagRow {
                name: t.name.clone().into(),
                count: t.count,
            })
            .collect();
        self.switcher
            .update(cx, |s, cx| s.set_state(state.clone(), cx));
        self.search
            .update(cx, |s, cx| s.set_state(state.clone(), cx));
        self.graph
            .update(cx, |g, cx| g.set_state(state.clone(), cx));
        self.settings
            .update(cx, |s, cx| s.set_state(state.clone(), cx));
        self.name_finder = None;
        cx.notify();
    }

    fn start_watching(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let (watcher, rx) = match watch::start(&state.root) {
            Ok(w) => w,
            Err(err) => {
                self.report(format!("File watching unavailable: {err}"), cx);
                return;
            }
        };
        self._watcher = Some(watcher);
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            if this.upgrade().is_none() {
                break;
            }
            let mut paths: Vec<PathBuf> = Vec::new();
            while let Ok(p) = rx.try_recv() {
                paths.push(p);
            }
            if paths.is_empty() {
                continue;
            }
            paths.sort();
            paths.dedup();
            if this
                .update(cx, |ws, cx| ws.files_changed(paths, cx))
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    fn files_changed(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let changes = cx
                .background_executor()
                .spawn(async move { state.apply(&paths) })
                .await;
            this.update(cx, |ws, cx| ws.after_changes(changes, cx)).ok();
        })
        .detach();
    }

    fn after_changes(&mut self, changes: Changes, cx: &mut Context<Self>) {
        if changes.files.is_empty() {
            return;
        }
        let Some(state) = self.state.clone() else {
            return;
        };
        if changes.structural() {
            self.refresh_vault_views(cx);
        } else {
            self.tags = state
                .cache()
                .tags()
                .values()
                .map(|t| TagRow {
                    name: t.name.clone().into(),
                    count: t.count,
                })
                .collect();
            self.search.update(cx, |s, cx| s.rerun(cx));
        }
        let n = changes.files.len();
        self.status = if n == 1 {
            format!("{} changed", changes.files[0].path()).into()
        } else {
            format!("{n} files changed").into()
        };
        // Back and forward must never land on a file the vault no longer
        // has, whoever removed it; an atomic external save arrives as a
        // removal too, so only a path that is really gone is forgotten.
        for change in &changes.files {
            if let FileChange::Removed(path) = change {
                if !state.vault().absolute(path).exists() {
                    self.forget_history(path);
                    let active = self.active_tab;
                    for (ix, tab) in self.tabs.iter_mut().enumerate() {
                        if ix != active && tab.path.as_deref() == Some(path.as_str()) {
                            tab.path = None;
                        }
                    }
                }
            }
        }
        if let Some(current) = self.current.clone() {
            let removed = changes
                .files
                .iter()
                .any(|c| matches!(c, FileChange::Removed(p) if *p == current));
            if removed && !state.vault().absolute(&current).exists() {
                // The open note is gone from the vault. The buffer stays on
                // screen because it may hold edits that exist nowhere else,
                // but it is no longer a file: the autosave is disarmed and
                // the panes stop describing it.
                self.detach_current(true, &state, cx);
                self.status =
                    format!("{current} was deleted; this buffer is no longer saved anywhere")
                        .into();
            } else if changes.files.iter().any(|c| c.path() == current) {
                self.show_file(&current, &state, cx);
            } else {
                self.refresh_panes(&state, cx);
            }
        } else if let Some(detached) = self.detached.clone() {
            // The file the detached buffer came from is back: a slow external
            // unlink-then-rewrite (git checkout, a sync client) lands as two
            // changes either side of the watcher's debounce. Reattach rather
            // than leave the pane showing a note the vault has again under a
            // banner saying it does not; `show_file` keeps unsaved edits and
            // reports that the file changed underneath them.
            if changes.files.iter().any(|c| c.path() == detached.as_ref())
                && state.vault().absolute(&detached).exists()
            {
                self.show_file(&detached, &state, cx);
            }
        }
        cx.notify();
    }

    /// Called once the startup flags have run, so `wait_loaded` can answer
    /// for a window that is finished opening rather than one still doing it.
    pub fn mark_started(&mut self) {
        self.started = true;
    }

    /// Whether the startup flags have been applied.
    pub fn is_started(&self) -> bool {
        self.started
    }

    pub fn is_loaded(&self) -> bool {
        self.state.is_some()
    }

    /// Selects a right-dock tab by name (`--pane` on the command line).
    pub fn show_right_tab(&mut self, name: &str, cx: &mut Context<Self>) {
        let which = match name.to_lowercase().as_str() {
            "outgoing" => RightTab::Outgoing,
            "outline" => RightTab::Outline,
            "tags" => RightTab::Tags,
            _ => RightTab::Backlinks,
        };
        self.select_right_tab(which, cx);
    }

    /// Shows a right-dock tab, bringing what it shows up to date first: the
    /// Outline is kept current only while it is the pane on screen.
    fn select_right_tab(&mut self, which: RightTab, cx: &mut Context<Self>) {
        self.right_tab = which;
        self.refresh_outline_from_buffer(cx);
        cx.notify();
    }

    pub fn editor(&self) -> &Entity<Editor> {
        &self.editor
    }

    pub fn mode(&self) -> ViewMode {
        self.mode
    }

    /// The vault path shown in the main area.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Shows the Search tab with `query`; focuses the field when a window is at hand.
    /// The dock comes back first: a focus handle on a pane that is not painted
    /// is outside the dispatch tree, so it would take every key and handle none.
    pub fn show_search(
        &mut self,
        query: &str,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        self.left_visible = true;
        self.left_tab = LeftTab::Search;
        self.search.update(cx, |s, cx| s.set_query(query, cx));
        if let Some(window) = window {
            window.focus(&self.search.focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Opens the quick switcher pre-filled with `query`.
    pub fn open_switcher(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut recent: Vec<String> = Vec::new();
        for path in self.history.iter().rev() {
            if !recent.contains(path) {
                recent.push(path.clone());
            }
        }
        self.close_editor_suggest(cx);
        self.modal = Modal::Switcher;
        self.switcher
            .update(cx, |s, cx| s.open(recent, query, window, cx));
        cx.notify();
    }

    /// Opens the command palette pre-filled with `query`.
    pub fn open_palette(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        let registry = cx.global::<Commands>().0.clone();
        let pinned: Vec<String> = self
            .state
            .as_ref()
            .map(|s| s.palette.pinned.clone())
            .unwrap_or_default();
        let in_table = self.mode.is_editing() && self.editor.read(cx).in_table();
        let commands: Vec<PaletteCommand> = registry
            .iter()
            .filter(|c| RUNNABLE.contains(&c.id.as_str()))
            // Obsidian's table commands hide unless the cursor is in a
            // table; their check callback is what does it there.
            .filter(|c| in_table || TableCommand::from_id(&c.id).is_none())
            .map(|c| PaletteCommand::from_command(c, pinned.contains(&c.id)))
            .collect();
        let recent = self.recent_commands.clone();
        self.close_editor_suggest(cx);
        self.modal = Modal::Palette;
        self.palette
            .update(cx, |p, cx| p.open(commands, recent, query, window, cx));
        cx.notify();
    }

    /// Opens a Settings dropdown's options, for `--menu`.
    pub fn open_settings_menu(&mut self, key: &str, cx: &mut Context<Self>) {
        self.settings.update(cx, |s, cx| s.open_menu(key, cx));
    }

    /// Opens the Settings modal, on `tab` when given (`--settings` on the command line).
    pub fn open_settings(
        &mut self,
        tab: Option<settings::Tab>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_editor_suggest(cx);
        self.modal = Modal::Settings;
        self.settings.update(cx, |s, cx| s.open(tab, window, cx));
        cx.notify();
    }

    /// A modal going up takes the autocompletion popup down with it: the
    /// popup would otherwise stay painted under the modal, still holding
    /// the keys it binds.
    fn close_editor_suggest(&mut self, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.close_suggest(cx));
    }

    fn close_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.modal != Modal::None {
            self.modal = Modal::None;
            let mode = self.mode;
            self.set_mode(mode, window, cx);
        }
    }

    fn run_command_action(
        &mut self,
        action: &RunCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = action.id.clone();
        self.run_command(&id, window, cx);
    }

    /// Runs a command by its Obsidian id; unknown or not-yet-implemented
    /// ids are reported in the status bar.
    pub fn run_command(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if RUNNABLE.contains(&id) {
            self.recent_commands.retain(|r| r != id);
            self.recent_commands.insert(0, id.to_string());
            self.recent_commands.truncate(20);
        }
        match id {
            "command-palette:open" => self.open_palette("", window, cx),
            "editor:save-file" => self.flush_save(cx),
            "editor:open-search" => self.open_find("", false, window, cx),
            "editor:open-search-replace" => self.open_find("", true, window, cx),
            "editor:add-cursor-above" | "editor:add-cursor-below" => {
                if self.editing_or_report(id, window, cx) {
                    let above = id == "editor:add-cursor-above";
                    self.editor
                        .update(cx, |e, cx| e.add_cursor(above, window, cx));
                }
            }
            "editor:toggle-source" => {
                let mode = match self.mode {
                    ViewMode::LivePreview => ViewMode::Source,
                    _ => ViewMode::LivePreview,
                };
                self.set_mode(mode, window, cx);
            }
            "switcher:open" => self.open_switcher("", window, cx),
            "global-search:open" => self.open_search_action(&OpenSearch, window, cx),
            "graph:open" => self.open_graph_action(&OpenGraph, window, cx),
            "graph:open-local" => self.show_graph(true, window, cx),
            "markdown:toggle-preview" => self.toggle_mode(&ToggleMode, window, cx),
            // The properties block is the same editable widget in reading
            // view, and `editor:toggle-fold-properties` was observed running
            // there with the preview leaf active (Specification.md, "Folding"),
            // so these three ask only that the note hold the keyboard.
            //
            // Obsidian opens an unnamed property row and writes nothing until
            // it is named, which is what `add_property_row` does.
            "markdown:add-metadata-property" => {
                if self.note_has_keys(window, cx) {
                    self.editor
                        .update(cx, |e, cx| e.add_property_row(window, cx));
                }
            }
            "markdown:clear-metadata-properties" => {
                if self.note_has_keys(window, cx) {
                    self.editor
                        .update(cx, |e, cx| e.clear_properties(window, cx));
                }
            }
            "editor:toggle-fold-properties" => {
                if self.note_has_keys(window, cx) {
                    self.editor.update(cx, |e, cx| e.toggle_properties_fold(cx));
                }
            }
            "editor:insert-table" => {
                if self.editing_or_report(id, window, cx) {
                    self.editor.update(cx, |e, cx| e.insert_table(window, cx));
                }
            }
            // The wrapping toggles: each writes the pair Obsidian's
            // markup uses around the selection, or takes it off again.
            "editor:toggle-bold"
            | "editor:toggle-italics"
            | "editor:toggle-strikethrough"
            | "editor:toggle-highlight"
            | "editor:toggle-code"
            | "editor:toggle-inline-math"
            | "editor:toggle-comments" => {
                if self.editing_or_report(id, window, cx) {
                    let marker = match id {
                        "editor:toggle-bold" => "**",
                        "editor:toggle-italics" => "*",
                        "editor:toggle-strikethrough" => "~~",
                        "editor:toggle-highlight" => "==",
                        "editor:toggle-code" => "`",
                        "editor:toggle-inline-math" => "$",
                        _ => "%%",
                    };
                    self.editor
                        .update(cx, |e, cx| e.toggle_wrap(marker, window, cx));
                }
            }
            "editor:toggle-bullet-list"
            | "editor:toggle-numbered-list"
            | "editor:toggle-blockquote" => {
                if self.editing_or_report(id, window, cx) {
                    let kind = match id {
                        "editor:toggle-bullet-list" => LinePrefix::Bullet,
                        "editor:toggle-numbered-list" => LinePrefix::Ordered,
                        _ => LinePrefix::Quote,
                    };
                    self.editor
                        .update(cx, |e, cx| e.toggle_line_prefix(kind, window, cx));
                }
            }
            "editor:toggle-checklist-status" => {
                if self.editing_or_report(id, window, cx) {
                    let flipped = self
                        .editor
                        .update(cx, |e, cx| e.toggle_checklist(window, cx));
                    if !flipped {
                        self.report("There is no checkbox to toggle", cx);
                    }
                }
            }
            "editor:set-heading-0"
            | "editor:set-heading-1"
            | "editor:set-heading-2"
            | "editor:set-heading-3"
            | "editor:set-heading-4"
            | "editor:set-heading-5"
            | "editor:set-heading-6" => {
                if self.editing_or_report(id, window, cx) {
                    let level = id
                        .rsplit('-')
                        .next()
                        .and_then(|n| n.parse::<u8>().ok())
                        .unwrap_or(0);
                    self.editor
                        .update(cx, |e, cx| e.set_heading(level, window, cx));
                }
            }
            "editor:indent-list" | "editor:unindent-list" => {
                if self.editing_or_report(id, window, cx) {
                    let indent = id == "editor:indent-list";
                    self.editor
                        .update(cx, |e, cx| e.indent_list(indent, window, cx));
                }
            }
            "editor:delete-paragraph" => {
                if self.editing_or_report(id, window, cx) {
                    self.editor
                        .update(cx, |e, cx| e.delete_paragraph(window, cx));
                }
            }
            "editor:swap-line-up" | "editor:swap-line-down" => {
                if self.editing_or_report(id, window, cx) {
                    let up = id == "editor:swap-line-up";
                    self.editor.update(cx, |e, cx| e.swap_lines(up, window, cx));
                }
            }
            "editor:insert-link" | "editor:insert-wikilink" | "editor:insert-embed" => {
                if self.editing_or_report(id, window, cx) {
                    let kind = match id {
                        "editor:insert-link" => LinkSyntax::Markdown,
                        "editor:insert-wikilink" => LinkSyntax::Wikilink,
                        _ => LinkSyntax::Embed,
                    };
                    self.editor
                        .update(cx, |e, cx| e.insert_link(kind, window, cx));
                }
            }
            "editor:follow-link" => {
                if self.editing_or_report(id, window, cx) {
                    self.editor.update(cx, |e, cx| e.follow_link_at_cursor(cx));
                }
            }
            "editor:open-link-in-new-leaf" => {
                if self.editing_or_report(id, window, cx) {
                    self.link_in_new_tab = true;
                    let found = self.editor.update(cx, |e, cx| e.follow_link_at_cursor(cx));
                    if !found {
                        self.link_in_new_tab = false;
                    }
                }
            }
            "insert-current-date" | "insert-current-time" => {
                if self.editing_or_report(id, window, cx) {
                    let date = id == "insert-current-date";
                    self.insert_stamp(date, window, cx);
                }
            }
            "insert-template" => {
                if self.editing_or_report(id, window, cx) {
                    self.open_template_chooser(window, cx);
                }
            }
            "daily-notes" => self.open_daily(None, window, cx),
            "daily-notes:goto-prev" => self.open_daily(Some(false), window, cx),
            "daily-notes:goto-next" => self.open_daily(Some(true), window, cx),
            _ if TableCommand::from_id(id).is_some() => {
                let cmd = TableCommand::from_id(id).expect("checked above");
                if self.editing_or_report(id, window, cx) {
                    let ran = self
                        .editor
                        .update(cx, |e, cx| e.table_command(cmd, window, cx));
                    if !ran {
                        self.report("The cursor is not in a table", cx);
                    }
                }
            }
            "app:go-back" => self.go_back(&GoBack, window, cx),
            "app:go-forward" => self.go_forward(&GoForward, window, cx),
            "app:open-settings" => self.open_settings(None, window, cx),
            "app:toggle-left-sidebar" => {
                self.left_visible = !self.left_visible;
                if !self.left_visible {
                    self.leave_search_field(window, cx);
                }
                cx.notify();
            }
            "app:toggle-right-sidebar" => {
                self.right_visible = !self.right_visible;
                self.refresh_outline_from_buffer(cx);
                cx.notify();
            }
            "file-explorer:open" => {
                self.leave_search_field(window, cx);
                self.left_visible = true;
                self.left_tab = LeftTab::Files;
                cx.notify();
            }
            "file-explorer:reveal-active-file" => {
                self.leave_search_field(window, cx);
                self.left_visible = true;
                self.left_tab = LeftTab::Files;
                if let Some(current) = self.current.clone() {
                    self.explorer.update(cx, |e, cx| e.reveal(&current, cx));
                }
                cx.notify();
            }
            "file-explorer:new-file"
            | "file-explorer:new-file-in-current-tab"
            | "file-explorer:new-file-in-new-pane" => {
                // Obsidian's "Create new note" goes to a new tab too, unless
                // the active one is empty; "in current tab" replaces the
                // note, and "to the right" would split, which is a new tab
                // in the one strip  has.
                let fresh_tab = match id {
                    "file-explorer:new-file-in-current-tab" => false,
                    "file-explorer:new-file-in-new-pane" => true,
                    _ => self.current.is_some(),
                };
                // The folder follows the note that was open, so it is
                // settled before the new tab empties the main area.
                let folder = self
                    .state
                    .as_ref()
                    .map(|s| {
                        s.vault().new_note_folder(
                            s.app().new_file_location.as_deref(),
                            s.app().new_file_folder_path.as_deref(),
                            self.current.as_deref(),
                        )
                    })
                    .unwrap_or_default();
                if fresh_tab {
                    self.new_tab(cx);
                }
                self.new_note(&folder, window, cx);
            }
            "file-explorer:new-folder" => self.start_new_folder("", window, cx),
            "file-explorer:move-file" => {
                if let Some(current) = self.current.clone() {
                    self.start_move(&current, window, cx);
                }
            }
            "file-explorer:duplicate-file" => {
                if let Some(current) = self.current.clone() {
                    self.duplicate(&current, cx);
                }
            }
            "workspace:edit-file-title" => {
                if let Some(current) = self.current.clone() {
                    self.start_rename(&current, window, cx);
                }
            }
            "workspace:new-tab" => self.new_tab(cx),
            "workspace:close" => self.close_tab(self.active_tab, cx),
            "workspace:close-others" => self.close_other_tabs(cx),
            "workspace:next-tab" => self.cycle_tab(1, cx),
            "workspace:previous-tab" => self.cycle_tab(self.tabs.len().saturating_sub(1), cx),
            "workspace:goto-last-tab" => self.activate_tab(self.tabs.len().saturating_sub(1), cx),
            "workspace:undo-close-pane" => self.undo_close_tab(cx),
            _ if id.starts_with("workspace:goto-tab-") => {
                let n: usize = id
                    .rsplit('-')
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                if n >= 1 && n <= self.tabs.len() {
                    self.activate_tab(n - 1, cx);
                }
            }
            "app:delete-file" => {
                if let Some(current) = self.current.clone() {
                    self.start_delete(&current, window, cx);
                }
            }
            "backlink:open" | "backlink:open-backlinks" => {
                self.right_visible = true;
                self.show_right_tab("backlinks", cx);
            }
            "outgoing-links:open" | "outgoing-links:open-for-current" => {
                self.right_visible = true;
                self.show_right_tab("outgoing", cx);
            }
            "outline:open" | "outline:open-for-current" => {
                self.right_visible = true;
                self.show_right_tab("outline", cx);
            }
            "tag-pane:open" => {
                self.right_visible = true;
                self.show_right_tab("tags", cx);
            }
            _ => {
                let name = self.command_name(id, cx);
                self.report(format!("{name}: not available in  yet"), cx);
            }
        }
    }

    /// The command's name as the palette shows it, for a message about it.
    fn command_name(&self, id: &str, cx: &App) -> String {
        cx.global::<Commands>()
            .0
            .get(id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    /// True when the note itself holds the keyboard: no modal is up and the
    /// focus is on the pane that shows it. A hotkey is bound in the
    /// workspace's key context and so fires wherever the keyboard is, while
    /// Obsidian scopes an editor command to the active editor; without this
    /// a Ctrl+B typed into the search field writes bold markers into the
    /// note behind it.
    fn note_has_keys(&self, window: &Window, cx: &App) -> bool {
        self.modal == Modal::None
            && (self.editor.read(cx).focus_handle_ref().is_focused(window)
                || self.reading.focus_handle(cx).is_focused(window))
    }

    /// True when an editor command can run. Obsidian hides a command whose
    /// check callback fails; 's palette cannot tell yet, so the
    /// command says why it did nothing rather than silently doing nothing —
    /// but only when the note is what the keys were meant for. A command
    /// that arrives while a dock field or a modal has the keyboard is
    /// addressed to that field, and saying anything about the note would be
    /// an answer to a question nobody asked.
    fn editing_or_report(&mut self, id: &str, window: &Window, cx: &mut Context<Self>) -> bool {
        if !self.note_has_keys(window, cx) {
            return false;
        }
        if self.mode.is_editing() {
            return true;
        }
        let name = self.command_name(id, cx);
        self.report(format!("{name}: only available while editing"), cx);
        false
    }

    fn open_search_action(&mut self, _: &OpenSearch, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.search.read(cx).query(cx);
        self.show_search(&query, Some(window), cx);
    }

    /// Shows the graph (`graph:open`); a second Ctrl+G returns to the note.
    pub fn show_graph(&mut self, local: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.graph.update(cx, |g, cx| g.set_local(local, cx));
        self.set_mode(ViewMode::Graph, window, cx);
    }

    fn open_graph_action(&mut self, _: &OpenGraph, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode == ViewMode::Graph {
            let back = self.mode_before_graph;
            self.set_mode(back, window, cx);
        } else {
            self.show_graph(false, window, cx);
        }
    }

    pub fn set_mode(&mut self, mode: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        if mode == ViewMode::Graph && self.mode != ViewMode::Graph {
            self.mode_before_graph = self.mode;
        }
        if !mode.is_editing() && self.mode.is_editing() {
            // The reading view shows what the editor has: save and rebuild now
            // rather than waiting for the autosave and the watcher.
            if self.editor.read(cx).is_dirty() {
                self.flush_save(cx);
                if let (Some(state), Some(current)) = (self.state.clone(), self.current.clone()) {
                    self.show_file(&current, &state, cx);
                }
            }
        }
        self.mode = mode;
        if mode.is_editing() {
            self.editor.update(cx, |e, cx| {
                e.set_live_preview(mode == ViewMode::LivePreview, cx)
            });
        }
        self.focus_main(window, cx);
        cx.notify();
    }

    /// Asks for the keyboard to be put back on the main area at the next
    /// paint.
    ///
    /// Where focus belongs when the main area changes is one rule — on
    /// whatever that area paints — but most of what changes it cannot
    /// enforce the rule itself: a dock row's click and the socket's `open`
    /// reach the workspace through a subscription or a handler that has no
    /// `Window`. They record the request instead, and [`Workspace::render`],
    /// which has the window and is building the very tree the focus has to
    /// land in, carries it out.
    fn request_focus_main(&mut self, cx: &mut Context<Self>) {
        self.refocus_main = true;
        cx.notify();
    }

    /// Whether the keyboard is somewhere nothing can use it: on the
    /// workspace root, which is where a click in a dock leaves it, on no
    /// element at all, or on a main view the main area is not painting.
    ///
    /// Anything else holds it on purpose — the find panel, the search box,
    /// a property row, a modal's input — and a note opening behind one of
    /// those must not take it away.
    fn keyboard_is_parked(&self, window: &Window, cx: &App) -> bool {
        if window.focused(cx).is_none() || self.focus_handle.is_focused(window) {
            return true;
        }
        let painted = self.main_view(cx);
        let unpainted =
            |view: MainView, handle: &FocusHandle| painted != view && handle.is_focused(window);
        unpainted(MainView::Editor, self.editor.read(cx).focus_handle_ref())
            || unpainted(MainView::Reading, &self.reading.focus_handle(cx))
            || unpainted(MainView::Graph, &self.graph.focus_handle(cx))
    }

    /// Puts the keyboard on whatever the main area actually paints: a handle
    /// on a view that is not there is outside the dispatch tree, and the keys
    /// meant for it reach nothing.
    fn focus_main(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refocus_main = false;
        match self.main_view(cx) {
            MainView::Reading => window.focus(&self.reading.focus_handle(cx), cx),
            MainView::Editor => window.focus(&self.editor.focus_handle(cx), cx),
            MainView::Graph => window.focus(&self.graph.focus_handle(cx), cx),
            MainView::Empty => window.focus(&self.focus_handle, cx),
        }
    }

    /// Hands the keyboard back to the main area before the left dock
    /// stops painting the search field. Hiding the dock or switching it to
    /// the Files tab unmounts that input, and focus left on an unmounted
    /// element is outside the dispatch tree: every key in the app, the
    /// hotkey that would undo this included, then reaches nothing.
    fn leave_search_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.focus_handle(cx).is_focused(window) {
            self.focus_main(window, cx);
        }
    }

    /// Opens the editor's find panel (with the replace row when asked),
    /// switching to editing first since the panel lives in the editor.
    /// An empty `query` keeps the panel's own (or the selection).
    pub fn open_find(
        &mut self,
        query: &str,
        replace: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.mode.is_editing() {
            let mode = self.editing_mode();
            self.set_mode(mode, window, cx);
        }
        self.editor
            .update(cx, |e, cx| e.open_find(query, replace, window, cx));
    }

    /// The editing mode the vault asks for: Live Preview unless
    /// `livePreview` is off.
    pub fn editing_mode(&self) -> ViewMode {
        let live = self
            .state
            .as_ref()
            .and_then(|s| s.app().live_preview)
            .unwrap_or(true);
        if live {
            ViewMode::LivePreview
        } else {
            ViewMode::Source
        }
    }

    /// The mode a note opens in at startup: `defaultViewMode` (`preview`
    /// is reading view; anything else, Obsidian's default included, edits).
    pub fn default_mode(&self) -> ViewMode {
        let preview = self
            .state
            .as_ref()
            .map(|s| s.app().default_view_mode.as_deref() == Some("preview"))
            .unwrap_or(false);
        if preview {
            ViewMode::Reading
        } else {
            self.editing_mode()
        }
    }

    /// Opens a vault path by exact match or by suffix (`--note` on the command
    /// line); `Name#Heading` or `Name#^id` also scrolls to that section.
    pub fn open_by_name(&mut self, name: &str, cx: &mut Context<Self>) -> bool {
        let Some(state) = self.state.clone() else {
            return false;
        };
        let link = _markdown::parse_linktext(name);
        let wanted = link.path;
        let found = state
            .vault()
            .files()
            .iter()
            .map(|f| f.path.as_str())
            .find(|p| {
                *p == wanted
                    || p.ends_with(wanted)
                    || p.trim_end_matches(".md") == wanted
                    || p.trim_end_matches(".md").ends_with(&format!("/{wanted}"))
            })
            .map(str::to_string);
        match found {
            Some(path) => {
                self.open_file(path, link.subpath, cx);
                true
            }
            None => false,
        }
    }

    /// Opens the first markdown note, if any.
    pub fn open_first(&mut self, cx: &mut Context<Self>) {
        let first = self
            .state
            .as_ref()
            .and_then(|s| s.vault().markdown_files().next().map(|f| f.path.clone()));
        if let Some(path) = first {
            self.open_file(path, None, cx);
        }
    }

    /// Opens a vault-relative path in the main area and records it in history.
    pub fn open_file(
        &mut self,
        path: String,
        subpath: Option<Subpath<'_>>,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.state.clone() else {
            self.current = Some(path);
            return;
        };
        if self.current.as_deref() != Some(path.as_str()) {
            self.flush_save(cx);
            self.history.truncate(self.history_ix);
            self.history.push(path.clone());
            self.history_ix = self.history.len();
            self.last_open_files.retain(|p| *p != path);
            self.last_open_files.insert(0, path.clone());
            self.last_open_files.truncate(LAST_OPEN_FILES_KEPT);
        }
        self.show_file(&path, &state, cx);
        // Opening a note is a move into the main area, wherever it was asked
        // for: an explorer row, a search hit, the switcher, a link, the
        // socket. Obsidian hands the keyboard to the note it just opened, and
        // without this the click that opened it leaves focus on the
        // workspace root, where typing and the arrow keys reach nothing.
        self.request_focus_main(cx);
        if self.current.as_deref() != Some(path.as_str()) {
            // It did not open, so it is not somewhere to go back to either.
            self.forget_history(&path);
            return;
        }
        if let Some(sub) = subpath {
            self.scroll_to_subpath(&path, &sub, &state, cx);
        }
    }

    fn show_file(&mut self, path: &str, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let previous = self.current.replace(path.to_string());
        if let Some(gone) = self.detached.take() {
            // The buffer was kept because its edits exist nowhere else. There
            // is no file to save them to, and resurrecting one the user
            // deleted would be worse, so the least this can do on the way out
            // is name what it is dropping rather than drop it in silence.
            let lost = {
                let e = self.editor.read(cx);
                e.is_dirty() && e.name.as_ref() != path
            };
            if lost {
                self.report(
                    format!(
                        "Discarded the unsaved edits in {gone}; nothing was saving that buffer"
                    ),
                    cx,
                );
            }
        }
        self.explorer.update(cx, |e, cx| e.reveal(path, cx));
        self.graph
            .update(cx, |g, cx| g.set_current(Some(path.to_string()), cx));
        let kind = FileKind::from_path(path);
        match kind {
            FileKind::Markdown => match std::fs::read_to_string(state.vault().absolute(path)) {
                Ok(text) => {
                    let started = Instant::now();
                    let model =
                        reading::build_with(path, &text, state.as_ref(), state.build_options());
                    let build_ms = started.elapsed().as_secs_f64() * 1000.0;
                    self.words = text.split_whitespace().count();
                    self.chars = text.chars().count();
                    self.reading
                        .update(cx, |r, cx| r.set_note(Some(path.to_string()), model, cx));
                    // The editor keeps its buffer when the file on disk is
                    // what it last saved (or still holds unsaved edits), and
                    // keeps its place when the note changed underneath it.
                    let (same, matches, dirty) = {
                        let e = self.editor.read(cx);
                        (e.name.as_ref() == path, e.matches_file(&text), e.is_dirty())
                    };
                    if same && matches {
                        // Own write or an unrelated event.
                    } else if same && dirty {
                        self.report(format!("{path} changed on disk; unsaved edits kept"), cx);
                        self.refresh_panes(state, cx);
                        cx.notify();
                        return;
                    } else if same {
                        self.editor.update(cx, |e, cx| e.reload(text, cx));
                    } else {
                        self.editor
                            .update(cx, |e, cx| e.set_text(path.to_string(), text, cx));
                    }
                    self.status =
                        format!("{path}  ·  reading view built in {build_ms:.1} ms").into();
                }
                Err(err) => {
                    // `current` may not name a file the app could not read:
                    // the panes, the explorer row and the autosave all key
                    // off it while the editor still holds another note, and
                    // that mismatch is what silently drops the edits typed
                    // into it. The view goes back to the note it is really
                    // showing, or lets go of the buffer when there is none.
                    self.report(format!("Could not read {path}: {err}"), cx);
                    self.current = previous.filter(|p| p.as_str() != path);
                    match self.current.clone() {
                        Some(back) => {
                            self.explorer.update(cx, |e, cx| e.reveal(&back, cx));
                            self.graph.update(cx, |g, cx| g.set_current(Some(back), cx));
                        }
                        None => self.detach_current(true, state, cx),
                    }
                }
            },
            FileKind::Image => {
                let model = reading::build_with(
                    path,
                    &format!("![[{path}]]"),
                    state.as_ref(),
                    state.build_options(),
                );
                self.reading
                    .update(cx, |r, cx| r.set_note(Some(path.to_string()), model, cx));
                self.release_buffer(cx);
                self.status = path.to_string().into();
            }
            other => {
                // Nothing of the note that was open may survive into a file
                // no view can draw: its text is not this file's and its links
                // would still be clickable underneath the placeholder. The
                // file's own bytes must not reach the editor either — that
                // buffer saves through the markdown writer.
                self.reading.update(cx, |r, cx| {
                    r.set_note(
                        None,
                        reading::build_with("", "", state.as_ref(), state.build_options()),
                        cx,
                    )
                });
                self.release_buffer(cx);
                self.status = format!("{path}: {other:?} files are not supported yet").into();
            }
        }
        self.refresh_panes(state, cx);
        cx.notify();
    }

    /// Plain text of a cached heading (markup stripped).
    fn heading_text(raw: &str) -> String {
        let doc = _markdown::parse(raw);
        match doc.blocks.first().map(|b| &b.kind) {
            Some(_markdown::BlockKind::Paragraph { content }) => {
                _markdown::plain_text(content)
            }
            _ => raw.to_string(),
        }
        .trim()
        .to_string()
    }

    /// Lets go of the buffer on the way into a file the editor cannot save:
    /// a canvas, an image, a PDF. Unsaved edits are not thrown away — they
    /// would exist nowhere else — so a dirty buffer stays on screen, marked
    /// as belonging to no file; a clean one goes.
    fn release_buffer(&mut self, cx: &mut Context<Self>) {
        let (dirty, name) = {
            let editor = self.editor.read(cx);
            (editor.is_dirty(), editor.name.clone())
        };
        if dirty && !name.is_empty() {
            self.save_task = None;
            self.detached = Some(name);
            return;
        }
        self.clear_buffer(cx);
    }

    /// Empties the editor and forgets what it was showing. Keeping the bytes
    /// of a canvas or a PDF in the editor would let the autosave write them
    /// back through the markdown writer.
    fn clear_buffer(&mut self, cx: &mut Context<Self>) {
        self.save_task = None;
        self.detached = None;
        self.editor
            .update(cx, |e, cx| e.set_text("", String::new(), cx));
        self.words = 0;
        self.chars = 0;
    }

    /// Lets go of the open note: nothing may write the buffer back, and no
    /// pane, graph or history entry may keep describing a file that is not
    /// there. `keep_buffer` is for a file that went away underneath the app,
    /// where the text on screen may hold edits that exist nowhere else; it
    /// stays, marked as saved nowhere, rather than being thrown away.
    fn detach_current(
        &mut self,
        keep_buffer: bool,
        state: &Arc<VaultState>,
        cx: &mut Context<Self>,
    ) {
        self.save_task = None;
        if keep_buffer {
            let name = self.editor.read(cx).name.clone();
            self.detached = (!name.is_empty()).then_some(name);
        } else {
            self.clear_buffer(cx);
        }
        self.reading.update(cx, |r, cx| {
            r.set_note(
                None,
                reading::build_with("", "", state.as_ref(), state.build_options()),
                cx,
            )
        });
        self.current = None;
        self.graph.update(cx, |g, cx| g.set_current(None, cx));
        self.clear_panes(cx);
    }

    /// Drops every row that describes the open note. The panes are not
    /// refreshed on the way out of a file, so without this the outline,
    /// backlinks and outgoing links of a note that is gone stay on screen
    /// and stay clickable.
    fn clear_panes(&mut self, cx: &mut Context<Self>) {
        self.outline.clear();
        self.linked.clear();
        self.outgoing_links.clear();
        self.backlink_rows.clear();
        self.outgoing_rows.clear();
        cx.notify();
    }

    /// Forgets `path`, and anything under it when it is a folder, so back
    /// and forward can never navigate to a file the vault no longer has.
    fn forget_history(&mut self, path: &str) {
        self.history_ix = prune_history(&mut self.history, self.history_ix, path);
        let active = self.active_tab;
        for (ix, tab) in self.tabs.iter_mut().enumerate() {
            if ix != active {
                tab.history_ix = prune_history(&mut tab.history, tab.history_ix, path);
            }
        }
    }

    fn refresh_panes(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let Some(path) = self.current.clone() else {
            self.clear_panes(cx);
            return;
        };
        let cache = state.cache();
        let texts = state.texts();
        self.outline = cache.file(&path).map(outline_rows).unwrap_or_default();

        // Linked mentions: every link into this file, with its line as context.
        let mut linked = Vec::new();
        for b in cache.backlinks(&path) {
            linked.push(PaneRow::Item {
                path: Some(b.source.to_string()),
                label: display_name(b.source.rsplit('/').next().unwrap_or(b.source))
                    .to_string()
                    .into(),
                detail: backlink_detail(&b, texts.get(b.source).map(|t| t.as_str())).into(),
                offset: Some(b.link.position.start.offset),
            });
        }
        let n = linked.len();
        linked.insert(0, PaneRow::Header(format!("Linked mentions ({n})").into()));
        self.linked = linked;

        // Outgoing: the links this file writes, properties included.
        let mut out = outgoing_link_rows(&cache, &path);
        let n = out.len();
        out.insert(0, PaneRow::Header(format!("Links ({n})").into()));
        self.outgoing_links = out;
        drop(cache);
        drop(texts);

        self.backlink_rows = self.linked.clone();
        self.outgoing_rows = self.outgoing_links.clone();
        self.compute_mentions(&path, state, cx);
        cx.notify();
    }

    /// Rebuilds the Outline from the editor's buffer instead of from the
    /// index.
    ///
    /// The index is a save behind while a note is being typed, so the rows
    /// name headings the file no longer has — and, worse, carry byte offsets
    /// into a version of the note that is not on screen, which is what makes
    /// clicking one land the caret in the wrong place. Only the Outline is
    /// rebuilt: backlinks and tags are about other files and cannot go stale
    /// from typing in this one. It costs a parse of the buffer, so it is
    /// paid only while the pane showing those offsets is up.
    fn refresh_outline_from_buffer(&mut self, cx: &mut Context<Self>) {
        if !self.right_visible || self.right_tab != RightTab::Outline {
            return;
        }
        let Some(path) = self.current.clone() else {
            return;
        };
        let rows = {
            let editor = self.editor.read(cx);
            // A clean buffer is the file the index read, and a buffer
            // holding another note says nothing about this one.
            if editor.name.as_ref() != path.as_str() || !editor.is_dirty() {
                return;
            }
            outline_rows(&index_markdown(editor.text()))
        };
        self.outline = rows;
        cx.notify();
    }

    /// Finds unlinked mentions in both directions on the background executor.
    fn compute_mentions(&mut self, path: &str, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        self.mention_generation += 1;
        let generation = self.mention_generation;
        let state = state.clone();
        let path = path.to_string();
        let finder = self.name_finder.clone();
        cx.spawn(async move |this, cx| {
            let (rows, finder) = cx
                .background_executor()
                .spawn(async move {
                    let finder = finder.or_else(|| {
                        let names = state.names();
                        let refs: Vec<Name<'_>> = names
                            .iter()
                            .map(|(text, path)| Name { text, path })
                            .collect();
                        Finder::new(&refs).map(Arc::new)
                    });
                    (mention_rows(&state, &path, finder.as_deref()), finder)
                })
                .await;
            this.update(cx, |ws, cx| {
                ws.name_finder = finder;
                if ws.mention_generation != generation || ws.current.as_deref() != Some(&rows.path)
                {
                    return;
                }
                let mut backlinks = ws.linked.clone();
                backlinks.extend(rows.incoming);
                ws.backlink_rows = backlinks;
                let mut outgoing = ws.outgoing_links.clone();
                outgoing.extend(rows.outgoing);
                ws.outgoing_rows = outgoing;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Scrolls whichever view is showing the open note to a byte offset.
    ///
    /// Both views move. The reading view is the one a reading-mode jump
    /// scrolls, but Live Preview is the default mode, and until this moved the
    /// editor too every outline row, backlink and search hit scrolled a view
    /// nobody was looking at.
    fn reveal_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.reading
            .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
        self.editor.update(cx, |e, cx| e.reveal_offset(offset, cx));
        // An outline row or a backlink moved the caret; the keyboard goes
        // where the caret went, not to the dock row that was clicked.
        self.request_focus_main(cx);
    }

    fn scroll_to_subpath(
        &mut self,
        path: &str,
        subpath: &Subpath<'_>,
        state: &Arc<VaultState>,
        cx: &mut Context<Self>,
    ) {
        let cache = state.cache();
        let Some(meta) = cache.file(path) else {
            return;
        };
        let offset = match subpath {
            Subpath::Headings(names) => {
                let Some(last) = names.last() else { return };
                meta.headings
                    .iter()
                    .find(|h| Self::heading_text(&h.heading).eq_ignore_ascii_case(last.trim()))
                    .map(|h| h.position.start.offset)
            }
            Subpath::Block(id) => meta
                .blocks
                .get(&id.to_lowercase())
                .or_else(|| meta.blocks.get(*id))
                .map(|b| b.position.start.offset),
        };
        drop(cache);
        if let Some(offset) = offset {
            self.reveal_offset(offset, cx);
        }
    }

    fn follow_link(&mut self, target: LinkTarget, cx: &mut Context<Self>) {
        let new_tab = std::mem::take(&mut self.link_in_new_tab);
        match target {
            LinkTarget::Internal(body) => {
                let Some(state) = self.state.clone() else {
                    return;
                };
                let source = self.current.clone().unwrap_or_default();
                let parsed = _markdown::parse_linktext(&body);
                let resolved = state.cache().resolve(&body, &source).map(str::to_string);
                match resolved {
                    Some(path) if new_tab => self.open_in_new_tab(path, parsed.subpath, cx),
                    Some(path) => {
                        if path == source {
                            if let Some(sub) = parsed.subpath {
                                self.scroll_to_subpath(&path, &sub, &state, cx);
                            }
                        } else {
                            self.open_file(path, parsed.subpath, cx);
                        }
                    }
                    None => {
                        // Obsidian creates the note a dangling link points at.
                        if new_tab {
                            self.new_tab(cx);
                        }
                        let name = parsed.path.to_string();
                        self.create_note_for(&name, None, cx);
                    }
                }
            }
            LinkTarget::External(url) => cx.open_url(&url),
            LinkTarget::Tag(tag) => self.show_search(&format!("tag:#{tag}"), None, cx),
            LinkTarget::Footnote(_) => {
                self.reading.update(cx, |r, cx| r.scroll_to_end(cx));
            }
        }
    }

    fn toggle_mode(&mut self, _: &ToggleMode, window: &mut Window, cx: &mut Context<Self>) {
        let mode = match self.mode {
            ViewMode::Reading => self.editing_mode(),
            ViewMode::LivePreview | ViewMode::Source | ViewMode::Graph => ViewMode::Reading,
        };
        self.set_mode(mode, window, cx);
    }

    fn go_back(&mut self, _: &GoBack, _: &mut Window, cx: &mut Context<Self>) {
        if self.history_ix > 1 {
            self.history_ix -= 1;
            self.navigate_history(cx);
        }
    }

    fn go_forward(&mut self, _: &GoForward, _: &mut Window, cx: &mut Context<Self>) {
        if self.history_ix < self.history.len() {
            self.history_ix += 1;
            self.navigate_history(cx);
        }
    }

    fn navigate_history(&mut self, cx: &mut Context<Self>) {
        let (Some(state), Some(path)) = (
            self.state.clone(),
            self.history.get(self.history_ix.wrapping_sub(1)).cloned(),
        ) else {
            return;
        };
        self.flush_save(cx);
        self.show_file(&path, &state, cx);
        self.request_focus_main(cx);
    }

    /// A task checkbox a reader clicked in the reading view. The editor
    /// still holds the open note's buffer while the reader is the view on
    /// screen, so the edit takes exactly the path a Live Preview click
    /// takes: one undo step, and the autosave writes it back. The reader
    /// is then rebuilt from that buffer rather than from the file, which
    /// the save has not reached yet.
    fn toggle_task(&mut self, path: &str, marker: TaskMarker, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        // A click that outlived the note it was made in must write nothing:
        // the marker's line and column belong to that note's bytes.
        if self.current.as_deref() != Some(path) || self.editor.read(cx).name.as_ref() != path {
            return;
        }
        if !self.editor.update(cx, |e, cx| {
            e.toggle_task_at(marker.line, marker.col, None, cx)
        }) {
            return;
        }
        let text = self.editor.read(cx).text().to_string();
        self.words = text.split_whitespace().count();
        self.chars = text.chars().count();
        let model = reading::build_with(path, &text, state.as_ref(), state.build_options());
        self.reading
            .update(cx, |r, cx| r.set_note(Some(path.to_string()), model, cx));
        cx.notify();
    }

    /// Writes the editor's buffer back after an edit settles (two seconds,
    /// restarted by every edit, the way Obsidian's autosave works).
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            this.update(cx, |ws, cx| ws.flush_save(cx)).ok();
        }));
    }

    /// Writes the editor's buffer to its file now, if it has unsaved edits.
    /// The bytes are the file's own line endings and BOM around the edited
    /// text, so nothing but the edit changes on disk.
    pub fn flush_save(&mut self, cx: &mut Context<Self>) {
        self.save_task = None;
        let Some(state) = self.state.clone() else {
            return;
        };
        let (dirty, name) = {
            let editor = self.editor.read(cx);
            (editor.is_dirty(), editor.name.to_string())
        };
        // Only the open note is written back; a buffer that is not a vault
        // file (the synthetic stress note) never touches the disk.
        if !dirty || self.current.as_deref() != Some(name.as_str()) {
            return;
        }
        let contents = self.editor.read(cx).file_contents();
        let absolute = state.vault().absolute(&name);
        // The vault is the source of truth. If the file is not the bytes
        // this note was loaded from or last saved as, something else wrote
        // it — Obsidian, in the mixed use the phase 3 gate is about — and
        // overwriting it would throw that away. A file that cannot be read
        // at all is the same answer only more so: a note deleted underneath
        // the app must not be recreated here from the buffer it left behind.
        // The buffer keeps its edits and says so instead.
        match std::fs::read_to_string(&absolute) {
            Ok(on_disk) if self.editor.read(cx).disk_unchanged(&on_disk) => {}
            Ok(_) => {
                self.report(
                    format!("{name} changed on disk; not overwritten, your edits are still here"),
                    cx,
                );
                return;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.report(
                    format!("{name} was deleted; not recreated, your edits are still here"),
                    cx,
                );
                return;
            }
            Err(err) => {
                self.report(
                    format!("Could not read {name}: {err}; not overwritten, your edits are kept"),
                    cx,
                );
                return;
            }
        }
        match std::fs::write(&absolute, &contents) {
            Ok(()) => {
                self.editor.update(cx, |e, _| e.mark_saved(&contents));
                self.report(format!("{name} saved"), cx);
            }
            Err(err) => {
                self.report(format!("Could not save {name}: {err}"), cx);
            }
        }
        cx.notify();
    }

    // ----- file operations -------------------------------------------------------

    fn open_prompt(
        &mut self,
        op: PendingOp,
        spec: PromptSpec,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending = Some(op);
        self.close_editor_suggest(cx);
        self.modal = Modal::Prompt;
        self.prompt.update(cx, |p, cx| p.open(spec, window, cx));
        cx.notify();
    }

    /// Shows a message that goes away on its own, the way Obsidian's
    /// Notice does. Everything a command has to say comes through here;
    /// the status bar keeps only what stays true.
    fn report(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = self.notices.push(message.into());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTICE_LIFETIME).await;
            this.update(cx, |ws, cx| {
                ws.notices.expire(id);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Creates `Untitled.md` (or the next free number) in `folder`, opens it
    /// for editing and asks for its name, the way Ctrl+N does in Obsidian.
    fn new_note(&mut self, folder: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let path = state.vault().available_path(folder, "Untitled", "md");
        match state.create_note(&path, "") {
            Ok(changes) => {
                self.after_changes(changes, cx);
                self.open_file(path.clone(), None, cx);
                let mode = self.editing_mode();
                self.set_mode(mode, window, cx);
                self.start_rename(&path, window, cx);
            }
            Err(err) => self.report(err, cx),
        }
    }

    /// Creates the note a link or the switcher names and opens it.
    fn create_note_for(&mut self, name: &str, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let path = state.new_note_path(name, self.current.as_deref());
        match state.create_note(&path, "") {
            Ok(changes) => {
                self.after_changes(changes, cx);
                self.open_file(path.clone(), None, cx);
                if let Some(window) = window {
                    let mode = self.editing_mode();
                    self.set_mode(mode, window, cx);
                }
                self.report(format!("Created {path}"), cx);
            }
            Err(err) => self.report(err, cx),
        }
    }

    fn create_from_link(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.create_note_for(name, Some(window), cx);
    }

    fn start_rename(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let is_folder = self
            .state
            .as_ref()
            .map(|s| s.vault().absolute(path).is_dir())
            .unwrap_or(false);
        let name = path.rsplit('/').next().unwrap_or(path);
        let initial = if is_folder { name } else { basename(name) };
        self.open_prompt(
            PendingOp::Rename(path.to_string()),
            PromptSpec {
                title: if is_folder {
                    "Rename folder".into()
                } else {
                    "Rename file".into()
                },
                detail: Some(path.to_string().into()),
                input: Some(initial.to_string()),
                placeholder: "Name".into(),
                confirm: "Rename".into(),
                ..PromptSpec::default()
            },
            window,
            cx,
        );
    }

    fn start_move(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let is_folder = state.vault().absolute(path).is_dir();
        let own_prefix = format!("{path}/");
        let parent = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let mut suggestions = vec!["/".to_string()];
        suggestions.extend(
            state
                .vault()
                .folders()
                .iter()
                .filter(|f| *f != path && !f.starts_with(&own_prefix) && f.as_str() != parent)
                .cloned(),
        );
        if parent.is_empty() {
            suggestions.remove(0);
        }
        self.open_prompt(
            PendingOp::Move(path.to_string()),
            PromptSpec {
                title: if is_folder {
                    "Move folder to...".into()
                } else {
                    "Move file to...".into()
                },
                detail: Some(path.to_string().into()),
                input: Some(String::new()),
                placeholder: "Type a folder".into(),
                suggestions,
                confirm: "Move".into(),
                ..PromptSpec::default()
            },
            window,
            cx,
        );
    }

    fn start_new_folder(&mut self, parent: &str, window: &mut Window, cx: &mut Context<Self>) {
        let initial = self
            .state
            .as_ref()
            .map(|s| s.vault().available_path(parent, "Untitled", ""))
            .unwrap_or_else(|| "Untitled".to_string());
        let initial = initial.rsplit('/').next().unwrap_or(&initial).to_string();
        self.open_prompt(
            PendingOp::NewFolder {
                parent: parent.to_string(),
            },
            PromptSpec {
                title: "New folder".into(),
                detail: Some(if parent.is_empty() {
                    "In the vault folder".into()
                } else {
                    format!("In {parent}").into()
                }),
                input: Some(initial),
                placeholder: "Folder name".into(),
                confirm: "Create".into(),
                ..PromptSpec::default()
            },
            window,
            cx,
        );
    }

    fn start_delete(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        if !state.confirm_delete() {
            self.delete_now(path, window, cx);
            return;
        }
        let where_to = match state.trash_option() {
            TrashOption::System => "It will be moved to the system trash.",
            TrashOption::Local => "It will be moved to the vault's .trash folder.",
            TrashOption::None => "It will be deleted permanently.",
        };
        let name = path.rsplit('/').next().unwrap_or(path);
        self.open_prompt(
            PendingOp::Delete(path.to_string()),
            PromptSpec {
                title: format!("Delete \"{name}\"?").into(),
                detail: Some(where_to.into()),
                input: None,
                confirm: "Delete".into(),
                ..PromptSpec::default()
            },
            window,
            cx,
        );
    }

    fn delete_now(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let was_current = self.current.as_deref() == Some(path)
            || self
                .current
                .as_deref()
                .is_some_and(|c| c.starts_with(&format!("{path}/")));
        if was_current {
            // Nothing may write the buffer back into a file that is gone,
            // and no pane may go on describing it.
            self.detach_current(false, &state, cx);
            // The main area may stop painting the view that was showing the
            // note, and a focus left on it would sit outside the dispatch
            // tree and swallow every key. The same rule as everywhere else
            // decides where it goes instead: the workspace root when nothing
            // is painted, the detached buffer when one stayed behind.
            self.focus_main(window, cx);
        }
        self.forget_history(path);
        match state.trash(path) {
            Ok(changes) => {
                self.after_changes(changes, cx);
                self.report(format!("Deleted {path}"), cx);
            }
            Err(err) => self.report(err, cx),
        }
    }

    fn duplicate(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        match state.duplicate(path) {
            Ok((copy, changes)) => {
                self.after_changes(changes, cx);
                self.open_file(copy.clone(), None, cx);
                self.report(format!("Copied {path} to {copy}"), cx);
            }
            Err(err) => self.report(err, cx),
        }
    }

    /// Enter in the prompt: finish the pending operation with `value`.
    fn prompt_submitted(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(op) = self.pending.clone() else {
            self.close_modal(window, cx);
            return;
        };
        let value = value.trim();
        match op {
            PendingOp::Rename(from) => {
                let is_folder = self
                    .state
                    .as_ref()
                    .map(|s| s.vault().absolute(&from).is_dir())
                    .unwrap_or(false);
                let folder = from.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                let name = from.rsplit('/').next().unwrap_or(&from);
                let ext = if is_folder {
                    None
                } else {
                    name.rsplit_once('.')
                        .map(|(_, e)| e)
                        .filter(|_| basename(name) != name)
                };
                let new_name = match ext {
                    Some(ext) => format!("{value}.{ext}"),
                    None => value.to_string(),
                };
                let to = if folder.is_empty() {
                    new_name
                } else {
                    format!("{folder}/{new_name}")
                };
                self.rename_with_links(&from, &to, window, cx);
            }
            PendingOp::Move(from) => {
                let folder = value.trim_matches('/');
                let name = from.rsplit('/').next().unwrap_or(&from);
                let to = if folder.is_empty() {
                    name.to_string()
                } else {
                    format!("{folder}/{name}")
                };
                self.rename_with_links(&from, &to, window, cx);
            }
            PendingOp::Delete(path) => {
                self.pending = None;
                self.close_modal(window, cx);
                self.delete_now(&path, window, cx);
            }
            PendingOp::NewFolder { parent } => {
                let path = if parent.is_empty() {
                    value.to_string()
                } else {
                    format!("{parent}/{value}")
                };
                let result = self
                    .state
                    .as_ref()
                    .map(|s| s.create_folder(&path))
                    .unwrap_or(Ok(()));
                match result {
                    Ok(()) => {
                        self.pending = None;
                        self.close_modal(window, cx);
                        self.refresh_vault_views(cx);
                        self.explorer.update(cx, |e, cx| e.reveal(&path, cx));
                        self.report(format!("Created folder {path}"), cx);
                    }
                    Err(err) => self.prompt.update(cx, |p, cx| p.set_error(err, cx)),
                }
            }
            PendingOp::UpdateLinks { from, to } => self.finish_rename(&from, &to, true, window, cx),
        }
    }

    /// The prompt's secondary button: "Don't update" the links.
    fn prompt_secondary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(PendingOp::UpdateLinks { from, to }) = self.pending.clone() {
            self.finish_rename(&from, &to, false, window, cx);
        }
    }

    /// Renames, rewriting links right away or asking first when
    /// `alwaysUpdateLinks` is off and there are links to rewrite.
    fn rename_with_links(
        &mut self,
        from: &str,
        to: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.state.clone() else {
            return;
        };
        if from == to {
            self.pending = None;
            self.close_modal(window, cx);
            return;
        }
        if state.update_links_automatically() {
            self.finish_rename(from, to, true, window, cx);
            return;
        }
        let (files, links) = state.rename_preview(from, to);
        if links == 0 {
            self.finish_rename(from, to, false, window, cx);
            return;
        }
        let name = from.rsplit('/').next().unwrap_or(from);
        self.open_prompt(
            PendingOp::UpdateLinks {
                from: from.to_string(),
                to: to.to_string(),
            },
            PromptSpec {
                title: "Update links?".into(),
                detail: Some(
                    format!(
                        "{links} link{} in {files} file{} point to {name}. Update them to the new name?",
                        if links == 1 { "" } else { "s" },
                        if files == 1 { "" } else { "s" }
                    )
                    .into(),
                ),
                input: None,
                confirm: "Update links".into(),
                secondary: Some("Don't update".into()),
                ..PromptSpec::default()
            },
            window,
            cx,
        );
    }

    fn finish_rename(
        &mut self,
        from: &str,
        to: &str,
        update_links: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.state.clone() else {
            return;
        };
        // The open note's edits must be on disk before its file moves or
        // its links are rewritten.
        self.flush_save(cx);
        match state.rename(from, to, update_links) {
            Ok(outcome) => {
                self.pending = None;
                self.close_modal(window, cx);
                self.apply_rename(outcome, cx);
            }
            Err(err) => self.prompt.update(cx, |p, cx| p.set_error(err, cx)),
        }
    }

    /// Follows the open note and the history to their new paths, then
    /// refreshes everything the way a filesystem event would.
    fn apply_rename(&mut self, outcome: RenameOutcome, cx: &mut Context<Self>) {
        let renamed: HashMap<&str, &str> = outcome
            .renamed
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        for entry in &mut self.history {
            if let Some(new) = renamed.get(entry.as_str()) {
                *entry = new.to_string();
            }
        }
        let active = self.active_tab;
        for (ix, tab) in self.tabs.iter_mut().enumerate() {
            if ix == active {
                continue;
            }
            for entry in tab.history.iter_mut().chain(tab.path.iter_mut()) {
                if let Some(new) = renamed.get(entry.as_str()) {
                    *entry = new.to_string();
                }
            }
        }
        if let Some(current) = self.current.clone() {
            if let Some(new) = renamed.get(current.as_str()) {
                let new = new.to_string();
                self.current = Some(new.clone());
                self.editor.update(cx, |e, _| {
                    if e.name.as_ref() == current {
                        e.set_name(new.clone());
                    }
                });
                self.reading.update(cx, |r, _| {
                    if r.path.as_deref() == Some(current.as_str()) {
                        r.path = Some(new.clone());
                    }
                });
            }
        }
        let summary = match outcome.renamed.as_slice() {
            [(from, to)] => format!("Renamed {from} to {to}"),
            all => format!("Moved {} files", all.len()),
        };
        let links = if outcome.rewritten_links > 0 {
            format!(
                ", updated {} link{} in {} file{}",
                outcome.rewritten_links,
                if outcome.rewritten_links == 1 {
                    ""
                } else {
                    "s"
                },
                outcome.rewritten_files,
                if outcome.rewritten_files == 1 {
                    ""
                } else {
                    "s"
                }
            )
        } else {
            String::new()
        };
        // A note that changed underneath keeps its own bytes; say which.
        let skipped = match outcome.stale.len() {
            0 => String::new(),
            n => format!(
                "; {n} file{} changed on disk and {} left alone",
                if n == 1 { "" } else { "s" },
                if n == 1 { "was" } else { "were" }
            ),
        };
        self.after_changes(outcome.changes, cx);
        if let Some(current) = self.current.clone() {
            self.explorer.update(cx, |e, cx| e.reveal(&current, cx));
        }
        self.report(format!("{summary}{links}{skipped}"), cx);
    }

    /// Renames the open note to `new_name` (same folder, same extension)
    /// right away; `--rename` on the command line, for smoke tests.
    pub fn rename_current(&mut self, new_name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.current.clone() else {
            return;
        };
        let folder = from.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let name = from.rsplit('/').next().unwrap_or(&from);
        let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("md");
        let to = if folder.is_empty() {
            format!("{new_name}.{ext}")
        } else {
            format!("{folder}/{new_name}.{ext}")
        };
        self.finish_rename(&from, &to, true, window, cx);
    }

    /// Opens one of the file prompts for the open note (`--prompt` on the
    /// command line, for screenshots): `rename`, `move`, `delete`, `folder`.
    pub fn open_file_prompt(&mut self, kind: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.current.clone() else {
            return;
        };
        match kind {
            "move" => self.start_move(&current, window, cx),
            "delete" => self.start_delete(&current, window, cx),
            "folder" => self.start_new_folder("", window, cx),
            _ => self.start_rename(&current, window, cx),
        }
    }

    /// Which view the main area paints for the mode and the open file. A
    /// view only gets the main area when it has this file to show: an
    /// editor with no file behind it — one left over from a note that is
    /// gone, or one never given a note, as on launch over a vault with
    /// nothing to open — takes edits nobody saves, and a reading view left
    /// behind keeps a deleted note's links on screen and clickable.
    fn main_view(&self, cx: &App) -> MainView {
        let has_note = self.reading.read(cx).path.is_some();
        let image = self
            .current
            .as_deref()
            .is_some_and(|p| FileKind::from_path(p) == FileKind::Image);
        let has_buffer = !self.editor.read(cx).name.is_empty();
        Self::main_view_for(self.mode, has_note, image, has_buffer)
    }

    /// The rule itself, over what the three views have to show: `has_note`
    /// is the reading view's built note, `image` whether the open file is
    /// one, and `has_buffer` whether the editor's buffer names a file.
    fn main_view_for(mode: ViewMode, has_note: bool, image: bool, has_buffer: bool) -> MainView {
        match mode {
            ViewMode::Graph => MainView::Graph,
            ViewMode::Reading if has_note => MainView::Reading,
            // An image has no source to edit, so it reads the same in the
            // editing modes as it does in the reading view.
            ViewMode::LivePreview | ViewMode::Source if image && has_note => MainView::Reading,
            ViewMode::LivePreview | ViewMode::Source if has_buffer => MainView::Editor,
            _ => MainView::Empty,
        }
    }

    fn render_main(&self, theme: &Theme, cx: &App) -> gpui::AnyElement {
        match self.main_view(cx) {
            MainView::Graph => self.graph.clone().into_any_element(),
            MainView::Reading => self.reading.clone().into_any_element(),
            MainView::Editor => self.render_editor(theme),
            MainView::Empty => self.render_no_file(theme),
        }
    }

    /// The editor, under a line naming the file it lost if its buffer is no
    /// longer backed by one. The user has to be told; the alternative is
    /// typing into a buffer that is never written and never mentioned.
    fn render_editor(&self, theme: &Theme) -> gpui::AnyElement {
        let Some(name) = self.detached.clone() else {
            return self.editor.clone().into_any_element();
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(12.))
                    .bg(theme.bg_alt)
                    .text_color(theme.text_muted)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(format!(
                        "{name} — this buffer no longer belongs to a file in the vault, \
                         and nothing saves it"
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.editor.clone().into_any_element()),
            )
            .into_any_element()
    }

    /// Shown in place of a view that has nothing to show: no open file, or
    /// one whose kind no view can draw.
    fn render_no_file(&self, theme: &Theme) -> gpui::AnyElement {
        let message: SharedString = match self.current.as_deref() {
            Some(path) => format!(
                "{:?} files are not supported yet",
                FileKind::from_path(path)
            )
            .into(),
            None => "No file is open".into(),
        };
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.text_muted)
            .child(message)
            .into_any_element()
    }

    // ----- tabs ------------------------------------------------------------------

    /// Copies the active tab's live fields into its slot.
    fn stash_active_tab(&mut self) {
        let leaf = self.tabs.get(self.active_tab).and_then(|t| t.leaf.clone());
        let tab = Tab {
            path: self.current.clone(),
            mode: self.mode,
            history: self.history.clone(),
            history_ix: self.history_ix,
            leaf,
        };
        match self.tabs.get_mut(self.active_tab) {
            Some(slot) => *slot = tab,
            None => {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Every tab as it stands, the active one from the live fields.
    fn tab_snapshots(&self) -> Vec<Tab> {
        let mut tabs = self.tabs.clone();
        if let Some(slot) = tabs.get_mut(self.active_tab) {
            slot.path = self.current.clone();
            slot.mode = self.mode;
            slot.history = self.history.clone();
            slot.history_ix = self.history_ix;
        }
        tabs
    }

    /// Puts `tabs[active_tab]` on screen: its note in its mode, or the
    /// empty state.
    fn load_tab(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get(self.active_tab).cloned() else {
            return;
        };
        self.history = tab.history;
        self.history_ix = tab.history_ix;
        if tab.mode == ViewMode::Graph && self.mode != ViewMode::Graph {
            self.mode_before_graph = self.mode;
        }
        self.mode = tab.mode;
        if tab.mode.is_editing() {
            self.editor.update(cx, |e, cx| {
                e.set_live_preview(tab.mode == ViewMode::LivePreview, cx)
            });
        }
        match (tab.path, self.state.clone()) {
            (Some(path), Some(state)) => {
                self.current = None;
                self.show_file(&path, &state, cx);
            }
            (None, Some(state)) => self.show_empty(&state, cx),
            (path, None) => self.current = path,
        }
        self.request_focus_main(cx);
        cx.notify();
    }

    /// The empty tab: no note in the main area and nothing describing one.
    /// A buffer with unsaved edits nowhere else stays, marked as saved
    /// nowhere, the way a file that went away leaves it.
    fn show_empty(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        self.release_buffer(cx);
        self.reading.update(cx, |r, cx| {
            r.set_note(
                None,
                reading::build_with("", "", state.as_ref(), state.build_options()),
                cx,
            )
        });
        self.current = None;
        self.explorer.update(cx, |e, cx| e.clear_active(cx));
        self.graph.update(cx, |g, cx| g.set_current(None, cx));
        self.clear_panes(cx);
        self.status = SharedString::default();
    }

    /// `workspace:new-tab`: an empty tab to the right of the active one,
    /// in the mode "Default view for new tabs" names.
    pub fn new_tab(&mut self, cx: &mut Context<Self>) {
        self.flush_save(cx);
        self.stash_active_tab();
        let mode = self.default_mode();
        let at = (self.active_tab + 1).min(self.tabs.len());
        self.tabs.insert(at, Tab::empty(mode));
        self.active_tab = at;
        self.load_tab(cx);
    }

    /// Opens `path` in a new tab, leaving the active one as it is.
    pub fn open_in_new_tab(
        &mut self,
        path: String,
        subpath: Option<Subpath<'_>>,
        cx: &mut Context<Self>,
    ) {
        self.new_tab(cx);
        self.open_file(path, subpath, cx);
    }

    pub fn activate_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix >= self.tabs.len() || ix == self.active_tab {
            return;
        }
        self.flush_save(cx);
        self.stash_active_tab();
        self.active_tab = ix;
        self.load_tab(cx);
    }

    /// `workspace:next-tab` and `previous-tab`: `step` tabs along, wrapping.
    fn cycle_tab(&mut self, step: usize, cx: &mut Context<Self>) {
        let n = self.tabs.len();
        if n > 1 {
            self.activate_tab((self.active_tab + step) % n, cx);
        }
    }

    /// `workspace:close`: the last close leaves the empty tab, as Obsidian's
    /// does; the tab to the right takes over, or the one to the left at the
    /// end of the strip.
    pub fn close_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.flush_save(cx);
        self.stash_active_tab();
        if ix >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(ix);
        self.remember_closed(ix, tab);
        if self.tabs.is_empty() {
            self.tabs.push(Tab::empty(self.default_mode()));
            self.active_tab = 0;
            self.load_tab(cx);
        } else if ix == self.active_tab {
            self.active_tab = ix.min(self.tabs.len() - 1);
            self.load_tab(cx);
        } else {
            if ix < self.active_tab {
                self.active_tab -= 1;
            }
            cx.notify();
        }
    }

    fn remember_closed(&mut self, ix: usize, tab: Tab) {
        // An empty tab is not worth bringing back.
        if tab.path.is_some() || tab.mode == ViewMode::Graph {
            self.closed_tabs.push((ix, tab));
            if self.closed_tabs.len() > CLOSED_TABS_KEPT {
                self.closed_tabs.remove(0);
            }
        }
    }

    /// `workspace:close-others`: every tab but the active one.
    fn close_other_tabs(&mut self, cx: &mut Context<Self>) {
        self.stash_active_tab();
        let keep = self.tabs.remove(self.active_tab);
        let others = std::mem::take(&mut self.tabs);
        for (ix, tab) in others.into_iter().enumerate() {
            self.remember_closed(ix, tab);
        }
        self.tabs.push(keep);
        self.active_tab = 0;
        cx.notify();
    }

    /// `workspace:undo-close-pane`: the most recently closed tab, back
    /// where it was.
    fn undo_close_tab(&mut self, cx: &mut Context<Self>) {
        let Some((ix, tab)) = self.closed_tabs.pop() else {
            return;
        };
        self.flush_save(cx);
        self.stash_active_tab();
        let at = ix.min(self.tabs.len());
        self.tabs.insert(at, tab);
        self.active_tab = at;
        self.load_tab(cx);
    }

    /// A sidebar's collapse toggle: it stays on screen when the sidebar is
    /// collapsed, which is what brings the sidebar back.
    fn sidebar_toggle(
        &self,
        left: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let visible = if left {
            self.left_visible
        } else {
            self.right_visible
        };
        let command = if left {
            "app:toggle-left-sidebar"
        } else {
            "app:toggle-right-sidebar"
        };
        let glyph = if left == visible { "⇤" } else { "⇥" };
        div()
            .id(if left { "toggle-left" } else { "toggle-right" })
            .flex_shrink_0()
            .w(px(26.))
            .h(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(14.))
            .text_color(theme.text_muted)
            .hover(|s| s.bg(theme.bg_alt).text_color(theme.text))
            .on_click(cx.listener(move |this, _, window, cx| this.run_command(command, window, cx)))
            .child(glyph)
    }

    /// The ribbon down the left edge: the quick switcher, the graph, today's
    /// daily note, a template and the palette at the top, Settings at the
    /// bottom. `workspace.json`'s `left-ribbon.hiddenItems` hides an entry
    /// by the plugin id its key starts with.
    fn render_ribbon(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let hidden = |plugin: &str| -> bool {
            self.layout
                .as_ref()
                .and_then(|l| l.get("left-ribbon")?.get("hiddenItems")?.as_object())
                .is_some_and(|items| {
                    items.iter().any(|(key, hidden)| {
                        key.split(':').next() == Some(plugin) && hidden.as_bool() == Some(true)
                    })
                })
        };
        let enabled = |plugin: &str| -> bool {
            self.state
                .as_ref()
                .is_some_and(|s| settings::plugin_enabled(s, plugin))
        };
        let toggle = self.sidebar_toggle(true, theme, cx);
        let button = |id: &'static str, glyph: &'static str, command: &'static str| {
            div()
                .id(id)
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .cursor_pointer()
                .text_size(px(16.))
                .text_color(theme.text_muted)
                .hover(|s| s.bg(theme.bg_alt).text_color(theme.text))
                .on_click(
                    cx.listener(move |this, _, window, cx| this.run_command(command, window, cx)),
                )
                .child(glyph)
        };
        let items: [(&'static str, &'static str, &'static str, &'static str); 5] = [
            ("ribbon-switcher", "⚲", "switcher:open", "switcher"),
            ("ribbon-graph", "◎", "graph:open", "graph"),
            ("ribbon-daily", "▦", "daily-notes", "daily-notes"),
            ("ribbon-template", "≡", "insert-template", "templates"),
            (
                "ribbon-palette",
                "⌨",
                "command-palette:open",
                "command-palette",
            ),
        ];
        let buttons: Vec<_> = items
            .into_iter()
            .filter(|(_, _, _, plugin)| enabled(plugin) && !hidden(plugin))
            .map(|(id, glyph, command, _)| button(id, glyph, command))
            .collect();
        let settings = button("ribbon-settings", "⚙", "app:open-settings");
        div()
            .flex()
            .flex_col()
            .items_center()
            .flex_shrink_0()
            .w(px(44.))
            .h_full()
            .py_1()
            .gap_1()
            .bg(theme.bg_secondary)
            .border_r_1()
            .border_color(theme.border)
            .child(toggle)
            .child(div().h(px(6.)))
            .children(buttons)
            .child(div().flex_1())
            .child(settings)
    }

    fn render_tab_strip(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let tabs = self.tab_snapshots();
        let active_ix = self.active_tab;
        let show_ribbon = self
            .state
            .as_ref()
            .map(|s| s.appearance().show_ribbon.unwrap_or(true))
            .unwrap_or(true);
        div()
            .flex()
            .flex_row()
            .items_end()
            .h(px(34.))
            .pt(px(4.))
            .px_1()
            .gap(px(2.))
            .bg(theme.bg_secondary)
            .border_b_1()
            .border_color(theme.border)
            .overflow_hidden()
            // Without the ribbon a collapsed left sidebar would have no
            // way back from the window.
            .when(!show_ribbon && !self.left_visible, |d| {
                d.child(self.sidebar_toggle(true, theme, cx).mb(px(2.)))
            })
            .children(tabs.iter().enumerate().map(|(ix, tab)| {
                let active = ix == active_ix;
                let title = tab.title();
                div()
                    .id(("tab", ix))
                    .group("tab")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .h(px(30.))
                    .min_w(px(60.))
                    .max_w(px(200.))
                    .pl_3()
                    .pr_1()
                    .rounded_t_md()
                    .cursor_pointer()
                    .text_size(px(12.))
                    .bg(if active { theme.bg } else { theme.bg_secondary })
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .when(!active, |d| d.hover(|s| s.bg(theme.bg_alt)))
                    .on_click(cx.listener(move |this, _, _, cx| this.activate_tab(ix, cx)))
                    .on_mouse_down(
                        gpui::MouseButton::Middle,
                        cx.listener(move |this, _, _, cx| this.close_tab(ix, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(title),
                    )
                    .child(
                        div()
                            .id(("tab-close", ix))
                            .px_1()
                            .rounded_sm()
                            .text_color(theme.text_muted)
                            .when(!active, |d| {
                                d.invisible().group_hover("tab", |s| s.visible())
                            })
                            .hover(|s| s.bg(theme.bg_active).text_color(theme.text))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_tab(ix, cx);
                            }))
                            .child("×"),
                    )
            }))
            .child(
                div()
                    .id("new-tab")
                    .mb(px(3.))
                    .px_2()
                    .py_px()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_size(px(14.))
                    .text_color(theme.text_muted)
                    .hover(|s| s.bg(theme.bg_alt).text_color(theme.text))
                    .on_click(cx.listener(|this, _, _, cx| this.new_tab(cx)))
                    .child("+"),
            )
            .child(div().flex_1())
            .child(self.sidebar_toggle(false, theme, cx).mb(px(2.)))
    }

    // ----- workspace.json --------------------------------------------------------

    /// Whether a `workspace.json` was read at load. The startup flags then
    /// leave the first note alone: with nothing recorded Obsidian shows the
    /// empty state rather than picking a note.
    pub fn layout_restored(&self) -> bool {
        self.layout_restored
    }

    /// Restores the layout `workspace.json` records: the main area's tabs
    /// and the active one, each sidebar's collapse state and its tab.
    /// Tab groups are flattened into the one strip  has.
    fn restore_layout(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let layout = match state.vault().workspace_json() {
            Ok(Some(layout)) => layout,
            Ok(None) => return,
            Err(err) => {
                self.report(format!("workspace.json could not be read: {err}"), cx);
                return;
            }
        };
        self.layout_restored = true;
        self.last_open_files = layout
            .get("lastOpenFiles")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(main) = layout.get("main") {
            let default_mode = self.default_mode();
            let mut leaves = Vec::new();
            collect_leaves(main, &mut leaves);
            let mut tabs: Vec<(String, Tab)> = Vec::new();
            for leaf in leaves {
                let Some(id) = leaf.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let view = leaf
                    .get("state")
                    .and_then(|s| s.get("type"))
                    .and_then(Value::as_str);
                let inner = leaf.get("state").and_then(|s| s.get("state"));
                let file = inner.and_then(|s| s.get("file")).and_then(Value::as_str);
                let tab = match (view, file) {
                    (_, Some(file)) => {
                        // A leaf naming a file the vault no longer has is
                        // dropped, the way Obsidian drops one it cannot open.
                        let known = state.vault().absolute(file).is_file();
                        if !known {
                            continue;
                        }
                        let mode = match inner.and_then(|s| s.get("mode")).and_then(Value::as_str) {
                            Some("preview") => ViewMode::Reading,
                            Some("source")
                                if inner
                                    .and_then(|s| s.get("source"))
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false) =>
                            {
                                ViewMode::Source
                            }
                            Some("source") => ViewMode::LivePreview,
                            _ => default_mode,
                        };
                        Tab {
                            path: Some(file.to_string()),
                            mode,
                            history: vec![file.to_string()],
                            history_ix: 1,
                            leaf: Some(leaf.clone()),
                        }
                    }
                    (Some("empty"), None) => Tab {
                        leaf: Some(leaf.clone()),
                        ..Tab::empty(default_mode)
                    },
                    (Some("graph"), None) => Tab {
                        leaf: Some(leaf.clone()),
                        ..Tab::empty(ViewMode::Graph)
                    },
                    _ => continue,
                };
                tabs.push((id.to_string(), tab));
            }
            if !tabs.is_empty() {
                let active_id = layout.get("active").and_then(Value::as_str);
                let active = tabs
                    .iter()
                    .position(|(id, _)| Some(id.as_str()) == active_id)
                    .or_else(|| tabs.iter().position(|(_, t)| t.path.is_some()))
                    .unwrap_or(0);
                self.tabs = tabs.into_iter().map(|(_, t)| t).collect();
                self.active_tab = active;
                self.load_tab(cx);
            }
        }
        let collapsed = |dock: &str| {
            layout
                .get(dock)
                .and_then(|d| d.get("collapsed"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        self.left_visible = !collapsed("left");
        self.right_visible = !collapsed("right");
        match dock_current_view(&layout, "left").as_deref() {
            Some("search") => self.left_tab = LeftTab::Search,
            Some(_) => self.left_tab = LeftTab::Files,
            None => {}
        }
        match dock_current_view(&layout, "right").as_deref() {
            Some("backlink") => self.right_tab = RightTab::Backlinks,
            Some("outgoing-link") => self.right_tab = RightTab::Outgoing,
            Some("outline") => self.right_tab = RightTab::Outline,
            Some("tag") => self.right_tab = RightTab::Tags,
            _ => {}
        }
        self.layout = Some(layout);
    }

    /// `workspace.json` as this session would leave it: the file that was
    /// read with the tabs, the active leaf, the sidebars' collapse state
    /// and `lastOpenFiles` brought up to date, and every other key as it
    /// was. A main area with more than one tab group is left alone — 
    /// flattened it to show it, and writing that back would be a loss.
    fn layout_json(&self) -> Value {
        let mut layout = self.layout.clone().unwrap_or_else(|| json!({}));
        let Some(object) = layout.as_object_mut() else {
            return layout;
        };
        let tabs = self.tab_snapshots();
        let leaves: Vec<Value> = tabs.iter().map(leaf_json).collect();
        let active_id = leaves
            .get(self.active_tab)
            .and_then(|l| l.get("id"))
            .cloned()
            .unwrap_or(Value::Null);
        let groups = object.get("main").map(count_tab_groups).unwrap_or(0);
        if groups <= 1 {
            let main = object.entry("main").or_insert_with(|| {
                json!({
                    "id": leaf_id(),
                    "type": "split",
                    "children": [{"id": leaf_id(), "type": "tabs", "children": []}],
                    "direction": "vertical",
                })
            });
            if let Some(group) = find_tab_group(main) {
                group.insert("children".into(), Value::Array(leaves));
                if self.active_tab > 0 {
                    group.insert("currentTab".into(), json!(self.active_tab));
                } else {
                    group.shift_remove("currentTab");
                }
            }
            object.insert("active".into(), active_id);
        }
        for (dock, visible) in [("left", self.left_visible), ("right", self.right_visible)] {
            if let Some(d) = object.get_mut(dock).and_then(Value::as_object_mut) {
                if visible {
                    d.shift_remove("collapsed");
                } else {
                    d.insert("collapsed".into(), json!(true));
                }
            }
        }
        object.insert("lastOpenFiles".into(), json!(self.last_open_files));
        layout
    }

    /// Writes `workspace.json` back if the session changed the layout.
    /// Called on the way out, where Obsidian writes it.
    pub fn save_layout(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let fresh = self.layout_json();
        if self.layout.as_ref() == Some(&fresh) {
            return;
        }
        if let Err(err) = state.vault().write_config("workspace.json", &fresh) {
            self.report(format!("workspace.json could not be written: {err}"), cx);
            return;
        }
        self.layout = Some(fresh);
    }

    /// Everything that has to reach the disk before the app goes away: the
    /// note's unsaved edits and the layout.
    pub fn on_quit(&mut self, cx: &mut Context<Self>) {
        self.flush_save(cx);
        self.save_layout(cx);
    }

    // ----- the control socket ----------------------------------------------------

    /// The whole shell as JSON, for the control socket's `state` method: a
    /// test driving the app over the socket reads this instead of a
    /// screenshot. Every key is always there, `null` when there is nothing
    /// to say, so a caller can index into it without guessing. Each view
    /// describes itself; the rest is the shell's own.
    pub fn describe(&self, text: bool, rows: usize, window: &Window, cx: &App) -> Value {
        let mut vault = Map::new();
        vault.insert(
            "root".into(),
            json!(self.state.as_ref().map(|s| s.root.display().to_string())),
        );
        vault.insert("name".into(), json!(self.vault_name.to_string()));
        vault.insert("loaded".into(), json!(self.state.is_some()));
        vault.insert(
            "loading".into(),
            json!(self.loading.as_ref().map(|l| l.to_string())),
        );

        let mut file = Map::new();
        file.insert("path".into(), json!(self.current));
        file.insert(
            "detached".into(),
            json!(self.detached.as_ref().map(|n| n.to_string())),
        );
        file.insert("mode".into(), json!(self.mode.as_str()));
        file.insert("dirty".into(), json!(self.editor.read(cx).is_dirty()));
        put_capped(
            &mut file,
            "history",
            self.history.iter().map(|p| json!(p)).collect(),
            rows,
        );
        file.insert("history_ix".into(), json!(self.history_ix));

        let tabs = json!({
            "active": self.active_tab,
            "items": self
                .tab_snapshots()
                .iter()
                .map(|t| json!({"path": t.path, "mode": t.mode.as_str(), "title": t.title()}))
                .collect::<Vec<Value>>(),
        });

        let mut left = Map::new();
        left.insert("visible".into(), json!(self.left_visible));
        left.insert(
            "tab".into(),
            json!(match self.left_tab {
                LeftTab::Files => "files",
                LeftTab::Search => "search",
            }),
        );
        left.insert("explorer".into(), self.explorer.read(cx).describe(rows));
        left.insert("search".into(), self.search.read(cx).describe(rows, cx));

        let mut right = Map::new();
        right.insert("visible".into(), json!(self.right_visible));
        right.insert(
            "tab".into(),
            json!(match self.right_tab {
                RightTab::Backlinks => "backlinks",
                RightTab::Outgoing => "outgoing",
                RightTab::Outline => "outline",
                RightTab::Tags => "tags",
            }),
        );
        put_capped(
            &mut right,
            "outline",
            self.outline.iter().map(outline_json).collect(),
            rows,
        );
        put_capped(
            &mut right,
            "backlinks",
            self.backlink_rows.iter().map(pane_row_json).collect(),
            rows,
        );
        put_capped(
            &mut right,
            "outgoing",
            self.outgoing_rows.iter().map(pane_row_json).collect(),
            rows,
        );
        put_capped(
            &mut right,
            "tags",
            self.tags.iter().map(tag_json).collect(),
            rows,
        );

        // Whether the graph is on screen is the shell's to say; the view
        // itself only knows what it would draw.
        let mut graph = self.graph.read(cx).describe();
        if let Some(object) = graph.as_object_mut() {
            object.insert("open".into(), json!(matches!(self.mode, ViewMode::Graph)));
        }

        json!({
            "vault": Value::Object(vault),
            "file": Value::Object(file),
            "tabs": tabs,
            "docks": {"left": Value::Object(left), "right": Value::Object(right)},
            "modal": self.describe_modal(rows, cx),
            "editor": self.editor.read(cx).describe(text, cx),
            "reading": self.reading.read(cx).describe(),
            "graph": graph,
            "status": self.status_text(),
            "notices": self.notices.texts(),
            "focus": {
                "name": self.focus_name(window, cx),
                "context_stack": context_names(window),
            },
            "window": {
                "width": window.viewport_size().width.as_f32(),
                "height": window.viewport_size().height.as_f32(),
                "scale": window.scale_factor(),
                "active": window.is_window_active(),
            },
        })
    }

    /// The overlay that is up, if any: its kind plus whatever the modal
    /// itself says about its query and rows.
    fn describe_modal(&self, rows: usize, cx: &App) -> Value {
        let mut modal = Map::new();
        modal.insert(
            "kind".into(),
            json!(match self.modal {
                Modal::None => "none",
                Modal::Switcher => "switcher",
                Modal::Palette => "palette",
                Modal::Settings => "settings",
                Modal::Prompt => "prompt",
                Modal::Chooser => "chooser",
            }),
        );
        // The four list modals fill these in; with no modal up they stay
        // empty rather than disappearing.
        modal.insert("query".into(), Value::Null);
        modal.insert("selected".into(), Value::Null);
        modal.insert("error".into(), Value::Null);
        modal.insert("rows".into(), Value::Array(Vec::new()));
        let view = match self.modal {
            Modal::Switcher => Some(self.switcher.read(cx).describe(rows, cx)),
            Modal::Palette => Some(self.palette.read(cx).describe(rows, cx)),
            Modal::Prompt => Some(self.prompt.read(cx).describe(rows, cx)),
            Modal::Chooser => Some(self.chooser.read(cx).describe(rows, cx)),
            Modal::Settings => Some(self.settings.read(cx).describe(cx)),
            Modal::None => None,
        };
        if let Some(Value::Object(view)) = view {
            for (key, value) in view {
                modal.insert(key, value);
            }
        }
        // Settings has tabs rather than rows; the tab it is on is the part
        // a caller steers by.
        modal.insert(
            "settings_tab".into(),
            modal.get("tab").cloned().unwrap_or(Value::Null),
        );
        modal.insert(
            "pending_op".into(),
            match &self.pending {
                None => Value::Null,
                Some(PendingOp::Rename(path)) => json!({"op": "rename", "path": path}),
                Some(PendingOp::Move(path)) => json!({"op": "move", "path": path}),
                Some(PendingOp::Delete(path)) => json!({"op": "delete", "path": path}),
                Some(PendingOp::NewFolder { parent }) => {
                    json!({"op": "new-folder", "parent": parent})
                }
                Some(PendingOp::UpdateLinks { from, to }) => {
                    json!({"op": "update-links", "from": from, "to": to})
                }
            },
        );
        Value::Object(modal)
    }

    /// Where typed text would land: "editor", "input" or "none". Deciding
    /// is separate from doing because only one of the two routes may run
    /// with this workspace borrowed: delivering a keystroke to a focused
    /// input re-enters the focused view, and that view can be this one.
    pub fn type_target(&self, window: &Window, cx: &App) -> &'static str {
        Self::typed_text_target(self.focus_name(window, cx), self.main_view(cx))
    }

    /// Which of the two routes `text` takes, for the focus the shell
    /// reports and the view the main area is painting.
    ///
    /// The editor is answered only while it is on screen. Its handle can
    /// keep the focus after the main area has stopped painting it — the
    /// keyboard reaches nothing from there, but this route does not go
    /// through the dispatch tree, and would otherwise fill a buffer that
    /// belongs to no file with text nothing ever saves.
    fn typed_text_target(focus: &str, painted: MainView) -> &'static str {
        match focus {
            // A property row or a table cell is the editor's own field.
            "editor" | "property-field" | "cell-field" => match painted {
                MainView::Editor => "editor",
                _ => "none",
            },
            "workspace" | "reading" | "graph" | "none" => "none",
            _ => "input",
        }
    }

    /// Types `text` into the editor, for the socket's `type` method, and
    /// says so. Only the editor is reachable from here: the inputs belong
    /// to the views that own them and are typed into from outside this
    /// borrow, so a caller asks [`Workspace::type_target`] first and
    /// anything but "editor" is answered the other way.
    pub fn type_into_focused(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> &'static str {
        match self.type_target(window, cx) {
            "editor" => {
                // `type_text` already routes to an open property row or
                // table cell rather than the body.
                self.editor
                    .update(cx, |e, cx| e.type_text(text, window, cx));
                "editor"
            }
            // Nothing else is reachable from here, so nothing else is
            // typed into: a focused input is answered by the caller, which
            // asked where the text would go before handing it over.
            _ => "none",
        }
    }

    /// A name for whatever has focus, for the state dump and to decide
    /// where typed text goes. Only handles this shell can reach are named
    /// by handle; the find panel's two fields belong to the editor, which
    /// is asked, and the Settings fields are named from the dispatch path.
    /// `explorer` never comes back: the file tree's rows are painted
    /// without a focus handle, so the keyboard is never in it.
    pub fn focus_name(&self, window: &Window, cx: &App) -> &'static str {
        if window.focused(cx).is_none() {
            return "none";
        }
        if self.editor.read(cx).focus_handle_ref().is_focused(window) {
            // An open property row or table cell takes the editor's keys
            // without taking the focus.
            return match self.editor.read(cx).field_kind() {
                Some("cell") => "cell-field",
                Some(_) => "property-field",
                None => "editor",
            };
        }
        if self.reading.focus_handle(cx).is_focused(window) {
            return "reading";
        }
        if self.graph.focus_handle(cx).is_focused(window) {
            return "graph";
        }
        if self.switcher.focus_handle(cx).is_focused(window) {
            return "switcher-input";
        }
        if self.palette.focus_handle(cx).is_focused(window) {
            return "palette-input";
        }
        if self.prompt.focus_handle(cx).is_focused(window) {
            return "prompt-input";
        }
        if self.chooser.focus_handle(cx).is_focused(window) {
            return "chooser-input";
        }
        if self.search.focus_handle(cx).is_focused(window) {
            return "search-input";
        }
        if self.settings.focus_handle(cx).is_focused(window) {
            return "settings";
        }
        if self.focus_handle.is_focused(window) {
            return "workspace";
        }
        // Both find fields sit in one key context, so the stack cannot tell
        // them apart; the editor holds the two handles and can.
        if let Some(name) = self.editor.read(cx).find_focus(window, cx) {
            return name;
        }
        let stack = context_names(window);
        if stack.iter().any(|c| c == "Settings") {
            return "settings";
        }
        "unknown"
    }

    /// Focuses one of the main views by name, for the socket's `focus`
    /// method. The mode is left alone: focus is all that is asked for.
    pub fn focus_view(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        match Self::focus_target(name, self.main_view(cx))? {
            // The root div paints in every frame, so its handle is always
            // somewhere the keyboard can sit.
            None => window.focus(&self.focus_handle, cx),
            Some(_) => self.focus_main(window, cx),
        }
        cx.notify();
        Ok(())
    }

    /// Which view the socket's `focus` names: `None` for the workspace
    /// root, or the main view asked for once it is the one being painted.
    ///
    /// A view the main area is not painting is refused rather than focused.
    /// Its handle is in no frame's dispatch tree, so the keyboard parked on
    /// it reaches nothing at all — not even the hotkey that would undo it,
    /// which is the dead state the mode switches were hardened against.
    fn focus_target(name: &str, painted: MainView) -> Result<Option<MainView>, String> {
        let wanted = match name {
            "workspace" => return Ok(None),
            "editor" => MainView::Editor,
            "reading" => MainView::Reading,
            "graph" => MainView::Graph,
            other => return Err(format!("unknown view: {other}")),
        };
        if wanted != painted {
            return Err(format!(
                "the main area is painting {}, not {name}: \
                 switch the mode first, or focus the view that is on screen",
                painted.as_str()
            ));
        }
        Ok(Some(wanted))
    }

    /// Empties the status bar and takes down every notice, so a caller
    /// can tell the next notice from the one already there.
    pub fn clear_status(&mut self, cx: &mut Context<Self>) {
        self.notices.clear();
        self.status = "".into();
        cx.notify();
    }

    /// What the last command said, which is how every command reports:
    /// the newest notice while one is up, the status bar otherwise.
    pub fn status_text(&self) -> String {
        match self.notices.latest() {
            Some(text) => text.to_string(),
            None => self.status.to_string(),
        }
    }

    /// The open vault's folder, if one is open.
    pub fn vault_root(&self) -> Option<PathBuf> {
        self.state.as_ref().map(|s| s.root.clone())
    }

    /// An absolute path for a vault-relative one, refusing anything that
    /// points outside the vault. This is the only path the socket reads and
    /// writes through, and the vault folder is the only thing  may
    /// touch.
    pub fn vault_path(&self, path: &str) -> Result<PathBuf, String> {
        let root = self
            .vault_root()
            .ok_or_else(|| "No vault is open".to_string())?;
        vault_path_in(&root, path)
    }

    /// The vault's own spelling of a vault-relative path, if the index
    /// carries one: that spelling is the key every pane is looked up by, so
    /// the caller's casing is only a request for it.
    pub fn vault_file_path(&self, path: &str) -> Option<String> {
        let state = self.state.as_ref()?;
        state
            .vault()
            .files()
            .iter()
            .find(|f| f.path.eq_ignore_ascii_case(path))
            .map(|f| f.path.clone())
    }

    /// The open vault's files, for the socket's `list_files`: the same list
    /// the explorer is built from, under `folder` when one is given.
    pub fn list_vault_files(&self, folder: Option<&str>) -> Vec<Value> {
        let Some(state) = self.state.as_ref() else {
            return Vec::new();
        };
        // Vault paths keep their case and compare without it, so a folder
        // stored as `Sub` answers to `sub` as well.
        let prefix = folder
            .map(|f| f.trim_matches('/'))
            .filter(|f| !f.is_empty())
            .map(|f| format!("{}/", f.to_lowercase()));
        state
            .vault()
            .files()
            .iter()
            .filter(|f| {
                prefix
                    .as_deref()
                    .is_none_or(|p| f.path.to_lowercase().starts_with(p))
            })
            .map(|f| json!({"path": f.path, "kind": kind_name(f.kind), "size": f.size}))
            .collect()
    }

    /// Every command in the registry with the hotkeys `hotkeys.json` leaves
    /// it, and whether [`Workspace::run_command`] can run it today.
    pub fn list_commands(&self, cx: &App) -> Vec<Value> {
        cx.global::<Commands>()
            .0
            .iter()
            .map(|c| {
                let hotkeys: Vec<String> = c.hotkeys.iter().filter_map(keystroke).collect();
                json!({
                    "id": c.id,
                    "name": c.name,
                    "hotkeys": hotkeys,
                    "runnable": RUNNABLE.contains(&c.id.as_str()),
                })
            })
            .collect()
    }

    /// The mode a socket request names, if it names one at all.
    pub fn open_mode(&self, mode: Option<&str>) -> Result<Option<ViewMode>, String> {
        match mode {
            None => Ok(None),
            Some(name) => ViewMode::parse(name)
                .map(Some)
                .ok_or_else(|| format!("unknown mode: {name}")),
        }
    }

    // ----- rendering -------------------------------------------------------------

    fn dock_tab(
        &self,
        label: &'static str,
        active: bool,
        theme: &Theme,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_2()
            .py_1()
            .text_size(px(11.))
            .cursor_pointer()
            .rounded_sm()
            .text_color(if active { theme.text } else { theme.text_muted })
            .when(active, |d| d.bg(theme.bg_active))
            .hover(|s| s.bg(theme.bg_alt))
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
            .child(label.to_uppercase())
    }

    fn render_outline(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.outline.get(ix)?.clone();
                let offset = row.offset;
                Some(
                    div()
                        .id(("outline", ix))
                        .w_full()
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .pl(px(8. + (row.level.saturating_sub(1)) as f32 * 12.))
                        .pr_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_size(px(13.))
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .overflow_hidden()
                        .hover(|s| s.bg(theme.bg_alt))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.reveal_offset(offset, cx);
                        }))
                        .child(row.text),
                )
            })
            .collect()
    }

    fn pane_row(
        &self,
        id: (&'static str, usize),
        row: PaneRow,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        match row {
            PaneRow::Header(text) => div()
                .id(id)
                .w_full()
                .h(px(40.))
                .flex()
                .items_end()
                .pb_1()
                .px_2()
                .text_size(px(10.))
                .text_color(theme.text_muted)
                .child(text.to_uppercase()),
            PaneRow::Item {
                path,
                label,
                detail,
                offset,
            } => {
                let clickable = path.is_some();
                div()
                    .id(id)
                    .w_full()
                    .h(px(40.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .px_2()
                    .rounded_sm()
                    .when(clickable, |d| {
                        d.cursor_pointer()
                            .hover(|s| s.bg(theme.bg_alt))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(path) = path.clone() {
                                    this.open_file(path, None, cx);
                                    if let Some(offset) = offset {
                                        this.reveal_offset(offset, cx);
                                    }
                                }
                            }))
                    })
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(if clickable {
                                theme.text
                            } else {
                                theme.text_muted
                            })
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(detail),
                    )
            }
        }
    }

    fn render_backlinks(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.backlink_rows.get(ix)?.clone();
                Some(self.pane_row(("backlink", ix), row, &theme, cx))
            })
            .collect()
    }

    fn render_outgoing(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.outgoing_rows.get(ix)?.clone();
                Some(self.pane_row(("outgoing", ix), row, &theme, cx))
            })
            .collect()
    }

    fn render_tags(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.tags.get(ix)?.clone();
                let name = row.name.clone();
                Some(
                    div()
                        .id(("tag", ix))
                        .w_full()
                        .h(px(24.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_size(px(13.))
                        .hover(|s| s.bg(theme.bg_alt))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.show_search(&format!("tag:#{name}"), None, cx);
                        }))
                        .child(
                            div()
                                .text_color(theme.tag)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .overflow_hidden()
                                .child(SharedString::from(format!("#{}", row.name))),
                        )
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_size(px(11.))
                                .child(SharedString::from(row.count.to_string())),
                        ),
                )
            })
            .collect()
    }

    fn render_right_dock(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.right_tab;
        let body: gpui::AnyElement = match tab {
            RightTab::Backlinks => {
                let count = self.backlink_rows.len();
                uniform_list("backlinks", count, cx.processor(Self::render_backlinks))
                    .size_full()
                    .px_1()
                    .into_any_element()
            }
            RightTab::Outgoing => {
                let count = self.outgoing_rows.len();
                uniform_list("outgoing", count, cx.processor(Self::render_outgoing))
                    .size_full()
                    .px_1()
                    .into_any_element()
            }
            RightTab::Outline => {
                let count = self.outline.len();
                if count == 0 {
                    empty_pane("No headings.", theme)
                } else {
                    uniform_list("outline", count, cx.processor(Self::render_outline))
                        .size_full()
                        .px_1()
                        .into_any_element()
                }
            }
            RightTab::Tags => {
                let count = self.tags.len();
                if count == 0 {
                    empty_pane("No tags.", theme)
                } else {
                    uniform_list("tags", count, cx.processor(Self::render_tags))
                        .track_scroll(&self.tag_scroll)
                        .size_full()
                        .px_1()
                        .into_any_element()
                }
            }
        };
        let tab_button =
            |this: &mut Self, label: &'static str, which: RightTab, cx: &mut Context<Self>| {
                this.dock_tab(
                    label,
                    tab == which,
                    theme,
                    move |this, _, cx| this.select_right_tab(which, cx),
                    cx,
                )
            };
        div()
            .flex()
            .flex_col()
            .w(px(280.))
            .h_full()
            .bg(theme.bg_secondary)
            .border_l_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(tab_button(self, "Backlinks", RightTab::Backlinks, cx))
                    .child(tab_button(self, "Outgoing", RightTab::Outgoing, cx))
                    .child(tab_button(self, "Outline", RightTab::Outline, cx))
                    .child(tab_button(self, "Tags", RightTab::Tags, cx)),
            )
            .child(div().flex_1().overflow_hidden().child(body))
    }
}

/// Unlinked mentions for `path`: other notes mentioning its names (incoming)
/// and names of other notes mentioned in it (outgoing).
fn mention_rows(state: &VaultState, path: &str, finder: Option<&Finder>) -> MentionRows {
    const MAX: usize = 200;
    let texts = state.texts();
    let cache = state.cache();
    let base = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
        .to_string();
    let mut own: Vec<String> = vec![base];
    if let Some(meta) = cache.file(path) {
        own.extend(frontmatter_aliases(meta));
    }
    let own_refs: Vec<Name<'_>> = own.iter().map(|t| Name { text: t, path }).collect();
    let own_finder = Finder::new(&own_refs);

    let mut incoming: Vec<PaneRow> = Vec::new();
    if let Some(f) = &own_finder {
        let mut found: Vec<(String, usize, String)> = texts
            .par_iter()
            .filter(|(p, _)| p.as_str() != path)
            .flat_map_iter(|(p, text)| {
                f.find(text)
                    .into_iter()
                    .map(|m| {
                        let line = text[m.line_offset..]
                            .lines()
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        (p.clone(), m.line_offset, line)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        found.sort();
        for (p, offset, line) in found.into_iter().take(MAX) {
            incoming.push(PaneRow::Item {
                path: Some(p.clone()),
                label: display_name(p.rsplit('/').next().unwrap_or(&p))
                    .to_string()
                    .into(),
                detail: line.into(),
                offset: Some(offset),
            });
        }
    }
    let n = incoming.len();
    incoming.insert(
        0,
        PaneRow::Header(format!("Unlinked mentions ({n})").into()),
    );

    let mut outgoing: Vec<PaneRow> = Vec::new();
    if let (Some(f), Some(text)) = (finder, texts.get(path)) {
        let mut seen = std::collections::HashSet::new();
        for m in f.find(text) {
            if m.path == path || !seen.insert(m.path.clone()) {
                continue;
            }
            let line = text[m.line_offset..]
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            outgoing.push(PaneRow::Item {
                path: Some(m.path.clone()),
                label: display_name(m.path.rsplit('/').next().unwrap_or(&m.path))
                    .to_string()
                    .into(),
                detail: line.into(),
                offset: Some(m.line_offset),
            });
            if outgoing.len() >= MAX {
                break;
            }
        }
    }
    let n = outgoing.len();
    outgoing.insert(
        0,
        PaneRow::Header(format!("Unlinked mentions ({n})").into()),
    );
    MentionRows {
        path: path.to_string(),
        incoming,
        outgoing,
    }
}

/// An absolute path inside `root` for a vault-relative one, refusing
/// everything that points outside the vault: the folder is the only thing
///  may write, and one socket line must not be able to aim a write
/// out of it.
///
/// The leaf usually does not exist yet - `write_file` is what creates it -
/// so the check cannot be a `canonicalize` of the whole path. It is the
/// deepest ancestor that exists at all that has to resolve inside the
/// vault, because a symlinked parent is how a path that is not there yet
/// still leaves. `symlink_metadata` counts a dangling link as existing: a
/// write through one creates the file at its target, wherever that is, and
/// nothing is left to resolve, so it is refused.
fn vault_path_in(root: &Path, path: &str) -> Result<PathBuf, String> {
    let mut absolute = root.to_path_buf();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => absolute.push(part),
            Component::CurDir => {}
            Component::ParentDir => return Err(format!("{path} leaves the vault")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{path} is not a vault-relative path"))
            }
        }
    }
    if absolute.as_path() == root {
        return Err(format!("{path} names the vault folder, not a file"));
    }
    let real_root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let mut existing: &Path = absolute.as_path();
    let inside = loop {
        if existing.symlink_metadata().is_ok() {
            break match existing.canonicalize() {
                Ok(real) => real.starts_with(&real_root),
                // A dangling link has nothing to resolve; so has a path the
                // process may not look at. Neither can be shown to stay in
                // the vault, so neither is written through.
                Err(_) => false,
            };
        }
        match existing.parent() {
            // The walk stops at `root` at the latest: it canonicalized
            // above, so it exists.
            Some(parent) if parent.starts_with(root) => existing = parent,
            _ => break false,
        }
    };
    if !inside {
        return Err(format!("{path} leaves the vault"));
    }
    Ok(absolute)
}

/// Inserts `list` under `key`, capped to `rows`, with a sibling
/// `{key}_total`. The total is always there, `null` when the cap dropped
/// nothing, so a caller has one rule for every list in the dump.
fn put_capped(into: &mut Map<String, Value>, key: &str, list: Vec<Value>, rows: usize) {
    let total = list.len();
    into.insert(
        key.to_string(),
        Value::Array(list.into_iter().take(rows).collect()),
    );
    into.insert(
        format!("{key}_total"),
        if total > rows {
            json!(total)
        } else {
            Value::Null
        },
    );
}

/// An Outline row, with the offset the pane scrolls the note to.
fn outline_json(row: &OutlineRow) -> Value {
    json!({"level": row.level, "text": row.text.to_string(), "offset": row.offset})
}

/// A Backlinks or Outgoing links row. A header carries nothing but its
/// label, and an unresolved link has no path to open.
fn pane_row_json(row: &PaneRow) -> Value {
    match row {
        PaneRow::Header(label) => json!({
            "kind": "header",
            "label": label.to_string(),
            "detail": Value::Null,
            "path": Value::Null,
            "offset": Value::Null,
        }),
        PaneRow::Item {
            path,
            label,
            detail,
            offset,
        } => json!({
            "kind": "item",
            "label": label.to_string(),
            "detail": detail.to_string(),
            "path": path,
            "offset": offset,
        }),
    }
}

/// A tag row, with the `#` the pane paints in front of the name.
fn tag_json(row: &TagRow) -> Value {
    json!({"name": format!("#{}", row.name), "count": row.count})
}

/// The key contexts of the focused element, outermost first: the names
/// `key_context` puts on each view, which is what a hotkey is matched
/// against.
pub(crate) fn context_names(window: &Window) -> Vec<String> {
    window
        .context_stack()
        .iter()
        .filter_map(|c| c.primary().map(|e| e.key.to_string()))
        .collect()
}

/// Removes `path`, and anything beneath it when it is a folder, from a
/// navigation history, answering where the one-based cursor lands. Entries
/// before the cursor carry it back with them, so back and forward still
/// reach the same notes they did.
fn prune_history(history: &mut Vec<String>, ix: usize, path: &str) -> usize {
    let prefix = format!("{path}/");
    let mut cursor = ix;
    let mut seen = 0;
    history.retain(|entry| {
        let keep = entry.as_str() != path && !entry.starts_with(&prefix);
        if !keep && seen < ix {
            cursor -= 1;
        }
        seen += 1;
        keep
    });
    cursor.min(history.len())
}

/// Every `leaf` node under `node`, depth first, in the order Obsidian
/// lays them out.
fn collect_leaves(node: &Value, out: &mut Vec<Value>) {
    if node.get("type").and_then(Value::as_str) == Some("leaf") {
        out.push(node.clone());
        return;
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_leaves(child, out);
        }
    }
}

/// How many `tabs` groups `node` holds.
fn count_tab_groups(node: &Value) -> usize {
    let own = usize::from(node.get("type").and_then(Value::as_str) == Some("tabs"));
    own + node
        .get("children")
        .and_then(Value::as_array)
        .map(|c| c.iter().map(count_tab_groups).sum())
        .unwrap_or(0)
}

/// The first `tabs` group under `node`, to put the leaves in.
fn find_tab_group(node: &mut Value) -> Option<&mut Map<String, Value>> {
    if node.get("type").and_then(Value::as_str) == Some("tabs") {
        return node.as_object_mut();
    }
    let children = node.get_mut("children")?.as_array_mut()?;
    children.iter_mut().find_map(find_tab_group)
}

/// The view type of the sidebar's current tab: the `tabs` group's
/// `currentTab` child (the first when the key is absent).
fn dock_current_view(layout: &Value, dock: &str) -> Option<String> {
    let mut groups = Vec::new();
    fn find_groups<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
        if node.get("type").and_then(Value::as_str) == Some("tabs") {
            out.push(node);
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            for child in children {
                find_groups(child, out);
            }
        }
    }
    find_groups(layout.get(dock)?, &mut groups);
    let group = groups.first()?;
    let ix = group.get("currentTab").and_then(Value::as_u64).unwrap_or(0) as usize;
    let leaf = group.get("children")?.as_array()?.get(ix)?;
    leaf.get("state")?.get("type")?.as_str().map(str::to_string)
}

/// A fresh leaf id in Obsidian's shape: sixteen hex digits.
fn leaf_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos
        ^ (COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    format!("{x:016x}")
}

/// The `workspace.json` leaf for a tab: the one it was restored from with
/// its state brought up to date, or a new one.
fn leaf_json(tab: &Tab) -> Value {
    let (mode, source) = match tab.mode {
        ViewMode::Reading => ("preview", false),
        ViewMode::Source => ("source", true),
        ViewMode::LivePreview | ViewMode::Graph => ("source", false),
    };
    let title = tab.title();
    let mut leaf = tab
        .leaf
        .clone()
        .filter(|l| {
            let file = l
                .get("state")
                .and_then(|s| s.get("state"))
                .and_then(|s| s.get("file"))
                .and_then(Value::as_str);
            let view = l
                .get("state")
                .and_then(|s| s.get("type"))
                .and_then(Value::as_str);
            match (&tab.path, tab.mode) {
                (Some(p), _) => file == Some(p.as_str()),
                (None, ViewMode::Graph) => view == Some("graph"),
                (None, _) => view == Some("empty"),
            }
        })
        .unwrap_or_else(|| match (&tab.path, tab.mode) {
            (Some(p), _) => json!({
                "id": leaf_id(),
                "type": "leaf",
                "state": {
                    "type": leaf_view_type(p),
                    "state": {"file": p},
                    "icon": "lucide-file",
                    "title": title,
                },
            }),
            (None, ViewMode::Graph) => json!({
                "id": leaf_id(),
                "type": "leaf",
                "state": {"type": "graph", "state": {}, "icon": "lucide-git-fork", "title": title},
            }),
            (None, _) => json!({
                "id": leaf_id(),
                "type": "leaf",
                "state": {"type": "empty", "state": {}, "icon": "lucide-file", "title": title},
            }),
        });
    if let Some(path) = &tab.path {
        if leaf_view_type(path) == "markdown" {
            if let Some(inner) = leaf
                .get_mut("state")
                .and_then(|s| s.get_mut("state"))
                .and_then(Value::as_object_mut)
            {
                inner.insert("mode".into(), json!(mode));
                inner.insert("source".into(), json!(source));
            }
        }
        if let Some(state) = leaf.get_mut("state").and_then(Value::as_object_mut) {
            state.insert("title".into(), json!(title));
        }
    }
    leaf
}

/// The leaf view type Obsidian gives a file of this kind.
fn leaf_view_type(path: &str) -> &'static str {
    match FileKind::from_path(path) {
        FileKind::Markdown => "markdown",
        FileKind::Image => "image",
        FileKind::Pdf => "pdf",
        FileKind::Canvas => "canvas",
        FileKind::Audio => "audio",
        FileKind::Video => "video",
        _ => "markdown",
    }
}

fn empty_pane(text: &'static str, theme: &Theme) -> gpui::AnyElement {
    div()
        .p_3()
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .child(text)
        .into_any_element()
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A view's focus handle only joins the dispatch tree when the frame
        // that paints it is prepainted, so this — the start of that frame —
        // is the first moment a focus asked for elsewhere can land. A modal
        // owns the keyboard while it is up, and closing one focuses the main
        // area anyway, so the request is dropped rather than queued.
        if std::mem::take(&mut self.refocus_main)
            && self.modal == Modal::None
            && self.keyboard_is_parked(window, cx)
        {
            self.focus_main(window, cx);
        }
        let theme = cx.global::<Theme>().clone();
        let title: SharedString = match &self.current {
            Some(p) => display_name(p.rsplit('/').next().unwrap_or(p))
                .to_string()
                .into(),
            None => "No file is open".into(),
        };
        let crumbs: SharedString = self
            .current
            .as_deref()
            .and_then(|p| p.rsplit_once('/').map(|(dir, _)| dir.replace('/', " / ")))
            .unwrap_or_default()
            .into();
        let mode_label = match self.mode {
            ViewMode::Reading => "Reading",
            ViewMode::LivePreview => "Live Preview",
            ViewMode::Source => "Source",
            ViewMode::Graph => "Graph",
        };
        let can_back = self.history_ix > 1;
        let can_forward = self.history_ix < self.history.len();

        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(36.))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(12.))
            .child(
                div()
                    .id("back")
                    .px_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(if can_back {
                        theme.text
                    } else {
                        theme.text_faint
                    })
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(|this, _, window, cx| this.go_back(&GoBack, window, cx)))
                    .child("←"),
            )
            .child(
                div()
                    .id("forward")
                    .px_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(if can_forward {
                        theme.text
                    } else {
                        theme.text_faint
                    })
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(
                        cx.listener(|this, _, window, cx| this.go_forward(&GoForward, window, cx)),
                    )
                    .child("→"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(!crumbs.is_empty(), |d| {
                        d.child(div().text_color(theme.text_muted).child(crumbs.clone()))
                            .child(div().text_color(theme.text_muted).child("/"))
                    })
                    .child(div().text_color(theme.text).child(title.clone())),
            )
            .child(
                div()
                    .id("graph")
                    .px_2()
                    .py_px()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(if self.mode == ViewMode::Graph {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_graph_action(&OpenGraph, window, cx)
                    }))
                    .child("Graph"),
            )
            .when(self.mode != ViewMode::Graph, |d| {
                d.child(
                    div()
                        .id("mode")
                        .px_2()
                        .py_px()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_color(theme.text_muted)
                        .hover(|s| s.bg(theme.bg_alt))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_mode(&ToggleMode, window, cx)
                        }))
                        .child(mode_label),
                )
            });

        let main_body: gpui::AnyElement = if let Some(loading) = &self.loading {
            div()
                .p_4()
                .text_color(theme.text_muted)
                .child(loading.clone())
                .into_any_element()
        } else {
            self.render_main(&theme, cx)
        };

        let wanted_title: SharedString = match &self.current {
            Some(_) => format!("{title} - {} - ", self.vault_name).into(),
            None => format!("{} - ", self.vault_name).into(),
        };
        if self.window_title != wanted_title {
            window.set_window_title(&wanted_title);
            self.window_title = wanted_title;
        }

        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .child(self.render_tab_strip(&theme, cx))
            .child(header)
            .child(div().flex_1().overflow_hidden().child(main_body));

        let show_ribbon = self
            .state
            .as_ref()
            .map(|s| s.appearance().show_ribbon.unwrap_or(true))
            .unwrap_or(true);
        let ribbon = show_ribbon.then(|| self.render_ribbon(&theme, cx));
        let left_tab = self.left_tab;
        let left_body: gpui::AnyElement = match left_tab {
            LeftTab::Files => self.explorer.clone().into_any_element(),
            LeftTab::Search => self.search.clone().into_any_element(),
        };
        let left = div()
            .flex()
            .flex_col()
            .w(px(280.))
            .h_full()
            .bg(theme.bg_secondary)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.dock_tab(
                        "Files",
                        left_tab == LeftTab::Files,
                        &theme,
                        |this, window, cx| {
                            this.leave_search_field(window, cx);
                            this.left_tab = LeftTab::Files;
                            cx.notify();
                        },
                        cx,
                    ))
                    .child(self.dock_tab(
                        "Search",
                        left_tab == LeftTab::Search,
                        &theme,
                        |this, window, cx| {
                            let q = this.search.read(cx).query(cx);
                            this.show_search(&q, Some(window), cx);
                        },
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .text_right()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(self.vault_name.clone()),
                    ),
            )
            .child(div().flex_1().overflow_hidden().child(left_body))
            // Obsidian keeps Settings in the ribbon; the footer button is
            // the way in only while the ribbon is hidden.
            .when(!show_ribbon, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .px_2()
                        .py_1()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .id("open-settings")
                                .px_2()
                                .py_px()
                                .rounded_sm()
                                .cursor_pointer()
                                .text_size(px(11.))
                                .text_color(theme.text_muted)
                                .hover(|s| s.bg(theme.bg_alt))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_settings(None, window, cx)
                                }))
                                .child("Settings"),
                        ),
                )
            });

        let (editor_status, words, chars) = if self.mode.is_editing() {
            let editor = self.editor.read(cx);
            let cursor = editor.cursor();
            let cursors = match editor.cursor_count() {
                1 => String::new(),
                n => format!("{n} cursors  ·  "),
            };
            let line_status = format!(
                "Ln {}, Col {}  ·  {cursors}parse {:.1} ms, {} spans  ·  shape {:.2} ms  ·  ",
                cursor.line + 1,
                cursor.col + 1,
                editor.stats.parse_ms,
                editor.stats.spans,
                editor.prepaint_ms()
            );
            (line_status, editor.stats.words, editor.stats.chars)
        } else {
            (String::new(), self.words, self.chars)
        };
        let status_bar = div()
            .flex()
            .flex_row()
            .justify_between()
            .h(px(22.))
            .px_3()
            .text_size(px(11.))
            .text_color(theme.text_muted)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .overflow_hidden()
                    .child(self.status.clone()),
            )
            .child(SharedString::from(format!(
                "{editor_status}{words} words, {chars} characters"
            )));

        // Every overlay occludes: a modal in Obsidian owns the pointer as
        // well as the keyboard, and without this the editor's raw window
        // listeners take the mouse-down that landed on the modal and focus
        // the note behind it, leaving the modal visible but keyboard-dead.
        // Occluding costs nothing a modal wants — a click outside still
        // reaches each list modal's `on_mouse_down_out`, which is bounds
        // based rather than hitbox based, and Settings closes on its own
        // backdrop click.
        let overlay: Option<gpui::AnyElement> = match self.modal {
            Modal::None => None,
            Modal::Switcher | Modal::Palette | Modal::Prompt | Modal::Chooser => {
                let child: gpui::AnyElement = match self.modal {
                    Modal::Switcher => self.switcher.clone().into_any_element(),
                    Modal::Palette => self.palette.clone().into_any_element(),
                    Modal::Chooser => self.chooser.clone().into_any_element(),
                    _ => self.prompt.clone().into_any_element(),
                };
                Some(
                    div()
                        .occlude()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_row()
                        .justify_center()
                        .items_start()
                        .pt(px(90.))
                        .child(child)
                        .into_any_element(),
                )
            }
            Modal::Settings => Some(
                div()
                    .id("settings-backdrop")
                    .occlude()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x00000080))
                    .flex()
                    .flex_row()
                    .justify_center()
                    .items_center()
                    .py(px(40.))
                    .px(px(40.))
                    .on_click(cx.listener(|this, _, window, cx| this.close_modal(window, cx)))
                    .child(
                        div()
                            .id("settings-frame")
                            .h_full()
                            .max_w_full()
                            .on_click(|_, _, cx| cx.stop_propagation())
                            .child(self.settings.clone()),
                    )
                    .into_any_element(),
            ),
        };
        // Obsidian reports an outcome with a Notice in the corner rather
        // than in the status bar, and the probe notes put it bottom right.
        let notices = (!self.notices.live.is_empty()).then(|| {
            div()
                .absolute()
                .bottom(px(30.))
                .right(px(16.))
                .flex()
                .flex_col()
                .items_end()
                .gap_1()
                .children(self.notices.live.iter().map(|notice| {
                    div()
                        .max_w(px(360.))
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(theme.bg_secondary)
                        .border_1()
                        .border_color(theme.border)
                        .text_size(px(12.))
                        .text_color(theme.text)
                        .child(notice.text.clone())
                }))
        });

        let left_visible = self.left_visible;
        let right_visible = self.right_visible;
        // A modal owns the keyboard while it is up, the way an Obsidian
        // modal's own keymap scope keeps app hotkeys from firing behind it.
        // gpui's key contexts nest rather than replace — the overlay is a
        // child of this div — so the workspace stands its handlers down
        // instead: without them a hotkey can no longer focus a pane behind
        // the modal, or swap the modal out from under its own Escape.
        let hotkeys_live = self.modal == Modal::None;

        div()
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .when(hotkeys_live, |d| {
                d.on_action(cx.listener(Self::toggle_mode))
                    .on_action(cx.listener(Self::go_back))
                    .on_action(cx.listener(Self::go_forward))
                    .on_action(cx.listener(Self::open_search_action))
                    .on_action(cx.listener(Self::open_graph_action))
                    .on_action(cx.listener(Self::run_command_action))
            })
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .font_family(theme.ui_font.clone())
            .text_color(theme.text)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    .children(ribbon)
                    .when(left_visible, |d| d.child(left))
                    .child(main)
                    .when(right_visible, |d| {
                        d.child(self.render_right_dock(&theme, cx))
                    }),
            )
            .child(status_bar)
            .children(overlay)
            .children(notices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn vault() -> (tempfile::TempDir, VaultState) {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("Sub")).unwrap();
        fs::create_dir_all(r.join(".obsidian")).unwrap();
        fs::write(r.join(".obsidian/app.json"), r#"{"trashOption":"local"}"#).unwrap();
        fs::write(r.join("A.md"), "# A\n\nSee [[B]] and [S](Sub/S.md).\n").unwrap();
        fs::write(
            r.join("B.md"),
            "\u{feff}---\r\nrelated: \"[[A]]\"\r\n---\r\n[[A]] and [[A#A|the a]] and `[[A]]`\r\n",
        )
        .unwrap();
        fs::write(r.join("Sub/S.md"), "[A](../A.md) [[B]]\n").unwrap();
        let state = VaultState::load(&r.to_path_buf()).unwrap();
        (dir, state)
    }

    /// A pane row flattened to the three things a test cares about.
    fn flat(r: &PaneRow) -> (Option<&str>, &str, &str) {
        match r {
            PaneRow::Header(h) => (None, h.as_ref(), ""),
            PaneRow::Item {
                path,
                label,
                detail,
                ..
            } => (path.as_deref(), label.as_ref(), detail.as_ref()),
        }
    }

    /// A link written in a property is an outgoing link like any other, and
    /// Obsidian gives it the property as its context. The properties come
    /// first, which is the order the file writes them in.
    #[test]
    fn outgoing_links_list_a_property_link_too() {
        let (_dir, state) = vault();
        let cache = state.cache();
        let rows = outgoing_link_rows(&cache, "B.md");
        assert_eq!(
            rows.iter().map(flat).collect::<Vec<_>>(),
            vec![
                (Some("A.md"), "A", "related"),
                (Some("A.md"), "A", "A.md"),
                (Some("A.md"), "A", "A.md"),
            ],
            "the frontmatter link, then the two body links"
        );
    }

    /// The Linked mentions row for a property link is named by its property;
    /// a body link keeps its line and the line's text.
    #[test]
    fn a_backlink_row_names_the_property_it_was_written_in() {
        let (_dir, state) = vault();
        let cache = state.cache();
        let texts = state.texts();
        let details: Vec<String> = cache
            .backlinks("A.md")
            .iter()
            .map(|b| backlink_detail(b, texts.get(b.source).map(|t| t.as_str())))
            .collect();
        assert_eq!(
            details,
            vec![
                "related",
                "4: [[A]] and [[A#A|the a]] and `[[A]]`",
                "4: [[A]] and [[A#A|the a]] and `[[A]]`",
                "1: [A](../A.md) [[B]]",
            ]
        );
    }

    fn history(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|e| e.to_string()).collect()
    }

    /// A deleted note leaves the history, so back and forward cannot land
    /// on a file the vault no longer has. Entries before the cursor take it
    /// back with them; the notes it can still reach do not change.
    fn leaf(id: &str, file: &str, mode: &str, source: bool) -> Value {
        json!({
            "id": id,
            "type": "leaf",
            "state": {
                "type": "markdown",
                "state": {"file": file, "mode": mode, "source": source},
                "icon": "lucide-file",
                "title": file.rsplit('/').next().unwrap().trim_end_matches(".md"),
                "pinned": true,
            },
        })
    }

    #[test]
    fn a_restored_leaf_is_written_back_as_it_was_read() {
        let read = leaf("a1072f377b3d1c69", "Sub/Note.md", "source", false);
        let tab = Tab {
            path: Some("Sub/Note.md".into()),
            mode: ViewMode::LivePreview,
            history: vec!["Sub/Note.md".into()],
            history_ix: 1,
            leaf: Some(read.clone()),
        };
        assert_eq!(leaf_json(&tab), read);
    }

    #[test]
    fn a_mode_change_rewrites_only_the_mode_keys() {
        let read = leaf("a1072f377b3d1c69", "Note.md", "source", false);
        let tab = Tab {
            path: Some("Note.md".into()),
            mode: ViewMode::Reading,
            history: Vec::new(),
            history_ix: 0,
            leaf: Some(read.clone()),
        };
        let written = leaf_json(&tab);
        assert_eq!(written["id"], read["id"]);
        assert_eq!(written["state"]["pinned"], json!(true));
        assert_eq!(written["state"]["state"]["mode"], json!("preview"));
        assert_eq!(written["state"]["state"]["source"], json!(false));
        let tab = Tab {
            mode: ViewMode::Source,
            ..tab
        };
        let written = leaf_json(&tab);
        assert_eq!(written["state"]["state"]["mode"], json!("source"));
        assert_eq!(written["state"]["state"]["source"], json!(true));
    }

    #[test]
    fn a_tab_that_moved_to_another_note_gets_a_fresh_leaf() {
        let read = leaf("a1072f377b3d1c69", "Old.md", "source", false);
        let tab = Tab {
            path: Some("New.md".into()),
            mode: ViewMode::LivePreview,
            history: Vec::new(),
            history_ix: 0,
            leaf: Some(read),
        };
        let written = leaf_json(&tab);
        let id = written["id"].as_str().unwrap();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(id, "a1072f377b3d1c69");
        assert_eq!(written["state"]["type"], json!("markdown"));
        assert_eq!(written["state"]["state"]["file"], json!("New.md"));
        assert_eq!(written["state"]["title"], json!("New"));
        assert!(written["state"].get("pinned").is_none());
    }

    #[test]
    fn an_empty_tab_and_a_graph_tab_write_their_own_leaf_types() {
        let empty = leaf_json(&Tab::empty(ViewMode::LivePreview));
        assert_eq!(empty["state"]["type"], json!("empty"));
        assert_eq!(empty["state"]["title"], json!("New tab"));
        let graph = leaf_json(&Tab::empty(ViewMode::Graph));
        assert_eq!(graph["state"]["type"], json!("graph"));
        assert_eq!(graph["state"]["icon"], json!("lucide-git-fork"));
    }

    #[test]
    fn leaves_are_collected_depth_first_across_groups() {
        let layout = json!({
            "type": "split",
            "children": [
                {"type": "tabs", "children": [leaf("a", "A.md", "source", false)]},
                {"type": "split", "children": [
                    {"type": "tabs", "children": [leaf("b", "B.md", "preview", false)]}
                ]},
            ],
        });
        let mut leaves = Vec::new();
        collect_leaves(&layout, &mut leaves);
        let ids: Vec<&str> = leaves.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(count_tab_groups(&layout), 2);
        let mut single = json!({"type": "split", "children": [{"type": "tabs", "children": []}]});
        assert_eq!(count_tab_groups(&single), 1);
        assert!(find_tab_group(&mut single).is_some());
    }

    #[test]
    fn a_docks_current_tab_is_the_leaf_current_tab_points_at() {
        let layout = json!({
            "left": {"type": "split", "children": [{
                "type": "tabs",
                "currentTab": 1,
                "children": [
                    {"type": "leaf", "state": {"type": "file-explorer"}},
                    {"type": "leaf", "state": {"type": "search"}},
                ],
            }]},
            "right": {"type": "split", "children": [{
                "type": "tabs",
                "children": [{"type": "leaf", "state": {"type": "backlink"}}],
            }]},
        });
        assert_eq!(
            dock_current_view(&layout, "left").as_deref(),
            Some("search")
        );
        assert_eq!(
            dock_current_view(&layout, "right").as_deref(),
            Some("backlink")
        );
        assert_eq!(dock_current_view(&layout, "main"), None);
    }

    #[test]
    fn prune_history_forgets_a_deleted_note() {
        let mut h = history(&["A.md", "B.md", "C.md"]);
        assert_eq!(prune_history(&mut h, 2, "B.md"), 1);
        assert_eq!(h, history(&["A.md", "C.md"]));

        let mut h = history(&["A.md", "B.md", "C.md"]);
        assert_eq!(prune_history(&mut h, 3, "A.md"), 2, "still at C.md");
        assert_eq!(h, history(&["B.md", "C.md"]));

        let mut h = history(&["A.md", "B.md"]);
        assert_eq!(prune_history(&mut h, 1, "B.md"), 1, "after the cursor");
        assert_eq!(h, history(&["A.md"]));
    }

    /// Deleting a folder deletes every note under it, and a repeat visit is
    /// a repeat entry, so more than one entry can go at once.
    #[test]
    fn prune_history_forgets_a_whole_folder() {
        let mut h = history(&["Sub/S.md", "A.md", "Sub/T.md", "A.md"]);
        assert_eq!(prune_history(&mut h, 4, "Sub"), 2);
        assert_eq!(h, history(&["A.md", "A.md"]));

        let mut h = history(&["A.md", "A.md", "B.md"]);
        assert_eq!(prune_history(&mut h, 3, "A.md"), 1);
        assert_eq!(h, history(&["B.md"]));
    }

    /// A path the history never held, and a prefix that is not a folder
    /// boundary, leave it alone.
    #[test]
    fn prune_history_keeps_what_it_does_not_name() {
        let mut h = history(&["A.md", "Subtotal/S.md"]);
        assert_eq!(prune_history(&mut h, 2, "Sub"), 2);
        assert_eq!(h, history(&["A.md", "Subtotal/S.md"]));

        let mut h = history(&["A.md"]);
        assert_eq!(prune_history(&mut h, 0, "A.md"), 0, "nothing open");
        assert!(h.is_empty());
    }

    /// The path a toggle takes: the flipped config reaches `app.json` and
    /// the state in hand, and the key the vault already had is left alone.
    #[test]
    fn a_setting_change_writes_its_file_and_updates_the_state() {
        let (dir, state) = vault();
        assert_eq!(state.app().trash_option.as_deref(), Some("local"));
        assert_eq!(state.app().vim_mode, None);

        let mut app = state.app().clone();
        app.vim_mode = Some(true);
        state.apply_setting(&SettingChange::App(app)).unwrap();

        assert_eq!(state.app().vim_mode, Some(true), "the state follows");
        let text = fs::read_to_string(dir.path().join(".obsidian/app.json")).unwrap();
        assert_eq!(
            text, "{\n  \"trashOption\": \"local\",\n  \"vimMode\": true\n}",
            "the file kept what it had and gained one key"
        );
    }

    /// A config file the vault does not have yet is created, and only with
    /// what changed.
    #[test]
    fn a_setting_change_creates_a_missing_config_file() {
        let (dir, state) = vault();
        let mut switcher = state.switcher().clone();
        switcher.show_attachments = Some(false);
        state
            .apply_setting(&SettingChange::Switcher(switcher))
            .unwrap();
        let text = fs::read_to_string(dir.path().join(".obsidian/switcher.json")).unwrap();
        assert_eq!(text, "{\n  \"showAttachments\": false\n}");
    }

    #[test]
    fn a_hotkeys_change_is_written_whole_so_a_restored_default_leaves_the_file() {
        let (dir, state) = vault();
        let path = dir.path().join(".obsidian/hotkeys.json");
        fs::write(
            &path,
            "\u{feff}{\n  \"editor:toggle-bold\": [],\n  \"my-plugin:thing\": [\n    {\n      \"modifiers\": [\n        \"Alt\"\n      ],\n      \"key\": \"X\"\n    }\n  ]\n}",
        )
        .unwrap();
        let mut overrides = state.vault().hotkeys().unwrap();
        assert_eq!(overrides.len(), 2);
        overrides.shift_remove("editor:toggle-bold");
        state
            .apply_setting(&SettingChange::Hotkeys(overrides))
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with('\u{feff}'), "the byte order mark stays");
        assert!(!text.contains("toggle-bold"), "{text}");
        assert!(text.contains("my-plugin:thing"), "{text}");
        assert_eq!(
            text.trim_start_matches('\u{feff}'),
            "{\n  \"my-plugin:thing\": [\n    {\n      \"modifiers\": [\n        \"Alt\"\n      ],\n      \"key\": \"X\"\n    }\n  ]\n}"
        );
    }

    /// The block id write: it reaches the target's bytes, keeps that
    /// note's own line endings, and refuses a file it did not plan from.
    #[test]
    fn writing_a_block_id_reaches_the_disk_and_refuses_a_stale_file() {
        use _suggest::blocks::{plan_id, Placement};
        let (dir, state) = vault();
        let path = "A.md";
        let before = fs::read_to_string(dir.path().join(path)).unwrap();
        let end = before.find("See [[B]]").unwrap() + "See [[B]] and [S](Sub/S.md).".len();
        let insertion = plan_id(&before, end, Placement::Inline, "abc123", "\n").expect("a plan");
        state.write_block_id(path, &before, &insertion).unwrap();
        let after = fs::read_to_string(dir.path().join(path)).unwrap();
        assert_eq!(after, insertion.applied(&before));
        assert!(
            after.contains("^abc123"),
            "the id is not in the file: {after}"
        );

        // The same insertion against bytes the file no longer holds is
        // refused rather than written.
        let err = state
            .write_block_id(path, &before, &insertion)
            .expect_err("the file changed since");
        assert!(err.contains("changed on disk"), "{err}");
        assert_eq!(
            fs::read_to_string(dir.path().join(path)).unwrap(),
            after,
            "a refused write still changed the file"
        );
    }

    /// A CRLF note keeps its own endings when an id is written into it,
    /// which is where  deliberately differs from Obsidian.
    #[test]
    fn a_block_id_written_into_a_crlf_note_keeps_its_endings() {
        use _suggest::blocks::{plan_id, Placement};
        let (dir, state) = vault();
        let path = "Crlf.md";
        let before = "# Crlf\r\n\r\npara in a CRLF file.\r\n";
        fs::write(dir.path().join(path), before).unwrap();
        let end = before.find("para in a CRLF file.").unwrap() + "para in a CRLF file.".len();
        let insertion = plan_id(before, end, Placement::Inline, "abc123", "\r\n").expect("a plan");
        state.write_block_id(path, before, &insertion).unwrap();
        let after = fs::read_to_string(dir.path().join(path)).unwrap();
        assert_eq!(after, "# Crlf\r\n\r\npara in a CRLF file. ^abc123\r\n");
        assert!(
            !after.contains("\n\n"),
            "a bare newline reached a CRLF note"
        );
    }

    /// Creating a daily note: the bytes on disk are the template expanded,
    /// the template itself is untouched, and a second run changes nothing.
    #[test]
    fn creating_a_daily_note_writes_the_template_and_nothing_else() {
        use _templates::{expand, Context, Stamp, DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT};
        let (dir, state) = vault();
        fs::create_dir_all(dir.path().join("Journal")).unwrap();
        let template = "# {{title}}\n\nWritten {{date}}.\n";
        fs::write(dir.path().join("Sub/Daily.md"), template).unwrap();
        let at = Stamp {
            year: 2026,
            month: 10,
            day: 18,
            hour: 9,
            minute: 7,
            second: 3,
        };
        let (path, exists) = state
            .vault()
            .daily_note("YYYY-MM-DD", "Journal", &at)
            .unwrap();
        assert_eq!((path.as_str(), exists), ("Journal/2026-10-18.md", false));
        let text = expand(
            template,
            &Context {
                at,
                date_format: DEFAULT_DATE_FORMAT,
                time_format: DEFAULT_TIME_FORMAT,
                title: "2026-10-18",
            },
        );
        state.create_note(&path, &text).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(&path)).unwrap(),
            "# 2026-10-18\n\nWritten 2026-10-18.\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("Sub/Daily.md")).unwrap(),
            template,
            "the template itself was written to"
        );
        // Running again finds it rather than creating a second one.
        let (again, exists) = state
            .vault()
            .daily_note("YYYY-MM-DD", "Journal", &at)
            .unwrap();
        assert_eq!((again.as_str(), exists), ("Journal/2026-10-18.md", true));
    }

    /// A daily note whose folder is missing is refused, and nothing is
    /// created — Obsidian reports the same and writes nothing.
    #[test]
    fn a_daily_note_with_no_folder_writes_nothing() {
        use _templates::Stamp;
        let (dir, state) = vault();
        let at = Stamp {
            year: 2026,
            month: 10,
            day: 18,
            hour: 0,
            minute: 0,
            second: 0,
        };
        let err = state
            .vault()
            .daily_note("YYYY-MM-DD", "NoSuchFolder", &at)
            .unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
        assert!(!dir.path().join("NoSuchFolder").exists());
    }

    #[test]
    fn rename_leaves_a_note_that_changed_underneath_alone() {
        // The rewrites are planned from the text the index holds. If the
        // other app saved one of those notes since, writing the plan would
        // throw that save away, so the note keeps its own bytes.
        let (dir, state) = vault();
        let theirs = "# B\n\nRewritten by the other app, with [[A]] still in it.\n";
        fs::write(dir.path().join("B.md"), theirs).unwrap();
        let outcome = state.rename("A.md", "Renamed A.md", true).unwrap();
        assert_eq!(outcome.stale, vec!["B.md".to_string()]);
        assert_eq!(
            fs::read_to_string(dir.path().join("B.md")).unwrap(),
            theirs,
            "the other app's save was overwritten"
        );
        // The notes that did not change are still rewritten.
        assert!(outcome.rewritten_files >= 1);
        let sub = fs::read_to_string(dir.path().join("Sub/S.md")).unwrap();
        assert!(
            sub.contains("Renamed A"),
            "an untouched note was not rewritten: {sub}"
        );
    }

    #[test]
    fn rename_rewrites_links_on_disk_and_in_the_index() {
        let (dir, state) = vault();
        let outcome = state.rename("A.md", "Renamed A.md", true).unwrap();
        assert_eq!(
            outcome.renamed,
            vec![("A.md".to_string(), "Renamed A.md".to_string())]
        );
        assert_eq!((outcome.rewritten_files, outcome.rewritten_links), (2, 4));
        let b = fs::read_to_string(dir.path().join("B.md")).unwrap();
        assert_eq!(
            b,
            "\u{feff}---\r\nrelated: \"[[Renamed A]]\"\r\n---\r\n[[Renamed A]] and [[Renamed A#A|the a]] and `[[A]]`\r\n",
            "the BOM and CRLF endings survive"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("Sub/S.md")).unwrap(),
            "[Renamed A](Renamed%20A.md) [[B]]\n"
        );
        assert!(dir.path().join("Renamed A.md").is_file());
        assert!(!dir.path().join("A.md").exists());
        {
            let cache = state.cache();
            assert!(cache.file("A.md").is_none());
            assert!(cache.file("Renamed A.md").is_some());
            // B's `related` property, its two body links, and S's markdown link.
            assert_eq!(cache.backlinks("Renamed A.md").len(), 4);
            assert!(cache.unresolved_links("B.md").is_none());
            assert_eq!(state.texts()["B.md"].as_str(), b);
        }
        assert!(outcome.changes.structural());
        // The watcher's events for the same paths find nothing new.
        let changes = state.apply(&[
            dir.path().join("A.md"),
            dir.path().join("Renamed A.md"),
            dir.path().join("B.md"),
        ]);
        assert!(changes
            .files
            .iter()
            .all(|c| matches!(c, FileChange::Modified(_))));
        assert_eq!(state.cache().backlinks("Renamed A.md").len(), 4);
    }

    #[test]
    fn rename_without_updating_links_leaves_them_dangling() {
        let (dir, state) = vault();
        let outcome = state.rename("A.md", "A2.md", false).unwrap();
        assert_eq!(outcome.rewritten_links, 0);
        assert!(fs::read_to_string(dir.path().join("B.md"))
            .unwrap()
            .contains("[[A]] and"));
        assert_eq!(
            state.cache().unresolved_links("B.md").unwrap().get("A"),
            Some(&3),
            "two body links and the frontmatter link"
        );
        assert!(
            state.rename("A2.md", "B.md", true).is_err(),
            "destination exists"
        );
        assert_eq!(
            state.rename_preview("B.md", "Sub/B.md"),
            (0, 0),
            "a unique name moving folders changes no shortest link"
        );
        assert_eq!(state.rename_preview("B.md", "Sub/Bee.md"), (2, 2));
    }

    /// The names the control socket calls the four modes by, both ways;
    /// `preview` is Obsidian's config value, not one of them.
    #[test]
    fn mode_names_round_trip() {
        for mode in [
            ViewMode::Reading,
            ViewMode::LivePreview,
            ViewMode::Source,
            ViewMode::Graph,
        ] {
            assert_eq!(ViewMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(ViewMode::parse("preview"), None);
        assert_eq!(ViewMode::parse(""), None);
    }

    /// A write through the control socket can only land inside the vault
    /// folder, and the path it is given is usually one that does not exist
    /// yet, so the folders it goes through are what the check has to look
    /// at.
    #[test]
    fn vault_paths_cannot_leave_the_vault() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir(root.join("Sub")).unwrap();
        fs::write(root.join("A.md"), "a").unwrap();

        assert_eq!(vault_path_in(&root, "A.md").unwrap(), root.join("A.md"));
        assert_eq!(
            vault_path_in(&root, "Sub/New.md").unwrap(),
            root.join("Sub/New.md"),
            "a note that is not there yet, under a folder that is"
        );
        assert_eq!(vault_path_in(&root, "./A.md").unwrap(), root.join("A.md"));

        assert!(vault_path_in(&root, "../escape.md").is_err());
        assert!(vault_path_in(&root, "Sub/../../escape.md").is_err());
        assert!(vault_path_in(&root, "/etc/passwd").is_err());
        assert!(vault_path_in(&root, "").is_err(), "the folder itself");

        // A symlinked folder in the vault: the path is innocent, the folder
        // it goes through is not.
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("there.md"), "o").unwrap();
        symlink(outside.path(), root.join("linked")).unwrap();
        assert!(vault_path_in(&root, "linked/new.md").is_err());
        assert!(vault_path_in(&root, "linked/there.md").is_err());

        // A dangling link resolves to nothing, and a write through it would
        // create the file wherever it points.
        symlink(outside.path().join("gone.md"), root.join("dangling.md")).unwrap();
        assert!(vault_path_in(&root, "dangling.md").is_err());
    }

    /// A file kind reaches the state dump in lower case.
    #[test]
    fn file_kinds_are_named_in_lower_case() {
        assert_eq!(kind_name(FileKind::Markdown), "markdown");
        assert_eq!(kind_name(FileKind::Other), "other");
    }

    #[test]
    fn create_duplicate_and_trash_keep_the_index_current() {
        let (dir, state) = vault();
        assert_eq!(
            state.new_note_path("Missing", Some("Sub/S.md")),
            "Missing.md"
        );
        assert_eq!(
            state.new_note_path("Deep/Also Missing", None),
            "Deep/Also Missing.md"
        );
        let changes = state.create_note("Deep/Also Missing.md", "").unwrap();
        assert_eq!(
            changes.files,
            vec![FileChange::Added("Deep/Also Missing.md".into())]
        );
        assert!(state.cache().file("Deep/Also Missing.md").is_some());
        assert!(state.vault().folders().contains(&"Deep".to_string()));
        let (copy, _) = state.duplicate("A.md").unwrap();
        assert_eq!(copy, "A 1.md");
        assert_eq!(
            fs::read_to_string(dir.path().join("A 1.md")).unwrap(),
            fs::read_to_string(dir.path().join("A.md")).unwrap()
        );
        let changes = state.trash("Sub").unwrap();
        assert_eq!(changes.removed_notes, vec!["Sub/S.md"]);
        assert!(dir.path().join(".trash/Sub/S.md").is_file());
        assert!(state.cache().file("Sub/S.md").is_none());
        assert_eq!(state.cache().backlinks("B.md").len(), 2, "A and its copy");
        assert!(state.vault().folders().iter().all(|f| f != "Sub"));
    }

    /// The socket's `focus` may not park the keyboard on a view the main
    /// area is not painting: that handle is in no frame's dispatch tree, and
    /// from there the app answers no key at all, the one that would undo it
    /// included.
    #[test]
    fn focus_refuses_a_view_the_main_area_is_not_painting() {
        assert_eq!(
            Workspace::focus_target("editor", MainView::Editor),
            Ok(Some(MainView::Editor))
        );
        assert_eq!(
            Workspace::focus_target("workspace", MainView::Empty),
            Ok(None),
            "the root div paints in every frame"
        );
        let err = Workspace::focus_target("reading", MainView::Editor).unwrap_err();
        assert!(err.contains("painting editor, not reading"), "{err}");
        let err = Workspace::focus_target("graph", MainView::Empty).unwrap_err();
        assert!(err.contains("painting nothing, not graph"), "{err}");
        assert_eq!(
            Workspace::focus_target("pane", MainView::Editor),
            Err("unknown view: pane".to_string())
        );
    }

    /// An editor whose buffer names no file never gets the main area: on
    /// launch over a vault with nothing to open the buffer belongs to
    /// nothing, and anything typed into it would be saved nowhere.
    #[test]
    fn the_main_area_paints_no_editor_without_a_file_behind_it() {
        for mode in [ViewMode::LivePreview, ViewMode::Source] {
            assert_eq!(
                Workspace::main_view_for(mode, false, false, false),
                MainView::Empty,
                "{mode:?} with nothing open"
            );
            assert_eq!(
                Workspace::main_view_for(mode, true, false, true),
                MainView::Editor,
                "{mode:?} with a note open"
            );
            // The `--synthetic` spike hands the editor a note that is in no
            // vault: the buffer names a file, and that is what mounts it.
            assert_eq!(
                Workspace::main_view_for(mode, false, false, true),
                MainView::Editor,
                "{mode:?} with a buffer that belongs to no vault"
            );
            assert_eq!(
                Workspace::main_view_for(mode, true, true, true),
                MainView::Reading,
                "{mode:?} over an image"
            );
        }
        assert_eq!(
            Workspace::main_view_for(ViewMode::Reading, false, false, true),
            MainView::Empty,
            "a buffer is not a built note"
        );
        assert_eq!(
            Workspace::main_view_for(ViewMode::Graph, false, false, false),
            MainView::Graph,
            "the graph needs no file"
        );
    }

    /// The socket's `type` reaches the editor only while the main area is
    /// painting it. The editor's handle can still hold the focus under the
    /// empty state, and this route does not go through the dispatch tree
    /// that would otherwise refuse it.
    #[test]
    fn typing_reaches_no_editor_the_main_area_is_not_painting() {
        for focus in ["editor", "property-field", "cell-field"] {
            assert_eq!(
                Workspace::typed_text_target(focus, MainView::Editor),
                "editor"
            );
            assert_eq!(Workspace::typed_text_target(focus, MainView::Empty), "none");
            assert_eq!(
                Workspace::typed_text_target(focus, MainView::Reading),
                "none"
            );
        }
        assert_eq!(
            Workspace::typed_text_target("prompt-input", MainView::Empty),
            "input",
            "a modal's field is on screen whatever the main area paints"
        );
        assert_eq!(
            Workspace::typed_text_target("workspace", MainView::Editor),
            "none"
        );
    }

    /// Outline rows carry the byte offsets of the text they were built from.
    /// The index is a save behind while a note is being typed, so rows built
    /// from it point into a version that is not on screen — typing a heading
    /// above another moves every offset below it, and a row clicked with the
    /// old offset lands somewhere else in the note.
    #[test]
    fn outline_rows_carry_the_offsets_of_the_text_they_were_built_from() {
        let saved = "# A\n\n### B\n\n## C\n";
        let typed = "## Typed heading\n\n# A\n\n### B\n\n## C\n";
        let flat = |rows: &[OutlineRow]| -> Vec<(u8, String, usize)> {
            rows.iter()
                .map(|r| (r.level, r.text.to_string(), r.offset))
                .collect()
        };
        assert_eq!(
            flat(&outline_rows(&index_markdown(saved))),
            vec![
                (1, "A".to_string(), saved.find("# A").unwrap()),
                (3, "B".to_string(), saved.find("### B").unwrap()),
                (2, "C".to_string(), saved.find("## C").unwrap()),
            ]
        );
        let buffer = outline_rows(&index_markdown(typed));
        assert_eq!(
            flat(&buffer),
            vec![
                (2, "Typed heading".to_string(), 0),
                (1, "A".to_string(), typed.find("# A").unwrap()),
                (3, "B".to_string(), typed.find("### B").unwrap()),
                (2, "C".to_string(), typed.find("## C").unwrap()),
            ]
        );
        for row in &buffer {
            assert!(
                typed[row.offset..].starts_with('#'),
                "row {:?} points at {:?}",
                row.text,
                &typed[row.offset..]
            );
        }
    }

    /// A notice leaves on its own timer, and only the one that timer named:
    /// a message put up later stays until its own turn comes.
    #[test]
    fn a_notice_expires_by_id_and_leaves_the_newer_one() {
        let mut n = Notices::default();
        let first = n.push("The cursor is not in a table".into());
        let second = n.push("A.md saved".into());
        assert_eq!(n.latest().map(|t| t.as_ref()), Some("A.md saved"));

        n.expire(first);
        assert_eq!(n.texts(), vec!["A.md saved".to_string()]);
        n.expire(second);
        assert!(n.texts().is_empty());
        assert!(n.latest().is_none(), "nothing is left to misread");
    }

    /// The autosave retries on a timer, so the same refusal arrives again
    /// and again: it renews the message on screen instead of stacking
    /// copies of it, and the renewed one outlives the first one's timer.
    #[test]
    fn a_repeated_notice_is_renewed_rather_than_stacked() {
        let mut n = Notices::default();
        let first = n.push("A.md changed on disk".into());
        let again = n.push("A.md changed on disk".into());
        assert_ne!(first, again);
        assert_eq!(n.texts(), vec!["A.md changed on disk".to_string()]);

        n.expire(first);
        assert_eq!(
            n.texts(),
            vec!["A.md changed on disk".to_string()],
            "the first timer must not take the renewed message down"
        );
        n.expire(again);
        assert!(n.texts().is_empty());
    }

    /// Clearing takes down everything, which is what the control socket's
    /// `notice_clear` is for.
    #[test]
    fn clearing_takes_every_notice_down() {
        let mut n = Notices::default();
        n.push("one".into());
        n.push("two".into());
        n.clear();
        assert!(n.latest().is_none());
        assert_eq!(n.push("three".into()), 3, "ids keep counting up");
    }
}
