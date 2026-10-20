//! Auto-pairing: the closer a typed bracket or Markdown marker brings with
//! it, the closer a typed one steps over instead of doubling, the pair a
//! Backspace takes away whole, and the pair a selection gets wrapped in.
//!
//! Every rule here is one the 2026-10-18 probe observed against Obsidian
//! 1.14.4 ("Observed auto-pair behavior" in the specification). The
//! characters and positions that probe left open are deliberately inert:
//! pairing writes bytes into the note that the user did not type, so a
//! guessed rule injects a stray `)` or `*` into prose and the vault keeps
//! it.

use crate::buffer::{char_class, prev_grapheme_boundary};
use std::ops::Range;

/// Obsidian's two `app.json` auto-pair toggles. Both read back `true` on a
/// vault that never touched them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// `autoPairBrackets`: `(`, `[`, `{`, `"` and `'`.
    pub brackets: bool,
    /// `autoPairMarkdown`: the backtick, `*` and `_`, plus the four
    /// characters that only ever wrap.
    pub markdown: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            brackets: true,
            markdown: true,
        }
    }
}

/// What a typed pair character does at one selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Nothing special: write the character the way any other key writes
    /// one, replacing whatever is selected.
    Insert,
    /// Write the character and this closer, leaving the cursor between
    /// them.
    Close(char),
    /// Write nothing and step over the closer already ahead of the cursor.
    Skip,
    /// Put these either side of the selection, which survives over the
    /// text between them.
    Wrap(char, char),
}

/// Which toggle owns a pair character, and where it closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `(`, `[`, `{`: these close unless a word character follows.
    Bracket,
    /// `"` and `'`: the bracket toggle owns them, but they were seen to
    /// close only where the Markdown markers do.
    Quote,
    /// The backtick, `*` and `_`.
    Marker,
    /// `~`, `=`, `$` and `%`, which close in no position under either
    /// toggle and only ever wrap a selection.
    WrapOnly,
}

impl Kind {
    fn enabled(self, opts: Options) -> bool {
        match self {
            Kind::Bracket | Kind::Quote => opts.brackets,
            // The probe ran the selection rows with both toggles on, so
            // which one owns `~ = $ %` is a judgement call; their wraps
            // are all Markdown syntax, so the Markdown toggle gets them.
            Kind::Marker | Kind::WrapOnly => opts.markdown,
        }
    }
}

/// The closer `open` brings with it. `<` is absent on purpose: it was the
/// one character observed to replace a selection rather than wrap it.
fn pair(open: char) -> Option<(char, Kind)> {
    Some(match open {
        '(' => (')', Kind::Bracket),
        '[' => (']', Kind::Bracket),
        '{' => ('}', Kind::Bracket),
        '"' => ('"', Kind::Quote),
        '\'' => ('\'', Kind::Quote),
        '`' => ('`', Kind::Marker),
        '*' => ('*', Kind::Marker),
        '_' => ('_', Kind::Marker),
        '~' => ('~', Kind::WrapOnly),
        '=' => ('=', Kind::WrapOnly),
        '$' => ('$', Kind::WrapOnly),
        '%' => ('%', Kind::WrapOnly),
        _ => return None,
    })
}

/// The opener a closing bracket belongs to.
fn opener(close: char) -> Option<char> {
    Some(match close {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => return None,
    })
}

/// What typing `typed` over `sel` does. `sel` is a byte range into `text`,
/// empty for a bare cursor.
pub fn act(text: &str, sel: Range<usize>, typed: char, opts: Options) -> Act {
    if !sel.is_empty() {
        return match pair(typed) {
            Some((close, kind)) if kind.enabled(opts) => Act::Wrap(typed, close),
            _ => Act::Insert,
        };
    }
    let at = sel.start;
    let next = text[at..].chars().next();
    // A closing bracket typed over one already there steps past it. Which
    // half of that rule Obsidian uses — that the editor planted this
    // closer, or that an opener before the cursor is still unmatched — the
    // probe could not separate, and the unmatched opener is the half that
    // needs no bookkeeping: typing `)` at `x|)` in a document with no `(`
    // anywhere was observed to write the character, not skip it.
    if opts.brackets && next == Some(typed) {
        if let Some(open) = opener(typed) {
            if unmatched(text, at, open, typed) {
                return Act::Skip;
            }
        }
    }
    let Some((close, kind)) = pair(typed) else {
        return Act::Insert;
    };
    if !kind.enabled(opts) {
        return Act::Insert;
    }
    // A second quote or marker steps over the closer the first one left
    // behind; a second bracket nests instead, which falls out of the
    // closing rule below.
    if matches!(kind, Kind::Quote | Kind::Marker) && next == Some(typed) {
        return Act::Skip;
    }
    if closes(text, at, kind) {
        Act::Close(close)
    } else {
        Act::Insert
    }
}

/// Whether a pair of `kind` closes at `at`.
fn closes(text: &str, at: usize, kind: Kind) -> bool {
    let next = text[at..].chars().next();
    match kind {
        Kind::Bracket => next.is_none_or(|c| char_class(c) != 1),
        // Quotes and markers were seen to close only at the end of a line
        // whose preceding character is a space — a word character, and a
        // bare `=`, both suppress them. Which half of that pair of
        // conditions Obsidian really tests is unprobed, so both are
        // required: closing too rarely costs a keystroke, closing too
        // often writes a stray marker into the note.
        Kind::Quote | Kind::Marker => {
            let prev = text[..at].chars().next_back();
            let after_space = prev.is_none_or(char::is_whitespace);
            let at_end_of_line = next.is_none_or(|c| c == '\n');
            after_space && at_end_of_line
        }
        Kind::WrapOnly => false,
    }
}

/// Whether an `open` before `at` is still waiting for its `close`.
fn unmatched(text: &str, at: usize, open: char, close: char) -> bool {
    let mut depth = 0i32;
    for c in text[..at].chars() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
        }
    }
    depth > 0
}

/// What a Backspace at `at` deletes: an empty pair either side of the
/// cursor goes whole, anything else loses one grapheme cluster. The pair
/// need not be one the editor inserted — a pasted `(|)` was observed to go
/// the same way.
pub fn backspace_range(text: &str, at: usize, opts: Options) -> Range<usize> {
    let start = prev_grapheme_boundary(text, at);
    if start == at {
        return at..at;
    }
    let Some((close, kind)) = lone_char(&text[start..at]).and_then(pair) else {
        return start..at;
    };
    // `~ = $ %` never plant a closer, so there is no pair of theirs to
    // take away.
    if kind == Kind::WrapOnly || !kind.enabled(opts) {
        return start..at;
    }
    match text[at..].chars().next() {
        Some(c) if c == close => start..at + c.len_utf8(),
        _ => start..at,
    }
}

/// The one character a cluster is made of, if it is made of one: a `(`
/// carrying a combining mark is its own cluster and no half of a pair.
fn lone_char(cluster: &str) -> Option<char> {
    let mut chars = cluster.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act_at(text: &str, at: usize, typed: char) -> Act {
        act(text, at..at, typed, Options::default())
    }

    /// The probe's five positions, for the three brackets: they close
    /// unless a word character follows.
    #[test]
    fn brackets_close_unless_a_word_character_follows() {
        for open in ['(', '[', '{'] {
            let close = pair(open).unwrap().0;
            assert_eq!(act_at("word", 0, open), Act::Insert, "|word {open}");
            assert_eq!(act_at("dont", 3, open), Act::Insert, "don|t {open}");
            assert_eq!(act_at("word", 4, open), Act::Close(close), "word| {open}");
            assert_eq!(act_at("say ", 4, open), Act::Close(close), "say | {open}");
            assert_eq!(act_at("()", 1, open), Act::Close(close), "(|) {open}");
        }
        // Mid-line: a space ahead is enough, whatever came before.
        assert_eq!(act_at("para text here", 4, '('), Act::Close(')'));
        assert_eq!(act_at("x = 1", 3, '('), Act::Close(')'));
    }

    /// Quotes and markers share one rule, and it is the narrow one.
    #[test]
    fn quotes_and_markers_close_only_after_a_space_at_end_of_line() {
        for open in ['"', '\'', '`', '*', '_'] {
            assert_eq!(act_at("word", 0, open), Act::Insert, "|word {open}");
            assert_eq!(act_at("dont", 3, open), Act::Insert, "don|t {open}");
            assert_eq!(act_at("word", 4, open), Act::Insert, "word| {open}");
            assert_eq!(act_at("say ", 4, open), Act::Close(open), "say | {open}");
            assert_eq!(act_at("()", 1, open), Act::Insert, "(|) {open}");
        }
        assert_eq!(act_at("para text here", 4, '*'), Act::Insert);
        assert_eq!(act_at("x = 1", 3, '*'), Act::Insert);
        // An empty document is the start of a line and the end of one.
        assert_eq!(act_at("", 0, '*'), Act::Close('*'));
    }

    #[test]
    fn four_characters_never_close() {
        for c in ['~', '=', '$', '%', '<'] {
            assert_eq!(act_at("", 0, c), Act::Insert, "{c} on an empty line");
            assert_eq!(act_at("say ", 4, c), Act::Insert, "say |{c}");
        }
    }

    #[test]
    fn a_second_bracket_nests_and_a_second_marker_skips() {
        assert_eq!(act_at("()", 1, '('), Act::Close(')'));
        for open in ['"', '\'', '`', '*', '_'] {
            let two: String = [open, open].iter().collect();
            assert_eq!(act_at(&two, 1, open), Act::Skip, "{open}|{open}");
        }
        // A second `~` is just another character.
        assert_eq!(act_at("~~", 1, '~'), Act::Insert);
    }

    #[test]
    fn a_closer_skips_over_its_own_pair_but_not_a_planted_one() {
        assert_eq!(act_at("()", 1, ')'), Act::Skip);
        assert_eq!(act_at("[[]]", 2, ']'), Act::Skip);
        assert_eq!(act_at("{}", 1, '}'), Act::Skip);
        // No `(` anywhere, so the `)` is written.
        assert_eq!(act_at("x)", 1, ')'), Act::Insert);
        // The opener is already matched by a closer of its own.
        assert_eq!(act_at("()x)", 3, ')'), Act::Insert);
    }

    #[test]
    fn every_pair_character_wraps_a_selection_except_the_angle_bracket() {
        for (open, close) in [
            ('(', ')'),
            ('[', ']'),
            ('{', '}'),
            ('"', '"'),
            ('\'', '\''),
            ('`', '`'),
            ('*', '*'),
            ('_', '_'),
            ('~', '~'),
            ('=', '='),
            ('$', '$'),
            ('%', '%'),
        ] {
            let got = act("alpha beta", 0..5, open, Options::default());
            assert_eq!(got, Act::Wrap(open, close), "wrapping with {open}");
        }
        assert_eq!(
            act("alpha beta", 0..5, '<', Options::default()),
            Act::Insert
        );
    }

    #[test]
    fn each_toggle_owns_its_own_characters() {
        let brackets = Options {
            brackets: true,
            markdown: false,
        };
        let markdown = Options {
            brackets: false,
            markdown: true,
        };
        let off = Options {
            brackets: false,
            markdown: false,
        };
        assert_eq!(act("say ", 4..4, '(', brackets), Act::Close(')'));
        assert_eq!(act("say ", 4..4, '*', brackets), Act::Insert);
        assert_eq!(act("say ", 4..4, '(', markdown), Act::Insert);
        assert_eq!(act("say ", 4..4, '*', markdown), Act::Close('*'));
        for c in ['(', '[', '{', '"', '\'', '`', '*', '_'] {
            assert_eq!(act("say ", 4..4, c, off), Act::Insert, "{c} with both off");
            assert_eq!(
                act("a b", 0..1, c, off),
                Act::Insert,
                "{c} wrap with both off"
            );
        }
        // The skip is part of the feature, so it goes away with it.
        assert_eq!(act("()", 1..1, ')', off), Act::Insert);
        assert_eq!(act("\"\"", 1..1, '"', off), Act::Insert);
    }

    #[test]
    fn backspace_takes_an_empty_pair_whole() {
        let opts = Options::default();
        assert_eq!(backspace_range("()", 1, opts), 0..2);
        assert_eq!(backspace_range("**", 1, opts), 0..2);
        assert_eq!(backspace_range("[]", 1, opts), 0..2);
        // Content inside: only the opener goes.
        assert_eq!(backspace_range("(x)", 1, opts), 0..1);
        // A plain character loses one.
        assert_eq!(backspace_range("plain", 3, opts), 2..3);
        // Nothing before the cursor.
        assert_eq!(backspace_range("ab", 0, opts), 0..0);
        // Multi-byte characters still go one character at a time.
        assert_eq!(backspace_range("é", 2, opts), 0..2);
    }

    /// One Backspace takes a whole grapheme cluster, so an accent, a flag
    /// pair or a joined emoji goes with the character it belongs to.
    #[test]
    fn backspace_takes_a_grapheme_cluster_whole() {
        let opts = Options::default();
        assert_eq!(backspace_range("cafe\u{301}", 6, opts), 3..6);
        assert_eq!(backspace_range("\u{1f1eb}\u{1f1f7}", 8, opts), 0..8);
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
        assert_eq!(backspace_range(family, family.len(), opts), 0..family.len());
        // An opener carrying a combining mark is its own cluster, not half
        // of a pair, so the closer after it stays.
        assert_eq!(backspace_range("(\u{301})", 3, opts), 0..3);
    }

    #[test]
    fn backspace_leaves_pairs_alone_when_their_toggle_is_off() {
        let off = Options {
            brackets: false,
            markdown: false,
        };
        assert_eq!(backspace_range("()", 1, off), 0..1);
        assert_eq!(backspace_range("**", 1, off), 0..1);
        // `~ = $ %` never planted a closer, so they never take two.
        assert_eq!(backspace_range("~~", 1, Options::default()), 0..1);
    }
}
