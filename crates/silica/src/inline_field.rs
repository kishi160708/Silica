//! A name edited where it is drawn, with no box of its own.
//!
//! Obsidian has two of these and they behave alike: the view header's
//! `.view-header-title`, editable in its resting state, and the file tree's
//! `.nav-file-title-content`, which F2 makes `contenteditable`. A caret sits
//! in running text, typing edits the name while the file keeps its old one,
//! Enter commits and Escape abandons.
//!
//! [`TitleField`] is the text model both share. This is everything above it
//! that GPUI needs — the shaping and painting, the click that lands the
//! caret, the platform input method's half, and the editing keys — so the
//! behaviour has one implementation and a host only says where its field is
//! drawn and what committing means.
//!
//! A field is *not* a [`crate::text_input::TextInput`]: that widget draws a
//! 30 px bordered box and keeps its caret private, so a click could not land
//! the caret where the pointer was.

use crate::text_input;
use crate::title_field::{self, TitleField};
use gpui::{
    fill, point, prelude::*, px, size, App, Bounds, ClipboardItem, Context, Hsla, Pixels,
    ShapedLine, SharedString, TextAlign, UTF16Selection, Window,
};
use std::ops::Range;

/// A field open somewhere: the text being typed into and the composition
/// the platform's input method has in it.
#[derive(Debug, Clone, Default)]
pub struct Editing {
    pub field: TitleField,
    /// The bytes an open composition will replace with its committed text.
    pub marked: Option<Range<usize>>,
}

impl Editing {
    /// A field over `text` with the caret at its end, which is where one
    /// opened by anything but a click starts.
    pub fn new(text: impl Into<String>) -> Editing {
        Editing {
            field: TitleField::new(text),
            marked: None,
        }
    }

    /// A field over `text` with all of it selected, which is the state F2
    /// leaves a tree row in: the whole basename, and never the extension.
    pub fn selected(text: impl Into<String>) -> Editing {
        let mut editing = Editing::new(text);
        editing.field.select_all();
        editing
    }

    /// Where typed or composed text lands: the range the platform named,
    /// else the composition it has open, else the selection — so a
    /// keystroke with text selected replaces it.
    fn input_range(&self, range_utf16: Option<Range<usize>>) -> Range<usize> {
        let text = self.field.text();
        range_utf16
            .map(|r| {
                title_field::utf16_to_byte(text, r.start)..title_field::utf16_to_byte(text, r.end)
            })
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.field.selection())
    }
}

/// Where a field was last painted and how its text was shaped: what a click
/// is measured against to find the offset it landed on, and what the input
/// method is told when it asks where the text is.
#[derive(Clone)]
pub struct Layout {
    pub bounds: Bounds<Pixels>,
    pub line: ShapedLine,
    pub line_height: Pixels,
    pub scroll_x: Pixels,
}

impl Layout {
    /// The byte offset a press at `position` landed on, or `None` for one
    /// past the end of the text — which is a press on whatever holds the
    /// field rather than on the field, and changes nothing.
    pub fn offset_for(&self, position: gpui::Point<Pixels>, text: &str) -> Option<usize> {
        let x = position.x - self.bounds.origin.x + self.scroll_x;
        if x > self.line.width + px(4.) {
            return None;
        }
        let mut at = self.line.closest_index_for_x(x).min(text.len());
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        Some(at)
    }
}

/// What a field works out before it paints.
pub struct Prepaint {
    line: ShapedLine,
    cursor_x: Pixels,
    /// The selection's two edges in the shaped line, when there is one.
    selection_x: Option<Range<Pixels>>,
    scroll_x: Pixels,
    line_height: Pixels,
    /// The field has the keyboard, so it carries a caret.
    caret: bool,
}

/// Shapes the line a field will paint, and works out how far it has to
/// scroll to keep the caret in view. `editing` is the open field, if the
/// host has one here; with none the text is painted as plain text.
///
/// Call this from a [`gpui::canvas`]'s prepaint, where the text style is
/// the one the field is drawn in.
pub fn shape(
    text: SharedString,
    editing: Option<&Editing>,
    color: Hsla,
    focused: bool,
    bounds: Bounds<Pixels>,
    previous_scroll_x: Pixels,
    window: &mut Window,
) -> Prepaint {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = window.line_height();
    let cursor = editing.map(|e| e.field.cursor()).unwrap_or(0);
    let selection = editing
        .map(|e| e.field.selection())
        .filter(|s| !s.is_empty());
    let mut run = style.to_run(text.len());
    run.color = color;
    let line = window
        .text_system()
        .shape_line(text.clone(), font_size, &[run], None);
    let cursor_x = if text.is_empty() {
        px(0.)
    } else {
        line.x_for_index(cursor)
    };
    let selection_x = selection.map(|s| line.x_for_index(s.start)..line.x_for_index(s.end));
    // * A name too long for the space it is drawn in scrolls to keep the
    //   caret in view, the way the crate's other field does.
    let width = bounds.size.width;
    let mut scroll_x = previous_scroll_x;
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
        selection_x,
        scroll_x,
        line_height,
        caret: editing.is_some() && focused,
    }
}

/// Paints the shaped line, its selection and its caret, and returns where
/// it landed so the next click can be measured against it.
pub fn paint(
    prepaint: Prepaint,
    bounds: Bounds<Pixels>,
    selection_color: Hsla,
    caret_color: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Layout {
    let y = bounds.origin.y + (bounds.size.height - prepaint.line_height) / 2.;
    let origin = point(bounds.origin.x - prepaint.scroll_x, y);
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        if let Some(selection) = &prepaint.selection_x {
            window.paint_quad(fill(
                Bounds::from_corners(
                    point(origin.x + selection.start, y),
                    point(origin.x + selection.end, y + prepaint.line_height),
                ),
                selection_color,
            ));
        }
        let _ = prepaint.line.paint(
            origin,
            prepaint.line_height,
            TextAlign::Left,
            None,
            window,
            cx,
        );
        if prepaint.caret {
            window.paint_quad(fill(
                Bounds::new(
                    point(origin.x + prepaint.cursor_x, y + px(2.)),
                    size(px(1.5), prepaint.line_height - px(4.)),
                ),
                caret_color,
            ));
        }
    });
    Layout {
        bounds,
        line: prepaint.line,
        line_height: prepaint.line_height,
        scroll_x: prepaint.scroll_x,
    }
}

// ----- the platform input method's half -------------------------------------
//
// Text comes in through GPUI's input handler so IME composition and dead
// keys work, as they do in the crate's other field. Each host registers one
// against its own focus handle and hands these the field it has open; with
// no field open every one of them answers as an empty field would.

pub fn text_for_range(
    editing: &Editing,
    range_utf16: Range<usize>,
    adjusted_range: &mut Option<Range<usize>>,
) -> Option<String> {
    let text = editing.field.text();
    let start = title_field::utf16_to_byte(text, range_utf16.start);
    let end = title_field::utf16_to_byte(text, range_utf16.end);
    adjusted_range
        .replace(title_field::byte_to_utf16(text, start)..title_field::byte_to_utf16(text, end));
    Some(text[start..end].to_string())
}

pub fn selected_text_range(editing: &Editing) -> UTF16Selection {
    let text = editing.field.text();
    let selection = editing.field.selection();
    UTF16Selection {
        range: title_field::byte_to_utf16(text, selection.start)
            ..title_field::byte_to_utf16(text, selection.end),
        reversed: editing.field.reversed(),
    }
}

pub fn marked_text_range(editing: &Editing) -> Option<Range<usize>> {
    let marked = editing.marked.clone()?;
    let text = editing.field.text();
    Some(
        title_field::byte_to_utf16(text, marked.start)
            ..title_field::byte_to_utf16(text, marked.end),
    )
}

pub fn replace_text_in_range(
    editing: &mut Editing,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
) {
    let range = editing.input_range(range_utf16);
    editing.field.replace(range, new_text);
    editing.marked = None;
}

pub fn replace_and_mark_text_in_range(
    editing: &mut Editing,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    new_selected_range: Option<Range<usize>>,
) {
    let range = editing.input_range(range_utf16);
    let landed = editing.field.replace(range.clone(), new_text);
    let marked = range.start..range.start + landed;
    if let Some(selected) = new_selected_range {
        let at = marked.start + title_field::utf16_to_byte(new_text, selected.end);
        editing.field.move_to(at, false);
    }
    editing.marked = Some(marked);
}

/// Where the composition sits on screen, which is what the input method
/// puts its candidate window beside.
pub fn bounds_for_range(
    editing: &Editing,
    layout: &Layout,
    range_utf16: Range<usize>,
    element_bounds: Bounds<Pixels>,
) -> Bounds<Pixels> {
    let text = editing.field.text();
    let start = title_field::utf16_to_byte(text, range_utf16.start);
    let end = title_field::utf16_to_byte(text, range_utf16.end);
    let x0 = element_bounds.origin.x - layout.scroll_x;
    Bounds::from_corners(
        point(x0 + layout.line.x_for_index(start), element_bounds.origin.y),
        point(
            x0 + layout.line.x_for_index(end),
            element_bounds.origin.y + layout.line_height,
        ),
    )
}

// ----- the editing keys ------------------------------------------------------

/// A view with one inline field open at a time.
pub trait Host: Sized + 'static {
    /// The open field, if there is one.
    fn editing(&mut self) -> Option<&mut Editing>;
    /// Enter: the name is taken.
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>);
    /// Escape: the name is left as it was.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>);
}

/// Binds the crate's field keys to the host's open field. They are the
/// same actions [`crate::text_input`] declares under the same key context,
/// so a field edited in place and one edited in a box take the same keys.
///
/// Attach this to the element that tracks the field's focus handle, and
/// give that element `key_context("TextInput")`.
pub fn bind_keys<T: Host, E: InteractiveElement>(el: E, cx: &mut Context<T>) -> E {
    el.on_action(
        cx.listener(|this, _: &text_input::Left, _, cx| apply(this, cx, |f| f.step(false, false))),
    )
    .on_action(
        cx.listener(|this, _: &text_input::Right, _, cx| apply(this, cx, |f| f.step(true, false))),
    )
    .on_action(cx.listener(|this, _: &text_input::SelectLeft, _, cx| {
        apply(this, cx, |f| f.step(false, true))
    }))
    .on_action(cx.listener(|this, _: &text_input::SelectRight, _, cx| {
        apply(this, cx, |f| f.step(true, true))
    }))
    .on_action(cx.listener(|this, _: &text_input::WordLeft, _, cx| {
        apply(this, cx, |f| f.step_word(false, false))
    }))
    .on_action(cx.listener(|this, _: &text_input::WordRight, _, cx| {
        apply(this, cx, |f| f.step_word(true, false))
    }))
    .on_action(cx.listener(|this, _: &text_input::SelectWordLeft, _, cx| {
        apply(this, cx, |f| f.step_word(false, true))
    }))
    .on_action(cx.listener(|this, _: &text_input::SelectWordRight, _, cx| {
        apply(this, cx, |f| f.step_word(true, true))
    }))
    .on_action(cx.listener(|this, _: &text_input::Home, _, cx| {
        apply(this, cx, |f| f.move_to_edge(false, false))
    }))
    .on_action(cx.listener(|this, _: &text_input::End, _, cx| {
        apply(this, cx, |f| f.move_to_edge(true, false))
    }))
    .on_action(cx.listener(|this, _: &text_input::SelectHome, _, cx| {
        apply(this, cx, |f| f.move_to_edge(false, true))
    }))
    .on_action(cx.listener(|this, _: &text_input::SelectEnd, _, cx| {
        apply(this, cx, |f| f.move_to_edge(true, true))
    }))
    .on_action(
        cx.listener(|this, _: &text_input::SelectAll, _, cx| apply(this, cx, |f| f.select_all())),
    )
    .on_action(
        cx.listener(|this, _: &text_input::Backspace, _, cx| apply(this, cx, |f| f.backspace())),
    )
    .on_action(cx.listener(|this, _: &text_input::Delete, _, cx| apply(this, cx, |f| f.delete())))
    .on_action(
        cx.listener(|this, _: &text_input::DeleteWordBackward, _, cx| {
            apply(this, cx, |f| f.delete_word_back())
        }),
    )
    .on_action(cx.listener(|this, _: &text_input::Clear, _, cx| {
        apply(this, cx, |f| {
            f.select_all();
            f.insert("");
        })
    }))
    .on_action(cx.listener(|this, _: &text_input::Copy, _, cx| copy(this, false, cx)))
    .on_action(cx.listener(|this, _: &text_input::Cut, _, cx| copy(this, true, cx)))
    .on_action(cx.listener(|this, _: &text_input::Paste, _, cx| paste(this, cx)))
    .on_action(cx.listener(|this, _: &text_input::Submit, window, cx| this.commit(window, cx)))
    .on_action(cx.listener(|this, _: &text_input::Cancel, window, cx| this.cancel(window, cx)))
    // Ctrl+E in a field is swallowed rather than toggling the mode behind
    // it, as it is in every other field here.
    .on_action(cx.listener(|_, _: &text_input::Swallow, _, _| {}))
}

/// Applies an editing key to the open field. Any edit drops the
/// composition: what the input method was holding is no longer what is
/// there.
pub fn apply<T: Host>(this: &mut T, cx: &mut Context<T>, f: impl FnOnce(&mut TitleField)) {
    if let Some(editing) = this.editing() {
        f(&mut editing.field);
        editing.marked = None;
        cx.notify();
    }
}

fn copy<T: Host>(this: &mut T, cut: bool, cx: &mut Context<T>) {
    let Some(editing) = this.editing() else {
        return;
    };
    let range = editing.field.selection();
    if range.is_empty() {
        return;
    }
    let text = editing.field.text()[range.clone()].to_string();
    cx.write_to_clipboard(ClipboardItem::new_string(text));
    if cut {
        if let Some(editing) = this.editing() {
            editing.field.replace(range, "");
            editing.marked = None;
            cx.notify();
        }
    }
}

fn paste<T: Host>(this: &mut T, cx: &mut Context<T>) {
    let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
        return;
    };
    apply(this, cx, |f| f.insert(&text));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F2 on a tree row selects the whole name, so the first keystroke
    /// replaces it: the basename, never the extension.
    #[test]
    fn a_field_opened_on_a_row_has_the_whole_name_selected() {
        let editing = Editing::selected("Beta");
        assert_eq!(editing.field.selection(), 0..4);
        assert!(editing.field.has_selection());
        // A field opened by anything but a click starts at the end with
        // nothing selected, which is the view header's resting state.
        assert_eq!(Editing::new("Beta").field.selection(), 4..4);
    }

    /// Typed text lands on the selection when the platform names no range,
    /// which is how the first keystroke after F2 replaces the name.
    #[test]
    fn typed_text_replaces_what_is_selected() {
        let mut editing = Editing::selected("Beta");
        replace_text_in_range(&mut editing, None, "Gamma");
        assert_eq!(editing.field.text(), "Gamma");
        assert_eq!(editing.marked, None);
    }

    /// An input method's composition is replaced by its next one rather
    /// than appended to, and committing it leaves no composition behind.
    #[test]
    fn a_composition_replaces_the_one_before_it() {
        let mut editing = Editing::new("");
        replace_and_mark_text_in_range(&mut editing, None, "n", None);
        assert_eq!(editing.field.text(), "n");
        assert_eq!(editing.marked, Some(0..1));
        replace_and_mark_text_in_range(&mut editing, None, "ni", None);
        assert_eq!(editing.field.text(), "ni");
        assert_eq!(editing.marked, Some(0..2));
        replace_text_in_range(&mut editing, None, "に");
        assert_eq!(editing.field.text(), "に");
        assert_eq!(editing.marked, None);
    }
}
