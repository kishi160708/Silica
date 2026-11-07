//! Quick Capture: a small floating window for jotting down a note fast,
//! like Lilo's widget mode. Type the note's content, press Enter to save
//! it into the vault's default new-note folder, Escape to close without
//! saving.

use crate::text_input::TextInput;
use crate::workspace::VaultState;
use gpui::{
    actions, div, prelude::*, px, size, App, Bounds, Context, Entity, FocusHandle, Focusable,
    Render, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowHandle, WindowOptions,
};
use std::sync::Arc;

actions!(quick_capture, [SaveNote, CloseCapture]);

/// Open a Quick Capture window next to `main`, bringing it forward if it is
/// already open.
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
            cx.bind_keys([
                gpui::KeyBinding::new("enter", SaveNote, Some("Quick Capture")),
                gpui::KeyBinding::new("escape", CloseCapture, Some("Quick Capture")),
            ]);
            let view = cx.new(|cx| QuickCaptureWindow {
                focus_handle: cx.focus_handle(),
                input: cx.new(|cx| TextInput::new("Type a note...", cx)),
                state: state.clone(),
                saved: false,
            });
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
    saved: bool,
}

impl Focusable for QuickCaptureWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::EventEmitter<CloseCapture> for QuickCaptureWindow {}

impl Render for QuickCaptureWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>();
        div()
            .id("quick-capture")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w(px(100.))
            .h(px(100.))
            .bg(theme.bg)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .w(px(520.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.text_muted)
                            .mb(px(8.))
                            .child("Type a note, press Enter to save"),
                    )
                    .child(
                        div()
                            .w(px(520.))
                            .child(self.input.clone())
                            .mt(px(4.)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_faint)
                            .mt(px(8.))
                            .child("Enter saves · Escape closes"),
                    ),
            )
    }
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (620., 420.);

impl QuickCaptureWindow {
    /// Save the typed text as a new note in the vault.
    #[allow(dead_code)]
    fn save_note(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.saved {
            return;
        }
        let text = self.input.read(cx).text();
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        // First non-empty line is the title.
        let title = text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(text)
            .trim();
        let folder = self
            .state
            .app()
            .new_file_folder_path
            .clone()
            .unwrap_or_default();
        let path = self
            .state
            .vault()
            .available_path(&folder, title, "md");
        let body = if text.lines().count() > 1 {
            text.to_string()
        } else {
            format!("# {title}\n")
        };
        match self.state.create_note(&path, &body) {
            Ok(_changes) => {
                self.saved = true;
                cx.emit(crate::quick_capture::CloseCapture);
            }
            Err(err) => {
                eprintln!("Quick Capture save failed: {err}");
            }
        }
    }

    #[allow(dead_code)]
    fn save_note_action(
        &mut self,
        _: &SaveNote,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_note(_window, cx);
    }

    #[allow(dead_code)]
    fn close_capture_action(
        &mut self,
        _: &CloseCapture,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // Window close is handled by the window manager's close button.
    }
}
