//! The canvas view's paint code: drawing cards, edges and the selection
//! chrome — the methods of the split-out render impl, unchanged apart from
//! living in their own module. Split from `canvas/view.rs` per the audit's
//! module-size finding.

use super::*;

impl Render for CanvasView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_editor(window, cx);
        let theme = cx.global::<Theme>().clone();
        let entity = cx.entity();
        let paint_entity = entity.clone();
        let error = self.error.clone();
        let floating = self.render_floating_menu(&theme, cx);
        let context = self.render_context_menu(&theme, cx);
        let card_menu = (error.is_none()).then(|| self.render_card_menu(&theme, cx));
        div()
            .key_context("CanvasView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::delete_selection))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::duplicate))
            .on_action(cx.listener(Self::group_selection))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::clear_selection))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::reset_zoom))
            .on_action(cx.listener(Self::zoom_to_fit))
            .on_action(cx.listener(Self::zoom_to_selection))
            .on_action(cx.listener(Self::nudge_left))
            .on_action(cx.listener(Self::nudge_right))
            .on_action(cx.listener(Self::nudge_up))
            .on_action(cx.listener(Self::nudge_down))
            .on_action(cx.listener(Self::nudge_left_far))
            .on_action(cx.listener(Self::nudge_right_far))
            .on_action(cx.listener(Self::nudge_up_far))
            .on_action(cx.listener(Self::nudge_down_far))
            .on_action(cx.listener(Self::edit_selected))
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(theme.bg)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |view, cx| view.prepaint(bounds, window, cx))
                    },
                    move |bounds, prepared, window, cx| {
                        let hitbox = prepared.hitbox.clone();
                        paint_entity
                            .update(cx, |view, cx| view.paint(bounds, prepared, window, cx));
                        let line_height = window.line_height();
                        // * An event sent from inside a frame's paint is
                        //   dropped; the fit made in prepaint is reported from
                        //   the next frame instead.
                        if paint_entity.read(cx).announce_viewport {
                            let e = paint_entity.clone();
                            window.on_next_frame(move |_, cx| {
                                e.update(cx, |v, cx| {
                                    v.announce_viewport = false;
                                    v.viewport_changed(cx);
                                });
                            });
                        }

                        let e = paint_entity.clone();
                        let hb = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble && hb.is_hovered(window) {
                                e.update(cx, |v, cx| v.mouse_down(event, window, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        let hb = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            let over = hb.is_hovered(window);
                            e.update(cx, |v, cx| {
                                if over || v.drag.is_some() {
                                    v.mouse_move(event, cx);
                                } else if v.hovered.take().is_some() {
                                    cx.notify();
                                }
                            });
                        });
                        let e = paint_entity.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble {
                                e.update(cx, |v, cx| v.mouse_up(event, cx));
                            }
                        });
                        let e = paint_entity.clone();
                        let hb = hitbox.clone();
                        window.on_mouse_event(
                            move |event: &ScrollWheelEvent, phase, window, cx| {
                                if phase == DispatchPhase::Bubble && hb.should_handle_scroll(window)
                                {
                                    e.update(cx, |v, cx| v.scroll(event, line_height, cx));
                                }
                            },
                        );
                        let e = paint_entity.clone();
                        let hb = hitbox.clone();
                        window.on_mouse_event(move |event: &PinchEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble && hb.is_hovered(window) {
                                e.update(cx, |v, cx| v.pinch(event, cx));
                            }
                        });
                        // * Space held turns a left drag into a pan; the key
                        //   events reach here while the view has the focus.
                        let e = paint_entity.clone();
                        window.on_key_event(move |event: &KeyDownEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble && event.keystroke.key == "space" {
                                e.update(cx, |v, cx| {
                                    if !v.space_held {
                                        v.space_held = true;
                                        cx.notify();
                                    }
                                });
                            }
                        });
                        let e = paint_entity.clone();
                        window.on_key_event(move |event: &KeyUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble && event.keystroke.key == "space" {
                                e.update(cx, |v, cx| {
                                    v.space_held = false;
                                    cx.notify();
                                });
                            }
                        });
                    },
                )
                .size_full(),
            )
            .children(error.map(|message| {
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child(format!("This canvas could not be read: {message}"))
            }))
            .children(floating)
            .children(card_menu)
            .child(self.render_controls(&theme, cx))
            .children(context)
    }
}

// ---- helpers ------------------------------------------------------------------

/// The tooltips of the control stack, as Obsidian words them.
pub(crate) fn control_label(id: &str) -> &'static str {
    match id {
        "settings" => "Canvas settings",
        "zoom-in" => "Zoom in",
        "reset" => "Reset zoom",
        "fit" => "Zoom to fit (Shift + 1)",
        "zoom-out" => "Zoom out",
        "undo" => "Undo",
        "redo" => "Redo",
        _ => "Canvas help",
    }
}

/// The file's canvas: an empty file is an empty canvas (what `canvas:new-file`
/// writes until something is in it).
pub(crate) fn parse(text: &str) -> Result<Canvas, String> {
    if text.trim().is_empty() {
        return Ok(Canvas::default());
    }
    Canvas::from_json(text).map_err(|e| e.to_string())
}

/// A number the way JavaScript prints it: no decimal point when it is whole.
/// The text with the task marker whose `[` is byte `col` of line `line`
/// flipped the way the editor flips one; `None` when those bytes are not a
/// marker (a card built before its text changed can name a stale place).
pub(crate) fn toggle_task_in(text: &str, line: usize, col: usize) -> Option<String> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let target = *lines.get(line)?;
    let checked = match target.as_bytes().get(col..col + 3) {
        Some([b'[', status, b']']) => *status != b' ',
        _ => return None,
    };
    let with = if checked { "[ ]" } else { "[x]" };
    let replaced = format!("{}{}{}", &target[..col], with, &target[col + 3..]);
    lines[line] = &replaced;
    Some(lines.join("\n"))
}

/// The side across from `side`: where an edge dropped on empty space
/// meets the card made there.
pub(crate) fn opposite(side: Side) -> Side {
    match side {
        Side::Top => Side::Bottom,
        Side::Bottom => Side::Top,
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

pub(crate) fn json_number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

/// Snaps the sides a handle moves to the grid, leaving the still sides
/// where `origin` has them. Candidate for `silica-canvas`.
pub(crate) fn snap_rect_sides(handle: Handle, origin: &Rect, target: &Rect) -> Rect {
    let moves_left = matches!(handle, Handle::Left | Handle::TopLeft | Handle::BottomLeft);
    let moves_right = matches!(
        handle,
        Handle::Right | Handle::TopRight | Handle::BottomRight
    );
    let moves_top = matches!(handle, Handle::Top | Handle::TopLeft | Handle::TopRight);
    let moves_bottom = matches!(
        handle,
        Handle::Bottom | Handle::BottomLeft | Handle::BottomRight
    );
    let (mut x, mut right) = (target.x, target.right());
    let (mut y, mut bottom) = (target.y, target.bottom());
    if moves_left {
        x = snap(x).min(origin.right() - MIN_NODE.0);
    }
    if moves_right {
        right = snap(right).max(origin.x + MIN_NODE.0);
    }
    if moves_top {
        y = snap(y).min(origin.bottom() - MIN_NODE.1);
    }
    if moves_bottom {
        bottom = snap(bottom).max(origin.y + MIN_NODE.1);
    }
    Rect::new(x, y, right - x, bottom - y)
}

/// The delta `Handle::resize` needs to take `now` to `target`: the moving
/// side's displacement on each axis. Candidate for `silica-canvas`.
pub(crate) fn resize_delta(handle: Handle, now: &Rect, target: &Rect) -> (f64, f64) {
    let dx = match handle {
        Handle::Left | Handle::TopLeft | Handle::BottomLeft => target.x - now.x,
        Handle::Right | Handle::TopRight | Handle::BottomRight => target.right() - now.right(),
        Handle::Top | Handle::Bottom => 0.0,
    };
    let dy = match handle {
        Handle::Top | Handle::TopLeft | Handle::TopRight => target.y - now.y,
        Handle::Bottom | Handle::BottomLeft | Handle::BottomRight => target.bottom() - now.bottom(),
        Handle::Left | Handle::Right => 0.0,
    };
    (dx, dy)
}

/// Shapes one line of label text in the window's font, cut to `max_width`
/// with an ellipsis when it would not fit.
pub(crate) fn shape_label(
    text: &str,
    font_size: Pixels,
    color: Hsla,
    max_width: Option<Pixels>,
    window: &mut Window,
) -> gpui::ShapedLine {
    let style = window.text_style();
    let shape = |text: SharedString, window: &mut Window| {
        let mut run = style.to_run(text.len());
        run.color = color;
        window
            .text_system()
            .shape_line(text, font_size, &[run], None)
    };
    let text: String = text.lines().next().unwrap_or("").to_string();
    let line = shape(text.clone().into(), window);
    let Some(max) = max_width else {
        return line;
    };
    if line.width <= max || text.is_empty() {
        return line;
    }
    // * Proportional cut: the labels are short and the fonts proportional
    //   enough that one pass lands within a character or two.
    let share = f32::from(max) / f32::from(line.width).max(1.0);
    let chars = text.chars().count();
    let keep = ((chars as f32 * share) as usize).saturating_sub(1);
    let cut: String = text.chars().take(keep).collect();
    shape(format!("{cut}…").into(), window)
}

/// Paints one line of label text at `origin` (its top-left).
pub(crate) fn paint_label(
    text: &str,
    font_size: Pixels,
    color: Hsla,
    origin: gpui::Point<Pixels>,
    max_width: Option<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let line = shape_label(text, font_size, color, max_width, window);
    let line_height = font_size * LABEL_LINE_HEIGHT as f32;
    let _ = line.paint(origin, line_height, TextAlign::Left, None, window, cx);
}
