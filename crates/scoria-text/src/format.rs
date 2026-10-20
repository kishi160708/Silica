//! What Obsidian's formatting commands write: the `**`/`*`/`==` toggles,
//! the heading and list prefixes, the link shapes, moving a line and
//! deleting a paragraph.
//!
//! Pure functions over the text the way [`crate::lists`] and
//! [`crate::plan`] are: each returns the edits in pre-edit coordinates, so
//! the buffer can apply a whole command as one undo step and a test can
//! read the bytes without a window.

use crate::lists::{self, Marker};
use std::ops::Range;

/// One line of `text`, without its newline.
fn line_text<'a>(text: &'a str, line_starts: &[usize], line: usize) -> &'a str {
    let start = line_starts[line];
    let end = line_starts
        .get(line + 1)
        .map(|n| n - 1)
        .unwrap_or(text.len());
    &text[start..end]
}

/// Where the list marker of `line` is, or would go: after the indentation
/// and any quote markers.
fn marker_point(prefix: &lists::Prefix) -> usize {
    match &prefix.marker {
        Some(Marker::Bullet { range, .. }) | Some(Marker::Ordered { range, .. }) => range.start,
        None => prefix.content,
    }
}

/// The marker and the space after it, which a list toggle removes.
fn marker_span(line: &str, prefix: &lists::Prefix) -> Option<Range<usize>> {
    let range = match &prefix.marker {
        Some(Marker::Bullet { range, .. }) | Some(Marker::Ordered { range, .. }) => range.clone(),
        None => return None,
    };
    let mut end = range.end;
    if line.as_bytes().get(end) == Some(&b' ') {
        end += 1;
    }
    Some(range.start..end)
}

/// Whether `marker` starts at byte `at` of `text`.
fn marker_at(text: &str, at: usize, marker: &str) -> bool {
    text.get(at..at + marker.len()) == Some(marker)
}

/// What a formatting toggle does to one selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Toggle {
    /// The marker is already there: strip these two ranges, the opener and
    /// the closer.
    Strip(Range<usize>, Range<usize>),
    /// Wrap this range in the marker; empty for a bare caret, which then
    /// gets the pair with the caret between the halves.
    Wrap(Range<usize>),
}

/// What `editor:toggle-bold` and the other wrapping toggles do to the
/// selection `sel`: strip the marker the selection already carries, inside
/// it or just outside it, or add the pair. `floor` is the end of the
/// previous selection's text, which an outward strip must not reach into.
///
/// A marker is only recognised when it is not part of a longer run of the
/// same character, so italics inside `**bold**` adds a third `*` rather
/// than eating one of the two that make the bold.
pub fn toggle_wrap(text: &str, sel: Range<usize>, marker: &str, floor: usize) -> Toggle {
    let m = marker.len();
    let inner = sel.len() >= 2 * m
        && marker_at(text, sel.start, marker)
        && marker_at(text, sel.end - m, marker)
        && (sel.len() == 2 * m || !marker_at(text, sel.start + m, marker));
    if inner {
        return Toggle::Strip(sel.start..sel.start + m, sel.end - m..sel.end);
    }
    let outer = sel.start >= floor + m
        && marker_at(text, sel.start - m, marker)
        && marker_at(text, sel.end, marker)
        && !(sel.start >= 2 * m && marker_at(text, sel.start - 2 * m, marker))
        && !marker_at(text, sel.end + m, marker);
    if outer {
        return Toggle::Strip(sel.start - m..sel.start, sel.end..sel.end + m);
    }
    Toggle::Wrap(sel)
}

/// The three bytes `editor:toggle-checklist-status` writes on each of
/// `lines` that already carries a checkbox: anything but a space becomes a
/// space, as the click on a rendered checkbox does (Specification.md,
/// "Observed task checkbox behavior"). Lines without a checkbox are left
/// alone: what Obsidian does to those is unprobed.
pub fn checklist_plan(
    text: &str,
    line_starts: &[usize],
    lines: Range<usize>,
) -> Vec<(Range<usize>, String)> {
    let mut edits = Vec::new();
    for l in lines {
        let lt = line_text(text, line_starts, l);
        let Some(task) = lists::parse_prefix(lt).task else {
            continue;
        };
        let checked = lt.as_bytes()[task.start + 1] != b' ';
        let with = if checked { "[ ]" } else { "[x]" };
        let at = line_starts[l] + task.start;
        edits.push((at..at + 3, with.to_string()));
    }
    edits
}

/// Where a heading's `#`s sit on `line`: after the indentation and any
/// quote markers.
fn heading_point(line: &str) -> usize {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    while i < b.len() && b[i] == b'>' {
        i += 1;
        if i < b.len() && b[i] == b' ' {
            i += 1;
        }
    }
    i
}

/// The `#`s already on `line` with the space after them, and their level.
fn heading_at(line: &str) -> (Range<usize>, u8) {
    let start = heading_point(line);
    let b = line.as_bytes();
    let mut i = start;
    while i < b.len() && b[i] == b'#' && i - start < 6 {
        i += 1;
    }
    if i == start || (i < b.len() && b[i] != b' ') {
        return (start..start, 0);
    }
    let level = (i - start) as u8;
    if i < b.len() && b[i] == b' ' {
        i += 1;
    }
    (start..i, level)
}

/// What `editor:set-heading-<n>` writes on `lines`: the line's `#`s
/// replaced by `level` of them, or removed when `level` is 0. Blank lines
/// in a multi-line selection are left alone.
pub fn heading_plan(
    text: &str,
    line_starts: &[usize],
    lines: Range<usize>,
    level: u8,
) -> Vec<(Range<usize>, String)> {
    let single = lines.len() == 1;
    let mut edits = Vec::new();
    for l in lines {
        let lt = line_text(text, line_starts, l);
        if lt.trim().is_empty() && !single {
            continue;
        }
        let (range, at) = heading_at(lt);
        if at == level {
            continue;
        }
        let with = if level == 0 {
            String::new()
        } else {
            format!("{} ", "#".repeat(level as usize))
        };
        let start = line_starts[l];
        edits.push((start + range.start..start + range.end, with));
    }
    edits
}

/// The line prefix a toggle adds or removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinePrefix {
    Bullet,
    Ordered,
    Quote,
}

/// Whether the line already carries `kind`.
fn has_prefix(prefix: &lists::Prefix, kind: LinePrefix) -> bool {
    match kind {
        LinePrefix::Bullet => matches!(prefix.marker, Some(Marker::Bullet { .. })),
        LinePrefix::Ordered => matches!(prefix.marker, Some(Marker::Ordered { .. })),
        LinePrefix::Quote => !prefix.quotes.is_empty(),
    }
}

/// What `editor:toggle-bullet-list`, `-numbered-list` and
/// `-blockquote` write on `lines`: the prefix is removed when every line
/// already has it and added to the ones that do not otherwise, which is
/// how a toggle over a mixed selection settles. An ordered list numbers
/// from 1 down the selection. Blank lines are skipped unless the selection
/// is a single line.
pub fn line_prefix_plan(
    text: &str,
    line_starts: &[usize],
    lines: Range<usize>,
    kind: LinePrefix,
) -> Vec<(Range<usize>, String)> {
    let single = lines.len() == 1;
    let touched: Vec<usize> = lines
        .filter(|l| single || !line_text(text, line_starts, *l).trim().is_empty())
        .collect();
    let all = !touched.is_empty()
        && touched.iter().all(|l| {
            let lt = line_text(text, line_starts, *l);
            has_prefix(&lists::parse_prefix(lt), kind)
        });
    let mut edits = Vec::new();
    let mut number = 1;
    for l in touched {
        let lt = line_text(text, line_starts, l);
        let start = line_starts[l];
        let p = lists::parse_prefix(lt);
        match kind {
            LinePrefix::Quote => {
                if all {
                    // The outermost `> ` of however many the line carries.
                    let at = lt.find('>').unwrap_or(0);
                    let mut end = at + 1;
                    if lt.as_bytes().get(end) == Some(&b' ') {
                        end += 1;
                    }
                    edits.push((start + at..start + end, String::new()));
                } else if !has_prefix(&p, kind) {
                    edits.push((start..start, "> ".to_string()));
                }
            }
            LinePrefix::Bullet | LinePrefix::Ordered => {
                let span = marker_span(lt, &p);
                if all {
                    let span = span.expect("every line was found to have a marker");
                    edits.push((start + span.start..start + span.end, String::new()));
                    continue;
                }
                // An item of the other kind is rewritten; one of this kind
                // is left alone, except that an ordered list renumbers.
                let ordered = kind == LinePrefix::Ordered;
                if has_prefix(&p, kind) && !ordered {
                    continue;
                }
                let with = if ordered {
                    let n = format!("{number}. ");
                    number += 1;
                    n
                } else {
                    "- ".to_string()
                };
                match span {
                    Some(span) => edits.push((start + span.start..start + span.end, with)),
                    None => {
                        let at = start + marker_point(&p);
                        edits.push((at..at, with));
                    }
                }
            }
        }
    }
    // A renumbering pass leaves the items that already had their number.
    edits.retain(|(r, new)| text[r.clone()] != **new);
    edits
}

/// The lines `editor:delete-paragraph` removes around `lines`: out to the
/// blank line either side, or the blank line itself when that is where the
/// cursor is. The newline before a block at the end of the file goes with
/// it, so the deletion does not leave a trailing blank line.
pub fn paragraph_span(text: &str, line_starts: &[usize], lines: Range<usize>) -> Range<usize> {
    let blank = |l: usize| line_text(text, line_starts, l).trim().is_empty();
    let mut first = lines.start;
    let mut last = lines.end - 1;
    if !blank(first) {
        while first > 0 && !blank(first - 1) {
            first -= 1;
        }
    }
    if !blank(last) {
        while last + 1 < line_starts.len() && !blank(last + 1) {
            last += 1;
        }
    }
    let start = line_starts[first];
    let end = line_starts.get(last + 1).copied().unwrap_or(text.len());
    if end == text.len() && start > 0 {
        start - 1..end
    } else {
        start..end
    }
}

/// The one replacement that moves `lines` over the line above or below,
/// and how far the moved text travels. `None` when there is no line that
/// way, so the command writes nothing.
///
/// The empty line a file ending in a newline has after its last one is not
/// a line to swap with: moving text past it would take the file's trailing
/// newline off, which is not this command's to change.
pub fn swap_lines_plan(
    text: &str,
    line_starts: &[usize],
    lines: Range<usize>,
    up: bool,
) -> Option<((Range<usize>, String), isize)> {
    let end_of = |l: usize| -> usize { line_starts.get(l + 1).copied().unwrap_or(text.len()) };
    let last = line_starts.len() - 1;
    let trailing_blank = line_starts[last] == text.len();
    if trailing_blank && lines.end - 1 == last {
        return None;
    }
    let block = line_starts[lines.start]..end_of(lines.end - 1);
    if up {
        if lines.start == 0 {
            return None;
        }
        let above = line_starts[lines.start - 1]..block.start;
        let moved = &text[block.clone()];
        let other = &text[above.clone()];
        // Only the last line of the file lacks a terminator, and after the
        // swap it is a different line that is last.
        let new = match moved.strip_suffix('\n') {
            Some(_) => format!("{moved}{other}"),
            None => format!("{moved}\n{}", &other[..other.len() - 1]),
        };
        Some(((above.start..block.end, new), -(above.len() as isize)))
    } else {
        if lines.end > last || (trailing_blank && lines.end == last) {
            return None;
        }
        let below = block.end..end_of(lines.end);
        let moved = &text[block.clone()];
        let other = &text[below.clone()];
        let (new, shift) = match other.strip_suffix('\n') {
            Some(_) => (format!("{other}{moved}"), other.len()),
            None => (
                format!("{other}\n{}", &moved[..moved.len() - 1]),
                other.len() + 1,
            ),
        };
        Some(((block.start..below.end, new), shift as isize))
    }
}

/// The link shape a command writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSyntax {
    /// `editor:insert-link`: `[text](url)`, the caret in the url.
    Markdown,
    /// `editor:insert-wikilink`: `[[text]]`, the caret on the target.
    Wikilink,
    /// `editor:insert-embed`: `![[text]]`.
    Embed,
}

/// The text one of the link commands writes around `inner`, and where in
/// it the caret lands: an empty selection leaves the caret where the
/// target is typed, so the autocompletion popup opens on it.
pub fn link_insertion(inner: &str, kind: LinkSyntax) -> (String, usize) {
    match kind {
        LinkSyntax::Markdown => {
            let caret = if inner.is_empty() { 1 } else { inner.len() + 3 };
            (format!("[{inner}]()"), caret)
        }
        LinkSyntax::Wikilink => (format!("[[{inner}]]"), inner.len() + 2),
        LinkSyntax::Embed => (format!("![[{inner}]]"), inner.len() + 3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(text: &str) -> Vec<usize> {
        let mut v = vec![0];
        v.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        v
    }

    fn apply(text: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
        let mut out = text.to_string();
        edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
        for (r, new) in edits {
            out.replace_range(r, &new);
        }
        out
    }

    #[test]
    fn wraps_and_unwraps_a_selection() {
        let text = "alpha beta";
        assert_eq!(toggle_wrap(text, 0..5, "**", 0), Toggle::Wrap(0..5));
        let text = "**alpha** beta";
        // The markers inside the selection.
        assert_eq!(toggle_wrap(text, 0..9, "**", 0), Toggle::Strip(0..2, 7..9));
        // The markers just outside it.
        assert_eq!(toggle_wrap(text, 2..7, "**", 0), Toggle::Strip(0..2, 7..9));
    }

    #[test]
    fn a_caret_between_markers_strips_them() {
        assert_eq!(
            toggle_wrap("****", 2..2, "**", 0),
            Toggle::Strip(0..2, 2..4)
        );
        assert_eq!(toggle_wrap("ab", 1..1, "**", 0), Toggle::Wrap(1..1));
    }

    #[test]
    fn italics_inside_bold_does_not_eat_a_star() {
        let text = "**bold**";
        assert_eq!(toggle_wrap(text, 2..6, "*", 0), Toggle::Wrap(2..6));
    }

    #[test]
    fn an_outward_strip_stops_at_the_previous_selection() {
        let text = "*a*b*";
        assert_eq!(toggle_wrap(text, 3..4, "*", 3), Toggle::Wrap(3..4));
    }

    #[test]
    fn checkbox_flips_to_the_two_observed_states() {
        let text = "- [ ] a\n- [x] b\n- [?] c\n- plain\ntext\n";
        let s = starts(text);
        let edits = checklist_plan(text, &s, 0..5);
        assert_eq!(
            apply(text, edits),
            "- [x] a\n- [ ] b\n- [ ] c\n- plain\ntext\n"
        );
    }

    #[test]
    fn heading_replaces_and_removes() {
        let text = "alpha\n## beta\n> quoted\n";
        let s = starts(text);
        assert_eq!(
            apply(text, heading_plan(text, &s, 0..1, 2)),
            "## alpha\n## beta\n> quoted\n"
        );
        assert_eq!(
            apply(text, heading_plan(text, &s, 1..2, 4)),
            "alpha\n#### beta\n> quoted\n"
        );
        assert_eq!(
            apply(text, heading_plan(text, &s, 1..2, 0)),
            "alpha\nbeta\n> quoted\n"
        );
        assert_eq!(
            apply(text, heading_plan(text, &s, 2..3, 1)),
            "alpha\n## beta\n> # quoted\n"
        );
    }

    #[test]
    fn heading_leaves_a_line_that_is_already_that_level() {
        let text = "### a\n";
        let s = starts(text);
        assert!(heading_plan(text, &s, 0..1, 3).is_empty());
    }

    #[test]
    fn bullet_toggles_over_a_selection() {
        let text = "one\ntwo\n";
        let s = starts(text);
        let on = apply(text, line_prefix_plan(text, &s, 0..2, LinePrefix::Bullet));
        assert_eq!(on, "- one\n- two\n");
        let s2 = starts(&on);
        assert_eq!(
            apply(&on, line_prefix_plan(&on, &s2, 0..2, LinePrefix::Bullet)),
            "one\ntwo\n"
        );
    }

    #[test]
    fn a_mixed_selection_gains_the_prefix_everywhere() {
        let text = "- one\ntwo\n";
        let s = starts(text);
        assert_eq!(
            apply(text, line_prefix_plan(text, &s, 0..2, LinePrefix::Bullet)),
            "- one\n- two\n"
        );
    }

    #[test]
    fn numbered_list_numbers_from_one_and_replaces_bullets() {
        let text = "- one\ntwo\n";
        let s = starts(text);
        assert_eq!(
            apply(text, line_prefix_plan(text, &s, 0..2, LinePrefix::Ordered)),
            "1. one\n2. two\n"
        );
    }

    #[test]
    fn blockquote_toggles_one_level() {
        let text = "a\nb\n";
        let s = starts(text);
        let on = apply(text, line_prefix_plan(text, &s, 0..2, LinePrefix::Quote));
        assert_eq!(on, "> a\n> b\n");
        let s2 = starts(&on);
        assert_eq!(
            apply(&on, line_prefix_plan(&on, &s2, 0..2, LinePrefix::Quote)),
            "a\nb\n"
        );
    }

    #[test]
    fn a_blank_line_in_a_multi_line_selection_is_left_alone() {
        let text = "a\n\nb\n";
        let s = starts(text);
        assert_eq!(
            apply(text, line_prefix_plan(text, &s, 0..3, LinePrefix::Bullet)),
            "- a\n\n- b\n"
        );
    }

    #[test]
    fn paragraph_spans_to_the_blank_lines() {
        let text = "one\n\ntwo\nthree\n\nfour\n";
        let s = starts(text);
        let span = paragraph_span(text, &s, 2..3);
        assert_eq!(&text[span.clone()], "two\nthree\n");
    }

    #[test]
    fn the_last_paragraph_takes_the_newline_before_it() {
        let text = "one\n\ntwo\n";
        let s = starts(text);
        let span = paragraph_span(text, &s, 2..3);
        assert_eq!(&text[span.clone()], "\ntwo\n");
    }

    #[test]
    fn swapping_moves_one_line_each_way() {
        let text = "a\nb\nc\n";
        let s = starts(text);
        let ((range, new), shift) =
            swap_lines_plan(text, &s, 1..2, true).expect("has a line above");
        assert_eq!(apply(text, vec![(range, new)]), "b\na\nc\n");
        assert_eq!(shift, -2);
        let ((range, new), shift) =
            swap_lines_plan(text, &s, 0..1, false).expect("has a line below");
        assert_eq!(apply(text, vec![(range, new)]), "b\na\nc\n");
        assert_eq!(shift, 2);
    }

    #[test]
    fn swapping_with_an_unterminated_last_line_keeps_the_terminator_last() {
        let text = "a\nb";
        let s = starts(text);
        let ((range, new), shift) = swap_lines_plan(text, &s, 0..1, false).expect("has a line");
        assert_eq!(apply(text, vec![(range, new)]), "b\na");
        assert_eq!(shift, 2);
        let ((range, new), shift) = swap_lines_plan(text, &s, 1..2, true).expect("has a line");
        assert_eq!(apply(text, vec![(range, new)]), "b\na");
        assert_eq!(shift, -2);
    }

    #[test]
    fn swapping_at_the_edges_writes_nothing() {
        let text = "a\nb\n";
        let s = starts(text);
        assert!(swap_lines_plan(text, &s, 0..1, true).is_none());
        assert!(swap_lines_plan(text, &s, 1..2, false).is_none());
    }

    #[test]
    fn link_insertions_put_the_caret_where_the_target_is_typed() {
        assert_eq!(link_insertion("", LinkSyntax::Markdown), ("[]()".into(), 1));
        assert_eq!(
            link_insertion("text", LinkSyntax::Markdown),
            ("[text]()".into(), 7)
        );
        assert_eq!(link_insertion("", LinkSyntax::Wikilink), ("[[]]".into(), 2));
        assert_eq!(
            link_insertion("Note", LinkSyntax::Embed),
            ("![[Note]]".into(), 7)
        );
    }
}
