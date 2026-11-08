//! Quick note: a one-window capture pad behind a single chord.
//!
//! Ctrl+Shift+N opens a small window, the note is typed in it and
//! Ctrl+Enter files it. The write goes through `Vault::create_note`,
//! which stats and indexes the one new file (`refresh_path`) instead of
//! rescanning the vault, so a capture never stalls the main window on a
//! large vault: the watcher that window already runs reports the new
//! file as the same one-path change.

use crate::editor::Editor;
use crate::text_input::{InputEvent, TextInput};
use crate::theme::Theme;
use gpui::{
    actions, div, prelude::*, px, size, App, Bounds, Context, Entity, FocusHandle, Focusable,
    FontWeight, KeyBinding, SharedString, Subscription, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowOptions,
};
use silica_vault::{FileChange, Vault};
use std::path::PathBuf;
use std::sync::OnceLock;

actions!(quick_note, [Open, Save, Close]);

/// The pad's size: a heading, a title, a body and two buttons with no
/// scrollbar at the default window size.
const WINDOW_SIZE: (f32, f32) = (560., 380.);

/// Every binding the pad owns, called where the app builds all of its
/// bindings (`bind_all_keys`). The chord is free in this app: the editor
/// takes Ctrl+Shift with the arrows and Z, settings Ctrl+Shift+F, and
/// Obsidian leaves Ctrl+Shift+N unbound for capture.
///
/// The app-level dispatch of `Open` is wired here once rather than per
/// call: `bind_all_keys` runs again whenever `hotkeys.json` changes, and
/// a second listener would open a second window for one chord.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-n", Open, None),
        KeyBinding::new("ctrl-enter", Save, Some("QuickNote")),
        KeyBinding::new("escape", Close, Some("QuickNote")),
    ]);
    static DISPATCHED: OnceLock<()> = OnceLock::new();
    if DISPATCHED.set(()).is_ok() {
        cx.on_action(|_: &Open, cx| {
            let _ = open(cx);
        });
    }
}

/// Opens the pad for the vault the registry last opened. A chord with no
/// vault at all says so on stderr and opens nothing: there is nowhere to
/// write, and a pad that saves nowhere is worse than no pad.
pub fn open(cx: &mut App) -> anyhow::Result<()> {
    let root = current_vault_root().ok_or_else(|| anyhow::anyhow!("no vault is open"))?;
    let vault = Vault::open(&root)?;
    let (w, h) = WINDOW_SIZE;
    let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Quick note".into()),
                ..Default::default()
            }),
            app_id: Some("silica".into()),
            window_decorations: Some(WindowDecorations::Server),
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(400.), px(280.))),
            ..Default::default()
        },
        move |window, cx| cx.new(|cx| QuickNote::new(vault, window, cx)),
    )?;
    Ok(())
}

/// The vault the registry last opened, the entry with the newest
/// timestamp — the same one the main window falls back to at startup.
fn current_vault_root() -> Option<PathBuf> {
    silica_vault::VaultRegistry::open()
        .ok()
        .flatten()
        .and_then(|registry| {
            registry
                .vaults()
                .into_iter()
                .max_by_key(|(_, entry)| entry.ts)
                .map(|(_, entry)| entry.path.clone())
        })
}

/// What the pad holds before it is a file: the title the file is named
/// after, and the body it is filled with.
pub struct Draft {
    pub title: String,
    pub body: String,
}

impl Draft {
    /// Whether there is anything to write. An empty pad closed with the
    /// chord still bound writes no file, rather than a note called
    /// "Untitled" with nothing in it.
    fn is_empty(&self) -> bool {
        self.title.trim().is_empty() && self.body.trim().is_empty()
    }
}

/// The file a draft becomes: where it goes and what it holds.
pub struct Plan {
    /// Vault-relative, with the name the vault numbered when it is taken.
    pub path: String,
    /// The file's whole text.
    pub content: String,
}

/// Whether a character is one no file name in the vault takes, the way
/// `validate_file_name` in the vault crate judges one.
fn illegal_in_name(c: char) -> bool {
    matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
}

/// The file name a title becomes: trimmed, with the illegal characters
/// replaced by `-`. `None` when nothing legal is left — a title of
/// separator characters alone names no file, the way Obsidian falls
/// back to "Untitled".
pub fn stem(title: &str) -> Option<String> {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| if illegal_in_name(c) { '-' } else { c })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == '-' || c == '.' || c == ' ');
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '-') {
        None
    } else {
        Some(cleaned.to_string())
    }
}

/// The whole text of the file: the title as a heading over the body, or
/// the body alone when there is no title to head it.
fn content(draft: &Draft) -> String {
    let title = draft.title.trim();
    let body = draft.body.trim_end();
    match (title.is_empty(), body.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("# {title}\n"),
        (true, false) => format!("{body}\n"),
        (false, false) => format!("# {title}\n\n{body}\n"),
    }
}

/// The path a draft takes in the vault root: its title's stem, or
/// "Untitled" when there is none, numbered by the vault when the name is
/// already there (Obsidian's "Untitled 1").
pub fn plan(vault: &Vault, draft: &Draft) -> Plan {
    let stem = stem(&draft.title).unwrap_or_else(|| "Untitled".to_string());
    Plan {
        path: vault.available_path("", &stem, "md"),
        content: content(draft),
    }
}

/// Writes the draft and answers with the change the index picked up.
/// `create_note` stats and indexes the one new file instead of
/// rescanning, which is what keeps a capture off the main window's
/// reload path. The message is what the pad shows when the write fails.
pub fn save(vault: &mut Vault, draft: &Draft) -> Result<Vec<FileChange>, String> {
    if draft.is_empty() {
        return Err("Nothing to save yet.".to_string());
    }
    let plan = plan(vault, draft);
    vault
        .create_note(&plan.path, &plan.content)
        .map_err(|error| error.to_string())
}

/// The capture pad: the title the file is named after, the body it is
/// filled with, and the error line a failed save leaves on screen.
pub struct QuickNote {
    focus_handle: FocusHandle,
    vault: Vault,
    title: Entity<TextInput>,
    body: Entity<Editor>,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl QuickNote {
    fn new(vault: Vault, window: &mut Window, cx: &mut Context<Self>) -> QuickNote {
        let title = cx.new(|cx| TextInput::new("Title", cx));
        let body = cx.new(|cx| Editor::new(window, cx));
        // * The pad opens on the body: the title is optional, and the
        // * note is what gets typed.
        let body_handle = body.focus_handle(cx);
        window.focus(&body_handle, cx);
        let subscriptions =
            vec![
                cx.subscribe_in(&title, window, |this, _, event: &InputEvent, window, cx| {
                    match event {
                        // * Enter in the title moves to the body rather than
                        // * submitting: a title alone is not a note.
                        InputEvent::Submit => {
                            let handle = this.body.focus_handle(cx);
                            window.focus(&handle, cx);
                        }
                        InputEvent::Cancel => window.remove_window(),
                        _ => {}
                    }
                }),
            ];
        QuickNote {
            focus_handle: cx.focus_handle(),
            vault,
            title,
            body,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn draft(&self, cx: &App) -> Draft {
        Draft {
            title: self.title.read(cx).text().to_string(),
            body: self.body.read(cx).text().to_string(),
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.draft(cx);
        match save(&mut self.vault, &draft) {
            // * The pad closes on success the way a capture tool does:
            // * the note is filed and the window that asked for it is
            // * gone.
            Ok(_) => window.remove_window(),
            Err(message) => {
                self.error = Some(message);
                cx.notify();
            }
        }
    }

    fn save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        self.submit(window, cx);
    }

    fn close(&mut self, _: &Close, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
    }
}

impl Render for QuickNote {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let button = |id: &'static str, label: SharedString, primary: bool, theme: &Theme| {
            div()
                .id(id)
                .px_3()
                .py_1()
                .rounded_md()
                .cursor_pointer()
                .text_size(px(13.))
                .when(primary, |d| d.bg(theme.accent).text_color(gpui::white()))
                .when(!primary, |d| {
                    d.bg(theme.bg_alt)
                        .text_color(theme.text)
                        .hover(|s| s.bg(theme.bg_active))
                })
                .child(label)
        };
        div()
            .id("quick-note")
            .key_context("QuickNote")
            // * gpui focuses a tracked handle on every mouse-down inside
            // * the element, so this has to be the handle `Focusable`
            // * hands out: a click in the body must not take the keyboard
            // * away from it.
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::close))
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_4()
            .bg(theme.bg)
            .text_color(theme.text)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Quick note"),
            )
            .child(self.title.clone())
            .child(
                div()
                    .id("quick-note-body")
                    .flex()
                    .flex_1()
                    .min_h(px(140.))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(self.body.clone()),
            )
            .children(self.error.clone().map(|message| {
                div()
                    .text_size(px(12.))
                    .text_color(gpui::hsla(0., 0.7, 0.6, 1.))
                    .child(message)
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .mt_1()
                    .child(
                        button("quick-note-cancel", "Cancel".into(), false, &theme)
                            .on_click(cx.listener(|_, _, window, _| window.remove_window())),
                    )
                    .child(
                        button("quick-note-save", "Save".into(), true, &theme)
                            .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(title: &str, body: &str) -> Draft {
        Draft {
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    fn vault_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().expect("a temp vault");
        for (name, text) in files {
            std::fs::write(dir.path().join(name), text).expect("a note on disk");
        }
        let vault = Vault::open(dir.path()).expect("open the temp vault");
        (dir, vault)
    }

    #[test]
    fn a_title_becomes_the_stem_of_the_name() {
        assert_eq!(stem("  Hello World  ").as_deref(), Some("Hello World"));
        assert_eq!(
            stem("a/b:c*d?e\"f<g>h|i").as_deref(),
            Some("a-b-c-d-e-f-g-h-i")
        );
        assert_eq!(stem("note...").as_deref(), Some("note"));
        assert_eq!(stem("///"), None);
        assert_eq!(stem("..."), None);
        assert_eq!(stem("   "), None);
    }

    #[test]
    fn a_fresh_draft_takes_its_title_as_the_path() {
        let (_dir, vault) = vault_with(&[]);
        let planned = plan(&vault, &draft("Hello World", "Body"));
        assert_eq!(planned.path, "Hello World.md");
        assert_eq!(planned.content, "# Hello World\n\nBody\n");
    }

    #[test]
    fn a_taken_name_is_numbered_by_the_vault() {
        let (_dir, vault) = vault_with(&[("Untitled.md", "old"), ("Hello.md", "old")]);
        assert_eq!(plan(&vault, &draft("", "Body")).path, "Untitled 1.md");
        assert_eq!(plan(&vault, &draft("Hello", "Body")).path, "Hello 1.md");
    }

    #[test]
    fn the_content_is_the_title_over_the_body() {
        let (_dir, vault) = vault_with(&[]);
        let both = plan(&vault, &draft("Title", "Body\n"));
        assert_eq!(both.content, "# Title\n\nBody\n");
        let titled = plan(&vault, &draft("Title", "  "));
        assert_eq!(titled.content, "# Title\n");
        let untitled = plan(&vault, &draft("", "Body\n"));
        assert_eq!(untitled.content, "Body\n");
    }

    #[test]
    fn an_empty_draft_writes_nothing() {
        let (dir, mut vault) = vault_with(&[("Other.md", "other")]);
        let before = vault.files().to_vec();
        let saved = save(&mut vault, &draft("  ", "  \n"));
        assert_eq!(saved.unwrap_err(), "Nothing to save yet.");
        assert_eq!(vault.files(), before.as_slice());
        assert!(!dir.path().join("Untitled.md").exists());
    }

    #[test]
    fn saving_indexes_the_one_new_file() {
        let (dir, mut vault) = vault_with(&[("Other.md", "other")]);
        let before = vault.files().len();
        let changes = save(&mut vault, &draft("Hello World", "Body")).expect("the write");
        // * One path in, one path out: a rescan would have reindexed the
        // * vault, and this is the signal that only the new file was
        // * touched.
        assert_eq!(
            changes,
            vec![FileChange::Added("Hello World.md".to_string())]
        );
        assert_eq!(vault.files().len(), before + 1);
        let indexed: Vec<&str> = vault.files().iter().map(|f| f.path.as_str()).collect();
        assert!(indexed.contains(&"Other.md"));
        assert!(indexed.contains(&"Hello World.md"));
        let on_disk = std::fs::read_to_string(dir.path().join("Hello World.md")).expect("read");
        assert_eq!(on_disk, "# Hello World\n\nBody\n");
    }
}
