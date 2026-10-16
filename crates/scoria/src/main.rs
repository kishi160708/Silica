//!  application shell.
//!
//! Milestone 0: open a window with the three-pane layout (left dock, main
//! area, right dock) and a status bar, and show the vault passed on the
//! command line. The main area holds the editor prototype from the first
//! spike gate; click a note in the file list to open it, or pass
//! `--synthetic N` to open a generated note of N lines.

mod editor;

use editor::Editor;
use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Bounds, Context, Entity, Focusable, KeyBinding,
    SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use _commands::CommandRegistry;
use _vault::Vault;
use std::fmt::Write as _;
use std::path::PathBuf;

actions!(, [Quit]);

/// What the shell knows about the current vault, computed once at startup.
struct VaultSummary {
    name: SharedString,
    root: PathBuf,
    display_root: SharedString,
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
                root: PathBuf::new(),
                display_root: "Pass a vault folder as the first argument".into(),
                file_count: 0,
                note_count: 0,
                notes: Vec::new(),
                error: None,
            };
        };
        match Vault::open(&path) {
            Ok(vault) => VaultSummary {
                name: vault.name().into(),
                root: vault.root().to_path_buf(),
                display_root: vault.root().display().to_string().into(),
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
                root: path.clone(),
                display_root: path.display().to_string().into(),
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
    editor: Entity<Editor>,
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

    fn open_note(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rel) = self.vault.notes.get(index) else {
            return;
        };
        let path = self.vault.root.join(rel.as_ref());
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let name = rel.clone();
                self.editor
                    .update(cx, |editor, cx| editor.set_text(name, text, cx));
                window.focus(&self.editor.focus_handle(cx), cx);
            }
            Err(err) => {
                let msg = format!("Could not read {}: {err}", path.display());
                self.editor
                    .update(cx, |editor, cx| editor.set_text("error", msg, cx));
            }
        }
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let files = div()
            .flex()
            .flex_col()
            .children(self.vault.notes.iter().enumerate().map(|(ix, path)| {
                div()
                    .id(ix)
                    .px_3()
                    .py_1()
                    .text_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(BORDER)))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_note(ix, window, cx)),
                    )
                    .child(path.clone())
            }));

        let (editor_name, status) = {
            let editor = self.editor.read(cx);
            let cursor = editor.cursor();
            let stats = editor.stats;
            let mut s = String::new();
            let _ = write!(
                s,
                "Ln {}, Col {}  ·  {} lines, {} KB  ·  parse {:.1} ms, {} spans  ·  shape {:.2} ms",
                cursor.line + 1,
                cursor.col + 1,
                editor.line_count(),
                stats.bytes / 1024,
                stats.parse_ms,
                stats.spans,
                editor.prepaint_ms()
            );
            let name = if editor.dirty {
                format!("{} •", editor.name)
            } else {
                editor.name.to_string()
            };
            (name, s)
        };

        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .text_sm()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(div().text_color(rgb(ACCENT)).child(editor_name))
                    .child(
                        div()
                            .text_color(rgb(TEXT_MUTED))
                            .child(self.vault.name.clone()),
                    ),
            )
            .children(
                self.vault
                    .error
                    .clone()
                    .map(|e| div().px_4().text_sm().text_color(rgb(0xe06c75)).child(e)),
            )
            .child(div().flex_1().overflow_hidden().child(self.editor.clone()));

        let status_bar = div()
            .flex()
            .flex_row()
            .justify_between()
            .h(px(22.0))
            .px_3()
            .text_xs()
            .text_color(rgb(TEXT_MUTED))
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(status)
            .child(format!(
                "{} commands  ·  {} files, {} notes  ·  {}",
                self.command_count,
                self.vault.file_count,
                self.vault.note_count,
                self.vault.display_root
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
            .child(status_bar)
    }
}

/// A deterministic note of `lines` lines exercising headings, wikilinks,
/// tags, emphasis, code, lists, and callouts, for the 50k-line spike.
fn synthetic_note(lines: usize) -> String {
    const WORDS: &[&str] = &[
        "vault", "note", "link", "graph", "heading", "callout", "tag", "cursor", "scroll",
        "render", "parser", "cache", "index", "editor", "canvas", "search", "theme", "plugin",
    ];
    let mut out = String::with_capacity(lines * 48);
    let mut seed: u64 = 7;
    let mut pick = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        WORDS[(seed % WORDS.len() as u64) as usize]
    };
    out.push_str("---\ntitle: Synthetic\ntags: [spike]\n---\n");
    let mut n = 4;
    while n < lines {
        match n % 40 {
            0 => {
                let _ = writeln!(out, "\n## Section {} {}", n / 40, pick());
                n += 2;
            }
            7 => {
                let _ = writeln!(
                    out,
                    "> [!tip] {} {}\n> {} {} [[Note {}]]",
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    n
                );
                n += 2;
            }
            12..=15 => {
                let _ = writeln!(
                    out,
                    "- [ ] {} {} #{} [[Note {}|{}]]",
                    pick(),
                    pick(),
                    pick(),
                    n % 97,
                    pick()
                );
                n += 1;
            }
            20 => {
                let _ = writeln!(out, "```rust\nfn {}() {{ // [[not a link]] }}\n```", pick());
                n += 3;
            }
            _ => {
                let _ = writeln!(
                    out,
                    "{} {} **{}** {} [[Note {}#{}]] {} `{}` #{}/{} {} *{}* =={}== {}.",
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    n % 211,
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    pick(),
                    pick()
                );
                n += 1;
            }
        }
    }
    out
}

struct Args {
    vault: Option<PathBuf>,
    note: Option<String>,
    synthetic: Option<usize>,
    /// Number of self-driven edit and navigation steps to run before quitting.
    stress: Option<usize>,
}

fn parse_args() -> Args {
    let mut args = Args {
        vault: None,
        note: None,
        synthetic: None,
        stress: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--synthetic" => {
                args.synthetic = it.next().and_then(|n| n.parse().ok()).or(Some(50_000))
            }
            "--stress" => args.stress = it.next().and_then(|n| n.parse().ok()).or(Some(600)),
            "--note" => args.note = it.next(),
            _ if args.vault.is_none() => args.vault = Some(PathBuf::from(a)),
            _ => args.note = Some(a),
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let vault = VaultSummary::load(args.vault);
    let command_count = CommandRegistry::with_known_ids().len();

    gpui_platform::application().run(move |cx: &mut App| {
        editor::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| {
                    let editor = cx.new(Editor::new);
                    cx.new(|_| Shell {
                        vault,
                        command_count,
                        editor,
                    })
                },
            )
            .expect("open main window");

        window
            .update(cx, |shell, window, cx| {
                if let Some(lines) = args.synthetic {
                    let text = synthetic_note(lines);
                    shell.editor.update(cx, |editor, cx| {
                        editor.set_text(format!("synthetic-{lines}.md"), text, cx)
                    });
                } else if let Some(note) = &args.note {
                    let index =
                        shell.vault.notes.iter().position(|n| {
                            n.as_ref() == note.as_str() || n.ends_with(note.as_str())
                        });
                    if let Some(ix) = index {
                        shell.open_note(ix, window, cx);
                    }
                } else if !shell.vault.notes.is_empty() {
                    shell.open_note(0, window, cx);
                }
                window.focus(&shell.editor.focus_handle(cx), cx);
            })
            .expect("window update");
        cx.activate(true);

        if let Some(steps) = args.stress {
            cx.spawn(async move |cx| {
                let started = std::time::Instant::now();
                for step in 0..steps {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(8))
                        .await;
                    let _ = window.update(cx, |shell, window, cx| {
                        shell
                            .editor
                            .update(cx, |editor, cx| editor.stress_step(step, window, cx));
                    });
                }
                // Let the last parse land before reporting.
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1500))
                    .await;
                let report = window
                    .update(cx, |shell, _, cx| shell.editor.read(cx).report())
                    .unwrap_or_default();
                eprintln!(
                    "stress: {steps} steps in {:.1} s\n{report}",
                    started.elapsed().as_secs_f64()
                );
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}
