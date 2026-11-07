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
    actions, div, prelude::*, px, size, App, Bounds, Context, Entity, FocusHandle, Focusable,
    KeyBinding, Render, SharedString, Subscription, TitlebarOptions, Window,
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
            let view = cx.new(|cx| QuickCaptureWindow::new(state, cx));
            let handle = view.read(cx).focus_handle.clone();
            window.focus(&handle, cx);
            view
        },
    )
}

/// The Quick Capture window's root view.
pub struct QuickCaptureWindow {
    focus_handle: FocusHandle,
    input: Entity<TextInput>,
    state: Arc<VaultState>,
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
    fn new(state: Arc<VaultState>, cx: &mut Context<Self>) -> QuickCaptureWindow {
        let input = cx.new(|cx| TextInput::new("Note title...", cx));
        let subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Submit = event {
                this.save(cx);
            }
        });
        QuickCaptureWindow {
            focus_handle: cx.focus_handle(),
            input,
            state,
            status: None,
            _subscriptions: vec![subscription],
        }
    }

    fn close(&mut self, _: &CloseQuickCapture, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    /// Writes the field's text as a new note, or reports why it could not.
    fn save(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        let folder = self
            .state
            .app()
            .new_file_folder_path
            .clone()
            .unwrap_or_default();
        let path = self.state.vault().available_path(&folder, &text, "md");
        let body = format!("# {text}\n");
        self.status = Some(match self.state.create_note(&path, &body) {
            Ok(_) => Ok(SharedString::from("Saved. Escape closes.")),
            Err(err) => Err(SharedString::from(format!("Save failed: {err}"))),
        });
        cx.notify();
    }
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
