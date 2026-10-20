//! The one line of context a search result shows per match.
//!
//! [`match_lines`] groups the hit ranges of a note by line and windows each
//! line around its first hit, the way Obsidian's result rows do: a few words
//! of lead-in, an ellipsis when the line was cut, and the hit offsets
//! rewritten against the snippet so the row can highlight them.
//!
//! Every offset here is a byte offset into text a person wrote, so none of it
//! may be reached by arithmetic alone: a window edge lands mid-character as
//! soon as the prose holds an emoji or a non-breaking space, which pasted web
//! text does constantly. The edges are snapped with [`floor`] and [`ceil`]
//! and the lead-in advances by the width of the character it found.

use std::ops::Range;

/// One match line shown under a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchLine {
    /// Byte offset of the line in the note, for scrolling.
    pub offset: usize,
    /// The line trimmed and windowed around the first hit.
    pub snippet: String,
    /// Hit ranges inside `snippet`.
    pub ranges: Vec<Range<usize>>,
}

/// What the snippet starts with when the line was cut on its left.
const ELLIPSIS: &str = "…";

/// Clamps `at` into `text` and rounds it down to a character boundary.
fn floor(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Clamps `at` into `text` and rounds it up to a character boundary.
fn ceil(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

/// Groups hit ranges by line and windows each line around its first hit.
pub fn match_lines(text: &str, ranges: &[Range<usize>]) -> Vec<MatchLine> {
    const BEFORE: usize = 40;
    const WIDTH: usize = 160;
    let mut out: Vec<MatchLine> = Vec::new();
    let mut line_start = 0;
    let mut rix = 0;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let mut hits: Vec<Range<usize>> = Vec::new();
        while rix < ranges.len() && ranges[rix].start < line_end {
            let r = &ranges[rix];
            // A stale result over a changed note can carry a range that no
            // longer falls on a boundary; widen it to the characters it
            // covers rather than splitting one.
            let s = floor(body, r.start.saturating_sub(line_start));
            let e = ceil(body, r.end.saturating_sub(line_start));
            if e > s {
                hits.push(s..e);
            }
            if r.end <= line_end {
                rix += 1;
            } else {
                break;
            }
        }
        if !hits.is_empty() {
            // Window: a few words before the first hit, then up to WIDTH bytes.
            let first = hits[0].start;
            let mut start = floor(body, first.saturating_sub(BEFORE));
            if start > 0 {
                // Resume at the character after the whitespace nearest the
                // left edge. That whitespace is not always one byte wide —
                // NBSP and the typographic spaces arrive with pasted prose —
                // so its own width is what moves the offset.
                if let Some((at, ws)) = body[start..first]
                    .char_indices()
                    .find(|(_, c)| c.is_whitespace())
                {
                    start += at + ws.len_utf8();
                }
            }
            let end = ceil(body, start + WIDTH);
            let window = &body[start..end];
            let snippet = window.trim_start();
            let base = start + (window.len() - snippet.len());
            let (prefix, shift) = if start > 0 {
                (ELLIPSIS, ELLIPSIS.len())
            } else {
                ("", 0)
            };
            out.push(MatchLine {
                offset: line_start,
                snippet: format!("{prefix}{snippet}"),
                ranges: hits
                    .iter()
                    .filter(|h| h.start >= base && h.end <= end)
                    .map(|h| h.start - base + shift..h.end - base + shift)
                    .collect(),
            });
        }
        line_start = line_end;
        if rix >= ranges.len() {
            break;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    /// The snippet and the text under each of its hit ranges, which is what a
    /// result row draws; slicing it here is the boundary assertion.
    fn shown(text: &str, needle: &str) -> Vec<(String, Vec<String>)> {
        let at = text.find(needle).expect("needle in text");
        match_lines(text, &[at..at + needle.len()])
            .into_iter()
            .map(|m| {
                let hits = m
                    .ranges
                    .iter()
                    .map(|r| m.snippet[r.clone()].to_string())
                    .collect();
                (m.snippet, hits)
            })
            .collect()
    }

    #[test]
    fn lines_window_around_the_first_hit() {
        let text = "alpha beta\ngamma delta epsilon\nzeta";
        let g = text.find("delta").unwrap();
        let z = text.find("zeta").unwrap();
        let lines = match_lines(text, &[g..g + 5, g + 6..g + 7, z..z + 4]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].offset, 11);
        assert_eq!(lines[0].snippet, "gamma delta epsilon");
        assert_eq!(lines[0].ranges, vec![6..11, 12..13]);
        assert_eq!(lines[1].snippet, "zeta");
        let long = format!("{} needle {}", "x".repeat(100), "y".repeat(300));
        let n = long.find("needle").unwrap();
        let lines = match_lines(&long, &[n..n + 6]);
        assert!(lines[0].snippet.starts_with('…'));
        assert_eq!(&lines[0].snippet[lines[0].ranges[0].clone()], "needle");
        assert!(lines[0].snippet.len() < 200);
    }

    #[test]
    fn a_multibyte_space_before_the_hit_does_not_split_a_character() {
        // Each of these puts a wider-than-one-byte space just before the hit
        // and far enough in that the window starts inside the lead-in: a
        // non-breaking space (what pasted web text is full of), a thin space,
        // and the CJK ideographic space.
        for space in ['\u{a0}', '\u{2009}', '\u{3000}'] {
            let text = format!("{}{space}needle tail", "a".repeat(45));
            let rows = shown(&text, "needle");
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].1, vec!["needle"], "space {:?}", space);
            assert!(rows[0].0.starts_with('…'), "space {:?}", space);
        }
    }

    #[test]
    fn every_alignment_of_a_multibyte_lead_in_is_safe() {
        // The window opens a fixed number of bytes before the hit, so sliding
        // the line along one byte at a time walks that edge through the middle
        // of every character in the lead-in: an emoji, a CJK ideograph, a
        // combining accent and a non-breaking space.
        let filler = "\u{1f642} \u{65e5}\u{672c}\u{8a9e} e\u{301}cole\u{a0}mot ";
        for pad in 0..filler.len() + 8 {
            let text = format!("{}{}needle tail", "x".repeat(pad), filler.repeat(4));
            let rows = shown(&text, "needle");
            assert_eq!(rows.len(), 1, "pad {pad}");
            assert_eq!(rows[0].1, vec!["needle"], "pad {pad}");
        }
    }

    #[test]
    fn a_long_multibyte_line_is_cut_on_a_boundary() {
        // No ASCII anywhere, so both edges of the window land mid-character
        // unless they are snapped.
        let text = format!("{}needle{}", "\u{65e5}".repeat(100), "\u{672c}".repeat(300));
        let rows = shown(&text, "needle");
        assert_eq!(rows[0].1, vec!["needle"]);
        assert!(rows[0].0.starts_with('\u{2026}'));
    }

    #[test]
    fn a_hit_range_off_a_boundary_widens_instead_of_panicking() {
        // What a stale result over a changed note looks like: the range cuts
        // the three-byte character in half at both ends.
        let text = "日本語";
        let lines = match_lines(text, &[1..5]);
        assert_eq!(lines[0].snippet, text);
        assert_eq!(&lines[0].snippet[lines[0].ranges[0].clone()], "日本");
    }
}
