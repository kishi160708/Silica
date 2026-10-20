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
use _search::{match_lines, natural_cmp, MatchLine, Search, SearchFile, TaskLine};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: String,
    pub name: String,
    /// Hit ranges inside `name`, when the query matched the file name.
    pub name_ranges: Vec<Range<usize>>,
    pub matches: Vec<MatchLine>,
}

impl FileResult {
    /// What this file contributes to the result count: one per match line,
    /// plus one for a name hit, which Obsidian counts but leaves out of the
    /// number beside the file name.
    fn results(&self) -> usize {
        self.matches.len() + usize::from(!self.name_ranges.is_empty())
    }
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
    /// Paths of the files whose match lines are hidden. Keyed by path so a
    /// re-run over a changed vault leaves the user's collapses alone.
    collapsed: HashSet<String>,
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
                // A new query builds a new result list, the way Obsidian's
                // startSearch empties its own; only a re-run over the same
                // query keeps the collapsed rows.
                this.collapsed.clear();
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

    /// What the pane is showing, for the control socket's state dump: the
    /// count of the summary line and the files behind it, the message a bad
    /// query produced, and the result rows themselves. A collapsed file still
    /// reports its matches, since the count beside its name does too.
    pub fn describe(&self, rows: usize, cx: &App) -> Value {
        let total: usize = self.results.iter().map(FileResult::results).sum();
        // The note text turns a match's byte offset into a line number; it is
        // absent for a file the vault dropped since the search ran.
        let texts = self.state.as_ref().map(|state| state.texts());
        let results: Vec<Value> = self
            .results
            .iter()
            .take(rows)
            .map(|file| {
                let text = texts.as_ref().and_then(|t| t.get(&file.path));
                let hits = file.matches.len();
                let matches: Vec<Value> = file
                    .matches
                    .iter()
                    .take(rows)
                    .map(|m| {
                        json!({
                            "line": text.and_then(|t| line_number(t.as_str(), m.offset)),
                            "text": m.snippet,
                        })
                    })
                    .collect();
                // One note can hold more hits than the dump carries, so it
                // says how many it has, the same way every other capped list
                // in the dump does: `null` when nothing was dropped.
                let shown = matches.len();
                json!({
                    "path": file.path,
                    "name": file.name,
                    "name_match": !file.name_ranges.is_empty(),
                    "collapsed": self.collapsed.contains(&file.path),
                    "matches": matches,
                    "matches_total": if hits > shown { Some(hits) } else { None },
                })
            })
            .collect();
        json!({
            "query": self.input.read(cx).text(),
            "error": self.error,
            "files": self.results.len(),
            "matches": total,
            "results": results,
        })
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
        if search.is_empty() {
            // `(`, `"`, a bare `OR`: the parser read what it could and found
            // no term. Obsidian's startSearch returns before adding a result
            // rather than listing the vault.
            self.results.clear();
            self.error = None;
            self.rebuild_rows();
            cx.notify();
            return;
        }
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
                pane.rebuild_rows();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rebuild_rows(&mut self) {
        // A file the new results dropped forgets it was collapsed.
        let live: HashSet<&str> = self.results.iter().map(|f| f.path.as_str()).collect();
        let mut collapsed = std::mem::take(&mut self.collapsed);
        collapsed.retain(|p| live.contains(p.as_str()));
        self.collapsed = collapsed;
        self.rows.clear();
        for (ix, file) in self.results.iter().enumerate() {
            self.rows.push(Row::File { ix });
            if !self.collapsed.contains(&file.path) {
                for m in 0..file.matches.len() {
                    self.rows.push(Row::Match { ix, m });
                }
            }
        }
    }

    fn toggle(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&path) {
            self.collapsed.insert(path);
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
                        let collapsed = self.collapsed.contains(&file.path);
                        let path = file.path.clone();
                        let name = StyledText::new(SharedString::from(file.name.clone()))
                            .with_highlights(highlights(&file.name_ranges, &theme));
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
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.toggle(path.clone(), cx)),
                            )
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
                        let text = StyledText::new(SharedString::from(line.snippet.clone()))
                            .with_highlights(highlights(&line.ranges, &theme));
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
        let total: usize = self.results.iter().map(FileResult::results).sum();
        let summary: SharedString = if let Some(err) = &self.error {
            err.clone()
        } else if self.input.read(cx).text().trim().is_empty() {
            String::new().into()
        } else {
            // Obsidian's count element reads "7 results" and names no file
            // count; a file name hit counts as one of them.
            format!("{total} results").into()
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

pub type Inputs = (
    Vec<String>,
    Vec<Range<usize>>,
    Vec<Range<usize>>,
    Vec<TaskLine>,
);

/// Per-file search inputs derived from the cache record: tags, block
/// ranges, section ranges and task lines.
pub fn inputs(meta: &CachedMetadata, text: &str) -> Inputs {
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
            let hits = search.matches(&file)?;
            let name = path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".md");
            Some(FileResult {
                path: path.to_string(),
                name: name.to_string(),
                name_ranges: clamp(&hits.name, name.len()),
                matches: match_lines(text, &hits.content),
            })
        })
        .collect();
    sort_results(&mut results);
    results
}

/// Orders results the way Obsidian's alphabetical sort does: a numeric,
/// locale-aware collation of the name, the path breaking a tie.
fn sort_results(results: &mut [FileResult]) {
    results.sort_by(|a, b| natural_cmp(&a.name, &b.name).then_with(|| a.path.cmp(&b.path)));
}

/// The matcher reports name hits against the base name with its extension;
/// the row shows the name without it, so a hit reaching into `.md` is cut
/// there and one that lies entirely inside it is dropped.
fn clamp(ranges: &[Range<usize>], len: usize) -> Vec<Range<usize>> {
    ranges
        .iter()
        .filter(|r| r.start < len)
        .map(|r| r.start..r.end.min(len))
        .collect()
}

fn highlights(ranges: &[Range<usize>], theme: &Theme) -> Vec<(Range<usize>, HighlightStyle)> {
    ranges
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
        .collect()
}

/// Zero-based line of a byte offset, like Obsidian's `Pos`. `None` when the
/// offset no longer falls on a character boundary of the text, which a stale
/// result over a changed note can do.
fn line_number(text: &str, offset: usize) -> Option<usize> {
    text.get(..offset)
        .map(|before| before.matches('\n').count())
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn result(path: &str) -> FileResult {
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".md");
        FileResult {
            path: path.to_string(),
            name: name.to_string(),
            name_ranges: Vec::new(),
            matches: Vec::new(),
        }
    }

    #[test]
    fn results_sort_the_way_obsidian_collates() {
        let mut results: Vec<FileResult> = ["Note 10.md", "9 needle.md", "Note 2.md", "_needle.md"]
            .iter()
            .map(|p| result(p))
            .collect();
        sort_results(&mut results);
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["_needle", "9 needle", "Note 2", "Note 10"]);
        // Same name in two folders: the path decides.
        let mut same: Vec<FileResult> = ["notes/a.md", "a.md"].iter().map(|p| result(p)).collect();
        sort_results(&mut same);
        assert_eq!(same[0].path, "a.md");
    }

    #[test]
    fn name_hits_are_cut_at_the_extension_and_counted_once() {
        // Ranges are against "Needle.md", the row shows "Needle".
        assert_eq!(clamp(&[0..6], 6), vec![0..6]);
        assert_eq!(clamp(&[4..9], 6), vec![4..6], "a hit over `.md` is cut");
        assert!(
            clamp(&[6..9], 6).is_empty(),
            "a hit inside `.md` is dropped"
        );
        let mut file = result("Needle.md");
        assert_eq!(file.results(), 0);
        file.name_ranges = vec![0..6];
        assert_eq!(file.results(), 1, "a name hit counts as one result");
        file.matches = vec![MatchLine {
            offset: 0,
            snippet: "needle".into(),
            ranges: vec![0..6],
        }];
        assert_eq!(file.results(), 2);
    }
}
