//! Editor prototype for the first spike gate: a 50k-line note with wikilink
//! and heading decorations that scrolls and types without stalls.
//!
//! The buffer is one `String` with a line-start index. Only the visible lines
//! are shaped and painted each frame by a custom element; decorations come
//! from the shared parser (`_markdown::highlight`), recomputed on a
//! background thread after every edit and swapped in when they arrive, so
//! typing never waits for a parse. Text input goes through GPUI's input
//! handler (so IME and dead keys work) scoped to the cursor's line; navigation
//! keys are actions bound in [`bind_keys`].

use gpui::{
    actions, div, fill, point, prelude::*, px, relative, rgb, size, App, Bounds, Context,
    DispatchPhase, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, FontStyle, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    KeyBinding, LayoutId, MouseButton, MouseDownEvent, PaintQuad, Pixels, Point, ScrollWheelEvent,
    ShapedLine, SharedString, StrikethroughStyle, Style, Task, TextAlign, TextRun, TextStyle,
    UTF16Selection, UnderlineStyle, Window,
};
use _markdown::highlight::{flatten, highlight_lines, HighlightKind, HighlightSpan};
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

actions!(
    editor,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        LineStart,
        LineEnd,
        PageUp,
        PageDown,
        DocumentStart,
        DocumentEnd,
        Backspace,
        Delete,
        Newline,
        Tab,
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
        KeyBinding::new("home", LineStart, ctx),
        KeyBinding::new("end", LineEnd, ctx),
        KeyBinding::new("pageup", PageUp, ctx),
        KeyBinding::new("pagedown", PageDown, ctx),
        KeyBinding::new("ctrl-home", DocumentStart, ctx),
        KeyBinding::new("ctrl-end", DocumentEnd, ctx),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("enter", Newline, ctx),
        KeyBinding::new("tab", Tab, ctx),
    ]);
}

const GUTTER: f32 = 16.0;
const COLOR_TEXT: u32 = 0xdcddde;
const COLOR_MUTED: u32 = 0x8a8a8a;
const COLOR_ACCENT: u32 = 0x7f6df2;
const COLOR_LINK: u32 = 0x8ab4f8;
const COLOR_TAG: u32 = 0x56b6c2;
const COLOR_MATH: u32 = 0xd19a66;
const COLOR_CODE_BG: u32 = 0x2a2a2a;
const COLOR_HIGHLIGHT_BG: u32 = 0x5a4d1a;
const COLOR_CURSOR: u32 = 0xe6e6e6;
const COLOR_CURSOR_LINE: u32 = 0x242424;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub line: usize,
    /// Byte offset within the line.
    pub col: usize,
}

/// Timing of the last decoration pass, shown in the status bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseStats {
    pub parse_ms: f64,
    pub spans: usize,
    pub bytes: usize,
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

/// Geometry recorded at paint time so input handling can map points to text.
#[derive(Debug, Clone, Default)]
struct Layout {
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    font_size: Pixels,
    text_style: Option<TextStyle>,
    cursor_line: Option<ShapedLine>,
    /// Time spent shaping the visible lines in the last frame.
    prepaint_ms: f64,
}

pub struct Editor {
    focus_handle: FocusHandle,
    pub name: SharedString,
    text: String,
    line_starts: Vec<usize>,
    cursor: Cursor,
    scroll_top: Pixels,
    highlights: Arc<Vec<Vec<HighlightSpan>>>,
    generation: u64,
    pub stats: ParseStats,
    marked: Option<Range<usize>>,
    layout: Layout,
    pending_parse: Option<Task<()>>,
    /// An edit arrived while a parse was running; parse again when it lands.
    parse_queued: bool,
    pub dirty: bool,
    /// Shaping time per painted frame since the note was opened.
    pub frame_samples: Sample,
    /// Parse time per completed decoration pass since the note was opened.
    pub parse_samples: Sample,
    /// Edit-to-decoration latency, including waiting for the background thread.
    pub parse_latency: Sample,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>) -> Editor {
        Editor {
            focus_handle: cx.focus_handle(),
            name: "untitled".into(),
            text: String::new(),
            line_starts: vec![0],
            cursor: Cursor::default(),
            scroll_top: px(0.),
            highlights: Arc::new(vec![Vec::new()]),
            generation: 0,
            stats: ParseStats::default(),
            marked: None,
            layout: Layout::default(),
            pending_parse: None,
            parse_queued: false,
            dirty: false,
            frame_samples: Sample::default(),
            parse_samples: Sample::default(),
            parse_latency: Sample::default(),
        }
    }

    /// Replaces the buffer with a note's text.
    pub fn set_text(
        &mut self,
        name: impl Into<SharedString>,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.name = name.into();
        self.text = text.replace("\r\n", "\n");
        self.cursor = Cursor::default();
        self.scroll_top = px(0.);
        self.marked = None;
        self.dirty = false;
        self.frame_samples = Sample::default();
        self.parse_samples = Sample::default();
        self.parse_latency = Sample::default();
        self.rebuild_lines();
        self.request_parse(cx);
        cx.notify();
    }

    /// One step of the self-driving stress run: types, breaks lines, pages
    /// and jumps through the document the way a user would, only faster.
    pub fn stress_step(&mut self, step: usize, window: &mut Window, cx: &mut Context<Self>) {
        match step % 12 {
            0..=5 => self.replace_text_in_range(None, "word ", window, cx),
            6 => self.newline(&Newline, window, cx),
            7 => self.page_down(&PageDown, window, cx),
            8 => self.move_down(&MoveDown, window, cx),
            9 => self.backspace(&Backspace, window, cx),
            10 => {
                let line = (step * 7919) % self.line_count();
                self.set_cursor(Cursor { line, col: 0 }, cx);
            }
            _ => self.move_up(&MoveUp, window, cx),
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
        self.cursor
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

    fn offset(&self, cursor: Cursor) -> usize {
        self.line_starts[cursor.line] + cursor.col
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

    fn visible_rows(&self) -> usize {
        let h = self.layout.bounds.size.height;
        let lh = self.layout.line_height;
        if lh > px(0.) {
            ((h / lh).floor() as usize).max(1)
        } else {
            40
        }
    }

    // ----- editing -------------------------------------------------------------

    fn replace(&mut self, range: Range<usize>, insert: &str, cx: &mut Context<Self>) {
        self.text.replace_range(range.clone(), insert);
        let offset = range.start + insert.len();
        self.rebuild_lines();
        self.cursor = self.cursor_at(offset);
        self.dirty = true;
        self.marked = None;
        self.request_parse(cx);
        self.scroll_to_cursor();
        cx.notify();
    }

    fn insert(&mut self, s: &str, cx: &mut Context<Self>) {
        let at = self.offset(self.cursor);
        self.replace(at..at, s, cx);
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
                    (lines, elapsed, spans, text.len())
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

    fn scroll_to_cursor(&mut self) {
        let lh = self.layout.line_height;
        if lh <= px(0.) {
            return;
        }
        let rows = self.visible_rows();
        let top = lh * self.cursor.line as f32;
        let bottom = top + lh;
        if top < self.scroll_top {
            self.scroll_top = top;
        } else if bottom > self.scroll_top + lh * rows as f32 {
            self.scroll_top = bottom - lh * rows as f32;
        }
    }

    fn set_cursor(&mut self, cursor: Cursor, cx: &mut Context<Self>) {
        self.cursor = cursor;
        self.marked = None;
        self.scroll_to_cursor();
        cx.notify();
    }

    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.offset(self.cursor);
        if at > 0 {
            let c = self.cursor_at(self.prev_char_boundary(at));
            self.set_cursor(c, cx);
        }
    }

    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.offset(self.cursor);
        if at < self.text.len() {
            let c = self.cursor_at(self.next_char_boundary(at));
            self.set_cursor(c, cx);
        }
    }

    fn move_lines(&mut self, delta: isize, cx: &mut Context<Self>) {
        let line =
            (self.cursor.line as isize + delta).clamp(0, self.line_count() as isize - 1) as usize;
        let col = self.clamp_col(line, self.cursor.col);
        self.set_cursor(Cursor { line, col }, cx);
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_lines(-1, cx);
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_lines(1, cx);
    }

    fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        let rows = self.visible_rows() as isize;
        self.scroll_top = (self.scroll_top - self.layout.line_height * rows as f32).max(px(0.));
        self.move_lines(-rows, cx);
    }

    fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        let rows = self.visible_rows() as isize;
        self.scroll_top += self.layout.line_height * rows as f32;
        self.move_lines(rows, cx);
    }

    fn line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
        let c = Cursor {
            line: self.cursor.line,
            col: 0,
        };
        self.set_cursor(c, cx);
    }

    fn line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let c = Cursor {
            line: self.cursor.line,
            col: self.line_text(self.cursor.line).len(),
        };
        self.set_cursor(c, cx);
    }

    fn document_start(&mut self, _: &DocumentStart, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(Cursor::default(), cx);
    }

    fn document_end(&mut self, _: &DocumentEnd, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor_at(self.text.len());
        self.set_cursor(c, cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.offset(self.cursor);
        if at > 0 {
            let start = self.prev_char_boundary(at);
            self.replace(start..at, "", cx);
        }
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.offset(self.cursor);
        if at < self.text.len() {
            let end = self.next_char_boundary(at);
            self.replace(at..end, "", cx);
        }
    }

    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        self.insert("\n", cx);
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        self.insert("\t", cx);
    }

    fn scroll_by(&mut self, dy: Pixels, cx: &mut Context<Self>) {
        let max = (self.layout.line_height * self.line_count() as f32
            - self.layout.bounds.size.height)
            .max(px(0.));
        self.scroll_top = (self.scroll_top - dy).clamp(px(0.), max);
        cx.notify();
    }

    fn click(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let layout = self.layout.clone();
        if layout.line_height <= px(0.) {
            return;
        }
        let y = position.y - layout.bounds.origin.y + self.scroll_top;
        let line = ((y / layout.line_height).floor() as usize).min(self.line_count() - 1);
        let x = position.x - layout.bounds.origin.x - px(GUTTER);
        let col = match &layout.text_style {
            Some(style) => {
                let shaped = shape_line(
                    window,
                    style,
                    layout.font_size,
                    self.line_text(line),
                    &self.highlights.get(line).cloned().unwrap_or_default(),
                );
                shaped.closest_index_for_x(x)
            }
            None => 0,
        };
        let col = self.clamp_col(line, col);
        window.focus(&self.focus_handle, cx);
        self.set_cursor(Cursor { line, col }, cx);
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
        let line = self.line_text(self.cursor.line);
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
        let line = self.line_text(self.cursor.line);
        let col = Self::byte_to_utf16(line, self.cursor.col);
        Some(UTF16Selection {
            range: col..col,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let line = self.line_text(self.cursor.line);
        self.marked
            .as_ref()
            .map(|r| Self::byte_to_utf16(line, r.start)..Self::byte_to_utf16(line, r.end))
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
        let line_start = self.line_starts[self.cursor.line];
        let line = self.line_text(self.cursor.line);
        let range = range_utf16
            .map(|r| Self::utf16_to_byte(line, r.start)..Self::utf16_to_byte(line, r.end))
            .or_else(|| self.marked.clone())
            .unwrap_or(self.cursor.col..self.cursor.col);
        self.replace(
            line_start + range.start..line_start + range.end,
            new_text,
            cx,
        );
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_start = self.line_starts[self.cursor.line];
        let line = self.line_text(self.cursor.line);
        let range = range_utf16
            .map(|r| Self::utf16_to_byte(line, r.start)..Self::utf16_to_byte(line, r.end))
            .or_else(|| self.marked.clone())
            .unwrap_or(self.cursor.col..self.cursor.col);
        self.replace(
            line_start + range.start..line_start + range.end,
            new_text,
            cx,
        );
        let marked = range.start..range.start + new_text.len();
        if let Some(sel) = new_selected_range {
            let col = marked.start + Self::utf16_to_byte(new_text, sel.end);
            self.cursor.col = self.clamp_col(self.cursor.line, col);
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
        let shaped = self.layout.cursor_line.as_ref()?;
        let line = self.line_text(self.cursor.line);
        let start = Self::utf16_to_byte(line, range_utf16.start);
        let end = Self::utf16_to_byte(line, range_utf16.end);
        let lh = self.layout.line_height;
        let y = element_bounds.origin.y + lh * self.cursor.line as f32 - self.scroll_top;
        Some(Bounds::from_corners(
            point(
                element_bounds.origin.x + px(GUTTER) + shaped.x_for_index(start),
                y,
            ),
            point(
                element_bounds.origin.x + px(GUTTER) + shaped.x_for_index(end),
                y + lh,
            ),
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
        div()
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .size_full()
            .font_family("DejaVu Sans Mono")
            .text_size(px(14.))
            .line_height(px(22.))
            .text_color(rgb(COLOR_TEXT))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::line_start))
            .on_action(cx.listener(Self::line_end))
            .on_action(cx.listener(Self::document_start))
            .on_action(cx.listener(Self::document_end))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::tab))
            .child(EditorElement {
                editor: cx.entity(),
            })
    }
}

// ---------------------------------------------------------------------------
// Element

/// Styles one flattened highlight segment on top of the base text style.
fn run_for(kinds: &[HighlightKind], base: &TextStyle, len: usize) -> TextRun {
    let mut font = base.font();
    let mut color: Hsla = base.color;
    let mut background_color = None;
    let mut underline = None;
    let mut strikethrough = None;
    for kind in kinds {
        match kind {
            HighlightKind::Heading(_) => {
                font.weight = FontWeight::BOLD;
                color = rgb(COLOR_ACCENT).into();
            }
            HighlightKind::Wikilink | HighlightKind::Embed | HighlightKind::Link => {
                color = rgb(COLOR_LINK).into();
                underline = Some(UnderlineStyle {
                    color: Some(rgb(COLOR_LINK).into()),
                    thickness: px(1.),
                    wavy: false,
                });
            }
            HighlightKind::Url | HighlightKind::Footnote => color = rgb(COLOR_LINK).into(),
            HighlightKind::Tag => color = rgb(COLOR_TAG).into(),
            HighlightKind::Code | HighlightKind::CodeBlock => {
                background_color = Some(rgb(COLOR_CODE_BG).into());
            }
            HighlightKind::Emphasis => font.style = FontStyle::Italic,
            HighlightKind::Strong => font.weight = FontWeight::BOLD,
            HighlightKind::Strikethrough => {
                strikethrough = Some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(color),
                });
            }
            HighlightKind::Highlight => background_color = Some(rgb(COLOR_HIGHLIGHT_BG).into()),
            HighlightKind::Comment
            | HighlightKind::Frontmatter
            | HighlightKind::BlockId
            | HighlightKind::ThematicBreak
            | HighlightKind::Html => color = rgb(COLOR_MUTED).into(),
            HighlightKind::Math => color = rgb(COLOR_MATH).into(),
            HighlightKind::Callout | HighlightKind::ListMarker | HighlightKind::Task => {
                color = rgb(COLOR_ACCENT).into();
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
fn runs_for_line(text: &str, spans: &[HighlightSpan], base: &TextStyle) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut at = 0;
    for (range, kinds) in flatten(spans) {
        let start = range.start.min(text.len());
        let end = range.end.min(text.len());
        if start > at {
            runs.push(base.to_run(start - at));
        }
        if end > start {
            runs.push(run_for(&kinds, base, end - start));
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
    text: &str,
    spans: &[HighlightSpan],
) -> ShapedLine {
    let display = text.trim_end_matches('\r');
    let runs = runs_for_line(display, spans, style);
    window.text_system().shape_line(
        SharedString::from(display.to_string()),
        font_size,
        &runs,
        None,
    )
}

struct EditorElement {
    editor: Entity<Editor>,
}

struct Prepaint {
    lines: Vec<(usize, ShapedLine)>,
    first_row: usize,
    offset_y: Pixels,
    line_height: Pixels,
    font_size: Pixels,
    text_style: TextStyle,
    cursor: Option<PaintQuad>,
    cursor_line_bg: Option<PaintQuad>,
    cursor_line: Option<ShapedLine>,
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
        let line_height = window.line_height();
        let editor = self.editor.read(cx);
        let highlights = editor.highlights.clone();
        let scroll_top = editor.scroll_top;
        let cursor = editor.cursor;
        let count = editor.line_count();

        let first_row = ((scroll_top / line_height).floor() as usize).min(count.saturating_sub(1));
        let offset_y = line_height * first_row as f32 - scroll_top;
        let rows = (bounds.size.height / line_height).ceil() as usize + 1;
        let last_row = (first_row + rows).min(count);

        let mut lines = Vec::with_capacity(rows);
        let mut cursor_quad = None;
        let mut cursor_line_bg = None;
        let mut cursor_line = None;
        for row in first_row..last_row {
            let text = editor.line_text(row);
            let spans = highlights.get(row).map(Vec::as_slice).unwrap_or(&[]);
            let shaped = shape_line(window, &text_style, font_size, text, spans);
            let y = bounds.origin.y + offset_y + line_height * (row - first_row) as f32;
            if row == cursor.line {
                let x = bounds.origin.x + px(GUTTER) + shaped.x_for_index(cursor.col);
                cursor_quad = Some(fill(
                    Bounds::new(point(x, y), size(px(2.), line_height)),
                    rgb(COLOR_CURSOR),
                ));
                cursor_line_bg = Some(fill(
                    Bounds::new(
                        point(bounds.origin.x, y),
                        size(bounds.size.width, line_height),
                    ),
                    rgb(COLOR_CURSOR_LINE),
                ));
                cursor_line = Some(shaped.clone());
            }
            lines.push((row, shaped));
        }
        Prepaint {
            lines,
            first_row,
            offset_y,
            line_height,
            font_size,
            text_style,
            cursor: cursor_quad,
            cursor_line_bg,
            cursor_line,
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
        let focused = focus_handle.is_focused(window);
        let line_height = prepaint.line_height;
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if let Some(bg) = prepaint.cursor_line_bg.take() {
                window.paint_quad(bg);
            }
            for (row, line) in prepaint.lines.drain(..) {
                let y = bounds.origin.y
                    + prepaint.offset_y
                    + line_height * (row - prepaint.first_row) as f32;
                let origin = point(bounds.origin.x + px(GUTTER), y);
                let _ = line.paint(origin, line_height, TextAlign::Left, None, window, cx);
            }
            if focused {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        });

        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble
                && event.button == MouseButton::Left
                && bounds.contains(&event.position)
            {
                editor.update(cx, |editor, cx| editor.click(event.position, window, cx));
            }
        });
        let editor = self.editor.clone();
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(line_height).y;
                editor.update(cx, |editor, cx| editor.scroll_by(delta, cx));
            }
        });

        let layout = Layout {
            bounds,
            line_height,
            font_size: prepaint.font_size,
            text_style: Some(prepaint.text_style.clone()),
            cursor_line: prepaint.cursor_line.take(),
            prepaint_ms: prepaint.prepaint_ms,
        };
        self.editor.update(cx, |editor, _| {
            editor.frame_samples.add(layout.prepaint_ms);
            editor.layout = layout;
        });
    }
}
