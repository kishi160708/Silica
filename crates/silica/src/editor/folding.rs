//! Code folding: computing what can fold, opening and closing folds,
//! unfolding around edits, and the fold gutter's hit test — the methods of
//! the split-out folding section, unchanged apart from living in their own
//! module. Split from `editor.rs` per the audit's module-size finding.

use std::ops::Range;

use gpui::{px, Context, Modifiers, Pixels, Point, Window};
use silica_text::Sel;

use crate::fold::{self, Foldable};
use crate::preview;
use crate::reading::model::{RBlock, Widget};
use silica_formats::properties::PropertiesInDocument;

use super::{shape_display, Editor, Shaped};

impl Editor {
    /// `foldHeading` and `foldIndent`, read from the vault's `app.json`
    /// as each parse starts; both default to on.
    pub(crate) fn fold_options(&self) -> fold::FoldOptions {
        let Some(state) = &self.suggest_vault else {
            return fold::FoldOptions::default();
        };
        let app = state.app();
        fold::FoldOptions {
            headings: app.fold_heading.unwrap_or(true),
            lists: app.fold_indent.unwrap_or(true),
        }
    }

    /// Settles the folds against the current foldables and recomputes the
    /// hidden lines.
    pub(crate) fn refresh_folds(&mut self) {
        self.hidden = self.folds.resolve(&self.foldables);
    }

    /// A cursor that arrived inside a fold — a link into the section, a
    /// search hit, an edit — opens what hides it, the way a link into a
    /// folded section was observed expanding it. A fold command that closes
    /// around the caret leaves the caret where it is, as CodeMirror's does:
    /// fold-less from inside the folded section still finds that section.
    pub(crate) fn unfold_at_cursor(&mut self) {
        let line = self.buf.cursor_at(self.buf.head).line;
        if fold::is_hidden(&self.hidden, line) && self.folds.unfold_hiding(&self.foldables, line) {
            self.refresh_folds();
        }
    }

    pub(crate) fn line_hidden(&self, line: usize) -> bool {
        fold::is_hidden(&self.hidden, line)
    }

    /// The foldable starting on `line`, if one does.
    pub(crate) fn foldable_at(&self, line: usize) -> Option<&Foldable> {
        self.foldables.iter().find(|f| f.line == line)
    }

    /// `editor:toggle-fold`: folds or unfolds the caret's line. Obsidian's
    /// command reports success on a line that starts no fold and changes
    /// nothing, which is what this does too.
    pub fn toggle_fold(&mut self, cx: &mut Context<Self>) {
        let line = self.buf.cursor_at(self.buf.head).line;
        self.toggle_fold_at(line, cx);
    }

    /// Folds or unfolds the foldable starting on `line`: the command on
    /// the caret's line, or a click on a line's indicator.
    pub(crate) fn toggle_fold_at(&mut self, line: usize, cx: &mut Context<Self>) {
        if self.folds.toggle(&self.foldables, line) {
            self.refresh_folds();
            cx.notify();
        }
    }

    /// `editor:fold-all`.
    pub fn fold_all(&mut self, cx: &mut Context<Self>) {
        self.folds.fold_all(&self.foldables);
        self.refresh_folds();
        cx.notify();
    }

    /// `editor:unfold-all`.
    pub fn unfold_all(&mut self, cx: &mut Context<Self>) {
        self.folds.unfold_all();
        self.refresh_folds();
        cx.notify();
    }

    /// `editor:fold-more` and `editor:fold-less`: one enclosing level out
    /// from the caret's section, or the outermost one back.
    pub fn fold_step(&mut self, more: bool, cx: &mut Context<Self>) {
        let line = self.buf.cursor_at(self.buf.head).line;
        let changed = if more {
            self.folds.fold_more(&self.foldables, line)
        } else {
            self.folds.fold_less(&self.foldables, line)
        };
        if changed {
            self.refresh_folds();
            cx.notify();
        }
    }

    /// The folds by key, for the reading view to take over on a mode
    /// switch.
    pub fn fold_keys(&self) -> Vec<fold::FoldKey> {
        self.folds.keys()
    }

    /// The reading view's folds, on a switch back to editing.
    pub fn set_fold_keys(&mut self, keys: Vec<fold::FoldKey>, cx: &mut Context<Self>) {
        self.folds = fold::Folds::from_keys(keys);
        self.pending_folds = None;
        self.refresh_folds();
        cx.notify();
    }

    /// The note's fold record for the store: the full extents, as the
    /// editor keeps them, plus `{0,0}` for a folded properties block.
    /// `None` while the first parse has not delivered the foldables a
    /// restored record is resolved against, so a note left at once keeps
    /// the record it was opened with.
    pub fn fold_record(&self) -> Option<fold::Record> {
        if self.pending_folds.is_some() {
            return None;
        }
        let mut folds: Vec<fold::FoldRange> = self
            .folds
            .record(&self.foldables)
            .into_iter()
            .map(|(from, to)| fold::FoldRange { from, to })
            .collect();
        if self.properties_folded {
            folds.insert(0, fold::PROPERTIES_RANGE);
        }
        Some(fold::Record {
            folds,
            lines: self.buf.line_count(),
        })
    }

    /// Restores a note's record: each `from` line that starts a fold folds
    /// once the parse has said which lines do.
    pub fn set_fold_record(&mut self, record: &fold::Record, cx: &mut Context<Self>) {
        self.properties_folded = record.folds.contains(&fold::PROPERTIES_RANGE);
        let froms: Vec<usize> = record
            .folds
            .iter()
            .filter(|r| **r != fold::PROPERTIES_RANGE)
            .map(|r| r.from)
            .collect();
        // * Before the first parse there is nothing to resolve against: the
        //   `from` lines wait for it. (A note with nothing foldable keeps
        //   them waiting, which costs nothing.)
        if self.foldables.is_empty() {
            self.pending_folds = Some(froms);
        } else {
            self.folds = fold::Folds::from_record(&self.foldables, froms);
            self.refresh_folds();
        }
        cx.notify();
    }

    pub fn properties_folded(&self) -> bool {
        self.properties_folded
    }

    pub fn set_properties_folded(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.properties_folded != on {
            self.properties_folded = on;
            cx.notify();
        }
    }

    /// The line under window point `position`, from the last frame.
    pub(crate) fn line_at_point(&self, position: Point<Pixels>) -> Option<usize> {
        let layout = &self.layout;
        if !layout.bounds.contains(&position) {
            return None;
        }
        let y = position.y - layout.bounds.origin.y;
        layout
            .visible
            .iter()
            .find(|v| y >= v.top && y < v.top + v.height())
            .map(|v| v.line)
    }

    /// Remembers the hovered line, which is where the fold indicator shows,
    /// and updates the hovered link preview when Ctrl/Cmd is held.
    pub(crate) fn hover_at(
        &mut self,
        position: Point<Pixels>,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line = self.line_at_point(position);
        if line != self.hover_line {
            // * Only a change that moves an indicator is worth a frame.
            let affects = |l: Option<usize>| l.is_some_and(|l| self.foldable_at(l).is_some());
            let repaint = affects(line) || affects(self.hover_line);
            self.hover_line = line;
            if repaint {
                cx.notify();
            }
        }

        if modifiers.control || modifiers.platform {
            if let Some(hit) = self.display_hit(window, position) {
                let col = self
                    .buf
                    .clamp_col(hit.line, hit.shaped.to_source(hit.display));
                if let Some(target) = self.link_at(hit.line, col, false) {
                    if let Some(state) = &self.suggest_vault {
                        let source = if self.name.is_empty() {
                            None
                        } else {
                            Some(self.name.as_ref())
                        };
                        let data = crate::page_preview::resolve_preview(&target, source, state);
                        let changed = match (&self.hover_preview, &data) {
                            (Some((_, old)), Some(new)) => old != new,
                            (Some(_), None) => true,
                            (None, Some(_)) => true,
                            (None, None) => false,
                        };
                        if changed {
                            self.hover_preview = data.map(|d| (position, d));
                            cx.notify();
                        }
                        return;
                    }
                }
            }
        }

        if self.hover_preview.is_some() {
            self.hover_preview = None;
            cx.notify();
        }
    }

    /// The widget covering `line` when Live Preview shows it rendered: no
    /// selection touches its lines, so its source is folded away.
    pub(crate) fn folded_widget(&self, line: usize) -> Option<&(Range<usize>, Widget)> {
        if !self.live_preview {
            return None;
        }
        let ix = self.widgets.partition_point(|(l, _)| l.end <= line);
        let entry = self.widgets.get(ix)?;
        if !entry.0.contains(&line) || entry.0.end > self.line_count() {
            return None;
        }
        self.widget_is_folded(entry).then_some(entry)
    }

    /// A widget that takes no room and draws nothing: the properties block
    /// under `propertiesInDocument: hidden`, where Obsidian shows neither
    /// the table nor the YAML behind it. Folding it to one line's height,
    /// which is what every other widget gets, leaves a band of empty space
    /// where the block used to be.
    pub(crate) fn widget_is_hidden(&self, widget: &Widget) -> bool {
        matches!(widget.block, crate::reading::model::RBlock::Properties(_))
            && self.properties_in_document == PropertiesInDocument::Hidden
    }

    pub(crate) fn widget_is_folded(&self, entry: &(Range<usize>, Widget)) -> bool {
        // * The properties block stays a table wherever the cursor is: it is
        //   edited in its own fields, not by revealing the YAML. `Show as
        //   source` in `propertiesInDocument` is what reveals it instead.
        if matches!(entry.1.block, crate::reading::model::RBlock::Properties(_)) {
            return self.properties_in_document != PropertiesInDocument::Source;
        }
        // * A table stays a grid wherever the cursor is, the way Obsidian's
        //   `table-editor` does: its cells are typed into directly.
        if matches!(entry.1.block, RBlock::Table { .. }) {
            return true;
        }
        let span = self.buf.line_starts()[entry.0.start]..self.buf.line_range(entry.0.end - 1).end;
        let primary = Sel {
            anchor: self.buf.anchor,
            head: self.buf.head,
            goal: None,
        };
        !std::iter::once(&primary)
            .chain(self.buf.extra.iter())
            .any(|s| preview::touches(&[s.range()], &span))
    }

    /// Shapes `line` the way the last frame did (same style and wrap width).
    pub(crate) fn shape(&self, window: &mut Window, line: usize) -> Option<Shaped> {
        let style = self.layout.text_style.as_ref()?;
        let spans = self.highlights.get(line).map(Vec::as_slice).unwrap_or(&[]);
        Some(shape_display(
            window,
            style,
            self.layout.font_size,
            self.layout.line_height,
            self.layout.wrap_width,
            self.display_for(line),
            spans,
            &self.palette,
        ))
    }

    pub(crate) fn rows_of(&self, window: &mut Window, line: usize) -> usize {
        self.shape(window, line).map(|l| l.rows()).unwrap_or(1)
    }

    /// Height of `line` on screen: its wrapped rows at its row height. A
    /// folded widget's height sits on its first line; its other lines are
    /// not on screen at all.
    pub(crate) fn height_of(&self, window: &mut Window, line: usize) -> Pixels {
        if self.line_hidden(line) {
            return px(0.);
        }
        if let Some((lines, widget)) = self.folded_widget(line) {
            if line != lines.start || self.widget_is_hidden(widget) {
                return px(0.);
            }
            return self
                .widget_heights
                .get(&lines.start)
                .copied()
                .unwrap_or(self.layout.line_height * lines.len() as f32);
        }
        self.shape(window, line)
            .map(|l| l.height())
            .unwrap_or(self.layout.line_height)
    }

    pub(crate) fn visible_rows(&self) -> usize {
        let h = self.layout.bounds.size.height;
        let lh = self.layout.line_height;
        if lh > px(0.) {
            ((h / lh).floor() as usize).max(1)
        } else {
            40
        }
    }

    /// The last line the view can start at: scrolled there, the note's end
    /// sits at the bottom of the view (what `clamp_scroll_end` enforces).
    /// Zero when the whole note fits. Walks back from the end over one
    /// screenful of lines, so it costs what the visible lines do.
    pub(crate) fn last_top_line(&self, window: &mut Window) -> usize {
        let h = self.layout.bounds.size.height;
        if h <= px(0.) {
            return 0;
        }
        let mut filled = px(0.);
        let mut line = self.line_count();
        while line > 0 {
            line -= 1;
            filled += self.height_of(window, line);
            if filled >= h {
                return line;
            }
        }
        0
    }
}
