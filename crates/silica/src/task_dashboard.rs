//! Task Dashboard: scan every note in the vault for task list items
//! (`- [ ]` and `- [x]`) and display them in one list, like Logseq's
//! task dashboard or Capacities' task view.
//!
//! Clicking a task opens its source note and puts the caret on the task's
//! line.

use crate::workspace::{VaultState, Workspace};
use gpui::{
    actions, div, prelude::*, px, size, AnyWindowHandle, App, Bounds, Context, FocusHandle,
    Focusable, KeyBinding, Render, SharedString, TitlebarOptions, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowHandle, WindowOptions,
};
use silica_markdown::{parse, plain_text, walk_blocks, BlockKind, LineIndex};
use std::sync::Arc;

actions!(task_dashboard, [OpenTask, CloseTaskDashboard]);

/// Escape closes the dashboard.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "escape",
        CloseTaskDashboard,
        Some("TaskDashboard"),
    )]);
}

/// Open the Task Dashboard window.
pub fn open(
    state: Arc<VaultState>,
    workspace: WeakEntity<Workspace>,
    main: AnyWindowHandle,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<TaskDashboardWindow>> {
    let (w, h) = WINDOW_SIZE;
    let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
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
                    skipped: 0,
                    scanning: true,
                    state: state.clone(),
                    workspace,
                    main,
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
/// Returns the tasks and the number of files that could not be read, so the
/// caller can say so rather than silently dropping them.
fn scan_vault_for_tasks(state: &VaultState) -> (Vec<TaskItem>, usize) {
    let mut tasks = Vec::new();
    let mut skipped = 0usize;
    let vault = state.vault();

    for file in vault.markdown_files() {
        let path = file.path.clone();
        let content = match std::fs::read_to_string(vault.absolute(&path)) {
            Ok(c) => c,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        tasks.extend(scan_note(&path, &content));
    }

    (tasks, skipped)
}

/// The Task Dashboard window's root view.
pub struct TaskDashboardWindow {
    focus_handle: FocusHandle,
    tasks: Vec<TaskItem>,
    /// Files the scan could not read, reported in the header.
    skipped: usize,
    /// Whether a scan is still running.
    scanning: bool,
    state: Arc<VaultState>,
    /// The workspace that opened this window: clicking a task asks it to
    /// open the note, since opening files is the workspace's job.
    workspace: WeakEntity<Workspace>,
    /// The main window, where that workspace lives.
    main: AnyWindowHandle,
}

impl TaskDashboardWindow {
    /// Runs the vault scan off the UI thread and fills the list when it
    /// lands. ponytail: one full scan per window open, no watcher-driven
    /// refresh; add an incremental update when the dashboard is meant to
    /// stay open beside the editor.
    fn start_scan(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let (tasks, skipped) = cx
                .background_executor()
                .spawn(async move { scan_vault_for_tasks(&state) })
                .await;
            this.update(cx, |this, cx| {
                this.tasks = tasks;
                this.skipped = skipped;
                this.scanning = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn close(&mut self, _: &CloseTaskDashboard, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    fn open_task(&mut self, task: &TaskItem, cx: &mut Context<Self>) {
        let path = task.path.to_string();
        let line = task.line;
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
                    editor.update(cx, |editor, cx| editor.scroll_to_line(line, window, cx));
                    window.activate_window();
                })
                .ok();
        })
        .ok();
    }
}

impl Focusable for TaskDashboardWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TaskDashboardWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // * Cloned out of the global: the sections below need `&mut cx`, and
        //   a borrow of the theme through `cx` would still be live.
        let theme = cx.global::<crate::theme::Theme>().clone();
        let pending: Vec<&TaskItem> = self.tasks.iter().filter(|t| !t.done).collect();
        let done: Vec<&TaskItem> = self.tasks.iter().filter(|t| t.done).collect();

        let mut header = format!(
            "Task Dashboard ({} pending, {} done)",
            pending.len(),
            done.len()
        );
        if self.scanning {
            header.push_str(" \u{2014} scanning\u{2026}");
        }
        if self.skipped > 0 {
            header.push_str(&format!(
                " \u{b7} {} unreadable file(s) skipped",
                self.skipped
            ));
        }

        let mut body = div()
            .id("task-dashboard-body")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll();
        if self.tasks.is_empty() && !self.scanning {
            body = body.child(
                div()
                    .px_4()
                    .py_6()
                    .text_size(px(13.))
                    .text_color(theme.text_muted)
                    .child("No tasks in this vault yet. Write `- [ ] something` in any note."),
            );
        } else {
            body = body
                .child(self.section("Pending", &pending, &theme, cx))
                .child(self.section("Completed", &done, &theme, cx));
        }

        div()
            .id("task-dashboard")
            .key_context("TaskDashboard")
            .on_action(cx.listener(Self::close))
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
    fn section(
        &self,
        title: &'static str,
        tasks: &[&TaskItem],
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut section = div().flex().flex_col();
        if !tasks.is_empty() {
            section = section.child(
                div()
                    .px_4()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(theme.text_muted)
                    .child(title),
            );
        }
        section.children(tasks.iter().map(|t| self.task_row(t, theme, cx)))
    }

    fn task_row(
        &self,
        task: &TaskItem,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let checkbox = if task.done { "[x]" } else { "[ ]" };
        let row = task.clone();
        div()
            .id(SharedString::from(format!(
                "task-{}:{}",
                task.path, task.line
            )))
            .flex()
            .items_center()
            .px_4()
            .py_1()
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
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (700., 500.);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    /// Both task notes are scanned, and a note that vanished between the
    /// index and the scan is counted rather than silently dropped: that
    /// is the race a real vault edit hits mid-scan.
    #[test]
    fn the_scan_counts_a_note_that_vanished_after_indexing() {
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
        // Indexed at load, gone from disk by the time the scan reads it.
        fs::remove_file(root.join("Gone.md")).unwrap();

        let (tasks, skipped) = scan_vault_for_tasks(&state);
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
        assert_eq!(skipped, 1, "the vanished note is counted, not dropped");
    }

    /// A vault with no task notes scans to nothing, not an error.
    #[test]
    fn an_empty_vault_scans_to_nothing() {
        let (_dir, state) = vault_with(&[("A.md", "# A\n"), ("B.md", "# B\n")]);
        let (tasks, skipped) = scan_vault_for_tasks(&state);
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
}
