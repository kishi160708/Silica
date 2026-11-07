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

        let doc = parse(&content);
        let index = LineIndex::new(&content);
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
                    path: path.clone().into(),
                    // * The item's own span, not the block's index: a block
                    //   index is not a line once a document has headings,
                    //   blank lines, or more than one block.
                    line: index.pos(item.span.start).line as usize,
                    text: text.trim().into(),
                    done: task_char.eq_ignore_ascii_case(&'x'),
                });
            }
        });
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
