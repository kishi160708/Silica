//!  application.
//!
//! ` <vault> [--note <path>]` opens a vault in the three-pane shell:
//! Files on the left, the reading view (or the source editor, Ctrl+E) in the
//! middle, and Backlinks, Outline and Tags on the right. `--synthetic N`
//! and `--stress` keep the editor spike from the first gate runnable; the
//! other flags open the app in a given state for screenshots and smoke
//! tests (see `docs//Development.md`).

mod chooser;
mod editor;
mod explorer;
mod graph;
mod mentions;
mod palette;
mod preview;
mod prompt;
mod properties;
mod reading;
mod search;
mod settings;
mod suggest;
mod switcher;
mod text_input;
mod theme;
mod watch;
mod workspace;

use gpui::{
    actions, prelude::*, px, size, App, Bounds, Focusable, KeyBinding, TitlebarOptions,
    WindowAppearance, WindowBounds, WindowOptions,
};
use _commands::CommandRegistry;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use theme::Theme;
use workspace::{Commands, ViewMode, Workspace};

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
    /// Open the graph view: `global` or `local`.
    graph: Option<String>,
    /// Open the command palette with this query, for screenshots.
    palette: Option<String>,
    /// Open Settings on this tab (`hotkeys`, `appearance`, ...).
    settings: Option<String>,
    /// Open the note in source mode.
    source: bool,
    /// Open the note in reading view, whatever `defaultViewMode` says.
    reading: bool,
    /// Open the find panel with this query.
    find: Option<String>,
    /// Open the find panel with this query and select every match (Find all).
    find_all: Option<String>,
    /// Rename the opened note to this name at startup (links rewritten).
    rename: Option<String>,
    /// Open a file prompt for the opened note: rename, move, delete, folder.
    prompt: Option<String>,
    /// Type this text at the cursor in the source editor, which is how the
    /// autocompletion popup is opened for a screenshot (`--type "[[al"`).
    type_text: Option<String>,
    /// Press these keys, comma separated, after `--type`: `enter`, `tab`,
    /// `shift-tab`, `shift-enter`, `escape`, `up`, `down`.
    accept: Option<String>,
    /// Open this cell of the note's first table for editing, before
    /// `--type`, the way clicking one does.
    cell: Option<usize>,
    /// Run these command ids at startup, comma separated, after everything
    /// else: `--command editor:table-row-after`.
    command: Option<String>,
    /// Open the template chooser and pick the template this matches.
    template: Option<String>,
    /// Open the Settings dropdown that writes this key, for screenshots.
    menu: Option<String>,
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
        graph: None,
        palette: None,
        settings: None,
        source: false,
        reading: false,
        find: None,
        find_all: None,
        rename: None,
        prompt: None,
        type_text: None,
        accept: None,
        cell: None,
        command: None,
        template: None,
        menu: None,
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
            "--graph" => args.graph = it.next().or(Some("global".into())),
            "--palette" => args.palette = it.next().or(Some(String::new())),
            "--settings" => args.settings = it.next().or(Some("general".into())),
            "--source" => args.source = true,
            "--reading" => args.reading = true,
            "--find" => args.find = it.next().or(Some(String::new())),
            "--find-all" => args.find_all = it.next().or(Some(String::new())),
            "--rename" => args.rename = it.next(),
            "--prompt" => args.prompt = it.next().or(Some("rename".into())),
            "--type" => args.type_text = it.next(),
            "--accept" | "--press" => args.accept = it.next().or(Some("enter".into())),
            "--cell" => args.cell = it.next().and_then(|n| n.parse().ok()),
            "--command" => args.command = it.next(),
            "--template" => args.template = it.next(),
            "--menu" => args.menu = it.next(),
            _ if args.vault.is_none() => args.vault = Some(PathBuf::from(a)),
            _ => args.note = Some(a),
        }
    }
    args
}

fn main() {
    let args = parse_args();
    // The theme comes from appearance.json and the key bindings from
    // hotkeys.json; read both before the window opens so the first frame
    // already has the right colors and keys.
    let vault = args
        .vault
        .as_ref()
        .and_then(|root| _vault::Vault::open(root).ok());
    // The window is what knows the desktop's colour scheme, and it does not
    // exist yet; dark is assumed here and the theme is rebuilt below once
    // the window can answer, which only matters for `theme: "system"`.
    let appearance = vault.as_ref().and_then(|v| v.appearance_config().ok());
    let theme = appearance
        .as_ref()
        .map(|a| Theme::from_appearance(a, true))
        .unwrap_or_else(Theme::dark);
    let mut registry = CommandRegistry::with_known_ids();
    if let Some(hotkeys) = vault.as_ref().and_then(|v| v.hotkeys().ok()) {
        registry.apply_overrides(&hotkeys);
    }
    drop(vault);
    let registry = Arc::new(registry);

    gpui_platform::application().run(move |cx: &mut App| {
        cx.set_global(theme);
        editor::bind_keys(cx);
        workspace::bind_keys(cx, &registry);
        cx.set_global(Commands(registry));
        text_input::bind_keys(cx);
        graph::bind_keys(cx);
        settings::bind_keys(cx);
        prompt::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);

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
        // Now there is a window to ask: `theme: "system"` follows the
        // desktop, which is Obsidian's default and what a vault with no
        // theme key means.
        if let Some(a) = appearance {
            let follows_system =
                !matches!(a.theme.as_deref(), Some("moonstone") | Some("obsidian"));
            if follows_system {
                let dark = window
                    .update(cx, |_, window, _| {
                        matches!(
                            window.appearance(),
                            WindowAppearance::Dark | WindowAppearance::VibrantDark
                        )
                    })
                    .unwrap_or(true);
                cx.set_global(Theme::from_appearance(&a, dark));
                window.update(cx, |_, _, cx| cx.notify()).ok();
            }
        }
        cx.on_action(move |_: &Quit, cx| {
            // Save before the window goes away.
            window.update(cx, |ws, _, cx| ws.flush_save(cx)).ok();
            cx.quit();
        });

        let note = args.note.clone();
        let search = args.search.clone();
        let switcher = args.switcher.clone();
        let pane = args.pane.clone();
        let graph = args.graph.clone();
        let palette = args.palette.clone();
        let settings_tab = args.settings.clone();
        let source = args.source;
        let type_text = args.type_text.clone();
        let accept = args.accept.clone();
        let cell = args.cell;
        let command = args.command.clone();
        let template = args.template.clone();
        let menu = args.menu.clone();
        let reading = args.reading;
        let find = args.find.clone();
        let find_all = args.find_all.clone();
        let rename = args.rename.clone();
        let prompt = args.prompt.clone();
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
                        // The vault's default view mode, unless a flag says otherwise.
                        let mode = if source {
                            ViewMode::Source
                        } else if reading {
                            ViewMode::Reading
                        } else {
                            ws.default_mode()
                        };
                        ws.set_mode(mode, window, cx);
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
                        if let Some(g) = &graph {
                            ws.show_graph(g == "local", window, cx);
                        }
                        if let Some(q) = &palette {
                            ws.open_palette(q, window, cx);
                        }
                        if let Some(t) = &settings_tab {
                            ws.open_settings(settings::Tab::parse(t), window, cx);
                        }
                        if let Some(q) = &find {
                            ws.open_find(q, false, window, cx);
                        }
                        if let Some(q) = &find_all {
                            ws.open_find(q, false, window, cx);
                            ws.editor()
                                .update(cx, |e, cx| e.find_all_matches(window, cx));
                        }
                        if let Some(name) = &rename {
                            ws.rename_current(name, window, cx);
                        }
                        if let Some(kind) = &prompt {
                            ws.open_file_prompt(kind, window, cx);
                        }
                        if let Some(ix) = cell {
                            ws.editor().update(cx, |e, cx| e.edit_cell(ix, window, cx));
                        }
                        if let Some(t) = &type_text {
                            ws.editor().update(cx, |e, cx| e.type_text(t, window, cx));
                        }
                        if let Some(keys) = &accept {
                            ws.editor()
                                .update(cx, |e, cx| e.press_keys(keys, window, cx));
                        }
                        if let Some(ids) = &command {
                            for id in ids.split(',') {
                                ws.run_command(id.trim(), window, cx);
                            }
                        }
                        if let Some(q) = &template {
                            ws.pick_template(q, window, cx);
                        }
                        if let Some(key) = &menu {
                            ws.open_settings_menu(key, cx);
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
                // Without `--synthetic` the run edits the opened note (and
                // saves it): wait for the vault to index and the note to open.
                while synthetic.is_none() {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(50))
                        .await;
                    let ready = window
                        .update(cx, |ws, window, cx| {
                            let open = ws.is_loaded() && ws.current().is_some();
                            if open && !ws.mode().is_editing() {
                                ws.set_mode(ViewMode::Source, window, cx);
                            }
                            open
                        })
                        .unwrap_or(true);
                    if ready {
                        break;
                    }
                }
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
                // Let the last parse land (and, on a real note, the autosave
                // fire) before reporting.
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(if synthetic.is_some() {
                        1500
                    } else {
                        3000
                    }))
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
