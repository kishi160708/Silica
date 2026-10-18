//! The buffer: the text, its line index, the selections over it and the
//! undo history, with one method per thing the editor's keys do to them.
//!
//! The editor view owns a [`TextBuffer`] and adds the window work around
//! it — shaping, scrolling, decorations, the find panel, the properties
//! table. Everything that decides *bytes* lives here, so the Writer gate's
//! round trips can drive the same code the keys do.

use crate::lists;
use crate::plan::{indent_plan, newline_plan, shifted};
use std::ops::Range;
use std::time::{Duration, Instant};

/// Typing or deleting within this of the previous step of the same kind
/// joins its undo group.
const UNDO_GROUP: Duration = Duration::from_millis(500);

/// A position in the text: zero-based line and byte offset within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub line: usize,
    /// Byte offset within the line.
    pub col: usize,
}

/// One selection: `head` is the moving end (the cursor), `anchor` the
/// fixed one; equal when nothing is selected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sel {
    pub anchor: usize,
    pub head: usize,
    /// Preferred x when moving between rows, in pixels. The buffer only
    /// carries it; the view measures and uses it.
    pub goal: Option<f32>,
}

impl Sel {
    pub fn at(offset: usize) -> Sel {
        Sel {
            anchor: offset,
            head: offset,
            goal: None,
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.head.min(self.anchor)..self.head.max(self.anchor)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.anchor
    }
}

/// Every selection in buffer order, and which one is the primary (the one
/// the find panel, IME and scrolling follow).
#[derive(Debug, Clone, PartialEq)]
pub struct Cursors {
    pub sels: Vec<Sel>,
    pub primary: usize,
}

impl Cursors {
    /// One cursor at `offset`, nothing selected.
    pub fn at(offset: usize) -> Cursors {
        Cursors {
            sels: vec![Sel::at(offset)],
            primary: 0,
        }
    }

    /// The selections' ends, for comparing where one step ended and the
    /// next began without regard to goal x.
    pub fn positions(&self) -> Vec<(usize, usize)> {
        self.sels.iter().map(|s| (s.anchor, s.head)).collect()
    }
}

/// What kind of step an edit is, for grouping the undo history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Backspace,
    Delete,
    Other,
}

/// One replacement: `old` at `at` became `new`.
#[derive(Debug)]
struct Edit {
    at: usize,
    old: String,
    new: String,
}

/// Edits undone and redone together, with the selections either side.
#[derive(Debug)]
struct Group {
    edits: Vec<Edit>,
    before: Cursors,
    after: Cursors,
    kind: EditKind,
    at: Instant,
}

#[derive(Debug, Default)]
struct History {
    undo: Vec<Group>,
    redo: Vec<Group>,
}

/// A file's text split from the bytes the buffer normalizes away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileText {
    pub bom: bool,
    /// The file used `\r\n` on most of its lines; a save puts that ending
    /// back on every line.
    pub crlf: bool,
    /// The file used both endings, so [`FileText::join`] cannot reproduce
    /// it. The buffer keeps such a file's bytes and writes them back
    /// verbatim while nothing has changed.
    pub mixed: bool,
    /// The body with `\n` line endings.
    pub text: String,
}

impl FileText {
    pub fn parse(raw: &str) -> FileText {
        let (bom, body) = match raw.strip_prefix('\u{feff}') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };
        // The dominant ending, not merely the presence of one: a single
        // stray `\r\n` in a file of 4000 Unix lines must not turn every
        // one of them on the next save.
        let breaks = body.matches('\n').count();
        let crlf_breaks = body.matches("\r\n").count();
        let crlf = crlf_breaks * 2 > breaks;
        let mixed = crlf_breaks != 0 && crlf_breaks != breaks;
        let text = if crlf_breaks > 0 {
            body.replace("\r\n", "\n")
        } else {
            body.to_string()
        };
        FileText {
            bom,
            crlf,
            mixed,
            text,
        }
    }

    pub fn join(bom: bool, crlf: bool, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 3);
        if bom {
            out.push('\u{feff}');
        }
        if crlf {
            out.push_str(&text.replace('\n', "\r\n"));
        } else {
            out.push_str(text);
        }
        out
    }
}

/// The text of one open file with everything the editor keeps about it that
/// a window is not needed for.
///
/// `text` and `line_starts` are private and always in step: the text only
/// ever changes through the editing methods, each of which rebuilds the
/// index and records the step for undo. The selections are public — moving
/// a cursor needs shaped lines, which only the view has.
#[derive(Debug, Default)]
pub struct TextBuffer {
    text: String,
    line_starts: Vec<usize>,
    /// The moving end of the primary selection; the cursor.
    pub head: usize,
    /// The fixed end of the primary selection; equals `head` when nothing
    /// is selected.
    pub anchor: usize,
    /// Preferred x when moving between rows.
    pub goal: Option<f32>,
    /// Further selections (Find all, Alt+click, add cursor above and
    /// below), in buffer order, disjoint from each other and the primary.
    pub extra: Vec<Sel>,
    /// IME composition range, absolute.
    pub marked: Option<Range<usize>>,
    history: History,
    dirty: bool,
    /// The file used `\r\n`; restored on save.
    crlf: bool,
    /// The file started with a byte order mark; restored on save.
    bom: bool,
    /// A file whose line endings are not all the same: its bytes and the
    /// text they loaded as. [`TextBuffer::file_contents`] writes those bytes
    /// back while the text still matches, so opening and saving such a file
    /// changes nothing; once it is edited, the dominant ending wins.
    mixed: Option<Box<MixedFile>>,
}

/// The bytes of a file the buffer cannot reproduce from its text alone.
#[derive(Debug)]
struct MixedFile {
    raw: String,
    text: String,
}

impl TextBuffer {
    /// An empty buffer, as a new editor starts.
    pub fn new() -> TextBuffer {
        let mut buf = TextBuffer::default();
        buf.rebuild_lines();
        buf
    }

    /// A buffer holding a file's bytes, the cursor at the start.
    pub fn from_file(raw: &str) -> TextBuffer {
        let mut buf = TextBuffer::new();
        buf.load(raw);
        buf
    }

    /// Replaces the text with a file's raw bytes, remembering its BOM and
    /// line endings and dropping the history. The caller places the cursor:
    /// opening a file puts it at the start, a reload clamps it where it was.
    pub fn load(&mut self, raw: &str) {
        let file = FileText::parse(raw);
        self.crlf = file.crlf;
        self.bom = file.bom;
        self.mixed = file.mixed.then(|| {
            Box::new(MixedFile {
                raw: raw.to_string(),
                text: file.text.clone(),
            })
        });
        self.text = file.text;
        self.marked = None;
        self.goal = None;
        self.extra.clear();
        self.dirty = false;
        self.history = History::default();
        self.rebuild_lines();
    }

    /// The bytes to write back: the text with the file's original line
    /// endings and byte order mark.
    pub fn file_contents(&self) -> String {
        match &self.mixed {
            // Untouched, and not reproducible from the text: its own bytes.
            Some(file) if file.text == self.text => file.raw.clone(),
            _ => FileText::join(self.bom, self.crlf, &self.text),
        }
    }

    /// Whether `raw` (a file's bytes) is this buffer, ignoring the BOM and
    /// line endings the buffer normalizes away.
    pub fn matches_file(&self, raw: &str) -> bool {
        FileText::parse(raw).text == self.text
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Marks the text changed; the history steps do their own bookkeeping
    /// and call this.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// The file's line ending, for text written into it from elsewhere
    /// (the properties table serializes with it).
    pub fn crlf(&self) -> bool {
        self.crlf
    }

    pub fn bom(&self) -> bool {
        self.bom
    }

    // ----- the text ------------------------------------------------------------

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    fn rebuild_lines(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        self.line_starts
            .extend(self.text.match_indices('\n').map(|(i, _)| i + 1));
    }

    /// The line's bytes without its newline.
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let end = match self.line_starts.get(line + 1) {
            Some(next) => next - 1,
            None => self.text.len(),
        };
        start..end
    }

    pub fn line_text(&self, line: usize) -> &str {
        &self.text[self.line_range(line)]
    }

    pub fn cursor_at(&self, offset: usize) -> Cursor {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        Cursor {
            line,
            col: offset - self.line_starts[line],
        }
    }

    pub fn clamp_col(&self, line: usize, col: usize) -> usize {
        let text = self.line_text(line);
        let mut col = col.min(text.len());
        while !text.is_char_boundary(col) {
            col -= 1;
        }
        col
    }

    pub fn clamp_offset(&self, offset: usize) -> usize {
        let mut at = offset.min(self.text.len());
        while !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    pub fn prev_char_boundary(&self, offset: usize) -> usize {
        let mut i = offset.saturating_sub(1);
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    pub fn next_char_boundary(&self, offset: usize) -> usize {
        let mut i = (offset + 1).min(self.text.len());
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    /// Word-boundary scan: whitespace, then a run of one character class.
    pub fn word_left(&self, from: usize) -> usize {
        let mut i = from;
        let class_at = |i: usize| self.text[..i].chars().next_back().map(char_class);
        while i > 0 && class_at(i) == Some(0) {
            i = self.prev_char_boundary(i);
        }
        let class = class_at(i);
        while i > 0 && class_at(i) == class {
            i = self.prev_char_boundary(i);
        }
        i
    }

    pub fn word_right(&self, from: usize) -> usize {
        let mut i = from;
        let len = self.text.len();
        let class_at = |i: usize| self.text[i..].chars().next().map(char_class);
        while i < len && class_at(i) == Some(0) {
            i = self.next_char_boundary(i);
        }
        let class = class_at(i);
        while i < len && class_at(i) == class {
            i = self.next_char_boundary(i);
        }
        i
    }

    pub fn word_at(&self, at: usize) -> Range<usize> {
        let class_before = self.text[..at].chars().next_back().map(char_class);
        let class_after = self.text[at..].chars().next().map(char_class);
        // Prefer the word the cursor is in or just after; whitespace selects itself.
        let class = match (class_before, class_after) {
            (_, Some(c)) if c != 0 => c,
            (Some(c), _) if c != 0 => c,
            (_, Some(c)) => c,
            (Some(c), None) => c,
            (None, None) => return at..at,
        };
        let mut start = at;
        while start > 0
            && self.text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| char_class(c) == class && c != '\n')
        {
            start = self.prev_char_boundary(start);
        }
        let mut end = at;
        while end < self.text.len()
            && self.text[end..]
                .chars()
                .next()
                .is_some_and(|c| char_class(c) == class && c != '\n')
        {
            end = self.next_char_boundary(end);
        }
        start..end
    }

    // ----- selections ----------------------------------------------------------

    /// The primary cursor's line and column.
    pub fn cursor(&self) -> Cursor {
        self.cursor_at(self.head)
    }

    /// The primary selection's byte range, empty at the cursor when
    /// nothing is selected.
    pub fn selection(&self) -> Range<usize> {
        self.head.min(self.anchor)..self.head.max(self.anchor)
    }

    /// How many cursors there are (one unless Find all or Alt+click added more).
    pub fn cursor_count(&self) -> usize {
        self.extra.len() + 1
    }

    /// Every selection in buffer order, with the primary's index.
    pub fn cursors(&self) -> Cursors {
        let primary = Sel {
            anchor: self.anchor,
            head: self.head,
            goal: self.goal,
        };
        let mut sels = self.extra.clone();
        let start = primary.range().start;
        let ix = sels.partition_point(|s| s.range().start < start);
        sels.insert(ix, primary);
        Cursors { sels, primary: ix }
    }

    /// Installs `cursors`, clamped to the text, with selections that
    /// overlap (or cursors at the same place) merged into one.
    pub fn set_cursors(&mut self, cursors: Cursors) {
        let mut sels: Vec<(Sel, bool)> = cursors
            .sels
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let sel = Sel {
                    anchor: self.clamp_offset(s.anchor),
                    head: self.clamp_offset(s.head),
                    goal: s.goal,
                };
                (sel, i == cursors.primary)
            })
            .collect();
        sels.sort_by_key(|(s, _)| (s.range().start, s.range().end));
        let mut merged: Vec<(Sel, bool)> = Vec::new();
        for (s, p) in sels {
            if let Some((last, lp)) = merged.last_mut() {
                let (lr, r) = (last.range(), s.range());
                let overlaps =
                    r.start < lr.end || (lr.is_empty() && r.is_empty() && r.start == lr.start);
                if overlaps {
                    let (start, end) = (lr.start.min(r.start), lr.end.max(r.end));
                    // The primary's direction wins; otherwise the earlier one's.
                    let reversed = if p && !*lp {
                        s.head < s.anchor
                    } else {
                        last.head < last.anchor
                    };
                    *last = Sel {
                        anchor: if reversed { end } else { start },
                        head: if reversed { start } else { end },
                        goal: None,
                    };
                    *lp |= p;
                    continue;
                }
            }
            merged.push((s, p));
        }
        let primary = merged.iter().position(|(_, p)| *p).unwrap_or(0);
        let (main, _) = merged.remove(primary);
        self.head = main.head;
        self.anchor = main.anchor;
        self.goal = main.goal;
        self.extra = merged.into_iter().map(|(s, _)| s).collect();
    }

    /// Drops every cursor but the primary.
    pub fn collapse_cursors(&mut self) {
        self.extra.clear();
    }

    /// Puts one cursor at `offset` (a selection when `anchor` is given),
    /// clamped to the text, dropping the rest.
    pub fn set_cursor(&mut self, offset: usize, anchor: Option<usize>) {
        self.collapse_cursors();
        self.head = self.clamp_offset(offset);
        self.anchor = match anchor {
            Some(a) => self.clamp_offset(a),
            None => self.head,
        };
        self.goal = None;
        self.marked = None;
    }

    /// Selects the whole text, as Ctrl+A does.
    pub fn select_all(&mut self) {
        self.collapse_cursors();
        self.anchor = 0;
        self.head = self.text.len();
        self.goal = None;
    }

    /// The lines a selection touches (the cursor's line when collapsed).
    pub fn selected_lines(&self) -> Range<usize> {
        let sel = self.selection();
        let first = self.cursor_at(sel.start).line;
        let mut last = self.cursor_at(sel.end).line;
        // A selection ending at a line start does not include that line.
        if last > first && sel.end == self.line_starts[last] {
            last -= 1;
        }
        first..last + 1
    }

    /// Keeps the extra cursors in place after the primary's edit replaced
    /// `range` with `inserted` bytes.
    fn shift_extras(&mut self, range: &Range<usize>, inserted: usize) {
        if self.extra.is_empty() {
            return;
        }
        let map = |at: usize| -> usize {
            if at >= range.end {
                at - range.len() + inserted
            } else if at > range.start {
                range.start + inserted
            } else {
                at
            }
        };
        for s in &mut self.extra {
            s.head = map(s.head);
            s.anchor = map(s.anchor);
        }
        let cursors = self.cursors();
        self.set_cursors(cursors);
    }

    // ----- editing -------------------------------------------------------------

    /// Replaces `range` with `insert` at the primary cursor, records it for
    /// undo and moves the cursor after the insertion; other cursors keep
    /// their place in the text.
    pub fn edit(&mut self, range: Range<usize>, insert: &str, kind: EditKind) {
        let before = self.cursors();
        let old = self.text[range.clone()].to_string();
        self.text.replace_range(range.clone(), insert);
        self.rebuild_lines();
        self.head = range.start + insert.len();
        self.anchor = self.head;
        self.marked = None;
        self.goal = None;
        self.dirty = true;
        self.shift_extras(&range, insert.len());
        self.record(
            vec![Edit {
                at: range.start,
                old,
                new: insert.to_string(),
            }],
            kind,
            before,
        );
    }

    /// One replacement per selection, in [`TextBuffer::cursors`] order, as
    /// one undo step; every cursor collapses after its own insertion.
    /// Ranges that reach into the previous selection's replacement are cut
    /// short. False when there was nothing to do.
    pub fn edit_all(&mut self, mut edits: Vec<(Range<usize>, String)>, kind: EditKind) -> bool {
        let cursors = self.cursors();
        debug_assert_eq!(edits.len(), cursors.sels.len());
        let mut floor = 0;
        for (range, _) in &mut edits {
            range.start = range.start.max(floor).min(range.end);
            floor = range.end;
        }
        if edits.iter().all(|(r, new)| r.is_empty() && new.is_empty()) {
            return false;
        }
        let mut after = Vec::with_capacity(edits.len());
        let mut delta: isize = 0;
        for (range, new) in &edits {
            let start = (range.start as isize + delta) as usize;
            after.push(Sel::at(start + new.len()));
            delta += new.len() as isize - range.len() as isize;
        }
        let after = Cursors {
            sels: after,
            primary: cursors.primary,
        };
        self.apply_edits(edits, after, kind);
        true
    }

    /// Inserts `s` at every cursor, replacing what each has selected.
    pub fn insert_all(&mut self, s: &str, kind: EditKind) -> bool {
        let edits = self
            .cursors()
            .sels
            .iter()
            .map(|sel| (sel.range(), s.to_string()))
            .collect();
        self.edit_all(edits, kind)
    }

    /// Deletes at every cursor: the selection, or what `range` gives for a
    /// collapsed one.
    pub fn delete_all(
        &mut self,
        range: impl Fn(&TextBuffer, usize) -> Range<usize>,
        kind: EditKind,
    ) -> bool {
        let edits = self
            .cursors()
            .sels
            .iter()
            .map(|sel| {
                let r = if sel.is_empty() {
                    range(self, sel.head)
                } else {
                    sel.range()
                };
                (r, String::new())
            })
            .collect();
        self.edit_all(edits, kind)
    }

    /// Applies several non-overlapping replacements as one undo step and
    /// puts the cursor at `cursor` (a selection when `anchor` is given),
    /// both in post-edit coordinates; other cursors are dropped. Ranges may
    /// come in any order. False when `edits` was empty.
    pub fn edit_many(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        cursor: usize,
        anchor: Option<usize>,
    ) -> bool {
        if edits.is_empty() {
            return false;
        }
        let after = Cursors {
            sels: vec![Sel {
                anchor: anchor.unwrap_or(cursor),
                head: cursor,
                goal: None,
            }],
            primary: 0,
        };
        self.apply_edits(edits, after, EditKind::Other);
        true
    }

    /// The text and history part of every multi-cursor edit: `after` is in
    /// post-edit coordinates.
    pub fn apply_edits(
        &mut self,
        mut edits: Vec<(Range<usize>, String)>,
        after: Cursors,
        kind: EditKind,
    ) {
        let before = self.cursors();
        // Back to front, so earlier offsets stay valid while later text
        // moves; of two edits at one offset the replacement goes before
        // the insertion.
        edits.sort_by_key(|e| (std::cmp::Reverse(e.0.start), std::cmp::Reverse(e.0.end)));
        let mut recorded = Vec::with_capacity(edits.len());
        for (range, new) in edits {
            let old = self.text[range.clone()].to_string();
            self.text.replace_range(range.clone(), &new);
            recorded.push(Edit {
                at: range.start,
                old,
                new,
            });
        }
        self.rebuild_lines();
        self.set_cursors(after);
        self.marked = None;
        self.dirty = true;
        self.record(recorded, kind, before);
    }

    /// Records one step's edits (in the order they were applied). A typing
    /// or deleting step that starts exactly where the previous one of the
    /// same kind ended, within [`UNDO_GROUP`], joins its group.
    fn record(&mut self, edits: Vec<Edit>, kind: EditKind, before: Cursors) {
        let after = self.cursors();
        let now = Instant::now();
        self.history.redo.clear();
        if kind != EditKind::Other {
            if let Some(last) = self.history.undo.last_mut() {
                if last.kind == kind
                    && last.after.positions() == before.positions()
                    && now.duration_since(last.at) < UNDO_GROUP
                {
                    last.edits.extend(edits);
                    last.after = after;
                    last.at = now;
                    return;
                }
            }
        }
        self.history.undo.push(Group {
            edits,
            before,
            after,
            kind,
            at: now,
        });
    }

    /// Undoes one step, restoring the selections it started from. False
    /// when the history is empty.
    pub fn undo(&mut self) -> bool {
        let Some(group) = self.history.undo.pop() else {
            return false;
        };
        for e in group.edits.iter().rev() {
            self.text.replace_range(e.at..e.at + e.new.len(), &e.old);
        }
        self.rebuild_lines();
        self.set_cursors(group.before.clone());
        self.history.redo.push(group);
        self.finish_history_step();
        true
    }

    /// Redoes one undone step. False when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(group) = self.history.redo.pop() else {
            return false;
        };
        for e in &group.edits {
            self.text.replace_range(e.at..e.at + e.old.len(), &e.new);
        }
        self.rebuild_lines();
        self.set_cursors(group.after.clone());
        self.history.undo.push(group);
        self.finish_history_step();
        true
    }

    /// Whether anything is on the undo stack, for a test that wants to
    /// wind a buffer all the way back.
    pub fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }

    fn finish_history_step(&mut self) {
        self.marked = None;
        self.goal = None;
        self.dirty = true;
    }

    // ----- the editing commands ------------------------------------------------

    /// Backspace: the selection at every cursor, or the character before a
    /// collapsed one.
    pub fn backspace(&mut self) -> bool {
        let kind = if self.selection().is_empty() {
            EditKind::Backspace
        } else {
            EditKind::Other
        };
        self.delete_all(|buf, at| buf.prev_char_boundary(at)..at, kind)
    }

    /// Delete: the selection at every cursor, or the character after a
    /// collapsed one.
    pub fn delete_forward(&mut self) -> bool {
        let kind = if self.selection().is_empty() {
            EditKind::Delete
        } else {
            EditKind::Other
        };
        self.delete_all(|buf, at| at..buf.next_char_boundary(at), kind)
    }

    pub fn delete_word_left(&mut self) -> bool {
        self.delete_all(|buf, at| buf.word_left(at)..at, EditKind::Other)
    }

    pub fn delete_word_right(&mut self) -> bool {
        self.delete_all(|buf, at| at..buf.word_right(at), EditKind::Other)
    }

    /// Enter: with `smart_indent_list`, a newline that continues list and
    /// quote markers and the numbering of ordered lists, and clears or
    /// outdents an empty item. With several cursors, or a selection, a
    /// plain newline at each.
    pub fn newline(&mut self, smart_indent_list: bool) -> bool {
        let sel = self.selection();
        if !smart_indent_list || sel.start != sel.end || !self.extra.is_empty() {
            return self.insert_all("\n", EditKind::Other);
        }
        let (edits, cursor) = newline_plan(&self.text, &self.line_starts, self.head);
        self.edit_many(edits, cursor, None)
    }

    /// Tab: indents list items and the selected lines by one step
    /// (renumbering the ordered lists it touches), and inserts the step
    /// anywhere else.
    pub fn indent(&mut self, use_tab: bool, tab_size: u8, smart_indent_list: bool) -> bool {
        let unit = lists::indent_unit(use_tab, tab_size);
        let sel = self.selection();
        let lines = self.selected_lines();
        let on_item = lists::parse_prefix(self.line_text(lines.start))
            .marker
            .is_some();
        if !self.extra.is_empty() || (sel.start == sel.end && !(smart_indent_list && on_item)) {
            return self.insert_all(&unit, EditKind::Typing);
        }
        let edits = indent_plan(
            &self.text,
            &self.line_starts,
            lines,
            &unit,
            true,
            smart_indent_list,
        );
        let (head, anchor) = (shifted(self.head, &edits), shifted(self.anchor, &edits));
        self.edit_many(edits, head, Some(anchor))
    }

    /// Shift+Tab: removes one indentation step from the selected lines,
    /// renumbering the ordered lists it touches.
    pub fn outdent(&mut self, smart_indent_list: bool) -> bool {
        let lines = self.selected_lines();
        let edits = indent_plan(
            &self.text,
            &self.line_starts,
            lines,
            "",
            false,
            smart_indent_list,
        );
        let (head, anchor) = (shifted(self.head, &edits), shifted(self.anchor, &edits));
        self.edit_many(edits, head, Some(anchor))
    }

    /// What each cursor copies: its selection, or its whole line when
    /// nothing is selected (what CodeMirror copies).
    pub fn copy_ranges(&self) -> Vec<Range<usize>> {
        self.cursors()
            .sels
            .iter()
            .map(|s| {
                if !s.is_empty() {
                    return s.range();
                }
                let line = self.cursor_at(s.head).line;
                let start = self.line_starts[line];
                let end = self
                    .line_starts
                    .get(line + 1)
                    .copied()
                    .unwrap_or(self.text.len());
                start..end
            })
            .collect()
    }

    /// The clipboard text for `ranges`: several cursors' texts joined with
    /// newlines.
    pub fn clipboard_text(&self, ranges: &[Range<usize>]) -> String {
        let parts: Vec<&str> = ranges.iter().map(|r| &self.text[r.clone()]).collect();
        parts.join("\n")
    }

    /// Pastes `text` at every cursor; a clipboard with one line per cursor
    /// gives each its own line, the way CodeMirror pastes a multi-cursor
    /// copy.
    pub fn paste(&mut self, text: &str) -> bool {
        let text = text.replace("\r\n", "\n");
        let cursors = self.cursors();
        let lines: Vec<&str> = text.split('\n').collect();
        let edits = cursors
            .sels
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let piece = if cursors.sels.len() > 1 && lines.len() == cursors.sels.len() {
                    lines[i].to_string()
                } else {
                    text.clone()
                };
                (s.range(), piece)
            })
            .collect();
        self.edit_all(edits, EditKind::Other)
    }
}

/// 0 for whitespace, 1 for word characters, 2 for punctuation.
pub fn char_class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_classes_split_identifiers_from_punctuation() {
        assert_eq!(char_class(' '), 0);
        assert_eq!(char_class('a'), 1);
        assert_eq!(char_class('_'), 1);
        assert_eq!(char_class('['), 2);
    }

    #[test]
    fn file_text_round_trips_bom_and_line_endings() {
        for raw in [
            "plain\nlf\n",
            "\u{feff}bom\nlf",
            "crlf\r\nlines\r\n",
            "\u{feff}# both\r\n\r\ntext\r\n",
            "",
            "no trailing newline",
        ] {
            let f = FileText::parse(raw);
            assert!(!f.text.contains('\r'), "{raw:?}");
            assert_eq!(FileText::join(f.bom, f.crlf, &f.text), raw, "{raw:?}");
        }
    }

    #[test]
    fn the_dominant_line_ending_wins_and_mixed_files_are_flagged() {
        // Two CRLF breaks against one bare LF: a Windows file.
        let f = FileText::parse("a\r\nb\r\nc\nd");
        assert_eq!(f.text, "a\nb\nc\nd");
        assert!(f.crlf && f.mixed);
        assert_eq!(FileText::join(f.bom, f.crlf, &f.text), "a\r\nb\r\nc\r\nd");
        // One stray CRLF in a Unix file does not make it a Windows one.
        let f = FileText::parse("a\r\nb\nc\nd");
        assert!(!f.crlf && f.mixed);
        assert_eq!(FileText::join(f.bom, f.crlf, &f.text), "a\nb\nc\nd");
        // Uniform files are not mixed either way.
        assert!(!FileText::parse("a\nb\n").mixed);
        assert!(!FileText::parse("a\r\nb\r\n").mixed);
    }

    /// A file whose endings disagree cannot be rebuilt from the text, so an
    /// untouched buffer writes the bytes it was given. One stray `\r\n` in
    /// a file of Unix lines used to turn every line ending on the next save.
    #[test]
    fn a_mixed_file_saves_as_itself_until_it_is_edited() {
        let raw = "# Changelog\r\nfirst\nsecond\nthird\n";
        let mut buf = TextBuffer::from_file(raw);
        assert_eq!(buf.file_contents(), raw);

        buf.set_cursor(buf.len(), None);
        buf.insert_all("fourth", EditKind::Typing);
        // Edited: the dominant ending, on every line.
        assert_eq!(
            buf.file_contents(),
            "# Changelog\nfirst\nsecond\nthird\nfourth"
        );
        buf.undo();
        assert_eq!(buf.file_contents(), raw, "undo did not restore the bytes");
    }

    #[test]
    fn a_loaded_file_comes_back_as_its_own_bytes() {
        for raw in ["# A\n\ntext\n", "\u{feff}bom\r\nlines\r\n", "no newline"] {
            let buf = TextBuffer::from_file(raw);
            assert_eq!(buf.file_contents(), raw, "{raw:?}");
            assert!(!buf.is_dirty());
            assert_eq!(buf.head, 0);
        }
    }

    #[test]
    fn undo_winds_an_edited_buffer_back_to_its_bytes() {
        let raw = "- one\n- two\n";
        let mut buf = TextBuffer::from_file(raw);
        buf.set_cursor(5, None);
        buf.newline(true);
        buf.insert_all("three", EditKind::Typing);
        assert_eq!(buf.text(), "- one\n- three\n- two\n");
        while buf.can_undo() {
            buf.undo();
        }
        assert_eq!(buf.file_contents(), raw);
    }

    #[test]
    fn the_line_index_follows_every_edit() {
        let mut buf = TextBuffer::from_file("a\nb\nc\n");
        assert_eq!(buf.line_count(), 4);
        buf.set_cursor(2, None);
        buf.insert_all("x\ny", EditKind::Typing);
        assert_eq!(buf.text(), "a\nx\nyb\nc\n");
        assert_eq!(buf.line_starts(), [0, 2, 4, 7, 9]);
        assert_eq!(buf.line_text(2), "yb");
        buf.undo();
        assert_eq!(buf.line_starts(), [0, 2, 4, 6]);
    }
}
