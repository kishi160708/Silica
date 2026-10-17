//! Source-mode editor: one `String` buffer with a line index, soft-wrapped
//! at the view width, with selections, undo and the decorations of the
//! shared parser.
//!
//! Only the lines on screen are shaped and painted each frame. The scroll
//! position is a line plus a pixel offset into it rather than an absolute
//! pixel, so wrapped-row heights are only ever measured for lines near the
//! viewport and a 50k-line note never needs a full layout pass. Decorations
//! come from `_markdown::highlight`, recomputed on a background thread
//! after every edit and swapped in when they arrive, so typing never waits
//! for a parse. Text input goes through GPUI's input handler (so IME and
//! dead keys work) scoped to the cursor's line; everything else is an action
//! bound in [`bind_keys`].
//!
//! The buffer holds `\n` line endings; the file's own ending and BOM are
//! remembered at load and restored by [`Editor::file_contents`], so a save
//! changes nothing but the edited bytes.

use crate::lists::{self, Continuation};
use crate::theme::Theme;
use gpui::{
    actions, div, fill, point, prelude::*, px, relative, size, App, Bounds, ClipboardItem, Context,
    DispatchPhase, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, Font, FontStyle, FontWeight, GlobalElementId, Hsla,
    InspectorElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, ScrollWheelEvent, SharedString, StrikethroughStyle,
    Style, Task, TextAlign, TextRun, TextStyle, UTF16Selection, UnderlineStyle, Window,
    WrappedLine,
};
use _markdown::highlight::{flatten, highlight_lines, HighlightKind, HighlightSpan};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

actions!(
    editor,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveWordLeft,
        MoveWordRight,
        LineStart,
        LineEnd,
        PageUp,
        PageDown,
        DocumentStart,
        DocumentEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectWordLeft,
        SelectWordRight,
        SelectLineStart,
        SelectLineEnd,
        SelectPageUp,
        SelectPageDown,
        SelectDocumentStart,
        SelectDocumentEnd,
        SelectAll,
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        Newline,
        NewlinePlain,
        Tab,
        Outdent,
        Undo,
        Redo,
        Copy,
        Cut,
        Paste,
    ]
);

/// Binds the editor's navigation and editing keys; call once at startup.
pub fn bind_keys(cx: &mut App) {
    let ctx = Some("Editor");
    cx.bind_keys([
        KeyBinding::new("left", MoveLeft, ctx),
        KeyBinding::new("right", MoveRight, ctx),
        KeyBinding::new("up", MoveUp, ctx),
        KeyBinding::new("down", MoveDown, ctx),
        KeyBinding::new("ctrl-left", MoveWordLeft, ctx),
        KeyBinding::new("ctrl-right", MoveWordRight, ctx),
        KeyBinding::new("home", LineStart, ctx),
        KeyBinding::new("end", LineEnd, ctx),
        KeyBinding::new("pageup", PageUp, ctx),
        KeyBinding::new("pagedown", PageDown, ctx),
        KeyBinding::new("ctrl-home", DocumentStart, ctx),
        KeyBinding::new("ctrl-end", DocumentEnd, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, ctx),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, ctx),
        KeyBinding::new("shift-home", SelectLineStart, ctx),
        KeyBinding::new("shift-end", SelectLineEnd, ctx),
        KeyBinding::new("shift-pageup", SelectPageUp, ctx),
        KeyBinding::new("shift-pagedown", SelectPageDown, ctx),
        KeyBinding::new("ctrl-shift-home", SelectDocumentStart, ctx),
        KeyBinding::new("ctrl-shift-end", SelectDocumentEnd, ctx),
        KeyBinding::new("ctrl-a", SelectAll, ctx),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("shift-backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("ctrl-backspace", DeleteWordLeft, ctx),
        KeyBinding::new("ctrl-delete", DeleteWordRight, ctx),
        KeyBinding::new("enter", Newline, ctx),
        KeyBinding::new("shift-enter", NewlinePlain, ctx),
        KeyBinding::new("tab", Tab, ctx),
        KeyBinding::new("shift-tab", Outdent, ctx),
        KeyBinding::new("ctrl-z", Undo, ctx),
        KeyBinding::new("ctrl-shift-z", Redo, ctx),
        KeyBinding::new("ctrl-y", Redo, ctx),
        KeyBinding::new("ctrl-c", Copy, ctx),
        KeyBinding::new("ctrl-x", Cut, ctx),
        KeyBinding::new("ctrl-v", Paste, ctx),
    ]);
}

/// Horizontal padding either side of the text.
const GUTTER: f32 = 24.0;
/// Obsidian's readable line length (`--file-line-width`).
const READABLE_WIDTH: f32 = 700.0;
/// Consecutive edits closer than this undo together, like CodeMirror.
const UNDO_GROUP: Duration = Duration::from_millis(500);
/// Width of the vertical scrollbar's track at the right edge.
const SCROLLBAR_WIDTH: f32 = 12.0;
/// The thumb never shrinks below this, so it can still be grabbed.
const SCROLLBAR_MIN_THUMB: f32 = 24.0;

pub enum EditorEvent {
    /// The buffer changed; the owner schedules a save.
    Edited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub line: usize,
    /// Byte offset within the line.
    pub col: usize,
}

/// Results of the last decoration pass, shown in the status bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseStats {
    pub parse_ms: f64,
    pub spans: usize,
    pub bytes: usize,
    pub words: usize,
    pub chars: usize,
}

/// Running totals for the stress run: count, sum and maximum of a duration.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    pub count: u64,
    pub total_ms: f64,
    pub max_ms: f64,
}

impl Sample {
    fn add(&mut self, ms: f64) {
        self.count += 1;
        self.total_ms += ms;
        if ms > self.max_ms {
            self.max_ms = ms;
        }
    }

    pub fn mean_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_ms / self.count as f64
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditKind {
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

/// Edits undone and redone together, with the selection either side.
#[derive(Debug)]
struct Group {
    edits: Vec<Edit>,
    before: (usize, usize),
    after: (usize, usize),
    kind: EditKind,
    at: Instant,
}

#[derive(Debug, Default)]
struct History {
    undo: Vec<Group>,
    redo: Vec<Group>,
}

/// Colors and fonts copied from the theme at render time, so shaping
/// outside a frame (clicks, cursor movement) matches what was painted.
#[derive(Clone)]
struct Palette {
    text: Hsla,
    muted: Hsla,
    accent: Hsla,
    link: Hsla,
    tag: Hsla,
    code_bg: Hsla,
    highlight_bg: Hsla,
    selection: Hsla,
    mono: SharedString,
}

impl Palette {
    fn from_theme(t: &Theme) -> Palette {
        let mut selection = t.accent;
        selection.a = if t.dark { 0.35 } else { 0.25 };
        Palette {
            text: t.text,
            muted: t.text_muted,
            accent: t.accent,
            link: t.link,
            tag: t.tag,
            code_bg: t.bg_alt,
            highlight_bg: t.highlight_bg,
            selection,
            mono: t.mono_font.clone(),
        }
    }
}

impl Default for Palette {
    fn default() -> Palette {
        Palette::from_theme(&Theme::dark())
    }
}

/// A line painted in the last frame: where it sat and how many rows it wrapped to.
#[derive(Debug, Clone, Copy)]
struct VisibleLine {
    line: usize,
    /// Top of the line relative to the element's origin.
    top: Pixels,
    rows: usize,
}

/// Geometry recorded at paint time so input handling can map points to text.
#[derive(Clone, Default)]
struct Layout {
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    font_size: Pixels,
    text_style: Option<TextStyle>,
    wrap_width: Option<Pixels>,
    /// Left edge of the text.
    content_x: Pixels,
    visible: Vec<VisibleLine>,
    cursor_line: Option<Arc<WrappedLine>>,
    /// Top of the cursor's line in window coordinates.
    cursor_top: Pixels,
    /// The scrollbar's track and thumb, when the note overflows the view.
    scrollbar: Option<Scrollbar>,
    /// Time spent shaping the visible lines in the last frame.
    prepaint_ms: f64,
}

/// Vertical scrollbar geometry in window coordinates. Positions are
/// line-based: the thumb sits at the first visible line's share of the
/// note and is as tall as the visible lines' share, so no full layout pass
/// is ever needed.
#[derive(Debug, Clone, Copy)]
struct Scrollbar {
    track: Bounds<Pixels>,
    thumb: Bounds<Pixels>,
    /// Lines the thumb can be dragged across: the count minus the visible ones.
    scrollable_lines: usize,
}

impl Scrollbar {
    /// Lays the thumb out for `first` as the first visible of `count`
    /// lines with `visible` of them on screen; `None` when everything fits.
    fn layout(
        track: Bounds<Pixels>,
        count: usize,
        first: usize,
        visible: usize,
        fits: bool,
    ) -> Option<Scrollbar> {
        if fits || count <= visible || track.size.height <= px(0.) {
            return None;
        }
        let track_h = track.size.height;
        let thumb_h = (track_h * (visible as f32 / count as f32))
            .max(px(SCROLLBAR_MIN_THUMB))
            .min(track_h);
        let scrollable_lines = count - visible;
        let frac = (first as f32 / scrollable_lines as f32).clamp(0., 1.);
        let top = track.origin.y + (track_h - thumb_h) * frac;
        let inset = px(3.);
        Some(Scrollbar {
            track,
            thumb: Bounds::new(
                point(track.origin.x + inset, top),
                size(track.size.width - inset * 2., thumb_h),
            ),
            scrollable_lines,
        })
    }

    /// The first visible line for a thumb whose top is at `y`.
    fn line_for_thumb_top(&self, y: Pixels) -> usize {
        let room = self.track.size.height - self.thumb.size.height;
        if room <= px(0.) {
            return 0;
        }
        let frac = ((y - self.track.origin.y) / room).clamp(0., 1.);
        (frac * self.scrollable_lines as f32).round() as usize
    }
}

pub struct Editor {
    focus_handle: FocusHandle,
    pub name: SharedString,
    text: String,
    line_starts: Vec<usize>,
    /// The moving end of the selection; the cursor.
    head: usize,
    /// The fixed end of the selection; equals `head` when nothing is selected.
    anchor: usize,
    /// Preferred x when moving between rows.
    goal_x: Option<Pixels>,
    /// IME composition range, absolute.
    marked: Option<Range<usize>>,
    scroll_line: usize,
    /// Pixels of `scroll_line` scrolled above the top of the view.
    scroll_px: Pixels,
    /// Reveal the cursor in the next frame (set when no window was at hand).
    pending_reveal: bool,
    dragging: bool,
    /// Dragging the scrollbar thumb: the grab point's offset from its top.
    thumb_drag: Option<Pixels>,
    highlights: Arc<Vec<Vec<HighlightSpan>>>,
    generation: u64,
    pub stats: ParseStats,
    layout: Layout,
    palette: Palette,
    pending_parse: Option<Task<()>>,
    /// An edit arrived while a parse was running; parse again when it lands.
    parse_queued: bool,
    history: History,
    dirty: bool,
    /// The file used `\r\n`; restored on save.
    crlf: bool,
    /// The file started with a byte order mark; restored on save.
    bom: bool,
    readable_line_length: bool,
    /// Enter continues list and quote markers (`smartIndentList`).
    smart_indent_list: bool,
    /// Tab inserts a tab (`useTab`) or `tab_size` spaces.
    use_tab: bool,
    tab_size: u8,
    /// Shaping time per painted frame since the note was opened.
    pub frame_samples: Sample,
    /// Parse time per completed decoration pass since the note was opened.
    pub parse_samples: Sample,
    /// Edit-to-decoration latency, including waiting for the background thread.
    pub parse_latency: Sample,
}

impl EventEmitter<EditorEvent> for Editor {}

impl Editor {
    pub fn new(cx: &mut Context<Self>) -> Editor {
        Editor {
            focus_handle: cx.focus_handle(),
            name: "untitled".into(),
            text: String::new(),
            line_starts: vec![0],
            head: 0,
            anchor: 0,
            goal_x: None,
            marked: None,
            scroll_line: 0,
            scroll_px: px(0.),
            pending_reveal: false,
            dragging: false,
            thumb_drag: None,
            highlights: Arc::new(vec![Vec::new()]),
            generation: 0,
            stats: ParseStats::default(),
            layout: Layout::default(),
            palette: Palette::default(),
            pending_parse: None,
            parse_queued: false,
            history: History::default(),
            dirty: false,
            crlf: false,
            bom: false,
            readable_line_length: true,
            smart_indent_list: true,
            use_tab: true,
            tab_size: 4,
            frame_samples: Sample::default(),
            parse_samples: Sample::default(),
            parse_latency: Sample::default(),
        }
    }

    /// Replaces the buffer with a file's raw text, remembering its BOM and
    /// line endings; the cursor goes to the start.
    pub fn set_text(&mut self, name: impl Into<SharedString>, raw: String, cx: &mut Context<Self>) {
        self.name = name.into();
        self.load(raw);
        self.head = 0;
        self.anchor = 0;
        self.scroll_line = 0;
        self.scroll_px = px(0.);
        self.frame_samples = Sample::default();
        self.parse_samples = Sample::default();
        self.parse_latency = Sample::default();
        self.request_parse(cx);
        cx.notify();
    }

    /// Replaces the buffer after the file changed on disk, keeping the
    /// cursor and scroll position where they were (clamped to the new text).
    pub fn reload(&mut self, raw: String, cx: &mut Context<Self>) {
        self.load(raw);
        self.head = self.clamp_offset(self.head);
        self.anchor = self.clamp_offset(self.anchor);
        self.scroll_line = self.scroll_line.min(self.line_count() - 1);
        self.request_parse(cx);
        cx.notify();
    }

    fn load(&mut self, raw: String) {
        let file = FileText::parse(&raw);
        self.crlf = file.crlf;
        self.bom = file.bom;
        self.text = file.text;
        self.marked = None;
        self.goal_x = None;
        self.dirty = false;
        self.history = History::default();
        self.rebuild_lines();
    }

    /// The bytes to write back: the buffer with the file's original line
    /// endings and byte order mark.
    pub fn file_contents(&self) -> String {
        FileText::join(self.bom, self.crlf, &self.text)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Whether `raw` (a file's bytes) is this buffer, ignoring the BOM and
    /// line endings the buffer normalizes away.
    pub fn matches_file(&self, raw: &str) -> bool {
        FileText::parse(raw).text == self.text
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn set_readable_line_length(&mut self, on: bool) {
        self.readable_line_length = on;
    }

    /// Editor settings from `app.json`: smart lists, and tabs or spaces.
    pub fn set_indent_options(&mut self, smart_indent_list: bool, use_tab: bool, tab_size: u8) {
        self.smart_indent_list = smart_indent_list;
        self.use_tab = use_tab;
        self.tab_size = tab_size;
    }

    /// One step of the self-driving stress run: types, breaks lines, pages
    /// and jumps through the document the way a user would, only faster.
    pub fn stress_step(&mut self, step: usize, window: &mut Window, cx: &mut Context<Self>) {
        match step % 16 {
            0..=5 => self.replace_text_in_range(None, "word ", window, cx),
            6 => self.newline(&Newline, window, cx),
            7 => self.page_down(&PageDown, window, cx),
            8 => self.move_down(&MoveDown, window, cx),
            9 => self.backspace(&Backspace, window, cx),
            10 => {
                let line = (step * 7919) % self.line_count();
                let at = self.line_starts[line];
                self.move_to(at, false, window, cx);
            }
            11 => self.move_up(&MoveUp, window, cx),
            12 => self.select_line_end(&SelectLineEnd, window, cx),
            13 => self.select_down(&SelectDown, window, cx),
            14 => self.delete(&Delete, window, cx),
            _ => self.undo(&Undo, window, cx),
        }
    }

    /// Summary of the stress run for the terminal.
    pub fn report(&self) -> String {
        format!(
            "frames {}: shape mean {:.2} ms, max {:.2} ms\nparses {}: parse mean {:.1} ms, max {:.1} ms\nedit-to-decoration latency mean {:.1} ms, max {:.1} ms\n{} lines, {} KB, {} spans",
            self.frame_samples.count,
            self.frame_samples.mean_ms(),
            self.frame_samples.max_ms,
            self.parse_samples.count,
            self.parse_samples.mean_ms(),
            self.parse_samples.max_ms,
            self.parse_latency.mean_ms(),
            self.parse_latency.max_ms,
            self.line_count(),
            self.stats.bytes / 1024,
            self.stats.spans
        )
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor_at(self.head)
    }

    /// The selected byte range, empty at the cursor when nothing is selected.
    pub fn selection(&self) -> Range<usize> {
        self.head.min(self.anchor)..self.head.max(self.anchor)
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Milliseconds the last frame spent shaping visible lines.
    pub fn prepaint_ms(&self) -> f64 {
        self.layout.prepaint_ms
    }

    // ----- lines ---------------------------------------------------------------

    fn rebuild_lines(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        self.line_starts
            .extend(self.text.match_indices('\n').map(|(i, _)| i + 1));
    }

    /// The line's bytes without its newline.
    fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let end = match self.line_starts.get(line + 1) {
            Some(next) => next - 1,
            None => self.text.len(),
        };
        start..end
    }

    fn line_text(&self, line: usize) -> &str {
        &self.text[self.line_range(line)]
    }

    fn cursor_at(&self, offset: usize) -> Cursor {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        Cursor {
            line,
            col: offset - self.line_starts[line],
        }
    }

    fn clamp_col(&self, line: usize, col: usize) -> usize {
        let text = self.line_text(line);
        let mut col = col.min(text.len());
        while !text.is_char_boundary(col) {
            col -= 1;
        }
        col
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        let mut at = offset.min(self.text.len());
        while !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    fn prev_char_boundary(&self, offset: usize) -> usize {
        let mut i = offset.saturating_sub(1);
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn next_char_boundary(&self, offset: usize) -> usize {
        let mut i = (offset + 1).min(self.text.len());
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    /// Word-boundary scan: whitespace, then a run of one character class.
    fn word_left(&self, from: usize) -> usize {
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

    fn word_right(&self, from: usize) -> usize {
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

    fn word_at(&self, at: usize) -> Range<usize> {
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

    // ----- shaping outside a frame ----------------------------------------------

    /// Shapes `line` the way the last frame did (same style and wrap width).
    fn shape(&self, window: &mut Window, line: usize) -> Option<WrappedLine> {
        let style = self.layout.text_style.as_ref()?;
        let spans = self.highlights.get(line).map(Vec::as_slice).unwrap_or(&[]);
        Some(shape_line(
            window,
            style,
            self.layout.font_size,
            self.layout.wrap_width,
            self.line_text(line),
            spans,
            &self.palette,
        ))
    }

    fn rows_of(&self, window: &mut Window, line: usize) -> usize {
        self.shape(window, line)
            .map(|l| l.wrap_boundaries().len() + 1)
            .unwrap_or(1)
    }

    /// The wrapped row of `col` within `line` and its x.
    fn row_and_x(&self, shaped: &WrappedLine, col: usize) -> (usize, Pixels) {
        let lh = self.layout.line_height;
        match shaped.position_for_index(col, lh) {
            Some(p) if lh > px(0.) => ((p.y / lh).round() as usize, p.x),
            _ => (0, px(0.)),
        }
    }

    fn index_at(&self, shaped: &WrappedLine, x: Pixels, row: usize) -> usize {
        let lh = self.layout.line_height;
        let rows = shaped.wrap_boundaries().len() + 1;
        let row = row.min(rows - 1);
        shaped
            .closest_index_for_position(point(x.max(px(0.)), lh * row as f32), lh)
            .unwrap_or_else(|i| i)
    }

    fn visible_rows(&self) -> usize {
        let h = self.layout.bounds.size.height;
        let lh = self.layout.line_height;
        if lh > px(0.) {
            ((h / lh).floor() as usize).max(1)
        } else {
            40
        }
    }

    // ----- scrolling -----------------------------------------------------------

    /// Brings `scroll_px` back inside `scroll_line`'s rows.
    fn normalize_scroll(&mut self, window: &mut Window) {
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
            self.scroll_px += lh * self.rows_of(window, self.scroll_line) as f32;
        }
        loop {
            let height = lh * self.rows_of(window, self.scroll_line) as f32;
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
    fn clamp_scroll_end(&mut self, window: &mut Window) {
        let lh = self.layout.line_height;
        let h = self.layout.bounds.size.height;
        if lh <= px(0.) || h <= px(0.) {
            return;
        }
        let mut filled = -self.scroll_px;
        let mut line = self.scroll_line;
        while line < self.line_count() && filled < h {
            filled += lh * self.rows_of(window, line) as f32;
            line += 1;
        }
        if filled < h {
            self.scroll_px -= h - filled;
            self.normalize_scroll(window);
        }
    }

    /// Scrolls the view down by `dy` (up when negative).
    fn scroll_by(&mut self, dy: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_px += dy;
        self.normalize_scroll(window);
        if dy > px(0.) {
            self.clamp_scroll_end(window);
        }
        cx.notify();
    }

    /// Puts `line` at the top of the view (clamped so the note's end stays
    /// at the bottom), from the scrollbar.
    fn scroll_to_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_line = line.min(self.line_count() - 1);
        self.scroll_px = px(0.);
        self.clamp_scroll_end(window);
        cx.notify();
    }

    /// Mouse down on the scrollbar: grabs the thumb, or jumps so the thumb
    /// centers on the click. Returns false when the point is off the track.
    fn scrollbar_mouse_down(
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

    fn scrollbar_drag(
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
    fn reveal_cursor(&mut self, window: &mut Window) {
        let lh = self.layout.line_height;
        let h = self.layout.bounds.size.height;
        if lh <= px(0.) || h <= px(0.) {
            self.pending_reveal = true;
            return;
        }
        let c = self.cursor_at(self.head);
        let Some(shaped) = self.shape(window, c.line) else {
            self.pending_reveal = true;
            return;
        };
        let (row, _) = self.row_and_x(&shaped, c.col);
        let row_px = lh * row as f32;
        if c.line < self.scroll_line || (c.line == self.scroll_line && row_px < self.scroll_px) {
            self.scroll_line = c.line;
            self.scroll_px = row_px;
            return;
        }
        // Walk down from the anchor; stop once past the viewport.
        let mut y = -self.scroll_px;
        let mut line = self.scroll_line;
        let mut cursor_y = None;
        while line <= c.line && y <= h {
            if line == c.line {
                cursor_y = Some(y + row_px);
                break;
            }
            y += lh * self.rows_of(window, line) as f32;
            line += 1;
        }
        if let Some(cy) = cursor_y {
            if cy + lh <= h {
                return;
            }
        }
        // Put the cursor's row at the bottom of the view.
        let mut need = h - lh;
        let mut line = c.line;
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
            px_in_line = lh * self.rows_of(window, line) as f32;
        }
    }

    // ----- editing -------------------------------------------------------------

    /// Replaces `range` with `insert`, records it for undo and moves the
    /// cursor after the insertion.
    fn edit(
        &mut self,
        range: Range<usize>,
        insert: &str,
        kind: EditKind,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let before = (self.anchor, self.head);
        let old = self.text[range.clone()].to_string();
        self.text.replace_range(range.clone(), insert);
        self.rebuild_lines();
        self.head = range.start + insert.len();
        self.anchor = self.head;
        self.marked = None;
        self.goal_x = None;
        self.dirty = true;
        self.record(
            Edit {
                at: range.start,
                old,
                new: insert.to_string(),
            },
            kind,
            before,
        );
        self.after_edit(window, cx);
    }

    fn after_edit(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        self.request_parse(cx);
        match window {
            Some(window) => self.reveal_cursor(window),
            None => self.pending_reveal = true,
        }
        cx.emit(EditorEvent::Edited);
        cx.notify();
    }

    fn record(&mut self, edit: Edit, kind: EditKind, before: (usize, usize)) {
        let after = (self.anchor, self.head);
        let now = Instant::now();
        self.history.redo.clear();
        if let Some(last) = self.history.undo.last_mut() {
            let contiguous = last.edits.last().is_some_and(|prev| match kind {
                EditKind::Typing => edit.at == prev.at + prev.new.len(),
                EditKind::Backspace => edit.at + edit.old.len() == prev.at,
                EditKind::Delete => edit.at == prev.at,
                EditKind::Other => false,
            });
            if last.kind == kind && contiguous && now.duration_since(last.at) < UNDO_GROUP {
                last.edits.push(edit);
                last.after = after;
                last.at = now;
                return;
            }
        }
        self.history.undo.push(Group {
            edits: vec![edit],
            before,
            after,
            kind,
            at: now,
        });
    }

    fn insert(&mut self, s: &str, kind: EditKind, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.selection();
        self.edit(range, s, kind, Some(window), cx);
    }

    /// Applies several non-overlapping replacements as one undo step and
    /// puts the cursor at `cursor` (a selection when `anchor` is given),
    /// both in post-edit coordinates. Ranges may come in any order.
    fn edit_many(
        &mut self,
        mut edits: Vec<(Range<usize>, String)>,
        cursor: usize,
        anchor: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if edits.is_empty() {
            return;
        }
        let before = (self.anchor, self.head);
        // Back to front, so earlier offsets stay valid while later text moves.
        edits.sort_by_key(|e| std::cmp::Reverse(e.0.start));
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
        self.head = self.clamp_offset(cursor);
        self.anchor = self.clamp_offset(anchor.unwrap_or(cursor));
        self.marked = None;
        self.goal_x = None;
        self.dirty = true;
        self.history.redo.clear();
        self.history.undo.push(Group {
            edits: recorded,
            before,
            after: (self.anchor, self.head),
            kind: EditKind::Other,
            at: Instant::now(),
        });
        self.after_edit(Some(window), cx);
    }

    /// Recomputes decorations on the background executor. One parse runs at
    /// a time; edits during a parse queue exactly one more, so fast typing
    /// still sees decorations refresh at the parser's pace instead of
    /// cancelling every pass before it finishes.
    fn request_parse(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        if self.pending_parse.is_some() {
            self.parse_queued = true;
            return;
        }
        self.parse_queued = false;
        let generation = self.generation;
        let text = self.text.clone();
        let requested_at = Instant::now();
        self.pending_parse = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let started = Instant::now();
                    let doc = _markdown::parse(&text);
                    let lines = highlight_lines(&doc);
                    let elapsed = started.elapsed();
                    let spans = lines.iter().map(Vec::len).sum();
                    let words = text.split_whitespace().count();
                    let chars = text.chars().count();
                    (lines, elapsed, spans, text.len(), words, chars)
                })
                .await;
            this.update(cx, |editor, cx| {
                let parse_ms = result.1.as_secs_f64() * 1000.0;
                // A slightly stale decoration set is better than none while typing.
                editor.highlights = Arc::new(result.0);
                editor.stats = ParseStats {
                    parse_ms,
                    spans: result.2,
                    bytes: result.3,
                    words: result.4,
                    chars: result.5,
                };
                editor.parse_samples.add(parse_ms);
                if editor.generation == generation {
                    editor
                        .parse_latency
                        .add(requested_at.elapsed().as_secs_f64() * 1000.0);
                }
                editor.pending_parse = None;
                if editor.parse_queued {
                    editor.request_parse(cx);
                }
                cx.notify();
            })
            .ok();
        }));
    }

    // ----- movement ------------------------------------------------------------

    /// Moves the cursor to `offset`, extending the selection when `select`.
    fn move_to(
        &mut self,
        offset: usize,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.head = self.clamp_offset(offset);
        if !select {
            self.anchor = self.head;
        }
        self.marked = None;
        self.reveal_cursor(window);
        cx.notify();
    }

    fn horizontal(
        &mut self,
        left: bool,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        let sel = self.selection();
        let target = if !select && sel.start != sel.end {
            // A plain arrow collapses the selection to its edge.
            if left {
                sel.start
            } else {
                sel.end
            }
        } else if left {
            self.prev_char_boundary(self.head)
        } else {
            self.next_char_boundary(self.head)
        };
        self.move_to(target, select, window, cx);
    }

    /// Moves `delta` wrapped rows up (negative) or down, keeping the goal x.
    fn vertical(
        &mut self,
        delta: isize,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let lh = self.layout.line_height;
        let c = self.cursor_at(self.head);
        let Some(shaped) = self.shape(window, c.line).filter(|_| lh > px(0.)) else {
            // No layout yet: move by buffer lines.
            let line = (c.line as isize + delta).clamp(0, self.line_count() as isize - 1) as usize;
            let col = self.clamp_col(line, c.col);
            let at = self.line_starts[line] + col;
            self.move_to(at, select, window, cx);
            return;
        };
        let (start_row, x) = self.row_and_x(&shaped, c.col);
        let goal = self.goal_x.unwrap_or(x);
        let mut line = c.line;
        let mut row = start_row;
        let mut rows = shaped.wrap_boundaries().len() + 1;
        let mut remaining = delta;
        while remaining != 0 {
            if remaining < 0 {
                if row > 0 {
                    row -= 1;
                } else if line > 0 {
                    line -= 1;
                    rows = self.rows_of(window, line);
                    row = rows - 1;
                } else {
                    break;
                }
                remaining += 1;
            } else {
                if row + 1 < rows {
                    row += 1;
                } else if line + 1 < self.line_count() {
                    line += 1;
                    rows = self.rows_of(window, line);
                    row = 0;
                } else {
                    break;
                }
                remaining -= 1;
            }
        }
        let target = if line == c.line && row == start_row {
            // Nowhere to go: the first row goes to the start, the last to the end.
            if delta < 0 {
                self.line_starts[line]
            } else {
                self.line_range(line).end
            }
        } else {
            let shaped = if line == c.line {
                shaped
            } else {
                match self.shape(window, line) {
                    Some(s) => s,
                    None => return,
                }
            };
            let col = self.clamp_col(line, self.index_at(&shaped, goal, row));
            self.line_starts[line] + col
        };
        self.move_to(target, select, window, cx);
        self.goal_x = Some(goal);
    }

    fn row_boundary(
        &mut self,
        end: bool,
        select: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        let c = self.cursor_at(self.head);
        let target = match self.shape(window, c.line) {
            Some(shaped) if self.layout.line_height > px(0.) => {
                let (row, _) = self.row_and_x(&shaped, c.col);
                let x = if end { px(f32::MAX / 4.) } else { px(0.) };
                let col = self.clamp_col(c.line, self.index_at(&shaped, x, row));
                self.line_starts[c.line] + col
            }
            _ => {
                if end {
                    self.line_range(c.line).end
                } else {
                    self.line_starts[c.line]
                }
            }
        };
        self.move_to(target, select, window, cx);
    }

    fn page(&mut self, down: bool, select: bool, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.visible_rows() as isize;
        let lh = self.layout.line_height;
        let dy = lh * rows as f32;
        self.scroll_by(if down { dy } else { -dy }, window, cx);
        self.vertical(if down { rows } else { -rows }, select, window, cx);
    }

    fn move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(true, false, window, cx);
    }

    fn move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(false, false, window, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(true, true, window, cx);
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(false, true, window, cx);
    }

    fn move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, false, window, cx);
    }

    fn move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, false, window, cx);
    }

    fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, window, cx);
    }

    fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, window, cx);
    }

    fn move_word_left(&mut self, _: &MoveWordLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        let at = self.word_left(self.head);
        self.move_to(at, false, window, cx);
    }

    fn move_word_right(&mut self, _: &MoveWordRight, window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        let at = self.word_right(self.head);
        self.move_to(at, false, window, cx);
    }

    fn select_word_left(
        &mut self,
        _: &SelectWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        let at = self.word_left(self.head);
        self.move_to(at, true, window, cx);
    }

    fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        let at = self.word_right(self.head);
        self.move_to(at, true, window, cx);
    }

    fn page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        self.page(false, false, window, cx);
    }

    fn page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        self.page(true, false, window, cx);
    }

    fn select_page_up(&mut self, _: &SelectPageUp, window: &mut Window, cx: &mut Context<Self>) {
        self.page(false, true, window, cx);
    }

    fn select_page_down(
        &mut self,
        _: &SelectPageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.page(true, true, window, cx);
    }

    fn line_start(&mut self, _: &LineStart, window: &mut Window, cx: &mut Context<Self>) {
        self.row_boundary(false, false, window, cx);
    }

    fn line_end(&mut self, _: &LineEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.row_boundary(true, false, window, cx);
    }

    fn select_line_start(
        &mut self,
        _: &SelectLineStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.row_boundary(false, true, window, cx);
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.row_boundary(true, true, window, cx);
    }

    fn document_start(&mut self, _: &DocumentStart, window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.move_to(0, false, window, cx);
    }

    fn document_end(&mut self, _: &DocumentEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.move_to(self.text.len(), false, window, cx);
    }

    fn select_document_start(
        &mut self,
        _: &SelectDocumentStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        self.move_to(0, true, window, cx);
    }

    fn select_document_end(
        &mut self,
        _: &SelectDocumentEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.goal_x = None;
        self.move_to(self.text.len(), true, window, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.anchor = 0;
        self.head = self.text.len();
        self.goal_x = None;
        cx.notify();
    }

    // ----- editing actions -------------------------------------------------------

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        let sel = self.selection();
        if sel.start != sel.end {
            self.edit(sel, "", EditKind::Other, Some(window), cx);
        } else if self.head > 0 {
            let start = self.prev_char_boundary(self.head);
            self.edit(start..self.head, "", EditKind::Backspace, Some(window), cx);
        }
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        let sel = self.selection();
        if sel.start != sel.end {
            self.edit(sel, "", EditKind::Other, Some(window), cx);
        } else if self.head < self.text.len() {
            let end = self.next_char_boundary(self.head);
            self.edit(self.head..end, "", EditKind::Delete, Some(window), cx);
        }
    }

    fn delete_word_left(
        &mut self,
        _: &DeleteWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sel = self.selection();
        let range = if sel.start != sel.end {
            sel
        } else {
            self.word_left(self.head)..self.head
        };
        if range.start != range.end {
            self.edit(range, "", EditKind::Other, Some(window), cx);
        }
    }

    fn delete_word_right(
        &mut self,
        _: &DeleteWordRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sel = self.selection();
        let range = if sel.start != sel.end {
            sel
        } else {
            self.head..self.word_right(self.head)
        };
        if range.start != range.end {
            self.edit(range, "", EditKind::Other, Some(window), cx);
        }
    }

    /// Enter: a newline that continues list and quote markers and the
    /// numbering of ordered lists, and clears or outdents an empty item.
    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        let sel = self.selection();
        if !self.smart_indent_list || sel.start != sel.end {
            self.insert("\n", EditKind::Other, window, cx);
            return;
        }
        let (edits, cursor) = newline_plan(&self.text, &self.line_starts, self.head);
        self.edit_many(edits, cursor, None, window, cx);
    }

    /// Shift+Enter: a plain newline.
    fn newline_plain(&mut self, _: &NewlinePlain, window: &mut Window, cx: &mut Context<Self>) {
        self.insert("\n", EditKind::Other, window, cx);
    }

    /// The lines a selection touches (the cursor's line when collapsed).
    fn selected_lines(&self) -> Range<usize> {
        let sel = self.selection();
        let first = self.cursor_at(sel.start).line;
        let mut last = self.cursor_at(sel.end).line;
        // A selection ending at a line start does not include that line.
        if last > first && sel.end == self.line_starts[last] {
            last -= 1;
        }
        first..last + 1
    }

    /// Tab: indents list items and selected lines by one step, and inserts
    /// the step anywhere else.
    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        let unit = lists::indent_unit(self.use_tab, self.tab_size);
        let sel = self.selection();
        let lines = self.selected_lines();
        let on_item = lists::parse_prefix(self.line_text(lines.start))
            .marker
            .is_some();
        if sel.start == sel.end && !(self.smart_indent_list && on_item) {
            self.insert(&unit, EditKind::Typing, window, cx);
            return;
        }
        let edits: Vec<(Range<usize>, String)> = lines
            .clone()
            .map(|l| {
                let at = self.line_starts[l] + lists::indent_point(self.line_text(l));
                (at..at, unit.clone())
            })
            .collect();
        let shift = |at: usize| -> usize {
            at + edits.iter().filter(|(r, _)| r.start <= at).count() * unit.len()
        };
        let (head, anchor) = (shift(self.head), shift(self.anchor));
        self.edit_many(edits, head, Some(anchor), window, cx);
    }

    /// Shift+Tab: removes one indentation step from the selected lines.
    fn outdent(&mut self, _: &Outdent, window: &mut Window, cx: &mut Context<Self>) {
        let lines = self.selected_lines();
        let edits: Vec<(Range<usize>, String)> = lines
            .clone()
            .filter_map(|l| {
                let start = self.line_starts[l];
                let r = lists::outdent_range(self.line_text(l))?;
                Some((start + r.start..start + r.end, String::new()))
            })
            .collect();
        let shift = |at: usize| -> usize {
            let removed: usize = edits
                .iter()
                .filter(|(r, _)| r.start < at)
                .map(|(r, _)| r.len().min(at - r.start))
                .sum();
            at - removed
        };
        let (head, anchor) = (shift(self.head), shift(self.anchor));
        self.edit_many(edits, head, Some(anchor), window, cx);
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        let Some(group) = self.history.undo.pop() else {
            return;
        };
        for e in group.edits.iter().rev() {
            self.text.replace_range(e.at..e.at + e.new.len(), &e.old);
        }
        self.rebuild_lines();
        (self.anchor, self.head) = group.before;
        self.history.redo.push(group);
        self.finish_history_step(window, cx);
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        let Some(group) = self.history.redo.pop() else {
            return;
        };
        for e in &group.edits {
            self.text.replace_range(e.at..e.at + e.old.len(), &e.new);
        }
        self.rebuild_lines();
        (self.anchor, self.head) = group.after;
        self.history.undo.push(group);
        self.finish_history_step(window, cx);
    }

    fn finish_history_step(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.head = self.clamp_offset(self.head);
        self.anchor = self.clamp_offset(self.anchor);
        self.marked = None;
        self.goal_x = None;
        self.dirty = true;
        self.after_edit(Some(window), cx);
    }

    /// The selection, or the whole cursor line when nothing is selected
    /// (what CodeMirror copies).
    fn copy_range(&self) -> Range<usize> {
        let sel = self.selection();
        if sel.start != sel.end {
            return sel;
        }
        let line = self.cursor_at(self.head).line;
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        start..end
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let range = self.copy_range();
        if range.start != range.end {
            cx.write_to_clipboard(ClipboardItem::new_string(self.text[range].to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.copy_range();
        if range.start != range.end {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[range.clone()].to_string(),
            ));
            self.edit(range, "", EditKind::Other, Some(window), cx);
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let text = text.replace("\r\n", "\n");
        self.insert(&text, EditKind::Other, window, cx);
    }

    // ----- mouse ---------------------------------------------------------------

    /// The buffer offset under a window point, using the last frame's layout.
    fn offset_at_point(&self, window: &mut Window, position: Point<Pixels>) -> Option<usize> {
        let layout = &self.layout;
        let lh = layout.line_height;
        if lh <= px(0.) || layout.visible.is_empty() {
            return None;
        }
        let y = position.y - layout.bounds.origin.y;
        let x = position.x - layout.content_x;
        let (vl, row) = match layout
            .visible
            .iter()
            .find(|v| y >= v.top && y < v.top + lh * v.rows as f32)
        {
            Some(v) => (*v, ((y - v.top) / lh).floor() as usize),
            None if y < layout.visible[0].top => (layout.visible[0], 0),
            None => {
                let last = *layout.visible.last()?;
                (last, last.rows - 1)
            }
        };
        let shaped = self.shape(window, vl.line)?;
        let col = self.clamp_col(vl.line, self.index_at(&shaped, x, row));
        Some(self.line_starts[vl.line] + col)
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.scrollbar_mouse_down(event.position, window, cx) {
            return;
        }
        let Some(at) = self.offset_at_point(window, event.position) else {
            return;
        };
        self.goal_x = None;
        self.marked = None;
        match event.click_count {
            2 => {
                let word = self.word_at(at);
                self.anchor = word.start;
                self.head = word.end;
            }
            n if n >= 3 => {
                let line = self.cursor_at(at).line;
                self.anchor = self.line_starts[line];
                self.head = self
                    .line_starts
                    .get(line + 1)
                    .copied()
                    .unwrap_or(self.text.len());
            }
            _ => {
                self.head = at;
                if !event.modifiers.shift {
                    self.anchor = at;
                }
                self.dragging = true;
            }
        }
        cx.notify();
    }

    fn mouse_drag(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.thumb_drag.is_some() {
            self.scrollbar_drag(position, window, cx);
            return;
        }
        if !self.dragging {
            return;
        }
        let bounds = self.layout.bounds;
        let lh = self.layout.line_height;
        if position.y < bounds.origin.y {
            self.scroll_by(-lh, window, cx);
        } else if position.y > bounds.bottom_left().y {
            self.scroll_by(lh, window, cx);
        }
        let clamped = point(
            position.x.max(bounds.origin.x).min(bounds.bottom_right().x),
            position
                .y
                .max(bounds.origin.y)
                .min(bounds.bottom_right().y - px(1.)),
        );
        if let Some(at) = self.offset_at_point(window, clamped) {
            self.head = at;
            cx.notify();
        }
    }

    // ----- utf-16 helpers for the input handler (cursor line only) -----------

    fn utf16_to_byte(line: &str, utf16: usize) -> usize {
        let mut units = 0;
        for (i, c) in line.char_indices() {
            if units >= utf16 {
                return i;
            }
            units += c.len_utf16();
        }
        line.len()
    }

    fn byte_to_utf16(line: &str, byte: usize) -> usize {
        line[..byte.min(line.len())]
            .chars()
            .map(char::len_utf16)
            .sum()
    }

    /// A line-relative UTF-16 range on the cursor's line, as an absolute byte range.
    fn line_relative(&self, range_utf16: Range<usize>) -> Range<usize> {
        let c = self.cursor_at(self.head);
        let line_start = self.line_starts[c.line];
        let line = self.line_text(c.line);
        line_start + Self::utf16_to_byte(line, range_utf16.start)
            ..line_start + Self::utf16_to_byte(line, range_utf16.end)
    }
}

/// The edits Enter makes at `head` with smart lists on: the newline and
/// continued prefix (or the clearing of an empty item), plus the renumbering
/// of the ordered items that follow. Returns them with the cursor's new offset.
fn newline_plan(
    text: &str,
    line_starts: &[usize],
    head: usize,
) -> (Vec<(Range<usize>, String)>, usize) {
    let line_ix = match line_starts.binary_search(&head) {
        Ok(i) => i,
        Err(i) => i - 1,
    };
    let line_start = line_starts[line_ix];
    let line_end = line_starts
        .get(line_ix + 1)
        .map(|n| n - 1)
        .unwrap_or(text.len());
    let line = &text[line_start..line_end];
    let col = head - line_start;
    match lists::continuation(line, col) {
        Continuation::Insert(prefix) => {
            let mut edits = vec![(head..head, format!("\n{prefix}"))];
            let cursor = head + 1 + prefix.len();
            let p = lists::parse_prefix(line);
            if let Some(lists::Marker::Ordered { number, .. }) = p.marker {
                if col >= p.content {
                    let following = line_starts[line_ix + 1..]
                        .iter()
                        .enumerate()
                        .map(|(i, &s)| {
                            let e = line_starts
                                .get(line_ix + 2 + i)
                                .map(|n| n - 1)
                                .unwrap_or(text.len());
                            &text[s..e]
                        });
                    for (ix, range, n) in lists::renumber_following(following, line, number + 2) {
                        let at = line_starts[line_ix + 1 + ix];
                        edits.push((at + range.start..at + range.end, n.to_string()));
                    }
                }
            }
            (edits, cursor)
        }
        Continuation::Clear { range, with } => {
            let abs = line_start + range.start..line_start + range.end;
            let cursor = abs.start + with.len() + head.saturating_sub(abs.end);
            (vec![(abs, with)], cursor)
        }
    }
}

/// A file's text split from the bytes the buffer normalizes away.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileText {
    bom: bool,
    /// The file used `\r\n` (any mixed endings become the dominant one).
    crlf: bool,
    /// The body with `\n` line endings.
    text: String,
}

impl FileText {
    fn parse(raw: &str) -> FileText {
        let (bom, body) = match raw.strip_prefix('\u{feff}') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };
        let crlf = body.contains("\r\n");
        let text = if crlf {
            body.replace("\r\n", "\n")
        } else {
            body.to_string()
        };
        FileText { bom, crlf, text }
    }

    fn join(bom: bool, crlf: bool, text: &str) -> String {
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

/// 0 for whitespace, 1 for word characters, 2 for punctuation.
fn char_class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let line = self.line_text(self.cursor().line);
        let start = Self::utf16_to_byte(line, range_utf16.start);
        let end = Self::utf16_to_byte(line, range_utf16.end);
        adjusted_range.replace(Self::byte_to_utf16(line, start)..Self::byte_to_utf16(line, end));
        Some(line[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let c = self.cursor();
        let range = self.line_range(c.line);
        let line = &self.text[range.clone()];
        let sel = self.selection();
        let (start, end) = if sel.start >= range.start && sel.end <= range.end {
            (sel.start - range.start, sel.end - range.start)
        } else {
            (c.col, c.col)
        };
        Some(UTF16Selection {
            range: Self::byte_to_utf16(line, start)..Self::byte_to_utf16(line, end),
            reversed: self.head < self.anchor,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let c = self.cursor();
        let range = self.line_range(c.line);
        let marked = self.marked.clone()?;
        if marked.start < range.start || marked.end > range.end {
            return None;
        }
        let line = &self.text[range.clone()];
        Some(
            Self::byte_to_utf16(line, marked.start - range.start)
                ..Self::byte_to_utf16(line, marked.end - range.start),
        )
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| self.line_relative(r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        let kind = if new_text.contains('\n') {
            EditKind::Other
        } else {
            EditKind::Typing
        };
        self.edit(range, new_text, kind, Some(window), cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| self.line_relative(r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        self.edit(range.clone(), new_text, EditKind::Typing, Some(window), cx);
        let marked = range.start..range.start + new_text.len();
        if let Some(sel) = new_selected_range {
            let at = marked.start + Self::utf16_to_byte(new_text, sel.end);
            self.head = self.clamp_offset(at);
            self.anchor = self.head;
        }
        self.marked = Some(marked);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let shaped = self.layout.cursor_line.clone()?;
        let line = self.line_text(self.cursor().line);
        let start = Self::utf16_to_byte(line, range_utf16.start);
        let end = Self::utf16_to_byte(line, range_utf16.end);
        let lh = self.layout.line_height;
        let (row, x0) = self.row_and_x(&shaped, start);
        let (_, x1) = self.row_and_x(&shaped, end);
        let y = self.layout.cursor_top + lh * row as f32;
        let x = self.layout.content_x;
        Some(Bounds::from_corners(
            point(x + x0, y),
            point(x + x1.max(x0), y + lh),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

impl Render for Editor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        self.palette = Palette::from_theme(&theme);
        let base = theme.base_font_size;
        div()
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .size_full()
            .font_family(theme.text_font.clone())
            .text_size(px(base))
            .line_height(px(base * 1.5))
            .text_color(theme.text)
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_word_left))
            .on_action(cx.listener(Self::move_word_right))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::line_start))
            .on_action(cx.listener(Self::line_end))
            .on_action(cx.listener(Self::document_start))
            .on_action(cx.listener(Self::document_end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::select_page_up))
            .on_action(cx.listener(Self::select_page_down))
            .on_action(cx.listener(Self::select_document_start))
            .on_action(cx.listener(Self::select_document_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_word_right))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::newline_plain))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::outdent))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .child(EditorElement {
                editor: cx.entity(),
            })
    }
}

// ---------------------------------------------------------------------------
// Element

/// Styles one flattened highlight segment on top of the base text style.
fn run_for(kinds: &[HighlightKind], base: &TextStyle, len: usize, p: &Palette) -> TextRun {
    let mut font = base.font();
    let mut color: Hsla = base.color;
    let mut background_color = None;
    let mut underline = None;
    let mut strikethrough = None;
    for kind in kinds {
        match kind {
            HighlightKind::Heading(_) => {
                font.weight = FontWeight::BOLD;
                color = p.accent;
            }
            HighlightKind::Wikilink | HighlightKind::Embed | HighlightKind::Link => {
                color = p.link;
                underline = Some(UnderlineStyle {
                    color: Some(p.link),
                    thickness: px(1.),
                    wavy: false,
                });
            }
            HighlightKind::Url | HighlightKind::Footnote => color = p.link,
            HighlightKind::Tag => color = p.tag,
            HighlightKind::Code | HighlightKind::CodeBlock => {
                font = Font {
                    family: p.mono.clone(),
                    ..font
                };
                background_color = Some(p.code_bg);
            }
            HighlightKind::Emphasis => font.style = FontStyle::Italic,
            HighlightKind::Strong => font.weight = FontWeight::BOLD,
            HighlightKind::Strikethrough => {
                strikethrough = Some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(color),
                });
            }
            HighlightKind::Highlight => background_color = Some(p.highlight_bg),
            HighlightKind::Comment
            | HighlightKind::Frontmatter
            | HighlightKind::BlockId
            | HighlightKind::ThematicBreak
            | HighlightKind::Html => color = p.muted,
            HighlightKind::Math => color = p.accent,
            HighlightKind::Callout | HighlightKind::ListMarker | HighlightKind::Task => {
                color = p.accent;
            }
        }
    }
    TextRun {
        len,
        font,
        color,
        background_color,
        underline,
        strikethrough,
    }
}

/// Builds runs covering the whole line, plain text between highlighted segments.
fn runs_for_line(
    text: &str,
    spans: &[HighlightSpan],
    base: &TextStyle,
    p: &Palette,
) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut at = 0;
    for (range, kinds) in flatten(spans) {
        let start = range.start.min(text.len());
        let end = range.end.min(text.len());
        if start > at {
            runs.push(base.to_run(start - at));
        }
        if end > start {
            runs.push(run_for(&kinds, base, end - start, p));
        }
        at = at.max(end);
    }
    if at < text.len() {
        runs.push(base.to_run(text.len() - at));
    }
    if runs.is_empty() {
        runs.push(base.to_run(0));
    }
    runs
}

fn shape_line(
    window: &mut Window,
    style: &TextStyle,
    font_size: Pixels,
    wrap_width: Option<Pixels>,
    text: &str,
    spans: &[HighlightSpan],
    p: &Palette,
) -> WrappedLine {
    let display = text.trim_end_matches('\r');
    let runs = runs_for_line(display, spans, style, p);
    window
        .text_system()
        .shape_text(
            SharedString::from(display.to_string()),
            font_size,
            &runs,
            wrap_width,
            None,
        )
        .ok()
        .and_then(|lines| lines.into_iter().next())
        .unwrap_or_default()
}

struct EditorElement {
    editor: Entity<Editor>,
}

struct Prepaint {
    lines: Vec<(VisibleLine, WrappedLine)>,
    line_height: Pixels,
    font_size: Pixels,
    text_style: TextStyle,
    wrap_width: Option<Pixels>,
    content_x: Pixels,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    cursor_line: Option<Arc<WrappedLine>>,
    cursor_top: Pixels,
    /// `scroll_px` was past the anchor line's rows; the clamped value.
    scroll_px: Pixels,
    scrollbar: Option<Scrollbar>,
    thumb_dragging: bool,
    prepaint_ms: f64,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let started = Instant::now();
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let lh = window.line_height();
        let editor = self.editor.read(cx);
        let palette = editor.palette.clone();
        let highlights = editor.highlights.clone();
        let count = editor.line_count();
        let scroll_line = editor.scroll_line.min(count - 1);
        let mut scroll_px = editor.scroll_px;
        let selection = editor.selection();
        let head = editor.head;
        let focused = editor.focus_handle.is_focused(window);
        let thumb_dragging = editor.thumb_drag.is_some();

        let available = (bounds.size.width - px(GUTTER * 2.)).max(px(50.));
        let content_width = if editor.readable_line_length {
            available.min(px(READABLE_WIDTH))
        } else {
            available
        };
        let content_x = bounds.origin.x + (bounds.size.width - content_width) / 2.;
        let wrap_width = Some(content_width);

        let mut lines = Vec::new();
        let mut selection_quads = Vec::new();
        let mut cursor_quad = None;
        let mut cursor_line = None;
        let mut cursor_top = px(0.);
        let mut y = bounds.origin.y - scroll_px;
        let mut line = scroll_line;
        while line < count && y < bounds.bottom_left().y {
            let text = editor.line_text(line);
            let spans = highlights.get(line).map(Vec::as_slice).unwrap_or(&[]);
            let shaped = shape_line(
                window,
                &text_style,
                font_size,
                wrap_width,
                text,
                spans,
                &palette,
            );
            let rows = shaped.wrap_boundaries().len() + 1;
            if line == scroll_line {
                let height = lh * rows as f32;
                if scroll_px >= height {
                    scroll_px = (height - lh).max(px(0.));
                    y = bounds.origin.y - scroll_px;
                }
            }
            let range = editor.line_range(line);
            if selection.start != selection.end
                && selection.start <= range.end
                && selection.end > range.start
            {
                let s = selection.start.max(range.start) - range.start;
                let e = selection.end.min(range.end) - range.start;
                let ps = shaped.position_for_index(s, lh).unwrap_or_default();
                let pe = shaped.position_for_index(e, lh).unwrap_or_default();
                let rs = (ps.y / lh).round() as usize;
                let re = (pe.y / lh).round() as usize;
                for r in rs..=re {
                    let x0 = if r == rs { ps.x } else { px(0.) };
                    let mut x1 = if r == re { pe.x } else { content_width };
                    if r == re && selection.end > range.end {
                        x1 += font_size * 0.4;
                    }
                    if x1 > x0 {
                        selection_quads.push(fill(
                            Bounds::new(
                                point(content_x + x0, y + lh * r as f32),
                                size(x1 - x0, lh),
                            ),
                            palette.selection,
                        ));
                    }
                }
            }
            if head >= range.start && head <= range.end {
                let p = shaped
                    .position_for_index(head - range.start, lh)
                    .unwrap_or_default();
                cursor_quad = Some(fill(
                    Bounds::new(point(content_x + p.x, y + p.y), size(px(2.), lh)),
                    palette.text,
                ));
                cursor_top = y;
            }
            lines.push((
                VisibleLine {
                    line,
                    top: y - bounds.origin.y,
                    rows,
                },
                shaped,
            ));
            y += lh * rows as f32;
            line += 1;
        }
        let cursor_line_ix = editor.cursor().line;
        if lines.iter().any(|(v, _)| v.line == cursor_line_ix) {
            // The input handler needs the cursor line's layout between
            // frames; shaping it again hits GPUI's per-frame layout cache.
            cursor_line = Some(Arc::new(shape_line(
                window,
                &text_style,
                font_size,
                wrap_width,
                editor.line_text(cursor_line_ix),
                highlights
                    .get(cursor_line_ix)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &palette,
            )));
        }
        if !focused {
            cursor_quad = None;
        }
        let fits =
            scroll_line == 0 && scroll_px == px(0.) && line >= count && y <= bounds.bottom_left().y;
        let track = Bounds::new(
            point(
                bounds.bottom_right().x - px(SCROLLBAR_WIDTH),
                bounds.origin.y,
            ),
            size(px(SCROLLBAR_WIDTH), bounds.size.height),
        );
        let scrollbar = Scrollbar::layout(track, count, scroll_line, lines.len(), fits);
        Prepaint {
            lines,
            line_height: lh,
            font_size,
            text_style,
            wrap_width,
            content_x,
            selection: selection_quads,
            cursor: cursor_quad,
            cursor_line,
            cursor_top,
            scroll_px,
            scrollbar,
            thumb_dragging,
            prepaint_ms: started.elapsed().as_secs_f64() * 1000.0,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );
        let line_height = prepaint.line_height;
        let content_x = prepaint.content_x;
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for quad in prepaint.selection.drain(..) {
                window.paint_quad(quad);
            }
            for (visible, line) in prepaint.lines.iter() {
                let origin = point(content_x, bounds.origin.y + visible.top);
                let _ = line.paint(origin, line_height, TextAlign::Left, None, window, cx);
            }
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
            if let Some(bar) = prepaint.scrollbar {
                let mut color = self.editor.read(cx).palette.muted;
                color.a = if prepaint.thumb_dragging { 0.6 } else { 0.35 };
                window.paint_quad(fill(bar.thumb, color).corner_radii(px(3.)));
            }
        });

        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble
                && event.button == MouseButton::Left
                && bounds.contains(&event.position)
            {
                editor.update(cx, |editor, cx| editor.mouse_down(event, window, cx));
            }
        });
        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && event.pressed_button == Some(MouseButton::Left) {
                editor.update(cx, |editor, cx| {
                    editor.mouse_drag(event.position, window, cx)
                });
            }
        });
        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                editor.update(cx, |editor, cx| {
                    editor.dragging = false;
                    if editor.thumb_drag.take().is_some() {
                        cx.notify();
                    }
                });
            }
        });
        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(line_height).y;
                editor.update(cx, |editor, cx| editor.scroll_by(-delta, window, cx));
            }
        });

        let layout = Layout {
            bounds,
            line_height,
            font_size: prepaint.font_size,
            text_style: Some(prepaint.text_style.clone()),
            wrap_width: prepaint.wrap_width,
            content_x,
            visible: prepaint.lines.iter().map(|(v, _)| *v).collect(),
            cursor_line: prepaint.cursor_line.take(),
            cursor_top: prepaint.cursor_top,
            scrollbar: prepaint.scrollbar,
            prepaint_ms: prepaint.prepaint_ms,
        };
        let scroll_px = prepaint.scroll_px;
        self.editor.update(cx, |editor, cx| {
            editor.frame_samples.add(layout.prepaint_ms);
            editor.layout = layout;
            editor.scroll_px = scroll_px;
            if editor.pending_reveal {
                editor.pending_reveal = false;
                editor.reveal_cursor(window);
                cx.notify();
            }
        });
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

    fn starts(text: &str) -> Vec<usize> {
        std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect()
    }

    fn apply(text: &str, head: usize) -> (String, usize) {
        let (mut edits, cursor) = newline_plan(text, &starts(text), head);
        edits.sort_by_key(|e| std::cmp::Reverse(e.0.start));
        let mut out = text.to_string();
        for (r, s) in edits {
            out.replace_range(r, &s);
        }
        (out, cursor)
    }

    #[test]
    fn enter_continues_items_and_renumbers_the_rest() {
        assert_eq!(apply("- a", 3), ("- a\n- ".into(), 6));
        assert_eq!(
            apply("1. a\n2. b\n3. c", 4),
            ("1. a\n2. \n3. b\n4. c".into(), 8)
        );
        // Splitting an item carries the rest of the text to the new one.
        assert_eq!(apply("- ab", 3), ("- a\n- b".into(), 6));
        // An empty item clears; a nested one outdents and keeps the cursor.
        assert_eq!(apply("- a\n- ", 6), ("- a\n".into(), 4));
        assert_eq!(apply("- a\n\t- ", 7), ("- a\n- ".into(), 6));
        assert_eq!(apply("> q", 3), ("> q\n> ".into(), 6));
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
    fn file_text_normalizes_mixed_endings_to_the_dominant_one() {
        let f = FileText::parse("a\r\nb\nc");
        assert_eq!(f.text, "a\nb\nc");
        assert!(f.crlf);
        assert_eq!(FileText::join(f.bom, f.crlf, &f.text), "a\r\nb\r\nc");
    }
}
