//! The workspace shell: left dock (Files), main area (reading view or source
//! editor), right dock (Backlinks, Outline, Tags), view header and status bar.
//!
//! The vault is indexed on a background thread when the window opens; until
//! then the shell shows a loading line and every pane is empty.

use crate::editor::Editor;
use crate::explorer::{display_name, ExplorerEvent, FileExplorer, FileTree};
use crate::reading::{self, LinkTarget, ReadingEvent, ReadingView};
use crate::search::{SearchEvent, SearchPane};
use crate::switcher::{QuickSwitcher, SwitcherEvent};
use crate::theme::Theme;
use gpui::{
    actions, div, prelude::*, px, uniform_list, App, Context, Entity, FocusHandle, Focusable,
    KeyBinding, SharedString, Subscription, UniformListScrollHandle, Window,
};
use rayon::prelude::*;
use _cache::{index_markdown, MetadataCache};
use _formats::config::AppConfig;
use _formats::FileKind;
use _markdown::Subpath;
use _vault::Vault;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

actions!(
    workspace,
    [ToggleMode, GoBack, GoForward, OpenSwitcher, OpenSearch]
);

/// Binds the shell's keys; call once at startup.
pub fn bind_keys(cx: &mut App) {
    let ctx = Some("Workspace");
    cx.bind_keys([
        KeyBinding::new("ctrl-e", ToggleMode, ctx),
        KeyBinding::new("ctrl-alt-left", GoBack, ctx),
        KeyBinding::new("ctrl-alt-right", GoForward, ctx),
        KeyBinding::new("ctrl-o", OpenSwitcher, ctx),
        KeyBinding::new("ctrl-shift-f", OpenSearch, ctx),
    ]);
}

/// Everything derived from the vault folder, built once off the UI thread.
pub struct VaultState {
    pub vault: Vault,
    pub cache: MetadataCache,
    /// Note text as indexed, for search; files are re-read when opened.
    pub texts: HashMap<String, Arc<String>>,
    pub app: AppConfig,
    pub index_ms: f64,
}

impl VaultState {
    pub fn load(root: &PathBuf) -> Result<VaultState, String> {
        let started = Instant::now();
        let vault = Vault::open(root).map_err(|e| e.to_string())?;
        let app = vault.app_config().unwrap_or_default();
        let notes: Vec<String> = vault.markdown_files().map(|f| f.path.clone()).collect();
        let loaded: Vec<(String, _cache::CachedMetadata, Arc<String>)> = notes
            .par_iter()
            .map(|path| {
                let text = std::fs::read_to_string(vault.absolute(path)).unwrap_or_default();
                let meta = index_markdown(&text);
                (path.clone(), meta, Arc::new(text))
            })
            .collect();
        let mut texts = HashMap::with_capacity(loaded.len());
        let metas: Vec<(String, _cache::CachedMetadata)> = loaded
            .into_iter()
            .map(|(path, meta, text)| {
                texts.insert(path.clone(), text);
                (path, meta)
            })
            .collect();
        let cache = MetadataCache::new(vault.link_resolver(), metas);
        Ok(VaultState {
            vault,
            cache,
            texts,
            app,
            index_ms: started.elapsed().as_secs_f64() * 1000.0,
        })
    }
}

impl VaultState {
    pub fn build_options(&self) -> reading::BuildOptions {
        reading::BuildOptions {
            strict_line_breaks: self.app.strict_line_breaks.unwrap_or(false),
        }
    }
}

impl reading::EmbedSource for VaultState {
    fn resolve(&self, linktext: &str, source_path: &str) -> Option<String> {
        self.cache
            .resolve(linktext, source_path)
            .map(str::to_string)
    }

    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.vault.absolute(path)).ok()
    }

    fn absolute(&self, path: &str) -> PathBuf {
        self.vault.absolute(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Reading,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Files,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Backlinks,
    Outline,
    Tags,
}

#[derive(Clone)]
struct OutlineRow {
    level: u8,
    text: SharedString,
    offset: usize,
}

#[derive(Clone)]
struct BacklinkRow {
    source: String,
    name: SharedString,
    context: SharedString,
    line: u32,
}

#[derive(Clone)]
struct TagRow {
    name: SharedString,
    count: usize,
}

pub struct Workspace {
    focus_handle: FocusHandle,
    state: Option<Arc<VaultState>>,
    vault_name: SharedString,
    loading: Option<SharedString>,
    explorer: Entity<FileExplorer>,
    search: Entity<SearchPane>,
    switcher: Entity<QuickSwitcher>,
    switcher_open: bool,
    left_tab: LeftTab,
    reading: Entity<ReadingView>,
    editor: Entity<Editor>,
    mode: ViewMode,
    current: Option<String>,
    history: Vec<String>,
    history_ix: usize,
    right_tab: RightTab,
    outline: Vec<OutlineRow>,
    backlinks: Vec<BacklinkRow>,
    tags: Vec<TagRow>,
    tag_scroll: UniformListScrollHandle,
    status: SharedString,
    words: usize,
    chars: usize,
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
        let reading = cx.new(ReadingView::new);
        let editor = cx.new(Editor::new);
        let subscriptions = vec![
            cx.subscribe(
                &explorer,
                |this, _, event: &ExplorerEvent, cx| match event {
                    ExplorerEvent::Open(path) => this.open_file(path.clone(), None, cx),
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
                        this.close_switcher(window, cx);
                        this.open_file(path.clone(), None, cx);
                    }
                    SwitcherEvent::Close => this.close_switcher(window, cx),
                },
            ),
            cx.subscribe(&reading, |this, _, event: &ReadingEvent, cx| match event {
                ReadingEvent::OpenLink(target) => this.follow_link(target.clone(), cx),
            }),
        ];
        let mut ws = Workspace {
            focus_handle: cx.focus_handle(),
            state: None,
            vault_name: "No vault".into(),
            loading: None,
            explorer,
            search,
            switcher,
            switcher_open: false,
            left_tab: LeftTab::Files,
            reading,
            editor,
            mode: ViewMode::Reading,
            current: None,
            history: Vec::new(),
            history_ix: 0,
            right_tab: RightTab::Backlinks,
            outline: Vec::new(),
            backlinks: Vec::new(),
            tags: Vec::new(),
            tag_scroll: UniformListScrollHandle::new(),
            status: "Pass a vault folder as the first argument".into(),
            words: 0,
            chars: 0,
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
        let show_unsupported = state.app.show_unsupported_files.unwrap_or(false);
        let tree = FileTree::new(state.vault.files(), show_unsupported);
        self.explorer.update(cx, |e, cx| e.set_tree(tree, cx));
        let readable = state.app.readable_line_length.unwrap_or(true);
        self.reading
            .update(cx, |r, _| r.set_readable_line_length(readable));
        self.tags = state
            .cache
            .tags()
            .values()
            .map(|t| TagRow {
                name: t.name.clone().into(),
                count: t.count,
            })
            .collect();
        self.status = format!(
            "{} files, {} notes indexed in {:.0} ms",
            state.vault.files().len(),
            state.cache.len(),
            state.index_ms
        )
        .into();
        let state = Arc::new(state);
        self.search
            .update(cx, |s, cx| s.set_state(state.clone(), cx));
        self.switcher
            .update(cx, |s, cx| s.set_state(state.clone(), cx));
        self.state = Some(state);
        if let Some(current) = self.current.clone() {
            self.open_file(current, None, cx);
        }
        cx.notify();
    }

    pub fn is_loaded(&self) -> bool {
        self.state.is_some()
    }

    pub fn editor(&self) -> &Entity<Editor> {
        &self.editor
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
        self.switcher_open = true;
        self.switcher
            .update(cx, |s, cx| s.open(recent, query, window, cx));
        cx.notify();
    }

    fn close_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.switcher_open {
            self.switcher_open = false;
            match self.mode {
                ViewMode::Reading => window.focus(&self.reading.focus_handle(cx), cx),
                ViewMode::Source => window.focus(&self.editor.focus_handle(cx), cx),
            }
            cx.notify();
        }
    }

    fn open_switcher_action(
        &mut self,
        _: &OpenSwitcher,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_switcher("", window, cx);
    }

    fn open_search_action(&mut self, _: &OpenSearch, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.search.read(cx).query(cx);
        self.show_search(&query, Some(window), cx);
    }

    pub fn set_mode(&mut self, mode: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        match mode {
            ViewMode::Reading => window.focus(&self.reading.focus_handle(cx), cx),
            ViewMode::Source => window.focus(&self.editor.focus_handle(cx), cx),
        }
        cx.notify();
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
            .vault
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
            .and_then(|s| s.vault.markdown_files().next().map(|f| f.path.clone()));
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
        let kind = FileKind::from_path(path);
        match kind {
            FileKind::Markdown => match std::fs::read_to_string(state.vault.absolute(path)) {
                Ok(text) => {
                    let started = Instant::now();
                    let model =
                        reading::build_with(path, &text, state.as_ref(), state.build_options());
                    let build_ms = started.elapsed().as_secs_f64() * 1000.0;
                    self.words = text.split_whitespace().count();
                    self.chars = text.chars().count();
                    self.reading
                        .update(cx, |r, cx| r.set_note(Some(path.to_string()), model, cx));
                    self.editor
                        .update(cx, |e, cx| e.set_text(path.to_string(), text, cx));
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

    fn refresh_panes(&mut self, state: &Arc<VaultState>, cx: &mut Context<Self>) {
        let Some(path) = self.current.clone() else {
            return;
        };
        self.outline = state
            .cache
            .file(&path)
            .map(|meta| {
                meta.headings
                    .iter()
                    .map(|h| OutlineRow {
                        level: h.level,
                        text: _markdown::plain_text(
                            &_markdown::parse(&h.heading)
                                .blocks
                                .first()
                                .map(|b| match &b.kind {
                                    _markdown::BlockKind::Paragraph { content } => {
                                        content.clone()
                                    }
                                    _ => Vec::new(),
                                })
                                .unwrap_or_default(),
                        )
                        .into(),
                        offset: h.position.start.offset,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut rows = Vec::new();
        let mut last_source: Option<(String, Vec<String>)> = None;
        for b in state.cache.backlinks(&path) {
            let lines = match &last_source {
                Some((s, lines)) if s == b.source => lines.clone(),
                _ => {
                    let lines: Vec<String> =
                        std::fs::read_to_string(state.vault.absolute(b.source))
                            .map(|t| t.lines().map(str::to_string).collect())
                            .unwrap_or_default();
                    last_source = Some((b.source.to_string(), lines.clone()));
                    lines
                }
            };
            let line = b.link.position.start.line;
            let context = lines
                .get(line as usize)
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            rows.push(BacklinkRow {
                source: b.source.to_string(),
                name: display_name(b.source.rsplit('/').next().unwrap_or(b.source))
                    .to_string()
                    .into(),
                context: context.into(),
                line,
            });
        }
        self.backlinks = rows;
        cx.notify();
    }

    fn scroll_to_subpath(
        &mut self,
        path: &str,
        subpath: &Subpath<'_>,
        state: &Arc<VaultState>,
        cx: &mut Context<Self>,
    ) {
        let Some(meta) = state.cache.file(path) else {
            return;
        };
        let offset = match subpath {
            Subpath::Headings(names) => {
                let Some(last) = names.last() else { return };
                meta.headings
                    .iter()
                    .find(|h| {
                        _markdown::plain_text(
                            &_markdown::parse(&h.heading)
                                .blocks
                                .first()
                                .map(|b| match &b.kind {
                                    _markdown::BlockKind::Paragraph { content } => {
                                        content.clone()
                                    }
                                    _ => Vec::new(),
                                })
                                .unwrap_or_default(),
                        )
                        .trim()
                        .eq_ignore_ascii_case(last.trim())
                    })
                    .map(|h| h.position.start.offset)
            }
            Subpath::Block(id) => meta
                .blocks
                .get(&id.to_lowercase())
                .or_else(|| meta.blocks.get(*id))
                .map(|b| b.position.start.offset),
        };
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
                match state.cache.resolve(&body, &source) {
                    Some(path) => {
                        let path = path.to_string();
                        if path == source {
                            if let Some(sub) = parsed.subpath {
                                self.scroll_to_subpath(&path, &sub, &state, cx);
                            }
                        } else {
                            self.open_file(path, parsed.subpath, cx);
                        }
                    }
                    None => {
                        self.status = format!("{} does not exist yet", parsed.path).into();
                        cx.notify();
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
            ViewMode::Reading => ViewMode::Source,
            ViewMode::Source => ViewMode::Reading,
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
        self.show_file(&path, &state, cx);
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

    fn render_backlinks(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let row = self.backlinks.get(ix)?.clone();
                let source = row.source.clone();
                Some(
                    div()
                        .id(("backlink", ix))
                        .h(px(44.))
                        .flex()
                        .flex_col()
                        .justify_center()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_alt))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_file(source.clone(), None, cx);
                        }))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.text)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .overflow_hidden()
                                .child(row.name),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.text_muted)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .overflow_hidden()
                                .child(SharedString::from(format!(
                                    "{}: {}",
                                    row.line + 1,
                                    row.context
                                ))),
                        ),
                )
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
                let count = self.backlinks.len();
                if count == 0 {
                    empty_pane("No backlinks found.", theme)
                } else {
                    uniform_list("backlinks", count, cx.processor(Self::render_backlinks))
                        .size_full()
                        .px_1()
                        .into_any_element()
                }
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
                    .child(self.dock_tab(
                        "Backlinks",
                        tab == RightTab::Backlinks,
                        theme,
                        |this, _, cx| {
                            this.right_tab = RightTab::Backlinks;
                            cx.notify();
                        },
                        cx,
                    ))
                    .child(self.dock_tab(
                        "Outline",
                        tab == RightTab::Outline,
                        theme,
                        |this, _, cx| {
                            this.right_tab = RightTab::Outline;
                            cx.notify();
                        },
                        cx,
                    ))
                    .child(self.dock_tab(
                        "Tags",
                        tab == RightTab::Tags,
                        theme,
                        |this, _, cx| {
                            this.right_tab = RightTab::Tags;
                            cx.notify();
                        },
                        cx,
                    )),
            )
            .child(div().flex_1().overflow_hidden().child(body))
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
            ViewMode::Source => "Source",
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
                    .id("mode")
                    .px_2()
                    .py_px()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(theme.text_muted)
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.toggle_mode(&ToggleMode, window, cx)
                        }),
                    )
                    .child(mode_label),
            );

        let main_body: gpui::AnyElement = if let Some(loading) = &self.loading {
            div()
                .p_4()
                .text_color(theme.text_muted)
                .child(loading.clone())
                .into_any_element()
        } else {
            match self.mode {
                ViewMode::Reading => self.reading.clone().into_any_element(),
                ViewMode::Source => self.editor.clone().into_any_element(),
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
            .child(div().flex_1().overflow_hidden().child(left_body));

        let editor_status = if self.mode == ViewMode::Source {
            let editor = self.editor.read(cx);
            let cursor = editor.cursor();
            format!(
                "Ln {}, Col {}  ·  parse {:.1} ms, {} spans  ·  shape {:.2} ms  ·  ",
                cursor.line + 1,
                cursor.col + 1,
                editor.stats.parse_ms,
                editor.stats.spans,
                editor.prepaint_ms()
            )
        } else {
            String::new()
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
                "{editor_status}{} words, {} characters",
                self.words, self.chars
            )));

        let overlay = self.switcher_open.then(|| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_row()
                .justify_center()
                .items_start()
                .pt(px(90.))
                .child(self.switcher.clone())
        });

        div()
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::toggle_mode))
            .on_action(cx.listener(Self::go_back))
            .on_action(cx.listener(Self::go_forward))
            .on_action(cx.listener(Self::open_switcher_action))
            .on_action(cx.listener(Self::open_search_action))
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
                    .child(left)
                    .child(main)
                    .child(self.render_right_dock(&theme, cx)),
            )
            .child(status_bar)
            .children(overlay)
    }
}
