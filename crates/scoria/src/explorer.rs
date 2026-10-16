//! The Files view: a folder tree over the vault's file list in the left dock.
//!
//! [`FileTree`] is the pure model (ordering, collapse state, visible rows)
//! and [`FileExplorer`] the GPUI entity that draws it with a uniform list, so
//! a 20k-file vault costs one row element per visible line.

use crate::theme::Theme;
use gpui::{
    div, prelude::*, px, uniform_list, Context, EventEmitter, ScrollStrategy, SharedString,
    UniformListScrollHandle, Window,
};
use _formats::FileKind;
use _vault::VaultFile;
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

/// One line of the tree with every folder expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Vault-relative path; folders without a trailing slash.
    pub path: String,
    /// Name as shown: markdown files lose `.md`, folders and attachments keep their name.
    pub name: String,
    pub depth: usize,
    pub is_folder: bool,
    pub kind: FileKind,
}

#[derive(Debug, Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<(String, FileKind)>,
}

/// Folder tree with per-folder collapse state.
#[derive(Debug, Default)]
pub struct FileTree {
    all: Vec<Row>,
    collapsed: HashSet<String>,
    visible: Vec<usize>,
}

fn sort_key(name: &str) -> (String, String) {
    (name.to_lowercase(), name.to_string())
}

impl FileTree {
    /// Builds the tree. Files of unknown kind are hidden unless
    /// `show_unsupported` (Obsidian's "Detect all file extensions").
    pub fn new<'a>(
        files: impl IntoIterator<Item = &'a VaultFile>,
        show_unsupported: bool,
    ) -> FileTree {
        let mut root = Folder::default();
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
            folder.files.push((name, file.kind));
        }
        let mut all = Vec::new();
        flatten(&root, "", 0, &mut all);
        let collapsed = all
            .iter()
            .filter(|r| r.is_folder)
            .map(|r| r.path.clone())
            .collect();
        let mut tree = FileTree {
            all,
            collapsed,
            visible: Vec::new(),
        };
        tree.recompute();
        tree
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
    #[cfg(test)]
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

    #[cfg(test)]
    pub fn collapse_all(&mut self) {
        self.collapsed = self
            .all
            .iter()
            .filter(|r| r.is_folder)
            .map(|r| r.path.clone())
            .collect();
        self.recompute();
    }

    #[cfg(test)]
    pub fn expand_all(&mut self) {
        self.collapsed.clear();
        self.recompute();
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

fn flatten(folder: &Folder, prefix: &str, depth: usize, out: &mut Vec<Row>) {
    let mut names: Vec<&String> = folder.folders.keys().collect();
    names.sort_by_key(|n| sort_key(n));
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
            is_folder: true,
            kind: FileKind::Other,
        });
        flatten(&folder.folders[name], &path, depth + 1, out);
    }
    let mut files: Vec<&(String, FileKind)> = folder.files.iter().collect();
    files.sort_by_key(|(n, _)| sort_key(display_name(n)));
    for (name, kind) in files {
        out.push(Row {
            path: if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            },
            name: display_name(name).to_string(),
            depth,
            is_folder: false,
            kind: *kind,
        });
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
}

pub struct FileExplorer {
    tree: FileTree,
    active: Option<String>,
    scroll: UniformListScrollHandle,
}

impl EventEmitter<ExplorerEvent> for FileExplorer {}

impl FileExplorer {
    pub fn new(_cx: &mut Context<Self>) -> FileExplorer {
        FileExplorer {
            tree: FileTree::default(),
            active: None,
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub fn set_tree(&mut self, mut tree: FileTree, cx: &mut Context<Self>) {
        let expanded = self.tree.expanded_folders();
        tree.expand_folders(&expanded);
        self.tree = tree;
        if let Some(active) = self.active.clone() {
            self.reveal(&active, cx);
        }
        cx.notify();
    }

    /// Marks `path` as the open file and scrolls it into view.
    pub fn reveal(&mut self, path: &str, cx: &mut Context<Self>) {
        self.active = Some(path.to_string());
        if let Some(ix) = self.tree.reveal(path) {
            self.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        }
        cx.notify();
    }

    fn click(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(row) = self.tree.row(ix).cloned() else {
            return;
        };
        if row.is_folder {
            self.tree.toggle(&row.path);
            cx.notify();
        } else {
            cx.emit(ExplorerEvent::Open(row.path));
        }
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
                let marker: &'static str = if row.is_folder {
                    if self.tree.is_collapsed(&row.path) {
                        "▸"
                    } else {
                        "▾"
                    }
                } else {
                    ""
                };
                let name: SharedString = row.name.clone().into();
                let faint = !row.is_folder && row.kind != FileKind::Markdown;
                Some(
                    div()
                        .id(("row", ix))
                        .h(px(24.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .pl(px(8. + row.depth as f32 * 14.))
                        .pr_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_size(px(13.))
                        .text_color(if faint { theme.text_muted } else { theme.text })
                        .when(active, |d| d.bg(theme.bg_active))
                        .hover(|s| s.bg(theme.bg_alt))
                        .on_click(cx.listener(move |this, _, _, cx| this.click(ix, cx)))
                        .child(
                            div()
                                .w(px(14.))
                                .flex_shrink_0()
                                .text_color(theme.text_muted)
                                .child(marker),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(name),
                        ),
                )
            })
            .collect()
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.tree.len();
        uniform_list("file-explorer", count, cx.processor(Self::render_rows))
            .track_scroll(&self.scroll)
            .size_full()
            .px_1()
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
            })
            .collect()
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
        let tree = FileTree::new(&files, false);
        let names: Vec<(usize, &str)> = tree.rows().map(|r| (r.depth, r.name.as_str())).collect();
        assert_eq!(names, vec![(0, "a"), (0, "b"), (0, "Alpha"), (0, "zeta")]);
        let shown = FileTree::new(&files, true);
        assert_eq!(
            shown.len(),
            5,
            "notes.zip appears with unsupported files on"
        );
    }

    #[test]
    fn toggle_and_reveal_expand_ancestors() {
        let files = files(&["a/deep/y.md", "a/x.png", "b/inner.md"]);
        let mut tree = FileTree::new(&files, false);
        assert_eq!(tree.len(), 2);
        tree.toggle("a");
        let names: Vec<&str> = tree.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "deep", "x.png", "b"]);
        tree.toggle("a");
        assert_eq!(tree.reveal("a/deep/y.md"), Some(2));
        let names: Vec<&str> = tree.rows().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "deep", "y", "x.png", "b"]);
        assert!(tree.rows().nth(2).unwrap().path == "a/deep/y.md");
        tree.collapse_all();
        assert_eq!(tree.len(), 2);
        tree.expand_all();
        assert_eq!(tree.len(), 6);
        let expanded = tree.expanded_folders();
        let mut rebuilt = FileTree::new(&files, false);
        rebuilt.expand_folders(&expanded);
        assert_eq!(rebuilt.len(), 6);
    }
}
