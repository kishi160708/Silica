//! Quick Capture: a small floating window for jotting down a note fast,
//! like Lilo's widget mode. Type a one-line title, press Enter to save it
//! into the vault's default new-note folder, Escape to close without
//! saving.
//!
//! The field is single-line on purpose: this is a capture box, not an
//! editor. A note that needs a body is opened in the editor proper, where
//! the multiline input already lives.

use crate::text_input::{InputEvent, TextInput};
use crate::workspace::VaultState;
use gpui::{
    actions, div, prelude::*, px, size, AnyWindowHandle, App, Bounds, Context, Entity, FocusHandle,
    Focusable, KeyBinding, Render, SharedString, Subscription, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowHandle, WindowOptions,
};
use std::sync::Arc;

actions!(quick_capture, [CloseQuickCapture]);

/// Escape closes the capture window. Bound to this window's key context so
/// it cannot swallow Escape anywhere else in the app.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "escape",
        CloseQuickCapture,
        Some("QuickCapture"),
    )]);
}

/// Open a Quick Capture window, bringing it forward if it is already open.
pub fn open(
    state: Arc<VaultState>,
    _main: gpui::AnyWindowHandle,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<QuickCaptureWindow>> {
    let (w, h) = WINDOW_SIZE;
    let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Quick Capture".into()),
                ..Default::default()
            }),
            app_id: Some("silica-quick-capture".into()),
            window_decorations: Some(WindowDecorations::Server),
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(400.), px(300.))),
            ..Default::default()
        },
        move |window, cx| {
            // * The window's own handle: the view needs it to close
            //   itself, and a view has no other way to reach the window
            //   it lives in.
            let handle = window.window_handle();
            let view = cx.new(|cx| QuickCaptureWindow::new(state, handle, cx));
            // * The field is focused, not the view: a focused view with an
            //   unfocused field inside it receives the keystrokes and
            //   drops them, and the capture box would open deaf.
            let field = view.read(cx).input.focus_handle(cx);
            window.focus(&field, cx);
            view
        },
    )
}

/// The Quick Capture window's root view.
pub struct QuickCaptureWindow {
    focus_handle: FocusHandle,
    input: Entity<TextInput>,
    state: Arc<VaultState>,
    /// This window, so the view can close itself when the field cancels:
    /// a subscription callback has a context but no window, and the
    /// window's own handle is the only way back to one.
    window: AnyWindowHandle,
    /// What the last save did, shown under the field. `None` before the
    /// first attempt.
    status: Option<Result<SharedString, SharedString>>,
    /// Held for the life of the window so `Submit` and `Cancel` keep
    /// arriving.
    _subscriptions: Vec<Subscription>,
}

impl QuickCaptureWindow {
    /// Builds the view and subscribes to its field. The subscription is
    /// stored on the view: a local would be dropped as soon as this returns
    /// and Enter would never arrive.
    fn new(
        state: Arc<VaultState>,
        window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> QuickCaptureWindow {
        let input = cx.new(|cx| TextInput::new("Note title...", cx));
        let subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            match event {
                InputEvent::Submit => this.save(cx),
                // * The field swallows Escape for its own Cancel binding,
                //   so the keystroke never reaches the view's own
                //   CloseQuickCapture action. Cancelling is still closing
                //   without saving, which is what Escape is for.
                InputEvent::Cancel => this.close_window(cx),
                _ => {}
            }
        });
        QuickCaptureWindow {
            focus_handle: cx.focus_handle(),
            input,
            state,
            window,
            status: None,
            _subscriptions: vec![subscription],
        }
    }

    fn close(&mut self, _: &CloseQuickCapture, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    /// Closes this window from a callback that has no window of its own.
    fn close_window(&mut self, cx: &mut Context<Self>) {
        self.window
            .update(cx, |_, window, _| window.remove_window())
            .ok();
    }

    /// Writes the field's text as a new note, or reports why it could not.
    fn save(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        let (path, body) = saved_note(&self.state, &text);
        self.status = Some(match self.state.create_note(&path, &body) {
            Ok(_) => Ok(SharedString::from("Saved. Escape closes.")),
            Err(err) => Err(SharedString::from(format!("Save failed: {err}"))),
        });
        cx.notify();
    }
}

/// The vault path and body a captured title is written as.
///
/// Split from `QuickCaptureWindow::save` so the naming rules — the
/// new-note folder, the collision counter, the heading body — are
/// testable without a window.
fn saved_note(state: &VaultState, text: &str) -> (String, String) {
    let folder = state.app().new_file_folder_path.clone().unwrap_or_default();
    let path = state.vault().available_path(&folder, text, "md");
    let body = format!("# {text}\n");
    (path, body)
}

impl Focusable for QuickCaptureWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuickCaptureWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>();
        let (hint, hint_color) = match &self.status {
            None => (
                SharedString::from("Enter saves \u{b7} Escape closes"),
                theme.text_faint,
            ),
            Some(Ok(msg)) => (msg.clone(), theme.text_muted),
            // * An error the user cannot see is the same as no error
            //   handling at all, which is why this is on screen and not
            //   only on stderr.
            Some(Err(msg)) => (msg.clone(), theme.text),
        };
        div()
            .id("quick-capture")
            .key_context("QuickCapture")
            .on_action(cx.listener(Self::close))
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.bg)
            .p(px(24.))
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.text_muted)
                    .child("Type a note title, press Enter to save"),
            )
            .child(div().w_full().child(self.input.clone()))
            .child(div().text_size(px(11.)).text_color(hint_color).child(hint))
    }
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (620., 240.);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A temp vault whose `app.json` is exactly `app`.
    fn vault(app_json: &str) -> (tempfile::TempDir, VaultState) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(root.join(".obsidian/app.json"), app_json).unwrap();
        let state = VaultState::load(&root.to_path_buf()).unwrap();
        (dir, state)
    }

    /// A first capture lands at the title's name in the vault root, with
    /// the title as an H1, and the note is really on disk.
    #[test]
    fn a_first_capture_lands_in_the_vault_root() {
        let (dir, state) = vault(r#"{"trashOption":"local"}"#);
        let (path, body) = saved_note(&state, "Meeting notes");
        assert_eq!(path, "Meeting notes.md");
        assert_eq!(body, "# Meeting notes\n");
        state.create_note(&path, &body).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(&path)).unwrap(),
            "# Meeting notes\n"
        );
    }

    /// A second capture of the same title takes the next free name: the
    /// first note is never overwritten by a repeat submission.
    #[test]
    fn a_second_capture_never_overwrites_the_first() {
        let (dir, state) = vault(r#"{"trashOption":"local"}"#);
        let (first, body) = saved_note(&state, "Ideas");
        assert_eq!(first, "Ideas.md");
        state.create_note(&first, &body).unwrap();
        let (second, _) = saved_note(&state, "Ideas");
        assert_eq!(second, "Ideas 1.md");
        state.create_note(&second, &body).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("Ideas.md")).unwrap(),
            "# Ideas\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("Ideas 1.md")).unwrap(),
            "# Ideas\n"
        );
    }

    /// The new-note folder setting redirects the capture, creating the
    /// folder if it does not exist yet.
    #[test]
    fn the_new_note_folder_setting_redirects_the_capture() {
        let (dir, state) = vault(r#"{"newFileFolderPath":"Inbox"}"#);
        let (path, body) = saved_note(&state, "Scratch");
        assert_eq!(path, "Inbox/Scratch.md");
        state.create_note(&path, &body).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("Inbox/Scratch.md")).unwrap(),
            "# Scratch\n"
        );
    }

    /// A Unicode title survives the whole round trip: path, body, disk.
    #[test]
    fn a_unicode_title_survives() {
        let (dir, state) = vault(r#"{"trashOption":"local"}"#);
        let (path, body) = saved_note(&state, "Café ☕ 日本語");
        assert_eq!(path, "Café ☕ 日本語.md");
        assert_eq!(body, "# Café ☕ 日本語\n");
        state.create_note(&path, &body).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(&path)).unwrap(),
            "# Café ☕ 日本語\n"
        );
    }

    /// A title with a path separator stays inside the vault: the segments
    /// are validated, so the note lands in a subfolder rather than at some
    /// arbitrary place, and no segment can escape the vault root.
    #[test]
    fn a_title_with_a_path_separator_stays_inside_the_vault() {
        let (dir, state) = vault(r#"{"trashOption":"local"}"#);
        let (path, body) = saved_note(&state, "plans/secret");
        assert_eq!(path, "plans/secret.md");
        state.create_note(&path, &body).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("plans/secret.md")).unwrap(),
            "# plans/secret\n"
        );
    }

    /// A title that would climb out of the vault is refused: `..` starts
    /// with a dot, so nothing escapes the vault root.
    #[test]
    fn a_climbing_title_is_refused() {
        let (dir, state) = vault(r#"{"trashOption":"local"}"#);
        let (path, body) = saved_note(&state, "../escape");
        assert!(
            state.create_note(&path, &body).is_err(),
            "a leading .. must not leave the vault"
        );
        assert!(!dir.path().parent().unwrap().join("escape.md").exists());
    }
}
