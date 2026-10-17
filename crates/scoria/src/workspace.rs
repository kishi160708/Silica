//! The workspace shell: left dock (Files, Search), main area (reading view
//! or source editor), right dock (Backlinks, Outgoing links, Outline, Tags),
//! view header, status bar, and the quick switcher, command palette and
//! Settings overlays. Commands run by id ([`Workspace::run_command`]);
//! hotkeys come from the command registry, so `hotkeys.json` applies.
//!
//! The vault is indexed on a background thread when the window opens and
//! then kept current by a filesystem watcher: changed paths are re-read and
//! re-indexed in place, link edges re-resolved, and every pane refreshed.

use crate::editor::{Editor, EditorEvent};
use crate::explorer::{display_name, ExplorerEvent, FileExplorer, FileTree};
use crate::graph::{GraphEvent, GraphView};
use crate::mentions::{Finder, Name};
use crate::palette::{CommandPalette, PaletteCommand, PaletteEvent};
use crate::prompt::{Prompt, PromptEvent, PromptSpec};
use crate::reading::{self, LinkTarget, ReadingEvent, ReadingView};
use crate::search::{SearchEvent, SearchPane};
use crate::settings::{self, SettingsEvent, SettingsView};
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
    basename, frontmatter_aliases, index_markdown, CachedMetadata, LinkFormat, MetadataCache,
};
use _commands::{keystroke, CommandRegistry};
use _formats::config::{
    AppConfig, AppearanceConfig, CommandPaletteConfig, CorePlugins, PluginManifest, SwitcherConfig,
};
use _formats::FileKind;
use _markdown::Subpath;
use _vault::{FileChange, TrashOption, Vault};
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
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
    "switcher:open",
    "global-search:open",
    "graph:open",
    "graph:open-local",
    "markdown:toggle-preview",
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
    pub app: AppConfig,
    pub appearance: AppearanceConfig,
    pub core_plugins: Option<CorePlugins>,
    pub community_plugins: Vec<String>,
    pub plugin_manifests: Vec<PluginManifest>,
    pub palette: CommandPaletteConfig,
    pub switcher: SwitcherConfig,
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
            app,
            appearance,
            core_plugins,
            community_plugins,
            plugin_manifests,
            palette,
            switcher,
            index_ms: started.elapsed().as_secs_f64() * 1000.0,
        })
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
            strict_line_breaks: self.app.strict_line_breaks.unwrap_or(false),
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
        LinkFormat::from_config(self.app.new_link_format.as_deref())
    }

    pub fn trash_option(&self) -> TrashOption {
        TrashOption::from_config(self.app.trash_option.as_deref())
    }

    /// `alwaysUpdateLinks`: on by default; off means ask before rewriting.
    pub fn update_links_automatically(&self) -> bool {
        self.app.always_update_links.unwrap_or(true)
    }

    /// `promptDelete`: on by default.
    pub fn confirm_delete(&self) -> bool {
        self.app.prompt_delete.unwrap_or(true)
    }

    /// Where a note created for the link text `name` goes, given the open
    /// note (`newFileLocation`, `newFileFolderPath`).
    pub fn new_note_path(&self, name: &str, current: Option<&str>) -> String {
        let vault = self.vault();
        let folder = vault.new_note_folder(
            self.app.new_file_location.as_deref(),
            self.app.new_file_folder_path.as_deref(),
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
        for rw in &plan {
            std::fs::write(self.root.join(&rw.new_path), rw.text.as_bytes())
                .map_err(|e| format!("Could not update links in {}: {e}", rw.new_path))?;
        }
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
            rewritten_files: plan.len(),
            rewritten_links: plan.iter().map(|r| r.links).sum(),
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

/// Which overlay is up, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    None,
    Switcher,
    Palette,
    Settings,
    Prompt,
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

pub struct Workspace {
    focus_handle: FocusHandle,
    state: Option<Arc<VaultState>>,
    vault_name: SharedString,
    loading: Option<SharedString>,
    explorer: Entity<FileExplorer>,
    search: Entity<SearchPane>,
    switcher: Entity<QuickSwitcher>,
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
    history: Vec<String>,
    history_ix: usize,
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
    status: SharedString,
    words: usize,
    chars: usize,
    _watcher: Option<watch::Watcher>,
    /// Pending autosave, two seconds after the last edit.
    save_task: Option<Task<()>>,
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
        let palette = cx.new(CommandPalette::new);
        let settings = cx.new(SettingsView::new);
        let prompt = cx.new(Prompt::new);
        let reading = cx.new(ReadingView::new);
        let editor = cx.new(|cx| Editor::new(window, cx));
        let graph = cx.new(GraphView::new);
        let subscriptions =
            vec![
                cx.subscribe_in(&graph, window, |this, _, event: &GraphEvent, window, cx| {
                    match event {
                        GraphEvent::Open(path) => {
                            let back = this.mode_before_graph;
                            this.set_mode(back, window, cx);
                            this.open_file(path.clone(), None, cx);
                        }
                    }
                }),
                cx.subscribe_in(
                    &explorer,
                    window,
                    |this, _, event: &ExplorerEvent, window, cx| match event {
                        ExplorerEvent::Open(path) => this.open_file(path.clone(), None, cx),
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
                            this.reading
                                .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
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
                    },
                ),
                cx.subscribe(&reading, |this, _, event: &ReadingEvent, cx| match event {
                    ReadingEvent::OpenLink(target) => this.follow_link(target.clone(), cx),
                }),
                cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| match event {
                    EditorEvent::Edited => this.schedule_save(cx),
                    EditorEvent::OpenLink(target) => this.follow_link(target.clone(), cx),
                }),
            ];
        // Closing the window must not lose edits waiting for the autosave.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            if let Some(ws) = this.upgrade() {
                ws.update(cx, |ws, cx| ws.flush_save(cx));
            }
            true
        });
        let mut ws = Workspace {
            focus_handle: cx.focus_handle(),
            state: None,
            vault_name: "No vault".into(),
            loading: None,
            explorer,
            search,
            switcher,
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
            history: Vec::new(),
            history_ix: 0,
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
            words: 0,
            chars: 0,
            _watcher: None,
            save_task: None,
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

    fn set_state(&mut self, state: VaultState, cx: &mut Context<Self>) {
        self.loading = None;
        let readable = state.app.readable_line_length.unwrap_or(true);
        self.reading
            .update(cx, |r, _| r.set_readable_line_length(readable));
        let (smart, use_tab, tab_size) = (
            state.app.smart_indent_list.unwrap_or(true),
            state.app.use_tab.unwrap_or(true),
            state.app.tab_size.unwrap_or(4),
        );
        self.editor.update(cx, |e, _| {
            e.set_readable_line_length(readable);
            e.set_indent_options(smart, use_tab, tab_size);
        });
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
            e.set_vault(Some(embed_source), build_options, cx)
        });
        self.refresh_vault_views(cx);
        self.start_watching(&state, cx);
        if let Some(current) = self.current.clone() {
            self.open_file(current, None, cx);
        }
        cx.notify();
    }

    /// Rebuilds everything that depends on the file list or the whole cache.
    fn refresh_vault_views(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let show_unsupported = state.app.show_unsupported_files.unwrap_or(false);
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
                self.status = format!("File watching unavailable: {err}").into();
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
        if let Some(current) = self.current.clone() {
            let removed = changes
                .files
                .iter()
                .any(|c| matches!(c, FileChange::Removed(p) if *p == current));
            if removed {
                self.status = format!("{current} was deleted").into();
                self.refresh_panes(&state, cx);
            } else if changes.files.iter().any(|c| c.path() == current) {
                self.show_file(&current, &state, cx);
            } else {
                self.refresh_panes(&state, cx);
            }
        }
        cx.notify();
    }

    pub fn is_loaded(&self) -> bool {
        self.state.is_some()
    }

    /// Selects a right-dock tab by name (`--pane` on the command line).
    pub fn show_right_tab(&mut self, name: &str, cx: &mut Context<Self>) {
        self.right_tab = match name.to_lowercase().as_str() {
            "outgoing" => RightTab::Outgoing,
            "outline" => RightTab::Outline,
            "tags" => RightTab::Tags,
            _ => RightTab::Backlinks,
        };
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
    pub fn show_search(
        &mut self,
        query: &str,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
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
        let commands: Vec<PaletteCommand> = registry
            .iter()
            .filter(|c| RUNNABLE.contains(&c.id.as_str()))
            .map(|c| PaletteCommand::from_command(c, pinned.contains(&c.id)))
            .collect();
        let recent = self.recent_commands.clone();
        self.modal = Modal::Palette;
        self.palette
            .update(cx, |p, cx| p.open(commands, recent, query, window, cx));
        cx.notify();
    }

    /// Opens the Settings modal, on `tab` when given (`--settings` on the command line).
    pub fn open_settings(
        &mut self,
        tab: Option<settings::Tab>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.modal = Modal::Settings;
        self.settings.update(cx, |s, cx| s.open(tab, window, cx));
        cx.notify();
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
                if self.mode.is_editing() {
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
            "app:go-back" => self.go_back(&GoBack, window, cx),
            "app:go-forward" => self.go_forward(&GoForward, window, cx),
            "app:open-settings" => self.open_settings(None, window, cx),
            "app:toggle-left-sidebar" => {
                self.left_visible = !self.left_visible;
                cx.notify();
            }
            "app:toggle-right-sidebar" => {
                self.right_visible = !self.right_visible;
                cx.notify();
            }
            "file-explorer:open" => {
                self.left_visible = true;
                self.left_tab = LeftTab::Files;
                cx.notify();
            }
            "file-explorer:reveal-active-file" => {
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
                let folder = self
                    .state
                    .as_ref()
                    .map(|s| {
                        s.vault().new_note_folder(
                            s.app.new_file_location.as_deref(),
                            s.app.new_file_folder_path.as_deref(),
                            self.current.as_deref(),
                        )
                    })
                    .unwrap_or_default();
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
                let name = cx
                    .global::<Commands>()
                    .0
                    .get(id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| id.to_string());
                self.status = format!("{name}: not available in  yet").into();
                cx.notify();
            }
        }
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
        match mode {
            ViewMode::Reading => window.focus(&self.reading.focus_handle(cx), cx),
            ViewMode::LivePreview | ViewMode::Source => {
                window.focus(&self.editor.focus_handle(cx), cx)
            }
            ViewMode::Graph => window.focus(&self.graph.focus_handle(cx), cx),
        }
        cx.notify();
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
            .and_then(|s| s.app.live_preview)
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
            .and_then(|s| s.app.default_view_mode.as_deref())
            == Some("preview");
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
        }
        self.show_file(&path, &state, cx);
        if let Some(sub) = subpath {
            self.scroll_to_subpath(&path, &sub, &state, cx);
        }
    }

    fn show_file(&mut self, path: &str, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        self.current = Some(path.to_string());
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
                        self.status = format!("{path} changed on disk; unsaved edits kept").into();
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
                    self.status = format!("Could not read {path}: {err}").into();
                }
            },
            FileKind::Image => {
                let model = reading::build_with(
                    path,
                    &format!("![[{path}]]"),
                    state.as_ref(),
                    state.build_options(),
                );
                self.words = 0;
                self.chars = 0;
                self.reading
                    .update(cx, |r, cx| r.set_note(Some(path.to_string()), model, cx));
                self.status = path.to_string().into();
            }
            other => {
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

    fn refresh_panes(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let Some(path) = self.current.clone() else {
            return;
        };
        let cache = state.cache();
        let texts = state.texts();
        self.outline = cache
            .file(&path)
            .map(|meta| {
                meta.headings
                    .iter()
                    .map(|h| OutlineRow {
                        level: h.level,
                        text: Self::heading_text(&h.heading).into(),
                        offset: h.position.start.offset,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Linked mentions: every link into this file, with its line as context.
        let mut linked = Vec::new();
        for b in cache.backlinks(&path) {
            let context = texts
                .get(b.source)
                .and_then(|t| t.lines().nth(b.link.position.start.line as usize))
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            linked.push(PaneRow::Item {
                path: Some(b.source.to_string()),
                label: display_name(b.source.rsplit('/').next().unwrap_or(b.source))
                    .to_string()
                    .into(),
                detail: format!("{}: {}", b.link.position.start.line + 1, context).into(),
                offset: Some(b.link.position.start.offset),
            });
        }
        let n = linked.len();
        linked.insert(0, PaneRow::Header(format!("Linked mentions ({n})").into()));
        self.linked = linked;

        // Outgoing: resolved then unresolved links from this file.
        let mut out = Vec::new();
        if let Some(meta) = cache.file(&path) {
            for link in meta.links.iter().chain(meta.embeds.iter()) {
                match cache.resolve(&link.link, &path) {
                    Some(target) => out.push(PaneRow::Item {
                        path: Some(target.to_string()),
                        label: display_name(target.rsplit('/').next().unwrap_or(target))
                            .to_string()
                            .into(),
                        detail: target.to_string().into(),
                        offset: None,
                    }),
                    None => out.push(PaneRow::Item {
                        path: None,
                        label: _markdown::parse_linktext(&link.link)
                            .path
                            .to_string()
                            .into(),
                        detail: "Not created yet".into(),
                        offset: None,
                    }),
                }
            }
        }
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
            self.reading
                .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
        }
    }

    fn follow_link(&mut self, target: LinkTarget, cx: &mut Context<Self>) {
        match target {
            LinkTarget::Internal(body) => {
                let Some(state) = self.state.clone() else {
                    return;
                };
                let source = self.current.clone().unwrap_or_default();
                let parsed = _markdown::parse_linktext(&body);
                let resolved = state.cache().resolve(&body, &source).map(str::to_string);
                match resolved {
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
        match std::fs::write(&absolute, contents) {
            Ok(()) => {
                self.editor.update(cx, |e, _| e.mark_saved());
                self.status = format!("{name} saved").into();
            }
            Err(err) => {
                self.status = format!("Could not save {name}: {err}").into();
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
        self.modal = Modal::Prompt;
        self.prompt.update(cx, |p, cx| p.open(spec, window, cx));
        cx.notify();
    }

    fn report(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.status = message.into();
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
            self.delete_now(path, cx);
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

    fn delete_now(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let was_current = self.current.as_deref() == Some(path)
            || self
                .current
                .as_deref()
                .is_some_and(|c| c.starts_with(&format!("{path}/")));
        if was_current {
            // Nothing may write the buffer back into a file that is gone.
            self.save_task = None;
            self.editor
                .update(cx, |e, cx| e.set_text("", String::new(), cx));
            self.reading.update(cx, |r, cx| {
                r.set_note(
                    None,
                    reading::build_with("", "", state.as_ref(), state.build_options()),
                    cx,
                )
            });
            self.current = None;
            self.words = 0;
            self.chars = 0;
        }
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
                self.delete_now(&path, cx);
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
        self.after_changes(outcome.changes, cx);
        if let Some(current) = self.current.clone() {
            self.explorer.update(cx, |e, cx| e.reveal(&current, cx));
        }
        self.report(format!("{summary}{links}"), cx);
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
                            this.reading
                                .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
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
                                        this.reading
                                            .update(cx, |r, cx| r.scroll_to_offset(offset, cx));
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
                    move |this, _, cx| {
                        this.right_tab = which;
                        cx.notify();
                    },
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

fn empty_pane(text: &'static str, theme: &Theme) -> gpui::AnyElement {
    div()
        .p_3()
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .child(text)
        .into_any_element()
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(div().text_color(theme.text).child(title)),
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
            match self.mode {
                ViewMode::Reading => self.reading.clone().into_any_element(),
                ViewMode::LivePreview | ViewMode::Source => self.editor.clone().into_any_element(),
                ViewMode::Graph => self.graph.clone().into_any_element(),
            }
        };

        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(main_body));

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
                        |this, _, cx| {
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
            .child(
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
            );

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

        let overlay: Option<gpui::AnyElement> = match self.modal {
            Modal::None => None,
            Modal::Switcher | Modal::Palette | Modal::Prompt => {
                let child: gpui::AnyElement = match self.modal {
                    Modal::Switcher => self.switcher.clone().into_any_element(),
                    Modal::Palette => self.palette.clone().into_any_element(),
                    _ => self.prompt.clone().into_any_element(),
                };
                Some(
                    div()
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
        let left_visible = self.left_visible;
        let right_visible = self.right_visible;

        div()
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::toggle_mode))
            .on_action(cx.listener(Self::go_back))
            .on_action(cx.listener(Self::go_forward))
            .on_action(cx.listener(Self::open_search_action))
            .on_action(cx.listener(Self::open_graph_action))
            .on_action(cx.listener(Self::run_command_action))
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
                    .when(left_visible, |d| d.child(left))
                    .child(main)
                    .when(right_visible, |d| {
                        d.child(self.render_right_dock(&theme, cx))
                    }),
            )
            .child(status_bar)
            .children(overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
            assert_eq!(cache.backlinks("Renamed A.md").len(), 3);
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
        assert_eq!(state.cache().backlinks("Renamed A.md").len(), 3);
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
}
