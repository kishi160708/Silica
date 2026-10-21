//! The Files view: a folder tree over the vault's file list in the left dock.
//!
//! [`FileTree`] is the pure model (ordering, collapse state, visible rows)
//! and [`FileExplorer`] the GPUI entity that draws it with a uniform list, so
//! a 20k-file vault costs one row element per visible line. Names are ordered
//! with [`natural_cmp`], the collation the search pane uses, so two lists of
//! the same files in one window cannot disagree, and the files of each
//! folder follow the leaf's [`SortOrder`]. A right click opens the file menu
//! (new note, new folder, rename, move, copy, delete); the explorer only
//! reports the choice, the workspace does the work. Above the tree sits
//! Obsidian's `nav-header`: New note, New folder, Change sort order,
//! Auto-reveal current file, and Expand all (Collapse all once a folder is
//! open).

use crate::assets::icon;
use crate::canvas::menu::tooltip;
use crate::theme::{Theme, ICON_SIZE};
use gpui::{
    anchored, deferred, div, point, prelude::*, px, uniform_list, ClickEvent, Context,
    EventEmitter, FontWeight, MouseButton, Pixels, Point, ScrollStrategy, SharedString,
    UniformListScrollHandle, Window,
};
use _formats::FileKind;
use _search::natural_cmp;
use _vault::VaultFile;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

/// The font size of a row's extension tag.
const TAG_SIZE: f32 = 10.0;

/// The header row: Obsidian's `.nav-header` is 40 px, 8 px of padding
/// around a centred row of 28 by 24 buttons 2 px apart.
const HEADER_HEIGHT: f32 = 40.0;

/// The order the files of each folder are listed in: the six values of the
/// Files leaf's `sortOrder`. Folders keep their name order whichever is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Alphabetical,
    AlphabeticalReverse,
    ByModifiedTime,
    ByModifiedTimeReverse,
    ByCreatedTime,
    ByCreatedTimeReverse,
}

impl SortOrder {
    /// Every order, in the order the menu lists them.
    pub const ALL: [SortOrder; 6] = [
        SortOrder::Alphabetical,
        SortOrder::AlphabeticalReverse,
        SortOrder::ByModifiedTime,
        SortOrder::ByModifiedTimeReverse,
        SortOrder::ByCreatedTime,
        SortOrder::ByCreatedTimeReverse,
    ];

    /// The string `workspace.json` records.
    pub fn as_str(self) -> &'static str {
        match self {
            SortOrder::Alphabetical => "alphabetical",
            SortOrder::AlphabeticalReverse => "alphabeticalReverse",
            SortOrder::ByModifiedTime => "byModifiedTime",
            SortOrder::ByModifiedTimeReverse => "byModifiedTimeReverse",
            SortOrder::ByCreatedTime => "byCreatedTime",
            SortOrder::ByCreatedTimeReverse => "byCreatedTimeReverse",
        }
    }

    pub fn parse(s: &str) -> Option<SortOrder> {
        SortOrder::ALL.into_iter().find(|o| o.as_str() == s)
    }

    /// The menu's label.
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Alphabetical => "File name (A to Z)",
            SortOrder::AlphabeticalReverse => "File name (Z to A)",
            SortOrder::ByModifiedTime => "Modified time (new to old)",
            SortOrder::ByModifiedTimeReverse => "Modified time (old to new)",
            SortOrder::ByCreatedTime => "Created time (new to old)",
            SortOrder::ByCreatedTimeReverse => "Created time (old to new)",
        }
    }

    /// How two files of one folder compare under this order. Names tie
    /// break every time sort, and the full file name breaks a tie between
    /// two display names, so the order of a rescan is stable.
    fn compare(self, a: &Entry, b: &Entry) -> Ordering {
        let by_name = |a: &Entry, b: &Entry| {
            natural_cmp(display_name(&a.name), display_name(&b.name))
                .then_with(|| natural_cmp(&a.name, &b.name))
        };
        match self {
            SortOrder::Alphabetical => by_name(a, b),
            SortOrder::AlphabeticalReverse => {
                natural_cmp(display_name(&b.name), display_name(&a.name))
                    .then_with(|| natural_cmp(&a.name, &b.name))
            }
            SortOrder::ByModifiedTime => b.mtime.cmp(&a.mtime).then_with(|| by_name(a, b)),
            SortOrder::ByModifiedTimeReverse => a.mtime.cmp(&b.mtime).then_with(|| by_name(a, b)),
            SortOrder::ByCreatedTime => b.ctime.cmp(&a.ctime).then_with(|| by_name(a, b)),
            SortOrder::ByCreatedTimeReverse => a.ctime.cmp(&b.ctime).then_with(|| by_name(a, b)),
        }
    }
}

/// One line of the tree with every folder expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Vault-relative path; folders without a trailing slash.
    pub path: String,
    /// Name as shown: a file loses its extension, which `tag` carries.
    pub name: String,
    /// The extension of a file that is not Markdown, upper-cased at the
    /// row's right end the way Obsidian's `nav-file-tag` shows it.
    pub tag: Option<String>,
    pub depth: usize,
    pub is_folder: bool,
    pub kind: FileKind,
}

/// A file as its folder holds it: what the sort orders need of it.
#[derive(Debug, Clone)]
struct Entry {
    name: String,
    kind: FileKind,
    mtime: u64,
    ctime: u64,
}

#[derive(Debug, Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<Entry>,
}

/// Folder tree with per-folder collapse state.
#[derive(Debug, Default)]
pub struct FileTree {
    root: Folder,
    sort: SortOrder,
    all: Vec<Row>,
    collapsed: HashSet<String>,
    visible: Vec<usize>,
}

impl FileTree {
    /// Builds the tree. Files of unknown kind are hidden unless
    /// `show_unsupported` (Obsidian's "Detect all file extensions");
    /// `folders` adds the empty folders the files alone would not show.
    pub fn new<'a>(
        files: impl IntoIterator<Item = &'a VaultFile>,
        folders: impl IntoIterator<Item = &'a String>,
        show_unsupported: bool,
    ) -> FileTree {
        let mut root = Folder::default();
        for path in folders {
            let mut folder = &mut root;
            for part in path.split('/').filter(|p| !p.is_empty()) {
                folder = folder.folders.entry(part.to_string()).or_default();
            }
        }
        for file in files {
            if file.kind == FileKind::Other && !show_unsupported {
                continue;
            }
            let mut parts: Vec<&str> = file.path.split('/').collect();
            let name = parts.pop().unwrap_or_default().to_string();
            let mut folder = &mut root;
            for part in parts {
                folder = folder.folders.entry(part.to_string()).or_default();
            }
            folder.files.push(Entry {
                name,
                kind: file.kind,
                mtime: file.mtime,
                ctime: file.ctime,
            });
        }
        let mut all = Vec::new();
        flatten(&root, "", 0, SortOrder::Alphabetical, &mut all);
        let collapsed = all
            .iter()
            .filter(|r| r.is_folder)
            .map(|r| r.path.clone())
            .collect();
        let mut tree = FileTree {
            root,
            sort: SortOrder::Alphabetical,
            all,
            collapsed,
            visible: Vec::new(),
        };
        tree.recompute();
        tree
    }

    pub fn sort(&self) -> SortOrder {
        self.sort
    }

    /// Re-lists every folder's files in `sort`; folders and their collapse
    /// state stay as they were.
    pub fn set_sort(&mut self, sort: SortOrder) {
        if self.sort == sort {
            return;
        }
        self.sort = sort;
        self.all.clear();
        flatten(&self.root, "", 0, sort, &mut self.all);
        self.recompute();
    }

    fn recompute(&mut self) {
        self.visible.clear();
        let mut hide_below: Option<usize> = None;
        for (ix, row) in self.all.iter().enumerate() {
            if let Some(depth) = hide_below {
                if row.depth > depth {
                    continue;
                }
                hide_below = None;
            }
            self.visible.push(ix);
            if row.is_folder && self.collapsed.contains(&row.path) {
                hide_below = Some(row.depth);
            }
        }
    }

    /// Rows currently shown, in order.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = &Row> {
        self.visible.iter().map(|&ix| &self.all[ix])
    }

    pub fn row(&self, visible_ix: usize) -> Option<&Row> {
        self.visible.get(visible_ix).map(|&ix| &self.all[ix])
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    /// Folders currently expanded, to carry over to a rebuilt tree.
    pub fn expanded_folders(&self) -> HashSet<String> {
        self.all
            .iter()
            .filter(|r| r.is_folder && !self.collapsed.contains(&r.path))
            .map(|r| r.path.clone())
            .collect()
    }

    /// Expands the given folders (unknown ones are ignored).
    pub fn expand_folders(&mut self, folders: &HashSet<String>) {
        self.collapsed.retain(|f| !folders.contains(f));
        self.recompute();
    }

    pub fn is_collapsed(&self, folder: &str) -> bool {
        self.collapsed.contains(folder)
    }

    pub fn toggle(&mut self, folder: &str) {
        if !self.collapsed.remove(folder) {
            self.collapsed.insert(folder.to_string());
        }
        self.recompute();
    }

    pub fn collapse_all(&mut self) {
        self.collapsed = self
            .all
            .iter()
            .filter(|r| r.is_folder)
            .map(|r| r.path.clone())
            .collect();
        self.recompute();
    }

    pub fn expand_all(&mut self) {
        self.collapsed.clear();
        self.recompute();
    }

    /// Whether any folder is open: the header's last button is "Collapse
    /// all" then and "Expand all" otherwise, as Obsidian's swaps.
    pub fn any_expanded(&self) -> bool {
        self.all
            .iter()
            .any(|r| r.is_folder && !self.collapsed.contains(&r.path))
    }

    /// Expands every ancestor of `path` and returns its visible row index.
    pub fn reveal(&mut self, path: &str) -> Option<usize> {
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                self.collapsed.remove(&prefix);
                prefix.push('/');
            }
            prefix.push_str(part);
        }
        self.recompute();
        self.visible_index(path)
    }

    pub fn visible_index(&self, path: &str) -> Option<usize> {
        self.visible
            .iter()
            .position(|&ix| self.all[ix].path == path)
    }
}

fn flatten(folder: &Folder, prefix: &str, depth: usize, sort: SortOrder, out: &mut Vec<Row>) {
    let mut names: Vec<&String> = folder.folders.keys().collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    for name in names {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        out.push(Row {
            path: path.clone(),
            name: name.clone(),
            depth,
            tag: None,
            is_folder: true,
            kind: FileKind::Other,
        });
        flatten(&folder.folders[name], &path, depth + 1, sort, out);
    }
    let mut files: Vec<&Entry> = folder.files.iter().collect();
    files.sort_by(|a, b| sort.compare(a, b));
    for entry in files {
        let name = &entry.name;
        out.push(Row {
            path: if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            },
            name: stem(name).to_string(),
            tag: file_tag(name),
            depth,
            is_folder: false,
            kind: entry.kind,
        });
    }
}

/// The name a [`FileKind`] goes by outside the app: in the control socket's
/// state dump, and anywhere else a kind has to survive as text.
pub fn kind_name(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Markdown => "markdown",
        FileKind::Canvas => "canvas",
        FileKind::Base => "base",
        FileKind::Image => "image",
        FileKind::Audio => "audio",
        FileKind::Video => "video",
        FileKind::Pdf => "pdf",
        FileKind::Other => "other",
    }
}

/// A file's name without its extension, for the explorer's rows: Obsidian
/// shows every file bare and tags the ones that are not Markdown.
fn stem(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => stem,
        _ => file_name,
    }
}

/// The row's tag for a file that is not Markdown: its extension in upper
/// case (`CANVAS`, `PNG`); none for a note or a file without one.
fn file_tag(file_name: &str) -> Option<String> {
    match file_name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty() && !ext.is_empty() && !ext.eq_ignore_ascii_case("md") =>
        {
            Some(ext.to_ascii_uppercase())
        }
        _ => None,
    }
}

/// Markdown files show without their extension, everything else with it.
pub fn display_name(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("md") && !stem.is_empty() => stem,
        _ => file_name,
    }
}

pub enum ExplorerEvent {
    Open(String),
    /// A Ctrl+click: the file goes to a new tab.
    OpenInNewTab(String),
    /// The header's "New note": what the `file-explorer:new-file` command
    /// does, the folder following `newFileLocation`.
    NewNoteFromHeader,
    /// Context-menu actions; paths are vault-relative, folders without a
    /// trailing slash (`""` is the vault root). The header's "New folder"
    /// is a `NewFolder` at the root.
    NewNote {
        folder: String,
    },
    NewFolder {
        parent: String,
    },
    Rename(String),
    Move(String),
    Duplicate(String),
    Delete(String),
    /// "Bookmark...": a file or folder for the Bookmarks pane.
    Bookmark(String),
}

/// The right-click menu over a row.
struct Menu {
    position: Point<Pixels>,
    row: Row,
}

pub struct FileExplorer {
    tree: FileTree,
    active: Option<String>,
    /// The leaf's `autoReveal`: whether opening a file expands its folders
    /// and scrolls to its row, or only highlights the row if it is on
    /// screen. Off in a fresh vault, as in Obsidian.
    auto_reveal: bool,
    scroll: UniformListScrollHandle,
    menu: Option<Menu>,
    /// The sort menu is open under its header button.
    sort_menu_open: bool,
}

impl EventEmitter<ExplorerEvent> for FileExplorer {}

impl FileExplorer {
    pub fn new(_cx: &mut Context<Self>) -> FileExplorer {
        FileExplorer {
            tree: FileTree::default(),
            active: None,
            auto_reveal: false,
            scroll: UniformListScrollHandle::new(),
            menu: None,
            sort_menu_open: false,
        }
    }

    /// A rebuilt tree: the folders that were open stay open, the sort order
    /// carries over and the open file keeps its highlight. Nothing is
    /// expanded or scrolled, so a file written elsewhere in the vault does
    /// not move the list.
    pub fn set_tree(&mut self, mut tree: FileTree, cx: &mut Context<Self>) {
        let expanded = self.tree.expanded_folders();
        tree.expand_folders(&expanded);
        tree.set_sort(self.tree.sort());
        self.tree = tree;
        cx.notify();
    }

    /// The leaf's state as `workspace.json` recorded it.
    pub fn set_leaf_state(&mut self, sort: SortOrder, auto_reveal: bool, cx: &mut Context<Self>) {
        self.tree.set_sort(sort);
        self.auto_reveal = auto_reveal;
        cx.notify();
    }

    pub fn sort_order(&self) -> SortOrder {
        self.tree.sort()
    }

    pub fn auto_reveal(&self) -> bool {
        self.auto_reveal
    }

    fn set_sort(&mut self, sort: SortOrder, cx: &mut Context<Self>) {
        self.tree.set_sort(sort);
        self.sort_menu_open = false;
        cx.notify();
    }

    /// "Auto-reveal current file": switching it on reveals the open file
    /// right away, as Obsidian does.
    fn toggle_auto_reveal(&mut self, cx: &mut Context<Self>) {
        self.auto_reveal = !self.auto_reveal;
        if self.auto_reveal {
            if let Some(active) = self.active.clone() {
                self.reveal(&active, cx);
            }
        }
        cx.notify();
    }

    /// "Expand all" opens every folder at every depth; "Collapse all", the
    /// same button once any folder is open, closes them all.
    fn toggle_all(&mut self, cx: &mut Context<Self>) {
        if self.tree.any_expanded() {
            self.tree.collapse_all();
        } else {
            self.tree.expand_all();
        }
        cx.notify();
    }

    /// The file that just became the open one: highlighted, and with
    /// auto-reveal on, also expanded into view.
    pub fn follow(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.auto_reveal {
            self.reveal(path, cx);
        } else {
            self.set_active(path, cx);
        }
    }

    /// Marks `path` as the open file without touching the tree: its row is
    /// highlighted if a folder is not hiding it.
    pub fn set_active(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.active.as_deref() != Some(path) {
            self.active = Some(path.to_string());
            cx.notify();
        }
    }

    /// No file is open: nothing is highlighted.
    pub fn clear_active(&mut self, cx: &mut Context<Self>) {
        if self.active.take().is_some() {
            cx.notify();
        }
    }

    /// Marks `path` as the open file and scrolls it into view.
    pub fn reveal(&mut self, path: &str, cx: &mut Context<Self>) {
        self.active = Some(path.to_string());
        if let Some(ix) = self.tree.reveal(path) {
            self.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        }
        cx.notify();
    }

    /// What the tree is showing, for the control socket's state dump: the
    /// rows a collapsed folder hides are not in it, because they are not on
    /// screen either.
    pub fn describe(&self, rows: usize) -> Value {
        let total = self.tree.len();
        let shown: Vec<Value> = self
            .tree
            .rows()
            .take(rows)
            .map(|row| {
                json!({
                    "path": row.path,
                    "name": row.name,
                    "depth": row.depth,
                    "folder": row.is_folder,
                    "kind": kind_name(row.kind),
                })
            })
            .collect();
        json!({
            "active": self.active,
            "sort_order": self.tree.sort().as_str(),
            "auto_reveal": self.auto_reveal,
            "any_expanded": self.tree.any_expanded(),
            "rows": shown,
            "rows_total": if total > shown.len() { Some(total) } else { None },
        })
    }

    fn click(&mut self, ix: usize, new_tab: bool, cx: &mut Context<Self>) {
        let Some(row) = self.tree.row(ix).cloned() else {
            return;
        };
        if row.is_folder {
            self.tree.toggle(&row.path);
            cx.notify();
        } else if new_tab {
            cx.emit(ExplorerEvent::OpenInNewTab(row.path));
        } else {
            cx.emit(ExplorerEvent::Open(row.path));
        }
    }

    fn open_menu(&mut self, ix: usize, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(row) = self.tree.row(ix).cloned() {
            self.menu = Some(Menu { position, row });
            cx.notify();
        }
    }

    fn close_menu(&mut self, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            cx.notify();
        }
    }

    fn render_menu(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.menu.as_ref()?;
        let theme = cx.global::<Theme>().clone();
        let row = menu.row.clone();
        let folder = if row.is_folder {
            row.path.clone()
        } else {
            row.path
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_string())
                .unwrap_or_default()
        };
        let items: Vec<(&'static str, ExplorerEvent)> = if row.is_folder {
            vec![
                (
                    "New note",
                    ExplorerEvent::NewNote {
                        folder: folder.clone(),
                    },
                ),
                (
                    "New folder",
                    ExplorerEvent::NewFolder {
                        parent: folder.clone(),
                    },
                ),
                ("Rename...", ExplorerEvent::Rename(row.path.clone())),
                ("Move folder to...", ExplorerEvent::Move(row.path.clone())),
                ("Bookmark...", ExplorerEvent::Bookmark(row.path.clone())),
                ("Delete", ExplorerEvent::Delete(row.path.clone())),
            ]
        } else {
            vec![
                (
                    "New note",
                    ExplorerEvent::NewNote {
                        folder: folder.clone(),
                    },
                ),
                (
                    "New folder",
                    ExplorerEvent::NewFolder {
                        parent: folder.clone(),
                    },
                ),
                ("Rename...", ExplorerEvent::Rename(row.path.clone())),
                ("Move file to...", ExplorerEvent::Move(row.path.clone())),
                ("Make a copy", ExplorerEvent::Duplicate(row.path.clone())),
                ("Bookmark...", ExplorerEvent::Bookmark(row.path.clone())),
                ("Delete", ExplorerEvent::Delete(row.path.clone())),
            ]
        };
        let mut list = div()
            .id("explorer-menu")
            .flex()
            .flex_col()
            .min_w(px(180.))
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(theme.text)
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu(cx)));
        for (ix, (label, event)) in items.into_iter().enumerate() {
            let event = std::cell::Cell::new(Some(event));
            list = list.child(
                div()
                    .id(("explorer-menu-item", ix))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.close_menu(cx);
                        if let Some(event) = event.take() {
                            cx.emit(event);
                        }
                    }))
                    .child(label),
            );
        }
        Some(
            deferred(
                anchored()
                    .position(menu.position)
                    .snap_to_window_with_margin(px(8.))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    /// A header button: Obsidian's `nav-action-button`, 28 by 24 with a
    /// 4 px radius, the icon muted until the pointer is over it; `active`
    /// is the wash and accent of an `is-active` one.
    fn header_button(
        id: &'static str,
        icon_name: &'static str,
        label: &'static str,
        active: bool,
        theme: &Theme,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex_shrink_0()
            .w(px(28.))
            .h(px(24.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .cursor_pointer()
            .when(active, |d| d.bg(theme.bg_alt))
            .hover(|s| s.bg(theme.bg_alt))
            .tooltip(tooltip(label))
            .child(icon(
                icon_name,
                ICON_SIZE,
                if active {
                    theme.accent
                } else {
                    theme.text_muted
                },
            ))
    }

    fn render_header(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = cx.global::<Theme>().clone();
        let any_expanded = self.tree.any_expanded();
        let (fold_id, fold_icon, fold_label) = if any_expanded {
            ("explorer-collapse-all", "chevrons-down-up", "Collapse all")
        } else {
            ("explorer-expand-all", "chevrons-up-down", "Expand all")
        };
        let sort_menu = self.render_sort_menu(&theme, cx);
        div()
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .p(px(8.))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(2.))
            .child(
                Self::header_button("explorer-new-note", "square-pen", "New note", false, &theme)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ExplorerEvent::NewNoteFromHeader))),
            )
            .child(
                Self::header_button(
                    "explorer-new-folder",
                    "folder-plus",
                    "New folder",
                    false,
                    &theme,
                )
                .on_click(cx.listener(|_, _, _, cx| {
                    cx.emit(ExplorerEvent::NewFolder {
                        parent: String::new(),
                    })
                })),
            )
            .child(
                // The menu hangs off this wrapper, so it opens under the
                // button wherever the dock is.
                div()
                    .relative()
                    .child(
                        Self::header_button(
                            "explorer-sort",
                            "arrow-up-narrow-wide",
                            "Change sort order",
                            false,
                            &theme,
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                this.sort_menu_open = !this.sort_menu_open;
                                cx.notify();
                            },
                        )),
                    )
                    .children(sort_menu),
            )
            .child(
                Self::header_button(
                    "explorer-auto-reveal",
                    "gallery-vertical",
                    "Auto-reveal current file",
                    self.auto_reveal,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_auto_reveal(cx))),
            )
            .child(
                Self::header_button(fold_id, fold_icon, fold_label, false, &theme)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_all(cx))),
            )
    }

    /// "Change sort order": the six orders in three pairs, a check against
    /// the one in force.
    fn render_sort_menu(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if !self.sort_menu_open {
            return None;
        }
        let current = self.tree.sort();
        let mut list = div()
            .id("explorer-sort-menu")
            .flex()
            .flex_col()
            .min_w(px(220.))
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(theme.text)
            // Keeps a click on an item from reaching the row under it.
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.sort_menu_open = false;
                cx.notify();
            }));
        for (ix, order) in SortOrder::ALL.into_iter().enumerate() {
            if ix > 0 && ix % 2 == 0 {
                list = list.child(div().my_1().h(px(1.)).bg(theme.border));
            }
            let checked = order == current;
            list = list.child(
                div()
                    .id(("explorer-sort-item", ix))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_sort(order, cx)))
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .children(checked.then(|| icon("check", 14., theme.text_muted))),
                    )
                    .child(order.label()),
            );
        }
        Some(
            deferred(
                anchored()
                    .offset(point(px(0.), px(26.)))
                    .snap_to_window_with_margin(px(8.))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    fn render_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.tree.row(ix)?;
                let active = self.active.as_deref() == Some(row.path.as_str());
                let marker = row.is_folder.then(|| {
                    if self.tree.is_collapsed(&row.path) {
                        "chevron-right"
                    } else {
                        "chevron-down"
                    }
                });
                let name: SharedString = row.name.clone().into();
                let tag: Option<SharedString> = row.tag.clone().map(Into::into);
                // A file no view can open stays faint; a canvas or an image
                // reads like a note, its tag saying what it is.
                let faint = !row.is_folder && row.kind == FileKind::Other;
                // Obsidian's row: 13 px text in a 25 px row with an 8 px
                // radius, muted until it is active or under the pointer.
                Some(
                    div()
                        .id(("row", ix))
                        .w_full()
                        .h(px(25.))
                        .mb(px(2.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .pl(px(8. + row.depth as f32 * 14.))
                        .pr_2()
                        .rounded(px(8.))
                        .cursor_pointer()
                        .text_size(px(13.))
                        .text_color(if faint {
                            theme.text_faint
                        } else if active {
                            theme.text
                        } else {
                            theme.text_muted
                        })
                        .when(active, |d| d.bg(theme.bg_active))
                        .hover(|s| s.bg(theme.bg_alt).text_color(theme.text))
                        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                            let modifiers = event.modifiers();
                            this.click(ix, modifiers.control || modifiers.platform, cx)
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                this.open_menu(ix, event.position, cx)
                            }),
                        )
                        .child(
                            div()
                                .w(px(16.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .opacity(0.85)
                                .children(
                                    marker.map(|name| {
                                        crate::assets::icon(name, 14., theme.text_muted)
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(name),
                        )
                        .children(tag.map(|tag| {
                            // Obsidian's `nav-file-tag`: small upper-case
                            // text at the row's right end.
                            div()
                                .flex_shrink_0()
                                .ml_2()
                                .text_size(px(TAG_SIZE))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.text_faint)
                                .child(tag)
                        })),
                )
            })
            .collect()
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.tree.len();
        let menu = self.render_menu(cx);
        let header = self.render_header(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                uniform_list("file-explorer", count, cx.processor(Self::render_rows))
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .w_full()
                    .min_h(px(0.))
                    .px_1(),
            )
            .children(menu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<VaultFile> {
        paths
            .iter()
            .map(|p| VaultFile {
                path: p.to_string(),
                kind: FileKind::from_path(p),
                size: 0,
                mtime: 0,
                ctime: 0,
            })
            .collect()
    }

    /// The orders Obsidian showed on 2026-10-21 for a vault whose files
    /// were created in one order and modified in another (the
    /// `explorer-header` probe): folders stay in name order under every
    /// sort, and only the files of each folder move.
    #[test]
    fn the_six_sort_orders_move_files_but_not_folders() {
        let created = [
            "Alpha.md",
            "beta.md",
            "Gamma.md",
            "n2.md",
            "n10.md",
            "Projects/Plan.md",
            "Projects/Deep/Inner.md",
            "Archive/old.md",
            "Projects/Budget.md",
            "cover.png",
        ];
        let modified = [
            "n10.md",
            "Archive/old.md",
            "Alpha.md",
            "Projects/Budget.md",
            "cover.png",
            "Gamma.md",
            "Projects/Deep/Inner.md",
            "beta.md",
            "Projects/Plan.md",
            "n2.md",
        ];
        let files: Vec<VaultFile> = created
            .iter()
            .enumerate()
            .map(|(ix, p)| VaultFile {
                path: p.to_string(),
                kind: FileKind::from_path(p),
                size: 0,
                mtime: 1 + modified.iter().position(|m| m == p).unwrap() as u64,
                ctime: 1 + ix as u64,
            })
            .collect();
        let empty = vec!["Empty".to_string()];
        let mut tree = FileTree::new(&files, &empty, false);
        tree.expand_all();
        let names = |tree: &FileTree| -> Vec<String> {
            tree.rows()
                .map(|r| {
                    if r.is_folder {
                        format!("{}/", r.name)
                    } else {
                        r.name.clone()
                    }
                })
                .collect()
        };
        let observed: [(SortOrder, &[&str]); 6] = [
            (
                SortOrder::Alphabetical,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Budget",
                    "Plan",
                    "Alpha",
                    "beta",
                    "cover",
                    "Gamma",
                    "n2",
                    "n10",
                ],
            ),
            (
                SortOrder::AlphabeticalReverse,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Plan",
                    "Budget",
                    "n10",
                    "n2",
                    "Gamma",
                    "cover",
                    "beta",
                    "Alpha",
                ],
            ),
            (
                SortOrder::ByModifiedTime,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Plan",
                    "Budget",
                    "n2",
                    "beta",
                    "Gamma",
                    "cover",
                    "Alpha",
                    "n10",
                ],
            ),
            (
                SortOrder::ByModifiedTimeReverse,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Budget",
                    "Plan",
                    "n10",
                    "Alpha",
                    "cover",
                    "Gamma",
                    "beta",
                    "n2",
                ],
            ),
            (
                SortOrder::ByCreatedTime,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Budget",
                    "Plan",
                    "cover",
                    "n10",
                    "n2",
                    "Gamma",
                    "beta",
                    "Alpha",
                ],
            ),
            (
                SortOrder::ByCreatedTimeReverse,
                &[
                    "Archive/",
                    "old",
                    "Empty/",
                    "Projects/",
                    "Deep/",
                    "Inner",
                    "Plan",
                    "Budget",
                    "Alpha",
                    "beta",
                    "Gamma",
                    "n2",
                    "n10",
                    "cover",
                ],
            ),
        ];
        for (order, rows) in observed {
            tree.set_sort(order);
            assert_eq!(names(&tree), rows, "{}", order.as_str());
            assert_eq!(SortOrder::parse(order.as_str()), Some(order));
        }
        // A collapsed folder stays collapsed across a re-sort.
        tree.toggle("Projects");
        tree.set_sort(SortOrder::Alphabetical);
        assert!(tree.is_collapsed("Projects"));
        assert!(tree.any_expanded(), "Archive is still open");
        tree.collapse_all();
        assert!(!tree.any_expanded());
    }

    #[test]
    fn folders_first_then_files_case_insensitively() {
        let files = files(&[
            "zeta.md",
            "Alpha.md",
            "b/inner.md",
            "a/x.png",
            "a/deep/y.md",
            "notes.zip",
        ]);
        let tree = FileTree::new(&files, &[], false);
        let names: Vec<(usize, &str)> = tree.rows().map(|r| (r.depth, r.name.as_str())).collect();
        assert_eq!(names, vec![(0, "a"), (0, "b"), (0, "Alpha"), (0, "zeta")]);
        let empty = vec!["Empty".to_string(), "a/void".to_string()];
        let with_empty = FileTree::new(&files, &empty, false);
        let names: Vec<&str> = with_empty.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "Empty", "Alpha", "zeta"]);
        let shown = FileTree::new(&files, &[], true);
        assert_eq!(
            shown.len(),
            5,
            "notes.zip appears with unsupported files on"
        );
    }

    /// Obsidian collates names numerically, so `n2` comes before `n10`; the
    /// search pane already does and the two panes have to agree.
    #[test]
    fn names_sort_numerically_within_each_group() {
        let files = files(&[
            "n10.md",
            "n2.md",
            "n1.md",
            "n20.md",
            "n3.md",
            "f10/a.md",
            "f2/a.md",
            "Img 10.png",
            "Img 2.png",
        ]);
        let tree = FileTree::new(&files, &[], false);
        let names: Vec<&str> = tree.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["f2", "f10", "Img 2", "Img 10", "n1", "n2", "n3", "n10", "n20"],
            "folders stay ahead of files, and both groups count"
        );
    }

    #[test]
    fn toggle_and_reveal_expand_ancestors() {
        let files = files(&["a/deep/y.md", "a/x.png", "b/inner.md"]);
        let mut tree = FileTree::new(&files, &[], false);
        assert_eq!(tree.len(), 2);
        tree.toggle("a");
        let names: Vec<&str> = tree.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "deep", "x", "b"]);
        tree.toggle("a");
        assert_eq!(tree.reveal("a/deep/y.md"), Some(2));
        let names: Vec<&str> = tree.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "deep", "y", "x", "b"]);
        assert!(tree.rows().nth(2).unwrap().path == "a/deep/y.md");
        tree.collapse_all();
        assert_eq!(tree.len(), 2);
        tree.expand_all();
        assert_eq!(tree.len(), 6);
        let expanded = tree.expanded_folders();
        let mut rebuilt = FileTree::new(&files, &[], false);
        rebuilt.expand_folders(&expanded);
        assert_eq!(rebuilt.len(), 6);
    }
}
