//! Task Dashboard: scan every note in the vault for task list items
//! (`- [ ]` and `- [x]`) and display them in one list, like Logseq's
//! task dashboard or Capacities' task view.
//! Clicking a task opens its source note at the right line.

use crate::workspace::VaultState;
use gpui::{
    actions, div, prelude::*, px, size, App, Bounds, Context, FocusHandle, Focusable,
    Render, SharedString, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions,
};
use silica_markdown::{parse, BlockKind, InlineKind};
use std::sync::Arc;

actions!(task_dashboard, [OpenTask, CloseTaskDashboard]);

/// Open the Task Dashboard window.
pub fn open(
    state: Arc<VaultState>,
    _main: gpui::AnyWindowHandle,
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
            let tasks = scan_vault_for_tasks(&state);
            let view = cx.new(|cx| TaskDashboardWindow {
                focus_handle: cx.focus_handle(),
                tasks,
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
    /// The zero-based line number of the task in the note.
    line: usize,
    /// The task's text (the list item content after the checkbox).
    text: SharedString,
    /// Whether the task is done (`[x]`) or pending (`[ ]`).
    done: bool,
}

/// Scan every markdown note in the vault for task list items.
fn scan_vault_for_tasks(state: &VaultState) -> Vec<TaskItem> {
    let mut tasks = Vec::new();
    let vault = state.vault();
    let files = vault.markdown_files();

    for file in files {
        let path = file.path.clone();
        let content = match std::fs::read_to_string(vault.absolute(&path)) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let doc = parse(&content);
        for (line_ix, block) in doc.blocks.iter().enumerate() {
            if let BlockKind::List { items, .. } = &block.kind {
                for item in items {
                    if let Some(task_char) = item.task {
                        let done = task_char == 'x';
                        let text = item
                            .children
                            .iter()
                            .filter_map(|child| match &child.kind {
                                BlockKind::Paragraph { content } => {
                                    let t: String = content
                                        .iter()
                                        .filter_map(|inline| match &inline.kind {
                                            InlineKind::Text(t) => Some(t.to_string()),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    if t.is_empty() { None } else { Some(t) }
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ");

                        tasks.push(TaskItem {
                            path: path.clone().into(),
                            line: line_ix,
                            text: text.into(),
                            done,
                        });
                    }
                }
            }
        }
    }

    tasks
}

/// The Task Dashboard window's root view.
pub struct TaskDashboardWindow {
    focus_handle: FocusHandle,
    tasks: Vec<TaskItem>,
}

impl Focusable for TaskDashboardWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TaskDashboardWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>();
        let pending: Vec<&TaskItem> = self.tasks.iter().filter(|t| !t.done).collect();
        let done: Vec<&TaskItem> = self.tasks.iter().filter(|t| t.done).collect();

        div()
            .id("task-dashboard")
            .flex()
            .flex_col()
            .w(px(100.))
            .h(px(100.))
            .bg(theme.bg)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.text)
                    .px_4()
                    .py_2()
                    .child(format!(
                        "Task Dashboard ({} pending, {} done)",
                        pending.len(),
                        done.len()
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .children(pending.iter().map(|t| self.task_row(t, theme)))
                    .children(done.iter().map(|t| self.task_row(t, theme))),
            )
    }
}

impl TaskDashboardWindow {
    fn task_row(&self, task: &TaskItem, theme: &crate::theme::Theme) -> impl IntoElement {
        let checkbox = if task.done { "[x]" } else { "[ ]" };
        div()
            .flex()
            .items_center()
            .px_4()
            .py_1()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(if task.done { theme.text_faint } else { theme.text })
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
                    .child(format!("{} (line {})", task.path, task.line)),
            )
            .cursor_pointer()
            .hover(|s| s.bg(theme.bg_alt))
    }
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (700., 500.);
