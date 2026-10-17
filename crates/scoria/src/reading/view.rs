//! GPUI rendering of a reading-view [`Model`].
//!
//! Top-level blocks go through a lazily measured `list`, so a long note only
//! lays out the blocks on screen. Inline content is one `InteractiveText`
//! per run of text, with the styled ranges from the model and clickable
//! ranges for links, tags and footnote references.

use super::model::{
    Embed, EmbedKind, InlineStyle, LinkTarget, Model, RBlock, RItem, RichText, Segment,
};
use crate::theme::Theme;
use gpui::{
    canvas, div, img, list, prelude::*, px, AnyElement, App, Context, EventEmitter, FocusHandle,
    Focusable, Font, FontStyle, FontWeight, HighlightStyle, Hsla, InteractiveText, ListAlignment,
    ListOffset, ListState, ObjectFit, Pixels, SharedString, StrikethroughStyle, StyledText,
    TextRun, UnderlineStyle, Window, WindowTextSystem,
};
use _markdown::Alignment;
use std::collections::HashSet;
use std::sync::Arc;

pub enum ReadingEvent {
    OpenLink(LinkTarget),
}

pub struct ReadingView {
    focus_handle: FocusHandle,
    pub path: Option<String>,
    model: Arc<Model>,
    list: ListState,
    /// Callouts whose fold state the reader flipped, by title id.
    toggled: HashSet<usize>,
    readable_line_length: bool,
    /// Width of the view as last painted; tables lay their columns out for it.
    width: Pixels,
}

impl EventEmitter<ReadingEvent> for ReadingView {}

impl Focusable for ReadingView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ReadingView {
    pub fn new(cx: &mut Context<Self>) -> ReadingView {
        ReadingView {
            focus_handle: cx.focus_handle(),
            path: None,
            model: Arc::new(Model::default()),
            list: ListState::new(0, ListAlignment::Top, px(1200.)),
            toggled: HashSet::new(),
            readable_line_length: true,
            width: px(900.),
        }
    }

    pub fn set_readable_line_length(&mut self, on: bool) {
        self.readable_line_length = on;
    }

    pub fn set_note(&mut self, path: Option<String>, model: Model, cx: &mut Context<Self>) {
        let same = path.is_some() && self.path == path;
        let top = self.list.logical_scroll_top();
        self.path = path;
        self.list.reset(model.blocks.len());
        if same {
            // A reload after a change on disk keeps the reader's place.
            self.list.scroll_to(top);
        }
        self.model = Arc::new(model);
        self.toggled.clear();
        cx.notify();
    }

    /// Scrolls so the top-level block `ix` is at the top of the view.
    pub fn scroll_to_block(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.model.blocks.len() {
            self.list.scroll_to(ListOffset {
                item_ix: ix,
                offset_in_item: px(0.),
            });
            cx.notify();
        }
    }

    pub fn scroll_to_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(ix) = self.model.block_at(offset) {
            self.scroll_to_block(ix, cx);
        }
    }

    pub fn scroll_to_end(&mut self, cx: &mut Context<Self>) {
        self.list.scroll_to_end();
        cx.notify();
    }

    fn toggle_callout(&mut self, id: usize, cx: &mut Context<Self>) {
        if !self.toggled.remove(&id) {
            self.toggled.insert(id);
        }
        cx.notify();
    }

    fn render_block(
        &mut self,
        ix: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let model = self.model.clone();
        let Some(block) = model.blocks.get(ix) else {
            return div().into_any_element();
        };
        // The block's inner width: the view minus its padding, capped at the
        // readable line length.
        let mut available = f32::from(self.width) - 64.;
        if self.readable_line_length {
            available = available.min(700.);
        }
        let r = Renderer {
            theme: &theme,
            view: cx.entity(),
            toggled: &self.toggled,
            text_system: _window.text_system().clone(),
            available: available.max(100.),
        };
        let inner = r.block(block, false);
        let last = ix + 1 == model.blocks.len();
        div()
            .w_full()
            .flex()
            .flex_row()
            .justify_center()
            .px(px(32.))
            .when(ix == 0, |d| d.pt(px(24.)))
            .when(last, |d| d.pb(px(120.)))
            .child(
                div()
                    .w_full()
                    .when(self.readable_line_length, |d| d.max_w(px(700.)))
                    .child(inner),
            )
            .into_any_element()
    }
}

impl Render for ReadingView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let base = theme.base_font_size;
        let view = cx.entity().downgrade();
        div()
            .key_context("ReadingView")
            .track_focus(&self.focus_handle)
            .size_full()
            .font_family(theme.text_font.clone())
            .text_size(px(base))
            .line_height(px(base * 1.6))
            .text_color(theme.text)
            .relative()
            .child(
                list(self.list.clone(), move |ix, window, cx| {
                    view.update(cx, |this, cx| this.render_block(ix, window, cx))
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .size_full(),
            )
            .child(
                // Records the painted width; a change lays the blocks out again.
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _, cx| {
                            view.update(cx, |this, cx| {
                                if this.width != bounds.size.width {
                                    this.width = bounds.size.width;
                                    cx.notify();
                                }
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
    }
}

struct Renderer<'a> {
    theme: &'a Theme,
    view: gpui::Entity<ReadingView>,
    toggled: &'a HashSet<usize>,
    text_system: Arc<WindowTextSystem>,
    /// Inner width of a top-level block, in pixels.
    available: f32,
}

impl Renderer<'_> {
    /// Width of `text` set in the note's font, in pixels.
    fn measure(&self, text: &str, font_size: f32, bold: bool, mono: bool) -> f32 {
        let t = self.theme;
        let font = Font {
            family: if mono {
                t.mono_font.clone()
            } else {
                t.text_font.clone()
            },
            features: Default::default(),
            fallbacks: None,
            weight: if bold {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            },
            style: FontStyle::Normal,
        };
        let run = TextRun {
            len: text.len(),
            font,
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = self.text_system.shape_line(
            SharedString::from(text.to_string()),
            px(font_size),
            &[run],
            None,
        );
        f32::from(line.width)
    }

    /// Column widths the way an HTML table's automatic layout sizes them: a
    /// column wants its longest cell (max-content) and needs at least its
    /// longest word (min-content); when the note is too narrow for every
    /// column's wish, the slack is shared in proportion to each column's
    /// range, and below the minimum the table overflows.
    fn column_widths(
        &self,
        header: &[RichText],
        rows: &[Vec<RichText>],
        font_size: f32,
    ) -> Vec<f32> {
        const PADDING: f32 = 17.;
        let count = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        let mut max = vec![0f32; count];
        let mut min = vec![0f32; count];
        let mut fold = |ix: usize, cell: &RichText, bold: bool| {
            let mono = cell.runs.iter().any(|(_, s)| s.code);
            let longest_line = cell
                .text
                .lines()
                .max_by_key(|l| l.chars().count())
                .unwrap_or("");
            let longest_word = cell
                .text
                .split_whitespace()
                .max_by_key(|w| w.chars().count())
                .unwrap_or("");
            max[ix] = max[ix].max(self.measure(longest_line, font_size, bold, mono));
            min[ix] = min[ix].max(self.measure(longest_word, font_size, bold, mono));
        };
        for (ix, cell) in header.iter().enumerate() {
            fold(ix, cell, true);
        }
        for row in rows {
            for (ix, cell) in row.iter().enumerate() {
                fold(ix, cell, false);
            }
        }
        // Two pixels of slack: the wrapper measures advances a little
        // differently from the shaper and must not break a line that fits.
        let max: Vec<f32> = max.iter().map(|m| m.ceil() + 2. + PADDING).collect();
        let min: Vec<f32> = min
            .iter()
            .map(|m| m.max(8.).ceil() + 2. + PADDING)
            .collect();
        let sum_max: f32 = max.iter().sum();
        let sum_min: f32 = min.iter().sum();
        if sum_max <= self.available {
            max
        } else if sum_min >= self.available {
            min
        } else {
            let share = (self.available - sum_min) / (sum_max - sum_min);
            max.iter()
                .zip(&min)
                .map(|(mx, mn)| (mn + (mx - mn) * share).floor())
                .collect()
        }
    }

    fn blocks(&self, blocks: &[RBlock], tight: bool) -> Vec<AnyElement> {
        blocks.iter().map(|b| self.block(b, tight)).collect()
    }

    fn block(&self, block: &RBlock, tight: bool) -> AnyElement {
        let t = self.theme;
        let base = t.base_font_size;
        let gap = if tight { px(0.) } else { px(base * 0.9) };
        match block {
            RBlock::Properties(rows) => div()
                .mb(px(base * 1.2))
                .flex()
                .flex_col()
                .border_1()
                .border_color(t.border)
                .rounded_md()
                .px_3()
                .py_2()
                .text_size(px(base * 0.875))
                .children(rows.iter().map(|(k, v)| {
                    div()
                        .flex()
                        .flex_row()
                        .gap_3()
                        .py_1()
                        .child(
                            div()
                                .w(px(140.))
                                .flex_shrink_0()
                                .text_color(t.text_muted)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .overflow_hidden()
                                .child(SharedString::from(k.clone())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(SharedString::from(v.clone())),
                        )
                }))
                .into_any_element(),
            RBlock::Paragraph(segments) => div()
                .mb(gap)
                .flex()
                .flex_col()
                .children(segments.iter().map(|s| self.segment(s)))
                .into_any_element(),
            RBlock::Heading { level, text, .. } => {
                let (size, weight, top) = match level {
                    1 => (1.802, FontWeight::BOLD, 1.6),
                    2 => (1.602, FontWeight::BOLD, 1.4),
                    3 => (1.424, FontWeight::SEMIBOLD, 1.2),
                    4 => (1.266, FontWeight::SEMIBOLD, 1.0),
                    5 => (1.125, FontWeight::SEMIBOLD, 0.9),
                    _ => (1.0, FontWeight::SEMIBOLD, 0.8),
                };
                div()
                    .mt(px(base * top))
                    .mb(px(base * 0.5))
                    .text_size(px(base * size))
                    .line_height(px(base * size * 1.3))
                    .font_weight(weight)
                    .child(self.rich(text))
                    .into_any_element()
            }
            RBlock::ThematicBreak => div()
                .my(px(base * 1.5))
                .h(px(1.))
                .w_full()
                .bg(t.border)
                .into_any_element(),
            RBlock::Code { id, language, text } => div()
                .mb(gap)
                .relative()
                .bg(t.bg_alt)
                .rounded_md()
                .py_2()
                .font_family(t.mono_font.clone())
                .text_size(px(base * 0.875))
                .line_height(px(base * 0.875 * 1.5))
                .child(
                    // Code keeps its columns: long lines scroll sideways.
                    div().id(("code", *id)).overflow_x_scroll().px_3().child(
                        div()
                            .whitespace_nowrap()
                            .child(SharedString::from(text.clone())),
                    ),
                )
                .children(language.as_ref().map(|lang| {
                    div()
                        .absolute()
                        .top_1()
                        .right_2()
                        .text_size(px(base * 0.7))
                        .text_color(t.text_faint)
                        .child(SharedString::from(lang.clone()))
                }))
                .into_any_element(),
            RBlock::Html(text) => div()
                .mb(gap)
                .font_family(t.mono_font.clone())
                .text_size(px(base * 0.875))
                .text_color(t.text_muted)
                .child(SharedString::from(text.clone()))
                .into_any_element(),
            RBlock::Quote(children) => div()
                .mb(gap)
                .border_l_2()
                .border_color(t.accent)
                .pl_4()
                .flex()
                .flex_col()
                .children(self.blocks(children, false))
                .into_any_element(),
            RBlock::Callout {
                kind,
                fold,
                title,
                children,
            } => {
                let color = t.callout_color(kind);
                let foldable = fold.is_some();
                let flipped = self.toggled.contains(&title.id);
                let collapsed = fold.unwrap_or(false) != flipped;
                let id = title.id;
                let view = self.view.clone();
                let mut bg = color;
                bg.a = if t.dark { 0.12 } else { 0.08 };
                div()
                    .mb(gap)
                    .bg(bg)
                    .border_l_4()
                    .border_color(color)
                    .rounded_md()
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id(("callout", id))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .when(foldable, |d| {
                                d.cursor_pointer().on_click(move |_, _, cx| {
                                    view.update(cx, |v, cx| v.toggle_callout(id, cx));
                                })
                            })
                            .child(div().flex_1().child(self.rich(title)))
                            .when(foldable, |d| {
                                d.child(div().text_color(t.text_muted).child(if collapsed {
                                    "▸"
                                } else {
                                    "▾"
                                }))
                            }),
                    )
                    .when(!collapsed && !children.is_empty(), |d| {
                        d.child(
                            div()
                                .mt_2()
                                .flex()
                                .flex_col()
                                .children(self.blocks(children, false)),
                        )
                    })
                    .into_any_element()
            }
            RBlock::List { tight, items } => div()
                .mb(gap)
                .flex()
                .flex_col()
                .children(items.iter().map(|item| self.item(item, *tight)))
                .into_any_element(),
            RBlock::Table {
                alignments,
                header,
                rows,
            } => {
                let font_size = base * 0.9375;
                let columns = self.column_widths(header, rows, font_size);
                let table_width: f32 = columns.iter().sum();
                let cell = |text: &RichText, ix: usize, head: bool| {
                    let align = alignments.get(ix).copied().unwrap_or(Alignment::None);
                    let width = columns.get(ix).copied().unwrap_or(80.);
                    div()
                        .w(px(width))
                        .flex_shrink_0()
                        .px_2()
                        .py_1()
                        .border_r_1()
                        .border_color(t.border)
                        .when(head, |d| d.font_weight(FontWeight::SEMIBOLD))
                        .map(|d| match align {
                            Alignment::Center => d.text_center(),
                            Alignment::Right => d.text_right(),
                            _ => d,
                        })
                        .child(self.rich(text))
                };
                // The table is as wide as its columns, like an HTML table.
                div()
                    .mb(gap)
                    .flex()
                    .flex_row()
                    .child(
                        div()
                            .w(px(table_width))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .border_1()
                            .border_color(t.border)
                            .rounded_sm()
                            .text_size(px(font_size))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .bg(t.bg_secondary)
                                    .border_b_1()
                                    .border_color(t.border)
                                    .children(
                                        header.iter().enumerate().map(|(i, c)| cell(c, i, true)),
                                    ),
                            )
                            .children(rows.iter().enumerate().map(|(ri, row)| {
                                div()
                                    .flex()
                                    .flex_row()
                                    .when(ri + 1 < rows.len(), |d| {
                                        d.border_b_1().border_color(t.border)
                                    })
                                    .children(
                                        row.iter().enumerate().map(|(i, c)| cell(c, i, false)),
                                    )
                            })),
                    )
                    .into_any_element()
            }
            RBlock::Math(text) => div()
                .mb(gap)
                .px_3()
                .py_2()
                .font_family(t.mono_font.clone())
                .text_color(t.accent)
                .text_center()
                .child(SharedString::from(text.clone()))
                .into_any_element(),
            RBlock::Footnotes(notes) => div()
                .mt(px(base * 2.))
                .pt(px(base))
                .border_t_1()
                .border_color(t.border)
                .text_size(px(base * 0.875))
                .flex()
                .flex_col()
                .children(notes.iter().map(|(n, blocks)| {
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(
                            div()
                                .w(px(28.))
                                .flex_shrink_0()
                                .text_color(t.text_muted)
                                .child(SharedString::from(format!("{n}."))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .children(self.blocks(blocks, true)),
                        )
                }))
                .into_any_element(),
        }
    }

    fn item(&self, item: &RItem, tight: bool) -> AnyElement {
        let t = self.theme;
        let base = t.base_font_size;
        let marker: AnyElement = match (item.task, item.number) {
            (Some(c), _) => {
                let checked = c != ' ';
                div()
                    .w(px(16.))
                    .h(px(16.))
                    .mt(px(base * 0.3))
                    .rounded_sm()
                    .border_1()
                    .border_color(if checked { t.accent } else { t.text_muted })
                    .when(checked, |d| d.bg(t.accent))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(t.bg)
                    .when(checked, |d| d.child("✓"))
                    .into_any_element()
            }
            (None, Some(n)) => div()
                .text_color(t.text_muted)
                .child(SharedString::from(format!("{n}.")))
                .into_any_element(),
            (None, None) => div().text_color(t.text_muted).child("•").into_any_element(),
        };
        let done = item.task.is_some_and(|c| c != ' ');
        div()
            .flex()
            .flex_row()
            .gap_2()
            .when(!tight, |d| d.mb(px(base * 0.4)))
            .child(
                div()
                    .min_w(px(22.))
                    .flex_shrink_0()
                    .flex()
                    .justify_end()
                    .child(marker),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .when(done, |d| d.text_color(t.text_muted))
                    .children(self.blocks(&item.children, tight)),
            )
            .into_any_element()
    }

    fn segment(&self, segment: &Segment) -> AnyElement {
        match segment {
            Segment::Text(text) => self.rich(text).into_any_element(),
            Segment::Embed(embed) => self.embed(embed),
        }
    }

    fn embed(&self, embed: &Embed) -> AnyElement {
        let t = self.theme;
        let base = t.base_font_size;
        match &embed.kind {
            EmbedKind::Image(path) => {
                let mut image = img(path.clone())
                    .max_w_full()
                    .object_fit(ObjectFit::Contain);
                if let Some(w) = embed.width {
                    image = image.w(px(w));
                }
                if let Some(h) = embed.height {
                    image = image.h(px(h));
                }
                div().my_1().child(image).into_any_element()
            }
            EmbedKind::Note {
                path,
                title,
                blocks,
            } => {
                let view = self.view.clone();
                let link = embed.link.clone();
                div()
                    .my_2()
                    .border_1()
                    .border_color(t.border)
                    .rounded_md()
                    .px_4()
                    .py_3()
                    .relative()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id(("embed-title", path.len() + title.len()))
                            .mb_2()
                            .text_size(px(base * 0.75))
                            .text_color(t.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.text_color(t.link))
                            .on_click(move |_, _, cx| {
                                let link = link.clone();
                                view.update(cx, |_, cx| {
                                    cx.emit(ReadingEvent::OpenLink(LinkTarget::Internal(link)))
                                });
                            })
                            .child(SharedString::from(format!("↗ {title}"))),
                    )
                    .children(self.blocks(blocks, false))
                    .into_any_element()
            }
            EmbedKind::Missing => div()
                .my_1()
                .text_color(t.text_muted)
                .italic()
                .child(SharedString::from(format!("{}: file not found", embed.alt)))
                .into_any_element(),
            EmbedKind::Unsupported(kind) => {
                let view = self.view.clone();
                let link = embed.link.clone();
                div()
                    .id(("embed-file", embed.link.len()))
                    .my_1()
                    .text_color(t.link)
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        let link = link.clone();
                        view.update(cx, |_, cx| {
                            cx.emit(ReadingEvent::OpenLink(LinkTarget::Internal(link)))
                        });
                    })
                    .child(SharedString::from(format!("{} ({kind:?})", embed.alt)))
                    .into_any_element()
            }
            EmbedKind::TooDeep => div()
                .my_1()
                .text_color(t.text_muted)
                .italic()
                .child(SharedString::from(format!(
                    "{}: embed nested too deep",
                    embed.alt
                )))
                .into_any_element(),
        }
    }

    fn rich(&self, text: &RichText) -> InteractiveText {
        let t = self.theme;
        let highlights: Vec<_> = text
            .runs
            .iter()
            .map(|(range, style)| (range.clone(), self.highlight(*style)))
            .collect();
        let mono: Vec<_> = text
            .runs
            .iter()
            .filter(|(_, s)| s.code || s.math || s.html)
            .map(|(range, _)| (range.clone(), t.mono_font.clone()))
            .collect();
        let mut styled =
            StyledText::new(SharedString::from(text.text.clone())).with_highlights(highlights);
        if !mono.is_empty() {
            styled = styled.with_font_family_overrides(mono);
        }
        let ranges: Vec<_> = text.links.iter().map(|(r, _)| r.clone()).collect();
        let targets: Vec<LinkTarget> = text.links.iter().map(|(_, l)| l.clone()).collect();
        let view = self.view.clone();
        InteractiveText::new(("rich", text.id), styled).on_click(ranges, move |ix, _, cx| {
            if let Some(target) = targets.get(ix).cloned() {
                view.update(cx, |_, cx| cx.emit(ReadingEvent::OpenLink(target)));
            }
        })
    }

    fn highlight(&self, s: InlineStyle) -> HighlightStyle {
        let t = self.theme;
        let mut h = HighlightStyle::default();
        if s.bold {
            h.font_weight = Some(FontWeight::BOLD);
        }
        if s.italic {
            h.font_style = Some(FontStyle::Italic);
        }
        if s.strike {
            h.strikethrough = Some(StrikethroughStyle {
                thickness: px(1.),
                color: Some(t.text_muted),
            });
            h.color = Some(t.text_muted);
        }
        if s.highlight {
            h.background_color = Some(t.highlight_bg);
        }
        if s.code {
            h.background_color = Some(t.bg_alt);
        }
        if s.math {
            h.color = Some(t.accent);
        }
        if s.html {
            h.color = Some(t.text_muted);
        }
        if s.tag {
            h.color = Some(t.tag);
        }
        if s.footnote {
            h.color = Some(t.link);
        }
        if s.link {
            h.color = Some(t.link);
            h.underline = Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(t.link),
                wavy: false,
            });
        }
        h
    }
}
