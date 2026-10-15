//!  application shell.
//!
//! Milestone 0: open a window with the three-pane layout (left dock, main
//! area, right dock) and a status bar, and show the vault passed on the
//! command line. Everything else is a placeholder for later milestones.

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use _commands::CommandRegistry;
use _vault::Vault;
use std::path::PathBuf;

/// What the shell knows about the current vault, computed once at startup.
struct VaultSummary {
    name: SharedString,
    root: SharedString,
    file_count: usize,
    note_count: usize,
    notes: Vec<SharedString>,
    error: Option<SharedString>,
}

impl VaultSummary {
    fn load(path: Option<PathBuf>) -> VaultSummary {
        let Some(path) = path else {
            return VaultSummary {
                name: "No vault".into(),
                root: "Pass a vault folder as the first argument".into(),
                file_count: 0,
                note_count: 0,
                notes: Vec::new(),
                error: None,
            };
        };
        match Vault::open(&path) {
            Ok(vault) => VaultSummary {
                name: vault.name().into(),
                root: vault.root().display().to_string().into(),
                file_count: vault.files().len(),
                note_count: vault.markdown_files().count(),
                notes: vault
                    .markdown_files()
                    .take(500)
                    .map(|f| SharedString::from(f.path.clone()))
                    .collect(),
                error: None,
            },
            Err(err) => VaultSummary {
                name: "Vault failed to open".into(),
                root: path.display().to_string().into(),
                file_count: 0,
                note_count: 0,
                notes: Vec::new(),
                error: Some(err.to_string().into()),
            },
        }
    }
}

struct Shell {
    vault: VaultSummary,
    command_count: usize,
}

const BG: u32 = 0x1e1e1e;
const BG_SIDEBAR: u32 = 0x202020;
const BORDER: u32 = 0x2f2f2f;
const TEXT: u32 = 0xdcddde;
const TEXT_MUTED: u32 = 0x9a9a9a;
const ACCENT: u32 = 0x7f6df2;

impl Shell {
    fn dock(&self, title: &'static str, body: impl IntoElement) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(280.0))
            .h_full()
            .bg(rgb(BG_SIDEBAR))
            .border_color(rgb(BORDER))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(TEXT_MUTED))
                    .child(title),
            )
            .child(div().flex_1().overflow_hidden().child(body))
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let files = div().flex().flex_col().children(
            self.vault
                .notes
                .iter()
                .map(|path| div().px_3().py_1().text_sm().child(path.clone())),
        );

        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(ACCENT))
                    .child(self.vault.name.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(TEXT_MUTED))
                    .child(self.vault.root.clone()),
            )
            .children(
                self.vault
                    .error
                    .clone()
                    .map(|e| div().text_sm().text_color(rgb(0xe06c75)).child(e)),
            );

        let status = div()
            .flex()
            .flex_row()
            .justify_end()
            .gap_4()
            .h(px(22.0))
            .px_3()
            .text_xs()
            .text_color(rgb(TEXT_MUTED))
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(format!("{} commands", self.command_count))
            .child(format!(
                "{} files, {} notes",
                self.vault.file_count, self.vault.note_count
            ));

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.dock("FILES", files))
                    .child(main)
                    .child(self.dock("BACKLINKS", div())),
            )
            .child(status)
    }
}

fn main() {
    let vault_path = std::env::args().nth(1).map(PathBuf::from);
    let vault = VaultSummary::load(vault_path);
    let command_count = CommandRegistry::with_known_ids().len();

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Shell {
                    vault,
                    command_count,
                })
            },
        )
        .expect("open main window");
        cx.activate(true);
    });
}
