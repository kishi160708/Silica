//!  application.
//!
//! ` <vault> [--note <path>]` opens a vault in the three-pane shell:
//! Files on the left, the reading view (or the source editor, Ctrl+E) in the
//! middle, and Backlinks, Outline and Tags on the right. `--synthetic N`
//! and `--stress` keep the editor spike from the first gate runnable.

mod editor;
mod explorer;
mod mentions;
mod reading;
mod search;
mod switcher;
mod text_input;
mod theme;
mod watch;
mod workspace;

use gpui::{
    actions, prelude::*, px, size, App, Bounds, Focusable, KeyBinding, TitlebarOptions,
    WindowBounds, WindowOptions,
};
use std::fmt::Write as _;
use std::path::PathBuf;
use theme::Theme;
use workspace::{ViewMode, Workspace};

actions!(, [Quit]);

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
    /// Pre-filled Search query, for screenshots.
    search: Option<String>,
    /// Open the quick switcher with this query, for screenshots.
    switcher: Option<String>,
    /// Right-dock tab to show: backlinks, outgoing, outline, tags.
    pane: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        vault: None,
        note: None,
        synthetic: None,
        stress: None,
        search: None,
        switcher: None,
        pane: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--synthetic" => {
                args.synthetic = it.next().and_then(|n| n.parse().ok()).or(Some(50_000))
            }
            "--stress" => args.stress = it.next().and_then(|n| n.parse().ok()).or(Some(600)),
            "--note" => args.note = it.next(),
            "--search" => args.search = it.next(),
            "--switcher" => args.switcher = it.next(),
            "--pane" => args.pane = it.next(),
            _ if args.vault.is_none() => args.vault = Some(PathBuf::from(a)),
            _ => args.note = Some(a),
        }
    }
    args
}

fn main() {
    let args = parse_args();
    // The theme comes from appearance.json; read it before the window opens
    // so the first frame already has the right colors.
    let theme = args
        .vault
        .as_ref()
        .and_then(|root| _vault::Vault::open(root).ok())
        .and_then(|v| v.appearance_config().ok())
        .map(|a| Theme::from_appearance(&a))
        .unwrap_or_else(Theme::dark);

    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_global(theme);
        editor::bind_keys(cx);
        workspace::bind_keys(cx);
        text_input::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(1400.0), px(900.0)), cx);
        let vault = args.vault.clone();
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
                |window, cx| cx.new(|cx| Workspace::new(vault, window, cx)),
            )
            .expect("open main window");

        let note = args.note.clone();
        let search = args.search.clone();
        let switcher = args.switcher.clone();
        let pane = args.pane.clone();
        let synthetic = args.synthetic;
        window
            .update(cx, |ws, window, cx| {
                if let Some(lines) = synthetic {
                    let text = synthetic_note(lines);
                    ws.editor().update(cx, |editor, cx| {
                        editor.set_text(format!("synthetic-{lines}.md"), text, cx)
                    });
                    ws.set_mode(ViewMode::Source, window, cx);
                } else {
                    window.focus(&ws.focus_handle(cx), cx);
                }
            })
            .expect("window update");
        cx.activate(true);

        // The vault indexes in the background; open the requested note once it lands.
        if synthetic.is_none() {
            cx.spawn(async move |cx| loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                let done = window
                    .update(cx, |ws, window, cx| {
                        if !ws.is_loaded() {
                            return false;
                        }
                        match &note {
                            Some(n) => {
                                if !ws.open_by_name(n, cx) {
                                    ws.open_first(cx);
                                }
                            }
                            None => ws.open_first(cx),
                        }
                        if let Some(q) = &search {
                            ws.show_search(q, Some(window), cx);
                        }
                        if let Some(q) = &switcher {
                            ws.open_switcher(q, window, cx);
                        }
                        if let Some(p) = &pane {
                            ws.show_right_tab(p, cx);
                        }
                        true
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            })
            .detach();
        }

        if let Some(steps) = args.stress {
            cx.spawn(async move |cx| {
                let started = std::time::Instant::now();
                for step in 0..steps {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(8))
                        .await;
                    let _ = window.update(cx, |ws, window, cx| {
                        ws.editor()
                            .update(cx, |editor, cx| editor.stress_step(step, window, cx));
                    });
                }
                // Let the last parse land before reporting.
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1500))
                    .await;
                let report = window
                    .update(cx, |ws, _, cx| ws.editor().read(cx).report())
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
