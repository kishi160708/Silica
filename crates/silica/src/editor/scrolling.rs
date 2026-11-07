//! Scroll positioning: keeping the cursor on screen after edits and
//! jumps, page scrolling, and the scrollbar's geometry — the methods of the
//! split-out scrolling section, unchanged apart from living in their own
//! module. Split from `editor.rs` per the audit's module-size finding.

use gpui::{px, Context, Pixels, Point, Window};

use super::Editor;

impl Editor {

    /// Brings `scroll_px` back inside `scroll_line`'s rows.
    pub(crate) fn normalize_scroll(&mut self, window: &mut Window) {
        let lh = self.layout.line_height;
        if lh <= px(0.) {
            return;
        }
        while self.scroll_px < px(0.) {
            if self.scroll_line == 0 {
                self.scroll_px = px(0.);
                break;
            }
            self.scroll_line -= 1;
            self.scroll_px += self.height_of(window, self.scroll_line);
        }
        loop {
            let height = self.height_of(window, self.scroll_line);
            if self.scroll_px >= height && self.scroll_line + 1 < self.line_count() {
                self.scroll_px -= height;
                self.scroll_line += 1;
            } else {
                if self.scroll_px >= height {
                    self.scroll_px = (height - lh).max(px(0.));
                }
                break;
            }
        }
    }

    /// Keeps the last line from scrolling above the bottom of the view.
    pub(crate) fn clamp_scroll_end(&mut self, window: &mut Window) {
        let lh = self.layout.line_height;
        let h = self.layout.bounds.size.height;
        if lh <= px(0.) || h <= px(0.) {
            return;
        }
        let mut filled = -self.scroll_px;
        let mut line = self.scroll_line;
        while line < self.line_count() && filled < h {
            filled += self.height_of(window, line);
            line += 1;
        }
        if filled < h {
            self.scroll_px -= h - filled;
            self.normalize_scroll(window);
        }
    }

    /// Scrolls the view down by `dy` (up when negative).
    pub(crate) fn scroll_by(&mut self, dy: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        if self.hover_preview.take().is_some() {
            cx.notify();
        }
        self.scroll_px += dy;
        self.normalize_scroll(window);
        if dy > px(0.) {
            self.clamp_scroll_end(window);
        }
        cx.notify();
    }

    /// Puts `line` at the top of the view (clamped so the note's end stays
    /// at the bottom), from the scrollbar and from the control socket.
    pub fn scroll_to_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_line = line.min(self.line_count() - 1);
        self.scroll_px = px(0.);
        self.clamp_scroll_end(window);
        cx.notify();
    }

    /// Mouse down on the scrollbar: grabs the thumb, or jumps so the thumb
    /// centers on the click. Returns false when the point is off the track.
    pub(crate) fn scrollbar_mouse_down(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(bar) = self.layout.scrollbar else {
            return false;
        };
        if !bar.track.contains(&position) {
            return false;
        }
        if bar.thumb.contains(&position) {
            self.thumb_drag = Some(position.y - bar.thumb.origin.y);
        } else {
            let half = bar.thumb.size.height / 2.;
            self.thumb_drag = Some(half);
            let line = bar.line_for_thumb_top(position.y - half);
            self.scroll_to_line(line, window, cx);
        }
        true
    }

    pub(crate) fn scrollbar_drag(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Some(grab), Some(bar)) = (self.thumb_drag, self.layout.scrollbar) else {
            return;
        };
        let line = bar.line_for_thumb_top(position.y - grab);
        if line != self.scroll_line || self.scroll_px != px(0.) {
            self.scroll_to_line(line, window, cx);
        }
    }

    /// Scrolls the least amount that puts the cursor's row on screen.
    pub(crate) fn reveal_cursor(&mut self, window: &mut Window) {
        let lh = self.layout.line_height;
        let h = self.layout.bounds.size.height;
        if lh <= px(0.) || h <= px(0.) {
            self.pending_reveal = true;
            return;
        }
        let c = self.buf.cursor_at(self.buf.head);
        let Some(shaped) = self.shape(window, c.line) else {
            self.pending_reveal = true;
            return;
        };
        let (row, _) = shaped.row_and_x(c.col);
        let rh = shaped.row_height;
        let row_px = rh * row as f32;
        let center = std::mem::take(&mut self.pending_center);
        if center {
            self.scroll_row_to(c.line, row_px, (h - rh) / 2., window);
            return;
        }
        if c.line < self.scroll_line || (c.line == self.scroll_line && row_px < self.scroll_px) {
            self.scroll_line = c.line;
            self.scroll_px = row_px;
            return;
        }
        // * Walk down from the anchor; stop once past the viewport.
        let mut y = -self.scroll_px;
        let mut line = self.scroll_line;
        let mut cursor_y = None;
        while line <= c.line && y <= h {
            if line == c.line {
                cursor_y = Some(y + row_px);
                break;
            }
            y += self.height_of(window, line);
            line += 1;
        }
        if let Some(cy) = cursor_y {
            if cy + rh <= h {
                return;
            }
        }
        // * Put the cursor's row at the bottom of the view.
        self.scroll_row_to(c.line, row_px, h - rh, window);
    }

    /// Scrolls so the row `row_px` into `line` sits `need` pixels under the
    /// top of the view, walking back over the lines above it.
    pub(crate) fn scroll_row_to(
        &mut self,
        line: usize,
        row_px: Pixels,
        mut need: Pixels,
        window: &mut Window,
    ) {
        let mut line = line;
        let mut px_in_line = row_px;
        loop {
            if px_in_line >= need {
                self.scroll_line = line;
                self.scroll_px = px_in_line - need;
                break;
            }
            need -= px_in_line;
            if line == 0 {
                self.scroll_line = 0;
                self.scroll_px = px(0.);
                break;
            }
            line -= 1;
            px_in_line = self.height_of(window, line);
        }
    }
}
