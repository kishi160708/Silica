//! Task Dashboard: scan every note in the vault for task list items
//! (`- [ ]` and `- [x]`) and display them in one list, like Logseq's
//! task dashboard or Capacities' task view.
//!
//! Clicking a task opens its source note and puts the caret on the task's
//! line.

use crate::workspace::{VaultState, Workspace};
use gpui::{
    actions, div, point, prelude::*, px, size, uniform_list, AnyWindowHandle, App, Bounds, Context,
    FocusHandle, Focusable, KeyBinding, Render, SharedString, TitlebarOptions,
    UniformListScrollHandle, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions,
};
use silica_markdown::{parse, plain_text, walk_blocks, BlockKind, LineIndex};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

actions!(
    task_dashboard,
    [
        OpenTask,
        CloseTaskDashboard,
        FocusNext,
        FocusPrev,
        OpenFocused
    ]
);

/// Escape closes the dashboard.
///
/// The arrows move a selection ring and Enter opens it, the way the file
/// tree's own keys work: a row is reachable without the pointer, which is
/// also what makes a click-free test possible on a bare X server.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CloseTaskDashboard, Some("TaskDashboard")),
        KeyBinding::new("down", FocusNext, Some("TaskDashboard")),
        KeyBinding::new("up", FocusPrev, Some("TaskDashboard")),
        KeyBinding::new("enter", OpenFocused, Some("TaskDashboard")),
    ]);
}

/// Open the Task Dashboard window.
pub fn open(
    state: Arc<VaultState>,
    workspace: WeakEntity<Workspace>,
    main: AnyWindowHandle,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<TaskDashboardWindow>> {
    let (w, h) = WINDOW_SIZE;
    // * Centred over the main window rather than on the display: the
    //   dashboard is a side glance at the vault, not a window on another
    //   monitor. A main window that has gone since falls back to the
    //   display's centre.
    let bounds = main
        .update(cx, |_, window, _| window.bounds())
        .ok()
        .map(|main_bounds| {
            let centre = main_bounds.center();
            Bounds {
                origin: point(centre.x - px(w / 2.), centre.y - px(h / 2.)),
                size: size(px(w), px(h)),
            }
        })
        .unwrap_or_else(|| Bounds::centered(None, size(px(w), px(h)), cx));
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Task Dashboard".into()),
                ..Default::default()
            }),
            app_id: Some("silica-task-dashboard".into()),
            window_decorations: Some(WindowDecorations::Server),
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(500.), px(400.))),
            ..Default::default()
        },
        move |window, cx| {
            // * The scan runs on a background thread: a large vault would
            //   otherwise freeze the window before its first frame. The view
            //   starts empty and fills in when the scan lands.
            let view = cx.new(|cx| {
                let mut view = TaskDashboardWindow {
                    focus_handle: cx.focus_handle(),
                    tasks: Vec::new(),
                    rows: Vec::new(),
                    skipped: 0,
                    scanning: true,
                    progress: 0,
                    selected: None,
                    scroll: UniformListScrollHandle::new(),
                    state: state.clone(),
                    workspace,
                    main,
                    cancel: Arc::new(AtomicBool::new(false)),
                };
                view.start_scan(cx);
                view
            });
            let handle = view.read(cx).focus_handle.clone();
            window.focus(&handle, cx);
            view
        },
    )
}

/// One task found in a vault note.
#[derive(Clone, Debug)]
struct TaskItem {
    /// The note's vault-relative path.
    path: SharedString,
    /// The zero-based source line of the task in the note.
    line: usize,
    /// The task's text (the list item content after the checkbox).
    text: SharedString,
    /// Whether the task is done (`[x]`) or pending (`[ ]`).
    done: bool,
}

/// Scan one note's content for task list items.
///
/// Split from `scan_vault_for_tasks` so the extraction and line-mapping
/// rules are testable without a vault on disk.
fn scan_note(path: &str, content: &str) -> Vec<TaskItem> {
    let doc = parse(content);
    let index = LineIndex::new(content);
    let mut tasks = Vec::new();
    // * `walk_blocks` descends into list children, so a task nested in
    //   another list item is found too. Without it only top-level items
    //   would show up.
    walk_blocks(&doc.blocks, &mut |block| {
        let BlockKind::List { items, .. } = &block.kind else {
            return;
        };
        for item in items {
            let Some(task_char) = item.task else {
                continue;
            };
            // * `plain_text` walks every inline kind -- links, wikilinks,
            //   emphasis, code, math, tags -- so a task is never shown
            //   with its markup dropped.
            let text = item
                .children
                .iter()
                .map(|child| match &child.kind {
                    BlockKind::Paragraph { content } => plain_text(content),
                    _ => String::new(),
                })
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            tasks.push(TaskItem {
                path: path.into(),
                // * The item's own span, not the block's index: a block
                //   index is not a line once a document has headings,
                //   blank lines, or more than one block.
                line: index.pos(item.span.start).line as usize,
                text: text.trim().into(),
                done: task_char.eq_ignore_ascii_case(&'x'),
            });
        }
    });
    tasks
}

/// Scan every markdown note in the vault for task list items.
///
/// Reads from the vault's in-memory note texts — the same map the search
/// and backlink panes use — with the disk as a fallback, so the scan does
/// not re-read what the vault already holds. Returns the tasks, the number
/// of notes neither source had (so the caller can say so rather than
/// silently dropping them), and stops early when `cancel` is raised: a
/// window closed mid-scan must not leave a thread walking the vault.
fn scan_vault_for_tasks(
    state: &VaultState,
    cancel: &AtomicBool,
    scanned: &AtomicUsize,
) -> (Vec<TaskItem>, usize) {
    let mut tasks = Vec::new();
    let mut skipped = 0usize;
    let paths: Vec<String> = state
        .vault()
        .markdown_files()
        .map(|file| file.path.clone())
        .collect();

    for (ix, path) in paths.iter().enumerate() {
        // * Checked every note rather than per block: the flag is read
        //   once per note and the cost is one atomic load.
        if ix % 64 == 0 && cancel.load(Ordering::Relaxed) {
            return (tasks, skipped);
        }
        match note_text(state, path) {
            Some(content) => tasks.extend(scan_note(path, &content)),
            None => skipped += 1,
        }
        scanned.store(ix + 1, Ordering::Relaxed);
    }

    (tasks, skipped)
}

/// Re-scans only the named notes: the incremental half of the dashboard,
/// for a change that lands while it is open. A note with no text anywhere
/// contributes nothing, which is how a deleted note leaves the list.
fn scan_notes(state: &VaultState, paths: &[String]) -> Vec<TaskItem> {
    paths
        .iter()
        .filter_map(|path| note_text(state, path).map(|text| (path.clone(), text)))
        .flat_map(|(path, text)| scan_note(&path, &text))
        .collect()
}

/// The byte offset where `line` starts, in `text`.
fn line_offset(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    // * split_inclusive keeps the terminator on each line, so summing the
    //   lengths of the first `line` lines lands exactly on the start of the
    //   next one — and a text with fewer lines sums to its own length,
    //   which the editor's clamp turns into its last offset.
    text.split_inclusive('\n').take(line).map(str::len).sum()
}

/// The text of one note: the vault's in-memory copy first, the disk as a
/// fallback, so a note the index knows but memory has not caught up with
/// is still scanned. `None` means neither had it.
fn note_text(state: &VaultState, path: &str) -> Option<String> {
    if let Some(text) = state.texts().get(path) {
        return Some(text.as_str().to_string());
    }
    // ponytail: the disk read is the fallback for a note the in-memory
    // index has not caught up with; drop it if `texts` is ever made to
    // hold every indexed note eagerly.
    std::fs::read_to_string(state.vault().absolute(path)).ok()
}

/// The Task Dashboard window's root view.
pub struct TaskDashboardWindow {
    focus_handle: FocusHandle,
    tasks: Vec<TaskItem>,
    /// The rendered rows, flattened so the list can be virtualized: a
    /// 50k-task vault must not build 50k elements per frame.
    rows: Vec<Row>,
    /// Files the scan could not read, reported in the header.
    skipped: usize,
    /// Whether a scan is still running.
    scanning: bool,
    /// Notes scanned so far, shown while the scan runs.
    progress: usize,
    /// The row the keyboard's ring is on: a task row's index in `rows`,
    /// so the arrows and Enter reach a task without the pointer.
    selected: Option<usize>,
    scroll: UniformListScrollHandle,
    state: Arc<VaultState>,
    /// The workspace that opened this window: clicking a task asks it to
    /// open the note, since opening files is the workspace's job.
    workspace: WeakEntity<Workspace>,
    /// The main window, where that workspace lives.
    main: AnyWindowHandle,
    /// Raised when the window is dropped: a scan still running walks the
    /// vault for nobody otherwise.
    cancel: Arc<AtomicBool>,
}

impl Drop for TaskDashboardWindow {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl TaskDashboardWindow {
    /// Runs the vault scan off the UI thread and fills the list when it
    /// lands, with a progress count on the header while it runs and a
    /// cancel the window's own Drop raises.
    fn start_scan(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let cancel = self.cancel.clone();
        let scanned = Arc::new(AtomicUsize::new(0));
        let progress = scanned.clone();
        cx.spawn(async move |this, cx| {
            let (tasks, skipped) = cx
                .background_executor()
                .spawn(async move { scan_vault_for_tasks(&state, &cancel, &scanned) })
                .await;
            this.update(cx, |this, cx| {
                this.tasks = tasks;
                this.skipped = skipped;
                this.scanning = false;
                this.rebuild_rows();
                cx.notify();
            })
            .ok();
        })
        .detach();
        // * The count is polled rather than pushed: the scan is on another
        //   thread, and one timer tick per repaint is cheaper than a
        //   channel per note.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;
            let alive = this
                .update(cx, |this, cx| {
                    this.progress = progress.load(Ordering::Relaxed);
                    let running = this.scanning;
                    if running {
                        cx.notify();
                    }
                    running
                })
                .unwrap_or(false);
            if !alive {
                break;
            }
        })
        .detach();
    }

    /// Re-scans only the notes that changed while the dashboard is open.
    ///
    /// ponytail: one incremental path per changed note, merged by
    /// replacing the note's rows; a full vault scan runs once at open.
    pub(crate) fn refresh_changed(&mut self, paths: &[String], cx: &mut Context<Self>) {
        if paths.is_empty() || self.scanning {
            return;
        }
        let state = self.state.clone();
        let changed: Vec<String> = paths.to_vec();
        let stale: HashSet<String> = changed.iter().cloned().collect();
        cx.spawn(async move |this, cx| {
            let scanned = cx
                .background_executor()
                .spawn(async move { scan_notes(&state, &changed) })
                .await;
            this.update(cx, |this, cx| {
                let gone: HashSet<&str> = stale.iter().map(String::as_str).collect();
                this.tasks.retain(|task| !gone.contains(task.path.as_ref()));
                this.tasks.extend(scanned);
                this.rebuild_rows();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Flattens the tasks into section labels and task rows, in one list.
    fn rebuild_rows(&mut self) {
        let mut rows = Vec::with_capacity(self.tasks.len() + 2);
        rows.push(Row::Label("Pending"));
        rows.extend(
            self.tasks
                .iter()
                .filter(|task| !task.done)
                .cloned()
                .map(Row::Task),
        );
        rows.push(Row::Label("Completed"));
        rows.extend(
            self.tasks
                .iter()
                .filter(|task| task.done)
                .cloned()
                .map(Row::Task),
        );
        self.rows = rows;
    }

    fn close(&mut self, _: &CloseTaskDashboard, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    /// The arrows skip the section labels and stop at the ends.
    fn move_selection(&mut self, forward: bool, cx: &mut Context<Self>) {
        let mut ix = self.selected;
        loop {
            let next = match ix {
                None if forward => Some(0),
                None => self.rows.len().checked_sub(1),
                Some(ix) if forward => Some(ix + 1),
                Some(ix) => ix.checked_sub(1),
            };
            let Some(next) = next.filter(|next| *next < self.rows.len()) else {
                break;
            };
            if matches!(self.rows[next], Row::Task(_)) {
                ix = Some(next);
                break;
            }
            ix = Some(next);
        }
        if ix != self.selected {
            self.selected = ix;
            cx.notify();
        }
    }

    fn focus_next(&mut self, _: &FocusNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(true, cx);
    }

    fn focus_prev(&mut self, _: &FocusPrev, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(false, cx);
    }

    /// Enter opens the selected task, like a click on its row.
    fn open_focused(&mut self, _: &OpenFocused, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(Row::Task(task)) = self.selected.and_then(|ix| self.rows.get(ix)) {
            let task = task.clone();
            self.open_task(&task, cx);
        }
    }

    fn open_task(&mut self, task: &TaskItem, cx: &mut Context<Self>) {
        let path = task.path.to_string();
        // * The task's byte offset, from the vault's own copy of the note:
        //   `reveal_offset` clamps against the buffer as it is, so the
        //   offset has to be the destination's, not the old note's.
        let offset = note_text(&self.state, &path)
            .map(|text| line_offset(&text, task.line))
            .unwrap_or(0);
        let workspace = self.workspace.clone();
        let main = self.main;
        // * Opening a note is the workspace's job and it lives in the main
        //   window, so the click is handed there. The caret lands on the
        //   task's line, which is what makes the click worth making.
        main.update(cx, |_, window, cx| {
            workspace
                .update(cx, |ws, cx| {
                    ws.open_file(path, None, cx);
                    let editor = ws.editor().clone();
                    // * reveal_offset, not a scroll: it puts the caret on the
                    //   line and reveals it on the next paint, so it survives
                    //   the editor not having laid the new note out yet —
                    //   which is the race a scroll set at this moment loses.
                    editor.update(cx, |editor, cx| editor.reveal_offset(offset, cx));
                    window.activate_window();
                })
                .ok();
        })
        .ok();
    }
}

/// One row of the flattened list: a section label, or a task.
enum Row {
    Label(&'static str),
    Task(TaskItem),
}

impl Focusable for TaskDashboardWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TaskDashboardWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // * Cloned out of the global: the rows below need `&mut cx`, and a
        //   borrow of the theme through `cx` would still be live.
        let theme = cx.global::<crate::theme::Theme>().clone();
        let pending = self.tasks.iter().filter(|t| !t.done).count();
        let done = self.tasks.iter().filter(|t| t.done).count();

        let mut header = format!("Task Dashboard ({pending} pending, {done} done)");
        if self.scanning {
            header.push_str(&format!(
                " \u{2014} scanning\u{2026} {} notes",
                self.progress
            ));
        }
        if self.skipped > 0 {
            header.push_str(&format!(
                " \u{b7} {} unreadable file(s) skipped",
                self.skipped
            ));
        }

        // * A uniform list, not a child per task: the explorer's own list
        //   is the same shape, and a 50k-task vault would otherwise build
        //   50k elements per frame for the twenty that are on screen.
        let mut body = div()
            .id("task-dashboard-body")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll();
        if self.rows.is_empty() && !self.scanning {
            body = body.child(
                div()
                    .px_4()
                    .py_6()
                    .text_size(px(13.))
                    .text_color(theme.text_muted)
                    .child("No tasks in this vault yet. Write `- [ ] something` in any note."),
            );
        } else {
            body = body.child(
                uniform_list(
                    "task-dashboard-rows",
                    self.rows.len(),
                    cx.processor(Self::render_rows),
                )
                .track_scroll(&self.scroll)
                .size_full(),
            );
        }

        div()
            .id("task-dashboard")
            .key_context("TaskDashboard")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close))
            .on_action(cx.listener(Self::focus_next))
            .on_action(cx.listener(Self::focus_prev))
            .on_action(cx.listener(Self::open_focused))
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.bg)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.text)
                    .px_4()
                    .py_2()
                    .child(header),
            )
            .child(body)
    }
}

impl TaskDashboardWindow {
    /// The visible slice of the flattened list: a section label, or a task
    /// row whose click opens the note at the task's own line.
    fn render_rows(
        &mut self,
        range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Stateful<gpui::Div>> {
        let theme = cx.global::<crate::theme::Theme>().clone();
        range
            .filter_map(|ix| {
                Some(match self.rows.get(ix)? {
                    Row::Label(title) => div()
                        .id(SharedString::from(format!("task-section-{ix}")))
                        .px_4()
                        .py_1()
                        .text_size(px(11.))
                        .text_color(theme.text_muted)
                        .child(*title),
                    Row::Task(task) => {
                        let row = task.clone();
                        let checkbox = if task.done { "[x]" } else { "[ ]" };
                        div()
                            .id(SharedString::from(format!(
                                "task-{}:{}",
                                task.path, task.line
                            )))
                            .flex()
                            .items_center()
                            .px_4()
                            .py_1()
                            .h(px(22.))
                            .border_b_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_alt))
                            .on_click(cx.listener(move |this, _, _, cx| this.open_task(&row, cx)))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(if task.done {
                                        theme.text_faint
                                    } else {
                                        theme.text
                                    })
                                    .child(checkbox),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.text)
                                    .ml(px(4.))
                                    .child(task.text.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_muted)
                                    .ml_auto()
                                    .child(format!("{}:{}", task.path, task.line + 1)),
                            )
                    }
                })
            })
            .collect()
    }
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (700., 500.);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A progress counter nothing reads, for a test that only wants the
    /// scan's results.
    static NO_PROGRESS: AtomicUsize = AtomicUsize::new(0);

    /// What the scan found in one note, as (line, text, done).
    fn found(path: &str, content: &str) -> Vec<(usize, String, bool)> {
        scan_note(path, content)
            .into_iter()
            .map(|t| (t.line, t.text.to_string(), t.done))
            .collect()
    }

    /// A task's line is the line it is written on, not the block's
    /// enumerate index: headings, blank lines and paragraphs above a list
    /// all shift a block index away from the source line.
    #[test]
    fn a_tasks_line_is_the_line_it_is_written_on() {
        let content = "# Heading\n\nSome prose.\n\n- [ ] First task\n- [x] Second task\n";
        assert_eq!(
            found("N.md", content),
            vec![
                (4, "First task".to_string(), false),
                (5, "Second task".to_string(), true),
            ],
            "both tasks sit below a heading, prose and a blank line"
        );
    }

    /// Two lists separated by more prose: the second list's lines keep
    /// counting from the top of the file.
    #[test]
    fn multiple_lists_in_one_note_share_one_line_counter() {
        let content = "- [ ] One\n\nProse between.\n\n- [ ] Two\n- [x] Three\n";
        assert_eq!(
            found("N.md", content),
            vec![
                (0, "One".to_string(), false),
                (4, "Two".to_string(), false),
                (5, "Three".to_string(), true),
            ],
        );
    }

    /// A nested task is found too, with its own line, and the parent
    /// appears before the child.
    #[test]
    fn nested_tasks_are_found_with_their_own_lines() {
        let content = "- [ ] Parent\n  - [ ] Child\n  - [x] Grandchild\n";
        let rows = found("N.md", content);
        let lines: Vec<usize> = rows.iter().map(|r| r.0).collect();
        assert_eq!(lines, vec![0, 1, 2], "parent, child, grandchild in order");
        assert_eq!(rows[0].1, "Parent");
        assert_eq!(rows[1].1, "Child");
    }

    /// Deeply indented tasks (four spaces) are still tasks.
    #[test]
    fn four_space_indented_tasks_are_found() {
        let content = "- [ ] Outer\n    - [ ] Inner\n";
        let rows = found("N.md", content);
        assert_eq!(rows.len(), 2, "the four-space-indented task is found");
        assert_eq!(rows[1].1, "Inner");
    }

    /// Links, wikilinks, emphasis and code keep their text: plain_text
    /// walks every inline kind rather than dropping the node.
    #[test]
    fn inline_markup_survives_in_the_task_text() {
        let content = "- [ ] Read [the docs](https://example.com)\n- [x] See [[Other Note]]\n- [ ] Fix **bold** and `code`\n";
        let rows = found("N.md", content);
        let texts: Vec<&str> = rows.iter().map(|r| r.1.as_str()).collect();
        assert_eq!(texts[0], "Read the docs");
        assert_eq!(texts[1], "See Other Note");
        assert_eq!(texts[2], "Fix bold and code");
    }

    /// A checkbox with nothing after it is not a task: the parser's task
    /// marker needs content past the `]`, so a stray `- [ ]` never shows
    /// up as a nameless row.
    #[test]
    fn an_unlabelled_checkbox_is_not_a_task() {
        assert!(
            found("N.md", "- [ ]\n").is_empty(),
            "an empty checkbox is not a task item"
        );
    }

    /// Non-task list items are not tasks.
    #[test]
    fn plain_list_items_are_not_tasks() {
        let content = "- one\n- two\n\ntext\n\n- [ ] real task\n";
        assert_eq!(
            found("N.md", content).len(),
            1,
            "only the checkbox is a task"
        );
    }

    /// Uppercase X is a completed checkbox too.
    #[test]
    fn an_uppercase_x_is_done() {
        let content = "- [X] shouted\n";
        assert_eq!(
            found("N.md", content),
            vec![(0, "shouted".to_string(), true)]
        );
    }

    /// Unicode titles and content survive both the path and the text.
    #[test]
    fn unicode_content_survives() {
        let content = "- [ ] Check café ☕ 日本語\n- [x] Ünïcödé ✓\n";
        let rows = found("Ünïcödé.md", content);
        assert_eq!(rows[0].1, "Check café ☕ 日本語");
        assert_eq!(rows[0].0, 0, "line 0");
        assert_eq!(rows[1].1, "Ünïcödé ✓");
        assert!(rows[1].2);
    }

    /// A note with no tasks at all yields nothing.
    #[test]
    fn a_note_without_tasks_yields_nothing() {
        assert!(found("N.md", "# Just a note\n\nWith prose.\n").is_empty());
    }

    /// The path travels with each task: two notes scanned separately keep
    /// their own rows.
    #[test]
    fn each_row_carries_its_notes_path() {
        let a = scan_note("A.md", "- [ ] in a\n");
        let b = scan_note("Sub/B.md", "- [ ] in b\n");
        assert_eq!(a[0].path.as_ref(), "A.md");
        assert_eq!(b[0].path.as_ref(), "Sub/B.md");
    }

    /// Both task notes are scanned from the vault's in-memory texts — the
    /// disk is the fallback, not the source — so a note deleted from disk
    /// after the load is still scanned from memory.
    #[test]
    fn the_scan_reads_the_vaults_own_texts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(
            root.join(".obsidian/app.json"),
            r#"{"trashOption":"local"}"#,
        )
        .unwrap();
        // * Every entry is written before the load: the vault indexes at
        //   load time, so a file created afterwards is invisible to the
        //   scan no matter what the disk says.
        fs::write(root.join("Tasks.md"), "- [ ] Buy milk\n").unwrap();
        fs::write(root.join("Done.md"), "- [x] Ship it\n").unwrap();
        fs::write(root.join("Gone.md"), "- [ ] removed mid-scan\n").unwrap();
        let state = VaultState::load(&root.to_path_buf()).unwrap();
        // Indexed at load, gone from disk by the time the scan reads it:
        // the in-memory text is the copy that counts.
        fs::remove_file(root.join("Gone.md")).unwrap();

        let (tasks, skipped) = scan_vault_for_tasks(&state, &AtomicBool::new(false), &NO_PROGRESS);
        let rows: Vec<(String, usize, String)> = tasks
            .iter()
            .map(|t| (t.path.to_string(), t.line, t.text.to_string()))
            .collect();
        assert!(
            rows.contains(&("Done.md".to_string(), 0, "Ship it".to_string())),
            "the done task is scanned: {rows:?}"
        );
        assert!(
            rows.contains(&("Tasks.md".to_string(), 0, "Buy milk".to_string())),
            "the pending task is scanned: {rows:?}"
        );
        assert!(
            rows.contains(&("Gone.md".to_string(), 0, "removed mid-scan".to_string())),
            "scanned from memory even though the disk copy is gone: {rows:?}"
        );
        assert_eq!(skipped, 0, "every note had text: in memory if not on disk");
    }

    /// A task's line becomes the byte offset that line starts at, so the
    /// caret can be put there without the editor having laid out the note.
    #[test]
    fn a_line_becomes_the_offset_it_starts_at() {
        let text = "# Heading\n\nSome prose.\n\n- [ ] First task\n- [x] Second\n";
        assert_eq!(line_offset(text, 0), 0);
        // "# Heading\n" (10) + "\n" (1) + "Some prose.\n" (12) + "\n" (1).
        assert_eq!(line_offset(text, 4), 24);
        // Plus "- [ ] First task\n" (17).
        assert_eq!(line_offset(text, 5), 41);
        // * Beyond the end: the total length, which an editor clamps.
        assert_eq!(line_offset(text, 99), text.len());
        // * With \r\n line endings the offset lands after the terminator.
        let crlf = "a\r\nb\r\nc\r\n";
        assert_eq!(line_offset(crlf, 1), 3);
        assert_eq!(line_offset(crlf, 2), 6);
    }

    /// A cancel raised before the scan starts stops it: a window closed
    /// while its scan is still walking the vault must not leave the walk
    /// running for nobody.
    #[test]
    fn a_cancelled_scan_stops_immediately() {
        let (_dir, state) = vault_with(&[("A.md", "- [ ] in a\n"), ("B.md", "- [ ] in b\n")]);
        let (tasks, skipped) = scan_vault_for_tasks(&state, &AtomicBool::new(true), &NO_PROGRESS);
        assert!(
            tasks.is_empty(),
            "a cancelled scan finds nothing: {tasks:?}"
        );
        assert_eq!(skipped, 0);
    }

    /// A note the index knows and neither source has is counted, not
    /// dropped silently.
    #[test]
    fn a_note_with_no_text_anywhere_is_counted() {
        let (_dir, state) = vault_with(&[("A.md", "- [ ] in a\n")]);
        assert_eq!(
            note_text(&state, "NoSuch.md"),
            None,
            "an unknown path has no text"
        );
        assert_eq!(
            note_text(&state, "A.md").as_deref(),
            Some("- [ ] in a\n"),
            "a known note reads from memory"
        );
    }

    /// The incremental path re-scans only the notes named, and a deleted
    /// note leaves the list: nothing to add is nothing to add.
    #[test]
    fn the_incremental_path_scans_only_the_named_notes() {
        let (_dir, state) = vault_with(&[("A.md", "- [ ] in a\n"), ("B.md", "- [ ] in b\n")]);
        let fresh = scan_notes(&state, &["A.md".to_string()]);
        assert_eq!(fresh.len(), 1, "only the named note is re-scanned");
        assert_eq!(fresh[0].text.as_ref(), "in a");
        assert!(
            scan_notes(&state, &["Gone.md".to_string()]).is_empty(),
            "a note with no text adds nothing, which is how a removed note leaves"
        );
    }

    /// A vault with no task notes scans to nothing, not an error.
    #[test]
    fn an_empty_vault_scans_to_nothing() {
        let (_dir, state) = vault_with(&[("A.md", "# A\n"), ("B.md", "# B\n")]);
        let (tasks, skipped) = scan_vault_for_tasks(&state, &AtomicBool::new(false), &NO_PROGRESS);
        assert!(
            tasks.is_empty(),
            "prose-only notes hold no tasks: {tasks:?}"
        );
        assert_eq!(skipped, 0);
    }

    /// A temp vault holding `(relative path, content)` notes, written
    /// before the load so the index sees them.
    fn vault_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, VaultState) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(
            root.join(".obsidian/app.json"),
            r#"{"trashOption":"local"}"#,
        )
        .unwrap();
        for (path, content) in notes {
            if let Some(parent) = root.join(path).parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(root.join(path), content).unwrap();
        }
        let state = VaultState::load(&root.to_path_buf()).unwrap();
        (dir, state)
    }

    /// A vault of `count` one-task notes, spread over folders the way a
    /// real vault is. Returns it with the load timed out separately, so a
    /// benchmark reading the printout can tell the two costs apart.
    fn vault_of(count: usize) -> (tempfile::TempDir, VaultState) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(
            root.join(".obsidian/app.json"),
            r#"{"trashOption":"local"}"#,
        )
        .unwrap();
        let started = std::time::Instant::now();
        for ix in 0..count {
            let folder = root.join(format!("Notes{:02}", ix % 20));
            fs::create_dir_all(&folder).unwrap();
            fs::write(
                folder.join(format!("Note{ix:06}.md")),
                format!("# Note {ix}\n\nprose\n\n- [ ] task {ix}\n"),
            )
            .unwrap();
        }
        let written = started.elapsed();
        let state = VaultState::load(&root.to_path_buf()).unwrap();
        println!(
            "{count} notes: load {written:?}, total {:?}",
            started.elapsed()
        );
        (dir, state)
    }

    /// The scan must cost the same per note at every vault size: a per-note
    /// cost that grows with the vault means the scan is re-doing work it
    /// already did — reading a file twice, or walking an index per note.
    ///
    /// Ignored because it writes tens of thousands of files and is a
    /// release check, not a unit test. Run it with
    /// `cargo test -p silica -- --ignored --nocapture task_dashboard::tests::scan_cost`.
    #[test]
    #[ignore = "writes up to 50k files; opt in for a release check"]
    fn scan_cost_stays_flat_as_the_vault_grows() {
        let mut costs: Vec<(usize, f64)> = Vec::new();
        for count in [1_000, 10_000, 50_000] {
            let (_dir, state) = vault_of(count);
            let started = std::time::Instant::now();
            let (tasks, skipped) =
                scan_vault_for_tasks(&state, &AtomicBool::new(false), &AtomicUsize::new(0));
            let scan = started.elapsed();
            assert_eq!(skipped, 0, "every note in the vault is readable");
            assert_eq!(tasks.len(), count, "every note holds one task");
            let per_note_ms = scan.as_secs_f64() * 1000.0 / count as f64;
            println!("{count} notes: scan {scan:?} = {per_note_ms:.4} ms/note");
            costs.push((count, per_note_ms));
        }
        // * The ceiling is generous — a debug build on a busy disk — but
        //   flat: a quadratic scan shows up as the largest vault costing
        //   many times the smallest per note.
        let (_, smallest) = costs[0];
        let (largest_count, largest) = costs[costs.len() - 1];
        assert!(
            largest < smallest * 3.0 + 0.05,
            "{largest_count} notes cost {largest:.4} ms/note against {smallest:.4} at 1000: {costs:?}"
        );
    }
}
