//! The Search view (Ctrl+Shift+F): a query field over the vault with
//! results grouped per file and one line of context per match.
//!
//! Each change to the query runs the compiled search over every indexed
//! note on the background executor; a generation counter drops results of
//! superseded runs.

use crate::text_input::{InputEvent, TextInput};
use crate::theme::Theme;
use crate::workspace::VaultState;
use gpui::{
    div, prelude::*, px, uniform_list, App, Context, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, HighlightStyle, SharedString, StyledText, Subscription, Window,
};
use rayon::prelude::*;
use _cache::{all_tags, CachedMetadata};
use _search::{Search, SearchFile, TaskLine};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

/// One match line shown under a file.
#[derive(Debug, Clone)]
pub struct MatchLine {
    /// Byte offset of the line in the note, for scrolling.
    pub offset: usize,
    /// The line trimmed and windowed around the first hit.
    pub snippet: String,
    /// Hit ranges inside `snippet`.
    pub ranges: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: String,
    pub name: String,
    pub matches: Vec<MatchLine>,
}

#[derive(Debug, Clone)]
enum Row {
    File { ix: usize },
    Match { ix: usize, m: usize },
}

pub enum SearchEvent {
    /// Open a file at a byte offset.
    Open(String, Option<usize>),
}

pub struct SearchPane {
    input: Entity<TextInput>,
    state: Option<Arc<VaultState>>,
    results: Vec<FileResult>,
    collapsed: HashSet<usize>,
    rows: Vec<Row>,
    error: Option<SharedString>,
    generation: u64,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SearchEvent> for SearchPane {}

impl Focusable for SearchPane {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

impl SearchPane {
    pub fn new(cx: &mut Context<Self>) -> SearchPane {
        let input = cx.new(|cx| TextInput::new("Search…", cx));
        let subscriptions = vec![cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Changed) {
                this.run(cx);
            }
        })];
        SearchPane {
            input,
            state: None,
            results: Vec::new(),
            collapsed: HashSet::new(),
            rows: Vec::new(),
            error: None,
            generation: 0,
            _subscriptions: subscriptions,
        }
    }

    pub fn set_state(&mut self, state: Arc<VaultState>, cx: &mut Context<Self>) {
        self.state = Some(state);
        self.run(cx);
    }

    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.set_text(query, cx));
    }

    pub fn query(&self, cx: &App) -> String {
        self.input.read(cx).text().to_string()
    }

    /// Runs the current query again, after the vault changed.
    pub fn rerun(&mut self, cx: &mut Context<Self>) {
        self.run(cx);
    }

    fn run(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).text().trim().to_string();
        self.generation += 1;
        let generation = self.generation;
        let Some(state) = self.state.clone() else {
            return;
        };
        if query.is_empty() {
            self.results.clear();
            self.error = None;
            self.rebuild_rows();
            cx.notify();
            return;
        }
        let search = match Search::new(&query) {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(e.to_string().into());
                self.results.clear();
                self.rebuild_rows();
                cx.notify();
                return;
            }
        };
        self.error = None;
        cx.spawn(async move |this, cx| {
            let results = cx
                .background_executor()
                .spawn(async move { run_search(&search, &state) })
                .await;
            this.update(cx, |pane, cx| {
                if pane.generation != generation {
                    return;
                }
                pane.results = results;
                pane.collapsed.clear();
                pane.rebuild_rows();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        for (ix, file) in self.results.iter().enumerate() {
            self.rows.push(Row::File { ix });
            if !self.collapsed.contains(&ix) {
                for m in 0..file.matches.len() {
                    self.rows.push(Row::Match { ix, m });
                }
            }
        }
    }

    fn toggle(&mut self, ix: usize, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&ix) {
            self.collapsed.insert(ix);
        }
        self.rebuild_rows();
        cx.notify();
    }

    fn render_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|row_ix| {
                let row = self.rows.get(row_ix)?.clone();
                Some(match row {
                    Row::File { ix } => {
                        let file = &self.results[ix];
                        let collapsed = self.collapsed.contains(&ix);
                        let name: SharedString = file.name.clone().into();
                        let count: SharedString = file.matches.len().to_string().into();
                        div()
                            .id(("search-file", row_ix))
                            .w_full()
                            .h(px(24.))
                            .px_2()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_alt))
                            .on_click(cx.listener(move |this, _, _, cx| this.toggle(ix, cx)))
                            .child(
                                div()
                                    .w(px(12.))
                                    .text_size(px(10.))
                                    .text_color(theme.text_muted)
                                    .child(if collapsed { "▸" } else { "▾" }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(theme.bg_active)
                                    .text_size(px(10.))
                                    .text_color(theme.text_muted)
                                    .child(count),
                            )
                    }
                    Row::Match { ix, m } => {
                        let file = &self.results[ix];
                        let line = &file.matches[m];
                        let path = file.path.clone();
                        let offset = line.offset;
                        let highlights: Vec<_> = line
                            .ranges
                            .iter()
                            .map(|r| {
                                (
                                    r.clone(),
                                    HighlightStyle {
                                        background_color: Some(theme.highlight_bg),
                                        font_weight: Some(FontWeight::SEMIBOLD),
                                        ..HighlightStyle::default()
                                    },
                                )
                            })
                            .collect();
                        let text = StyledText::new(SharedString::from(line.snippet.clone()))
                            .with_highlights(highlights);
                        div()
                            .id(("search-match", row_ix))
                            .w_full()
                            .h(px(24.))
                            .pl(px(22.))
                            .pr_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .text_size(px(12.))
                            .text_color(theme.text_muted)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .hover(|s| s.bg(theme.bg_alt))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.emit(SearchEvent::Open(path.clone(), Some(offset)));
                            }))
                            .child(text)
                    }
                })
            })
            .collect()
    }
}

impl Render for SearchPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let total: usize = self.results.iter().map(|f| f.matches.len()).sum();
        let summary: SharedString = if let Some(err) = &self.error {
            err.clone()
        } else if self.input.read(cx).text().trim().is_empty() {
            String::new().into()
        } else {
            format!("{total} results in {} files", self.results.len()).into()
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(div().p_2().child(self.input.clone()))
            .when(!summary.is_empty(), |d| {
                d.child(
                    div()
                        .px_3()
                        .pb_1()
                        .text_size(px(11.))
                        .text_color(if self.error.is_some() {
                            theme.accent
                        } else {
                            theme.text_muted
                        })
                        .child(summary),
                )
            })
            .child(
                uniform_list(
                    "search-results",
                    self.rows.len(),
                    cx.processor(Self::render_rows),
                )
                .flex_1()
                .px_1(),
            )
    }
}

type Inputs = (
    Vec<String>,
    Vec<Range<usize>>,
    Vec<Range<usize>>,
    Vec<TaskLine>,
);

/// Per-file search inputs derived from the cache record: tags, block
/// ranges, section ranges and task lines.
fn inputs(meta: &CachedMetadata, text: &str) -> Inputs {
    let tags: Vec<String> = all_tags(meta)
        .into_iter()
        .map(|t| t.to_lowercase())
        .collect();
    let blocks: Vec<Range<usize>> = meta
        .sections
        .iter()
        .map(|s| s.position.start.offset..s.position.end.offset)
        .collect();
    let mut sections = Vec::new();
    let mut at = 0;
    for h in &meta.headings {
        let start = h.position.start.offset;
        if start > at {
            sections.push(at..start);
        }
        at = start;
    }
    sections.push(at..text.len());
    let tasks: Vec<TaskLine> = meta
        .list_items
        .iter()
        .filter_map(|item| {
            item.task.map(|c| TaskLine {
                range: item.position.start.offset..item.position.end.offset,
                done: c != ' ',
            })
        })
        .collect();
    (tags, blocks, sections, tasks)
}

/// Runs `search` over every note; results sorted by file name.
pub fn run_search(search: &Search, state: &VaultState) -> Vec<FileResult> {
    let cache = state.cache();
    let texts = state.texts();
    let files: Vec<(&str, &CachedMetadata)> = cache.files().collect();
    let mut results: Vec<FileResult> = files
        .par_iter()
        .filter_map(|(path, meta)| {
            let text = texts.get(*path)?;
            let (tags, blocks, sections, tasks) = inputs(meta, text);
            let file = SearchFile {
                path,
                content: text,
                tags: &tags,
                frontmatter: meta.frontmatter.as_ref(),
                blocks: &blocks,
                sections: &sections,
                tasks: &tasks,
            };
            let ranges = search.matches(&file)?;
            Some(FileResult {
                path: path.to_string(),
                name: path
                    .rsplit('/')
                    .next()
                    .unwrap_or(path)
                    .trim_end_matches(".md")
                    .to_string(),
                matches: match_lines(text, &ranges),
            })
        })
        .collect();
    results.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    results
}

/// Groups hit ranges by line and windows each line around its first hit.
fn match_lines(text: &str, ranges: &[Range<usize>]) -> Vec<MatchLine> {
    const BEFORE: usize = 40;
    const WIDTH: usize = 160;
    let mut out: Vec<MatchLine> = Vec::new();
    let mut line_start = 0;
    let mut rix = 0;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let mut hits: Vec<Range<usize>> = Vec::new();
        while rix < ranges.len() && ranges[rix].start < line_end {
            let r = &ranges[rix];
            let s = r.start.max(line_start) - line_start;
            let e = r.end.min(line_start + body.len()).max(line_start) - line_start;
            if e > s {
                hits.push(s..e);
            }
            if r.end <= line_end {
                rix += 1;
            } else {
                break;
            }
        }
        if !hits.is_empty() {
            // Window: a few words before the first hit, then up to WIDTH bytes.
            let first = hits[0].start;
            let mut start = first.saturating_sub(BEFORE);
            while !body.is_char_boundary(start) {
                start -= 1;
            }
            if let Some(sp) = body[start..first].find(char::is_whitespace) {
                if start > 0 {
                    start += sp + 1;
                }
            }
            let mut end = (start + WIDTH).min(body.len());
            while !body.is_char_boundary(end) {
                end += 1;
            }
            let snippet = body[start..end].trim_start().to_string();
            let trimmed = body[start..end].len() - body[start..end].trim_start().len();
            let base = start + trimmed;
            let snippet_ranges: Vec<Range<usize>> = hits
                .iter()
                .filter(|h| h.start >= base && h.end <= end)
                .map(|h| h.start - base..h.end - base)
                .collect();
            out.push(MatchLine {
                offset: line_start,
                snippet: if start > 0 {
                    format!("…{snippet}")
                } else {
                    snippet
                },
                ranges: if start > 0 {
                    snippet_ranges
                        .into_iter()
                        .map(|r| r.start + 3..r.end + 3)
                        .collect()
                } else {
                    snippet_ranges
                },
            });
        }
        line_start = line_end;
        if rix >= ranges.len() {
            break;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    #[test]
    fn lines_window_around_the_first_hit() {
        let text = "alpha beta\ngamma delta epsilon\nzeta";
        let g = text.find("delta").unwrap();
        let z = text.find("zeta").unwrap();
        let lines = match_lines(text, &[g..g + 5, g + 6..g + 7, z..z + 4]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].offset, 11);
        assert_eq!(lines[0].snippet, "gamma delta epsilon");
        assert_eq!(lines[0].ranges, vec![6..11, 12..13]);
        assert_eq!(lines[1].snippet, "zeta");
        let long = format!("{} needle {}", "x".repeat(100), "y".repeat(300));
        let n = long.find("needle").unwrap();
        let lines = match_lines(&long, &[n..n + 6]);
        assert!(lines[0].snippet.starts_with('…'));
        assert_eq!(&lines[0].snippet[lines[0].ranges[0].clone()], "needle");
        assert!(lines[0].snippet.len() < 200);
    }
}
