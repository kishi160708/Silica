//! The find-and-replace panel: opening it, matching, stepping through
//! matches, and replacing — the methods of the split-out section, unchanged
//! apart from living in their own module. Split from `editor.rs` per the
//! audit's module-size finding.

use std::ops::Range;

use gpui::{Context, Focusable, Window};
use silica_text::{find, shifted, Cursors, EditKind, Sel};

use super::{CloseFind, Editor, FindAll, FindNext, FindPrevious, FocusOtherField};

impl Editor {
    /// Opens the panel (Ctrl+F, or Ctrl+H with the replace row). The
    /// selection, when it sits on one line, becomes the query; an explicit
    /// `query` wins; otherwise the previous query stays.
    pub fn open_find(
        &mut self,
        query: &str,
        replace: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find.open = true;
        if replace {
            self.find.replace_shown = true;
        }
        let sel = self.selection();
        let selected = (sel.start != sel.end && !self.buf.text()[sel.clone()].contains('\n'))
            .then(|| self.buf.text()[sel].to_string());
        let text = if !query.is_empty() {
            Some(query.to_string())
        } else {
            selected
        };
        match text {
            Some(t) => self.find.query.update(cx, |q, cx| q.set_text(t, cx)),
            None => self.refresh_matches(cx),
        }
        // * Obsidian's panel opens on the query whichever command opened
        //   it; `replace` only decides whether the replacement row shows.
        //   The whole query is selected the way a browser input's is, so the
        //   first keystroke replaces the last note's query instead of
        //   appending to it, and the old query is still there to keep.
        self.find.query.update(cx, |q, cx| q.select_all(cx));
        window.focus(&self.find.query.focus_handle(cx), cx);
        cx.notify();
    }

    /// Tab and Shift+Tab in the find panel move between the query and the
    /// replacement. With two fields each key does the same thing, and
    /// without the replacement row there is nowhere else to go.
    pub(crate) fn focus_other_field(
        &mut self,
        _: &FocusOtherField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.find.replace_shown {
            return;
        }
        let on_query = self.find.query.focus_handle(cx).is_focused(window);
        let field = if on_query {
            &self.find.replacement
        } else {
            &self.find.query
        };
        // * Tabbing into a field selects what it holds, as it does between
        //   two HTML inputs.
        field.update(cx, |f, cx| f.select_all(cx));
        window.focus(&field.focus_handle(cx), cx);
        cx.notify();
    }

    /// Escape: closes the autocompletion popup first, then abandons an open
    /// field, then drops extra cursors, then closes the panel.
    pub(crate) fn close_find(
        &mut self,
        _: &CloseFind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.suggest.is_some() {
            self.close_suggest(cx);
        } else if self.cancel_table_field(cx) || self.cancel_property_field(cx) {
        } else if !self.buf.extra.is_empty() && self.focus_handle.is_focused(window) {
            self.buf.collapse_cursors();
            cx.notify();
        } else if self.find.open {
            self.find.open = false;
            window.focus(&self.focus_handle, cx);
            cx.notify();
        } else {
            cx.propagate();
        }
    }

    /// Flips match case or regex and searches again.
    pub(crate) fn toggle_find_option(&mut self, regex: bool, cx: &mut Context<Self>) {
        if regex {
            self.find.opts.regex = !self.find.opts.regex;
        } else {
            self.find.opts.match_case = !self.find.opts.match_case;
        }
        self.query_changed(cx);
    }

    /// Recomputes the matches for the current query (after an edit, or
    /// when the query changes); the selected match is kept where it still is.
    pub(crate) fn refresh_matches(&mut self, cx: &mut Context<Self>) {
        if !self.find.open {
            return;
        }
        let query = self.find.query.read(cx).text().to_string();
        self.find.matches = find::find_matches(self.buf.text(), &query, self.find.opts);
        let sel = self.selection();
        self.find.current = self.find.matches.iter().position(|m| *m == sel);
    }

    /// Typing in the query selects the first match from the cursor on.
    pub(crate) fn query_changed(&mut self, cx: &mut Context<Self>) {
        if !self.find.open {
            return;
        }
        self.refresh_matches(cx);
        let from = self.selection().start;
        if let Some(ix) = find::next_from(&self.find.matches, from) {
            self.select_match(ix, cx);
        } else {
            self.find.current = None;
            cx.notify();
        }
    }

    pub(crate) fn select_match(&mut self, ix: usize, cx: &mut Context<Self>) {
        let m = self.find.matches[ix].clone();
        self.buf.collapse_cursors();
        self.buf.anchor = m.start;
        self.buf.head = m.end;
        self.find.current = Some(ix);
        self.buf.goal = None;
        self.pending_reveal = true;
        cx.notify();
    }

    /// Enter and Shift+Enter in the query: the next or previous match.
    pub(crate) fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let sel = self.selection();
        let ix = if forward {
            let from = if self
                .find
                .current
                .is_some_and(|c| self.find.matches[c] == sel)
            {
                sel.end
            } else {
                sel.start
            };
            find::next_from(&self.find.matches, from)
        } else {
            find::previous_from(&self.find.matches, sel.start)
        };
        match ix {
            Some(ix) => self.select_match(ix, cx),
            None => self.find.current = None,
        }
    }

    pub(crate) fn find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        self.find_step(true, cx);
    }

    pub(crate) fn find_previous(
        &mut self,
        _: &FindPrevious,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_step(false, cx);
    }

    /// Find all (Alt+Enter in the query): every match becomes a selection,
    /// the panel closes and the editor takes focus, so typing edits them
    /// all at once.
    pub(crate) fn find_all(&mut self, _: &FindAll, window: &mut Window, cx: &mut Context<Self>) {
        self.find_all_matches(window, cx);
    }

    /// [`Editor::find_all`] for the command line (`--find-all`).
    pub fn find_all_matches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // * A query set in the same update has not reported its change yet.
        self.refresh_matches(cx);
        if self.find.matches.is_empty() {
            return;
        }
        let sels: Vec<Sel> = self
            .find
            .matches
            .iter()
            .map(|m| Sel {
                anchor: m.start,
                head: m.end,
                goal: None,
            })
            .collect();
        let primary = self.find.current.unwrap_or(0).min(sels.len() - 1);
        self.buf.set_cursors(Cursors { sels, primary });
        self.find.open = false;
        window.focus(&self.focus_handle, cx);
        self.pending_reveal = true;
        cx.notify();
    }

    /// What replaces the match at `m` under the current options.
    pub(crate) fn replacement_for(&self, m: &Range<usize>, cx: &Context<Self>) -> String {
        let with = self.find.replacement.read(cx).text().to_string();
        let query = self.find.query.read(cx).text();
        match find::compile(query, self.find.opts) {
            Some(re) => find::replacement_for(self.buf.text(), m, &re, self.find.opts, &with),
            None => with,
        }
    }

    /// Replaces the selected match and moves to the next one; with no match
    /// selected, only moves.
    pub(crate) fn replace_next(&mut self, cx: &mut Context<Self>) {
        let sel = self.selection();
        if let Some(ix) = self.find.current.filter(|&c| self.find.matches[c] == sel) {
            let m = self.find.matches[ix].clone();
            let with = self.replacement_for(&m, cx);
            self.edit_many_no_window(vec![(m.clone(), with.clone())], m.start + with.len(), cx);
        }
        self.find_step(true, cx);
    }

    /// Replaces every match in one undo step.
    pub(crate) fn replace_all(&mut self, cx: &mut Context<Self>) {
        if self.find.matches.is_empty() {
            return;
        }
        let edits: Vec<(Range<usize>, String)> = self
            .find
            .matches
            .iter()
            .map(|m| (m.clone(), self.replacement_for(m, cx)))
            .collect();
        // * The cursor lands after the last replacement in the new text.
        let last_end = edits.last().map(|(r, _)| r.end).unwrap_or(0);
        let cursor = shifted(last_end, &edits);
        self.edit_many_no_window(edits, cursor, cx);
    }

    /// `edit_many` from a field's event, where no window is at hand: the
    /// cursor is revealed on the next frame.
    pub(crate) fn edit_many_no_window(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        cursor: usize,
        cx: &mut Context<Self>,
    ) {
        self.buf
            .apply_edits(edits, Cursors::at(cursor), EditKind::Other);
        self.after_edit(None, cx, None);
    }
}
