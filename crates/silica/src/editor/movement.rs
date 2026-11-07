//! Cursor and selection movement: word/paragraph/line steps, selection
//! extension, multi-cursor placement and scrolling to the cursor — the
//! methods of the split-out movement section, unchanged apart from living
//! in their own module. Split from `editor.rs` per the audit's
//! module-size finding.

use gpui::{px, Context, Window};
use silica_text::{Cursors, Sel};

use super::{
    Editor, AddCursorAbove, AddCursorBelow, DocumentEnd, DocumentStart, LineEnd, LineStart,
    MoveDown, MoveLeft, MoveRight, MoveUp, MoveWordLeft, MoveWordRight, PageDown, PageUp,
    SelectAll, SelectDocumentEnd, SelectDocumentStart, SelectDown, SelectLeft, SelectLineEnd,
    SelectLineStart, SelectPageDown, SelectPageUp, SelectRight, SelectUp, SelectWordLeft,
    SelectWordRight,
};

impl Editor {

    /// Moves the primary cursor to `offset`, extending its selection when
    /// `select`; other cursors are dropped.
    pub(crate) fn move_to(
        &mut self,
        offset: usize,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.buf.collapse_cursors();
        self.buf.head = self.buf.clamp_offset(offset);
        if !select {
            self.buf.anchor = self.buf.head;
        }
        self.buf.marked = None;
        self.buf.forget_planted();
        // * A cursor move refreshes a popup that is open, and closes it once
        //   the cursor has left the query.
        self.update_suggest(false, cx);
        self.reveal_cursor(window);
        cx.notify();
    }

    /// Moves every cursor to where `target` sends it (its new head and
    /// goal x), extending the selections when `select`.
    pub(crate) fn move_each(
        &mut self,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
        mut target: impl FnMut(&Self, &mut Window, Sel) -> (usize, Option<f32>),
    ) {
        let cursors = self.buf.cursors();
        let mut sels = Vec::with_capacity(cursors.sels.len());
        for s in &cursors.sels {
            let (head, goal) = target(self, window, *s);
            let head = self.buf.clamp_offset(head);
            sels.push(Sel {
                anchor: if select { s.anchor } else { head },
                head,
                goal,
            });
        }
        self.buf.set_cursors(Cursors {
            sels,
            primary: cursors.primary,
        });
        self.buf.marked = None;
        // * A cursor move refreshes a popup that is open, and closes it once
        //   the cursor has left the query.
        self.update_suggest(false, cx);
        self.reveal_cursor(window);
        cx.notify();
    }

    pub(crate) fn horizontal(
        &mut self,
        left: bool,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_each(select, window, cx, |this, _, s| {
            let sel = s.range();
            let target = if !select && !s.is_empty() {
                // * A plain arrow collapses the selection to its edge.
                if left {
                    sel.start
                } else {
                    sel.end
                }
            } else if left {
                this.buf.prev_char_boundary(s.head)
            } else {
                this.buf.next_char_boundary(s.head)
            };
            (target, None)
        });
    }

    /// Moves `delta` wrapped rows up (negative) or down, keeping the goal x.
    pub(crate) fn vertical(
        &mut self,
        delta: isize,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_each(select, window, cx, |this, window, s| {
            this.vertical_target(window, s.head, s.goal, delta)
        });
    }

    /// Where `delta` rows up or down from `head` lands, and the goal x to
    /// keep for the next vertical move.
    pub(crate) fn vertical_target(
        &self,
        window: &mut Window,
        head: usize,
        goal_x: Option<f32>,
        delta: isize,
    ) -> (usize, Option<f32>) {
        let lh = self.layout.line_height;
        let c = self.buf.cursor_at(head);
        let Some(shaped) = self.shape(window, c.line).filter(|_| lh > px(0.)) else {
            // * No layout yet: move by buffer lines.
            let line = (c.line as isize + delta).clamp(0, self.line_count() as isize - 1) as usize;
            let col = self.buf.clamp_col(line, c.col);
            return (self.buf.line_starts()[line] + col, None);
        };
        let (start_row, x) = shaped.row_and_x(c.col);
        let goal = goal_x.map(px).unwrap_or(x);
        let mut line = c.line;
        let mut row = start_row;
        let mut rows = shaped.rows();
        let mut remaining = delta;
        // * The lines a fold hides are not on screen, so a vertical step
        //   passes over them.
        let above = |l: usize| (0..l).rev().find(|&c| !self.line_hidden(c));
        let below = |l: usize| (l + 1..self.line_count()).find(|&c| !self.line_hidden(c));
        while remaining != 0 {
            if remaining < 0 {
                if row > 0 {
                    row -= 1;
                } else if let Some(prev) = above(line) {
                    line = prev;
                    rows = self.rows_of(window, line);
                    row = rows - 1;
                } else {
                    break;
                }
                remaining += 1;
            } else {
                if row + 1 < rows {
                    row += 1;
                } else if let Some(next) = below(line) {
                    line = next;
                    rows = self.rows_of(window, line);
                    row = 0;
                } else {
                    break;
                }
                remaining -= 1;
            }
        }
        let target = if line == c.line && row == start_row {
            // * Nowhere to go: the first row goes to the start, the last to the end.
            if delta < 0 {
                self.buf.line_starts()[line]
            } else {
                self.buf.line_range(line).end
            }
        } else {
            let shaped = if line == c.line {
                shaped
            } else {
                match self.shape(window, line) {
                    Some(s) => s,
                    None => return (head, goal_x),
                }
            };
            let col = self.buf.clamp_col(line, shaped.index_at(goal, row));
            self.buf.line_starts()[line] + col
        };
        (target, Some(f32::from(goal)))
    }

    pub(crate) fn row_boundary(
        &mut self,
        end: bool,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_each(select, window, cx, |this, window, s| {
            let c = this.buf.cursor_at(s.head);
            let target = match this.shape(window, c.line) {
                Some(shaped) if this.layout.line_height > px(0.) => {
                    let (row, _) = shaped.row_and_x(c.col);
                    let x = if end { px(f32::MAX / 4.) } else { px(0.) };
                    let col = this.buf.clamp_col(c.line, shaped.index_at(x, row));
                    this.buf.line_starts()[c.line] + col
                }
                _ => {
                    if end {
                        this.buf.line_range(c.line).end
                    } else {
                        this.buf.line_starts()[c.line]
                    }
                }
            };
            (target, None)
        });
    }

    /// `editor:add-cursor-above` and `below`: a new cursor one row above
    /// the topmost (or below the bottommost), at the same x.
    pub fn add_cursor(&mut self, above: bool, window: &mut Window, cx: &mut Context<Self>) {
        let cursors = self.buf.cursors();
        let from = if above {
            cursors.sels[0]
        } else {
            cursors.sels[cursors.sels.len() - 1]
        };
        let (head, goal) =
            self.vertical_target(window, from.head, from.goal, if above { -1 } else { 1 });
        if self.buf.cursor_at(head).line == self.buf.cursor_at(from.head).line {
            return;
        }
        let mut sels = cursors.sels;
        sels.push(Sel {
            anchor: head,
            head,
            goal,
        });
        self.buf.set_cursors(Cursors {
            sels,
            primary: cursors.primary,
        });
        self.reveal_cursor(window);
        cx.notify();
    }

    pub(crate) fn add_cursor_above(
        &mut self,
        _: &AddCursorAbove,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_cursor(true, window, cx);
    }

    pub(crate) fn add_cursor_below(
        &mut self,
        _: &AddCursorBelow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_cursor(false, window, cx);
    }

    pub(crate) fn page(&mut self, down: bool, select: bool, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.visible_rows() as isize;
        let lh = self.layout.line_height;
        let dy = lh * rows as f32;
        self.scroll_by(if down { dy } else { -dy }, window, cx);
        self.vertical(if down { rows } else { -rows }, select, window, cx);
    }

    pub(crate) fn move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.active_field() {
            f.left();
            cx.notify();
            return;
        }
        self.horizontal(true, false, window, cx);
    }

    pub(crate) fn move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.active_field() {
            f.right();
            cx.notify();
            return;
        }
        self.horizontal(false, false, window, cx);
    }

    pub(crate) fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(true, true, window, cx);
    }

    pub(crate) fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(false, true, window, cx);
    }

    pub(crate) fn move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(-1, cx) {
            return;
        }
        // * Leaving the line forgets the closers auto-pairing planted on it:
        //   coming back and typing one writes it (Specification.md,
        //   "Observed auto-pair behavior").
        self.buf.forget_planted();
        self.vertical(-1, false, window, cx);
    }

    pub(crate) fn move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(1, cx) {
            return;
        }
        self.buf.forget_planted();
        self.vertical(1, false, window, cx);
    }

    pub(crate) fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, window, cx);
    }

    pub(crate) fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, window, cx);
    }

    pub(crate) fn move_word_left(&mut self, _: &MoveWordLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.move_each(false, window, cx, |this, _, s| {
            (this.buf.word_left(s.head), None)
        });
    }

    pub(crate) fn move_word_right(&mut self, _: &MoveWordRight, window: &mut Window, cx: &mut Context<Self>) {
        self.move_each(false, window, cx, |this, _, s| {
            (this.buf.word_right(s.head), None)
        });
    }

    pub(crate) fn select_word_left(
        &mut self,
        _: &SelectWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_each(true, window, cx, |this, _, s| {
            (this.buf.word_left(s.head), None)
        });
    }

    pub(crate) fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_each(true, window, cx, |this, _, s| {
            (this.buf.word_right(s.head), None)
        });
    }

    pub(crate) fn page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(-(crate::suggest::VISIBLE as isize), cx) {
            return;
        }
        self.page(false, false, window, cx);
    }

    pub(crate) fn page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(crate::suggest::VISIBLE as isize, cx) {
            return;
        }
        self.page(true, false, window, cx);
    }

    pub(crate) fn select_page_up(&mut self, _: &SelectPageUp, window: &mut Window, cx: &mut Context<Self>) {
        self.page(false, true, window, cx);
    }

    pub(crate) fn select_page_down(
        &mut self,
        _: &SelectPageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.page(true, true, window, cx);
    }

    pub(crate) fn line_start(&mut self, _: &LineStart, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(isize::MIN, cx) {
            return;
        }
        if let Some(f) = self.active_field() {
            f.home();
            cx.notify();
            return;
        }
        self.row_boundary(false, false, window, cx);
    }

    pub(crate) fn line_end(&mut self, _: &LineEnd, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggest_step(isize::MAX, cx) {
            return;
        }
        if let Some(f) = self.active_field() {
            f.end();
            cx.notify();
            return;
        }
        self.row_boundary(true, false, window, cx);
    }

    pub(crate) fn select_line_start(
        &mut self,
        _: &SelectLineStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.row_boundary(false, true, window, cx);
    }

    pub(crate) fn select_line_end(&mut self, _: &SelectLineEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.row_boundary(true, true, window, cx);
    }

    pub(crate) fn document_start(&mut self, _: &DocumentStart, window: &mut Window, cx: &mut Context<Self>) {
        self.buf.goal = None;
        self.move_to(0, false, window, cx);
    }

    pub(crate) fn document_end(&mut self, _: &DocumentEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.buf.goal = None;
        self.move_to(self.buf.len(), false, window, cx);
    }

    pub(crate) fn select_document_start(
        &mut self,
        _: &SelectDocumentStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.buf.goal = None;
        self.move_to(0, true, window, cx);
    }

    pub(crate) fn select_document_end(
        &mut self,
        _: &SelectDocumentEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.buf.goal = None;
        self.move_to(self.buf.len(), true, window, cx);
    }

    pub(crate) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        // ! An open field takes the keys the way backspace and delete give
        //   them to it. It carries a caret and no selection, so there is
        //   nothing for Ctrl+A to take; what it must not do is select the
        //   document behind the field, which the next Ctrl+X would delete.
        if self.field_kind().is_some() {
            return;
        }
        self.buf.select_all();
        cx.notify();
    }
}
