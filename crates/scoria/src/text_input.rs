//! A single-line text field for the quick switcher and search.
//!
//! Text comes in through GPUI's input handler so IME composition and dead
//! keys work; navigation and editing keys are actions. The field paints
//! itself with a canvas element: the line is shaped once per frame, scrolled
//! horizontally to keep the cursor visible, and the input handler is
//! registered with the painted bounds.

use crate::theme::Theme;
use gpui::{
    actions, canvas, div, fill, point, prelude::*, px, size, App, Bounds, Context, DispatchPhase,
    ElementInputHandler, EntityInputHandler, EventEmitter, FocusHandle, Focusable, KeyBinding,
    MouseButton, MouseDownEvent, Pixels, ShapedLine, SharedString, TextAlign, UTF16Selection,
    Window,
};
use std::ops::Range;

actions!(
    text_input,
    [
        Left,
        Right,
        Home,
        End,
        Backspace,
        Delete,
        DeleteWordBackward,
        Submit,
        Cancel,
        Up,
        Down,
        Clear,
    ]
);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("TextInput");
    cx.bind_keys([
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("ctrl-a", Home, ctx),
        KeyBinding::new("ctrl-e", End, ctx),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("ctrl-backspace", DeleteWordBackward, ctx),
        KeyBinding::new("ctrl-u", Clear, ctx),
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("escape", Cancel, ctx),
        KeyBinding::new("up", Up, ctx),
        KeyBinding::new("down", Down, ctx),
    ]);
}

pub enum InputEvent {
    Changed,
    Submit,
    Cancel,
    Up,
    Down,
}

#[derive(Default, Clone)]
struct Layout {
    bounds: Bounds<Pixels>,
    line: Option<ShapedLine>,
    scroll_x: Pixels,
    line_height: Pixels,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    text: String,
    /// Byte offset of the cursor.
    cursor: usize,
    marked: Option<Range<usize>>,
    placeholder: SharedString,
    layout: Layout,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> TextInput {
        TextInput {
            focus_handle: cx.focus_handle(),
            text: String::new(),
            cursor: 0,
            marked: None,
            placeholder: placeholder.into(),
            layout: Layout::default(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>) {
        self.placeholder = placeholder.into();
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.marked = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn replace(&mut self, range: Range<usize>, insert: &str, cx: &mut Context<Self>) {
        self.text.replace_range(range.clone(), insert);
        self.cursor = range.start + insert.len();
        self.marked = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn prev_boundary(&self, at: usize) -> usize {
        let mut i = at.saturating_sub(1);
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn next_boundary(&self, at: usize) -> usize {
        let mut i = (at + 1).min(self.text.len());
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.cursor = self.prev_boundary(self.cursor);
        cx.notify();
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.cursor = self.next_boundary(self.cursor);
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.cursor = 0;
        cx.notify();
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.cursor = self.text.len();
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.cursor > 0 {
            let start = self.prev_boundary(self.cursor);
            self.replace(start..self.cursor, "", cx);
        }
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.cursor < self.text.len() {
            let end = self.next_boundary(self.cursor);
            self.replace(self.cursor..end, "", cx);
        }
    }

    fn delete_word_backward(
        &mut self,
        _: &DeleteWordBackward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = &self.text[..self.cursor];
        let trimmed = before.trim_end();
        let start = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        if start < self.cursor {
            self.replace(start..self.cursor, "", cx);
        }
    }

    fn clear(&mut self, _: &Clear, _: &mut Window, cx: &mut Context<Self>) {
        self.replace(0..self.text.len(), "", cx);
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Submit);
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Cancel);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Up);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Down);
    }

    fn click(&mut self, x: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if let Some(line) = &self.layout.line {
            let rel = x - self.layout.bounds.origin.x + self.layout.scroll_x;
            let ix = line.closest_index_for_x(rel).min(self.text.len());
            let mut ix = ix;
            while !self.text.is_char_boundary(ix) {
                ix -= 1;
            }
            self.cursor = ix;
        }
        cx.notify();
    }

    fn utf16_to_byte(text: &str, utf16: usize) -> usize {
        let mut units = 0;
        for (i, c) in text.char_indices() {
            if units >= utf16 {
                return i;
            }
            units += c.len_utf16();
        }
        text.len()
    }

    fn byte_to_utf16(text: &str, byte: usize) -> usize {
        text[..byte.min(text.len())]
            .chars()
            .map(char::len_utf16)
            .sum()
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = Self::utf16_to_byte(&self.text, range_utf16.start);
        let end = Self::utf16_to_byte(&self.text, range_utf16.end);
        adjusted_range
            .replace(Self::byte_to_utf16(&self.text, start)..Self::byte_to_utf16(&self.text, end));
        Some(self.text[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let c = Self::byte_to_utf16(&self.text, self.cursor);
        Some(UTF16Selection {
            range: c..c,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked.as_ref().map(|r| {
            Self::byte_to_utf16(&self.text, r.start)..Self::byte_to_utf16(&self.text, r.end)
        })
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| {
                Self::utf16_to_byte(&self.text, r.start)..Self::utf16_to_byte(&self.text, r.end)
            })
            .or_else(|| self.marked.clone())
            .unwrap_or(self.cursor..self.cursor);
        self.replace(range, new_text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| {
                Self::utf16_to_byte(&self.text, r.start)..Self::utf16_to_byte(&self.text, r.end)
            })
            .or_else(|| self.marked.clone())
            .unwrap_or(self.cursor..self.cursor);
        self.replace(range.clone(), new_text, cx);
        let marked = range.start..range.start + new_text.len();
        if let Some(sel) = new_selected_range {
            self.cursor =
                (marked.start + Self::utf16_to_byte(new_text, sel.end)).min(self.text.len());
        }
        self.marked = Some(marked);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.layout.line.as_ref()?;
        let start = Self::utf16_to_byte(&self.text, range_utf16.start);
        let end = Self::utf16_to_byte(&self.text, range_utf16.end);
        let x0 = element_bounds.origin.x - self.layout.scroll_x;
        Some(Bounds::from_corners(
            point(x0 + line.x_for_index(start), element_bounds.origin.y),
            point(
                x0 + line.x_for_index(end),
                element_bounds.origin.y + self.layout.line_height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

struct Prepaint {
    line: ShapedLine,
    cursor_x: Pixels,
    scroll_x: Pixels,
    line_height: Pixels,
    focused: bool,
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let entity = cx.entity();
        let paint_entity = entity.clone();
        div()
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::clear))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .w_full()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .rounded_md()
            .bg(theme.bg_alt)
            .border_1()
            .border_color(if self.focus_handle.is_focused(_window) {
                theme.accent
            } else {
                theme.border
            })
            .text_size(px(13.))
            .text_color(theme.text)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let style = window.text_style();
                        let font_size = style.font_size.to_pixels(window.rem_size());
                        let line_height = window.line_height();
                        let (text, cursor, placeholder, focused, prev_scroll) = {
                            let input = entity.read(cx);
                            (
                                input.text.clone(),
                                input.cursor,
                                input.placeholder.clone(),
                                input.focus_handle.is_focused(window),
                                input.layout.scroll_x,
                            )
                        };
                        let theme = cx.global::<Theme>();
                        let (shown, color) = if text.is_empty() {
                            (placeholder.to_string(), theme.text_faint)
                        } else {
                            (text.clone(), theme.text)
                        };
                        let mut run = style.to_run(shown.len());
                        run.color = color;
                        let line = window.text_system().shape_line(
                            SharedString::from(shown),
                            font_size,
                            &[run],
                            None,
                        );
                        let cursor_x = if text.is_empty() {
                            px(0.)
                        } else {
                            line.x_for_index(cursor)
                        };
                        let width = bounds.size.width;
                        let mut scroll_x = prev_scroll;
                        if cursor_x - scroll_x > width - px(2.) {
                            scroll_x = cursor_x - width + px(2.);
                        }
                        if cursor_x - scroll_x < px(0.) {
                            scroll_x = cursor_x;
                        }
                        if line.width < width {
                            scroll_x = px(0.);
                        }
                        Prepaint {
                            line,
                            cursor_x,
                            scroll_x,
                            line_height,
                            focused,
                        }
                    },
                    move |bounds, prepaint: Prepaint, window, cx| {
                        let focus_handle = paint_entity.read(cx).focus_handle.clone();
                        window.handle_input(
                            &focus_handle,
                            ElementInputHandler::new(bounds, paint_entity.clone()),
                            cx,
                        );
                        let y = bounds.origin.y + (bounds.size.height - prepaint.line_height) / 2.;
                        let origin = point(bounds.origin.x - prepaint.scroll_x, y);
                        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                            let _ = prepaint.line.paint(
                                origin,
                                prepaint.line_height,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            );
                            if prepaint.focused {
                                let x = bounds.origin.x + prepaint.cursor_x - prepaint.scroll_x;
                                let color = cx.global::<Theme>().text;
                                window.paint_quad(fill(
                                    Bounds::new(
                                        point(x, y + px(2.)),
                                        size(px(1.5), prepaint.line_height - px(4.)),
                                    ),
                                    color,
                                ));
                            }
                        });
                        let click_entity = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                                && bounds.contains(&event.position)
                            {
                                click_entity.update(cx, |input, cx| {
                                    input.click(event.position.x, window, cx)
                                });
                            }
                        });
                        paint_entity.update(cx, |input, _| {
                            input.layout = Layout {
                                bounds,
                                line: Some(prepaint.line),
                                scroll_x: prepaint.scroll_x,
                                line_height: prepaint.line_height,
                            };
                        });
                    },
                )
                .size_full(),
            )
    }
}
