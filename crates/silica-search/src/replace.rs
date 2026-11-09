//! Vault-wide find and replace: the query side that finds what to change,
//! and the replay side that writes the change back.
//!
//! A preview is a set of byte ranges into the note it was made from. The
//! rewrite does not trust them — it re-finds the pattern in the text it is
//! about to write and replaces what it finds there. A range that has moved
//! since the preview was made (an edit while the panel was open, or an index
//! a watcher tick behind the file) therefore cannot punch a hole in a note,
//! and a note that no longer holds the pattern is left exactly as it is.
//!
//! A literal find is escaped, so `.` matches a full stop and nothing else;
//! `regex` leaves the pattern to the engine. `$1`..`$9` in the replacement
//! are capture groups, `$&` the whole match and `$$` a literal `$`.

use crate::snippet::{match_lines, MatchLine};
use regex::{Regex, RegexBuilder};
use std::ops::Range;

/// One line of a preview: the line the match is on, and the windowed text
/// around it with the hit ranges rewritten into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewLine {
    /// 1-based line number in the note.
    pub line: usize,
    /// The trimmed line, windowed around its first hit.
    pub match_line: MatchLine,
}

impl PreviewLine {
    /// The text the row shows.
    pub fn snippet(&self) -> &str {
        &self.match_line.snippet
    }

    /// The hit ranges inside [`PreviewLine::snippet`].
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.match_line.ranges
    }
}

/// A find-and-replace compiled once and run over every note in the vault.
#[derive(Debug, Clone)]
pub struct FindReplace {
    re: Regex,
    with: String,
}

impl FindReplace {
    /// Compiles `find` (a literal, or a pattern under `regex`) matched
    /// case-insensitively unless `match_case`.
    ///
    /// `None` for an empty find, a pattern that will not compile, or a
    /// pattern that matches the empty string — an `a*` would otherwise put
    /// the replacement between every character of every note. An empty
    /// *replacement* is fine: that is a delete.
    pub fn new(find: &str, with: &str, match_case: bool, regex: bool) -> Option<FindReplace> {
        if find.is_empty() {
            return None;
        }
        let pattern = if regex {
            find.to_string()
        } else {
            regex::escape(find)
        };
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(!match_case)
            .multi_line(true)
            .size_limit(1 << 22)
            .build()
            .ok()?;
        if re.is_match("") {
            return None;
        }
        Some(FindReplace {
            re,
            with: with.to_string(),
        })
    }

    /// Every match in `text`, in order: non-overlapping, never empty, and
    /// byte ranges into `text`. Regex ranges always fall on character
    /// boundaries, so these can be sliced with.
    pub fn find(&self, text: &str) -> Vec<Range<usize>> {
        self.re
            .find_iter(text)
            .map(|m| m.range())
            .filter(|r| r.end > r.start)
            .collect()
    }

    /// How many matches `text` holds.
    pub fn count(&self, text: &str) -> usize {
        self.find(text).len()
    }

    /// The matches as the lines a preview shows: one per line that holds
    /// one, numbered, windowed the way a search result row is.
    pub fn preview(&self, text: &str) -> Vec<PreviewLine> {
        let hits = self.find(text);
        match_lines(text, &hits)
            .into_iter()
            .map(|match_line| PreviewLine {
                line: line_of(text, match_line.offset),
                match_line,
            })
            .collect()
    }

    /// The text as it reads with every match replaced, or `None` when
    /// nothing matched. The pattern is re-found here rather than replayed
    /// from the ranges a preview was made with, so a stale range cannot
    /// reach the file being written.
    pub fn rewrite(&self, text: &str) -> Option<String> {
        let matches = self.find(text);
        if matches.is_empty() {
            return None;
        }
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for m in matches {
            out.push_str(&text[last..m.start]);
            out.push_str(&self.expansion(text, &m));
            last = m.end;
        }
        if last < text.len() {
            out.push_str(&text[last..]);
        }
        Some(out)
    }

    /// What one match becomes: the replacement, with its `$` groups filled
    /// in from `text`. A `$` naming a group the pattern has none of is
    /// dropped, as the editor's own replace drops it.
    fn expansion(&self, text: &str, m: &Range<usize>) -> String {
        if !self.with.contains('$') {
            return self.with.clone();
        }
        let Some(caps) = self
            .re
            .captures_at(text, m.start)
            .filter(|c| c.get(0).is_some_and(|g| g.range() == *m))
        else {
            return self.with.clone();
        };
        let mut out = String::with_capacity(self.with.len());
        let mut chars = self.with.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('$') => {
                    chars.next();
                    out.push('$');
                }
                Some('&') => {
                    chars.next();
                    out.push_str(&text[m.clone()]);
                }
                Some(d) if d.is_ascii_digit() => {
                    chars.next();
                    let n = d.to_digit(10).unwrap_or(0) as usize;
                    if let Some(g) = caps.get(n) {
                        out.push_str(g.as_str());
                    }
                }
                _ => out.push('$'),
            }
        }
        out
    }
}

/// The 1-based line number the byte offset `at` falls on.
fn line_of(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    text[..at].bytes().filter(|b| *b == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_is_escaped_and_case_insensitive_by_default() {
        let fr = FindReplace::new("a.b", "x", false, false).expect("compiles");
        let text = "a.b axb A.B";
        // * The `.` in the find is a full stop, so `axb` is not a hit; both
        //   spellings of it are, unless case matters.
        assert_eq!(fr.find(text), vec![0..3, 8..11]);
        assert_eq!(fr.rewrite(text).unwrap(), "x axb x");
        // * Case sensitive: only the one spelling.
        let cs = FindReplace::new("a.b", "x", true, false).expect("compiles");
        assert_eq!(cs.count("a.b axb A.B"), 1);
    }

    #[test]
    fn a_regex_uses_its_capture_groups() {
        let fr = FindReplace::new(r"(\w+)-(\d+)", "$2/$1", false, true).expect("compiles");
        let text = "fix alpha-12 and beta-34";
        assert_eq!(fr.count(text), 2);
        assert_eq!(fr.rewrite(text).unwrap(), "fix 12/alpha and 34/beta");
        // * A group the pattern has none of is dropped, as the editor's own
        //   replace does; `$$` and `$&` mean what they do everywhere else.
        let keep = FindReplace::new(r"\d+", "[$9]", false, true).expect("compiles");
        assert_eq!(keep.rewrite("a 1 b").unwrap(), "a [] b");
        let both = FindReplace::new(r"\d+", "$$&$$", false, true).expect("compiles");
        assert_eq!(both.rewrite("a 1 b").unwrap(), "a $&$ b");
    }

    #[test]
    fn an_empty_find_or_one_that_matches_nothing_is_refused() {
        assert!(FindReplace::new("", "x", false, false).is_none());
        assert!(FindReplace::new("   ", "x", false, false).is_some());
        // * A pattern matching the empty string would write the replacement
        //   between every character.
        assert!(FindReplace::new("a*", "x", false, true).is_none());
        // * An empty replacement is a delete, and is fine.
        let del = FindReplace::new("gone", "", false, false).expect("compiles");
        assert_eq!(del.rewrite("is gone now").unwrap(), "is  now");
    }

    #[test]
    fn a_note_without_the_pattern_is_left_alone() {
        let fr = FindReplace::new("needle", "pin", false, false).expect("compiles");
        let text = "nothing here at all";
        assert_eq!(fr.find(text), Vec::<Range<usize>>::new());
        assert_eq!(fr.preview(text), Vec::<PreviewLine>::new());
        assert_eq!(fr.rewrite(text), None, "no match is no write");
    }

    #[test]
    fn a_preview_shows_the_line_and_the_context_around_it() {
        let fr = FindReplace::new("mango", "fruit", false, false).expect("compiles");
        let text = "title\n\none mango here\ntail\nmango again\n";
        let rows = fr.preview(text);
        assert_eq!(rows.len(), 2, "one row per line that holds a match");
        assert_eq!(rows[0].line, 3);
        assert_eq!(rows[0].snippet(), "one mango here");
        assert_eq!(&rows[0].snippet()[rows[0].ranges()[0].clone()], "mango");
        assert_eq!(rows[1].line, 5);
        // * A long line is windowed, and the hit still reads inside the
        //   window it was found in.
        let long = format!("{}mango tail", "x".repeat(200));
        let rows = fr.preview(&long);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].snippet().starts_with('\u{2026}'));
        assert_eq!(&rows[0].snippet()[rows[0].ranges()[0].clone()], "mango");
    }

    #[test]
    fn a_multibyte_note_is_rewritten_without_splitting_a_character() {
        let fr = FindReplace::new("\u{65e5}", "\u{672c}", false, false).expect("compiles");
        let text = "\u{65e5}\u{672c}\u{8a9e} \u{65e5}\u{672c}\u{8a9e}";
        let out = fr.rewrite(text).expect("matches");
        assert_eq!(out, "\u{672c}\u{672c}\u{8a9e} \u{672c}\u{672c}\u{8a9e}");
        assert!(out.is_char_boundary(out.find('\u{8a9e}').unwrap()));
    }

    #[test]
    fn a_rewrite_replaces_every_match_and_leaves_the_rest() {
        let fr = FindReplace::new("ab", "AB", false, false).expect("compiles");
        assert_eq!(fr.rewrite("ab abab ab").unwrap(), "AB ABAB AB");
        // * One match per position: matches never overlap.
        let overlapping = FindReplace::new("aa", "b", false, false).expect("compiles");
        assert_eq!(overlapping.count("aaaa"), 2);
        assert_eq!(overlapping.rewrite("aaaa").unwrap(), "bb");
    }

    #[test]
    fn a_stale_range_does_not_reach_the_rewrite() {
        // * The ranges a preview was made with point into a text that has
        //   since changed; the rewrite re-finds the pattern instead.
        let fr = FindReplace::new("mango", "fruit", false, false).expect("compiles");
        let before = "one mango here\n";
        let stale = fr.find(before);
        let after = "one MANGO here, mango\n";
        // * The old ranges, applied to the new text, would land nowhere
        //   near a match; the rewrite answers from the new text alone.
        let out = fr.rewrite(after).expect("matches");
        assert_eq!(out, "one fruit here, fruit\n");
        assert_eq!(stale.len(), 1);
    }
}
