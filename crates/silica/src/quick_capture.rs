//! Quick Capture: a small floating window for jotting down a note fast,
//! like Lilo's widget mode. Type the note's content, press Enter to save
//! it into the vault's default new-note folder, Escape to close without
//! saving.

use crate::workspace::VaultState;
use gpui::{
    div, prelude::*, px, size, App, Bounds, Context, Entity, FocusHandle, Focusable,
    Render, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowHandle, WindowOptions,
};
use crate::text_input::{InputEvent, TextInput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Open a Quick Capture window next to `main`, bringing it forward if it is
/// already open.
pub fn open(
    state: Arc<VaultState>,
    _main: gpui::AnyWindowHandle,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<QuickCaptureWindow>> {
    let (w, h) = WINDOW_SIZE;
    let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
    let state_for_sub = state.clone();
    let saved = Arc::new(AtomicBool::new(false));
    let saved_for_sub = saved.clone();
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
            let input = cx.new(|cx| TextInput::new("Type a note...", cx));
            let view = cx.new(|cx| QuickCaptureWindow {
                focus_handle: cx.focus_handle(),
                input: input.clone(),
                saved: saved.clone(),
            });
            let handle = view.read(cx).focus_handle.clone();
            let _subscription = cx.subscribe(&input, move |this: Entity<TextInput>, event: &InputEvent, cx: &mut App| {
                if let InputEvent::Submit = event {
                    let text = this.read(cx).text();
                    let text = text.trim();
                    if text.is_empty() {
                        return;
                    }
                    let title = text
                        .lines()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or(text)
                        .trim();
                    let folder = state_for_sub
                        .app()
                        .new_file_folder_path
                        .clone()
                        .unwrap_or_default();
                    let path = state_for_sub.vault().available_path(&folder, title, "md");
                    let body = if text.lines().count() > 1 {
                        text.to_string()
                    } else {
                        format!("# {title}\n")
                    };
                    match state_for_sub.create_note(&path, &body) {
                        Ok(_) => {
                            saved_for_sub.store(true, Ordering::Relaxed);
                        }
                        Err(err) => {
                            eprintln!("Quick Capture save failed: {err}");
                        }
                    }
                }
            });
            window.focus(&handle, cx);
            view
        },
    )
}

/// The Quick Capture window's root view.
pub struct QuickCaptureWindow {
    focus_handle: FocusHandle,
    input: Entity<TextInput>,
    saved: Arc<AtomicBool>,
}

impl Focusable for QuickCaptureWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuickCaptureWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>();
        let saved = self.saved.load(Ordering::Relaxed);
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
                            .child(if saved {
                                "Saved. Close this window."
                            } else {
                                "Enter saves · Escape closes"
                            }),
                    ),
            )
    }
}

/// Window size.
const WINDOW_SIZE: (f32, f32) = (620., 420.);
