//! Syntax highlighting for fenced code, compatible with Obsidian's.
//!
//! Obsidian highlights a rendered code block with Prism.js, so a community
//! theme styles `.token.keyword` and the **token names are the compatibility
//! surface** — the argument `docs/silica/Specification.md` makes for this
//! axis. This crate is therefore a port of Prism rather than a wrapper round
//! another highlighter: [`engine`] is its tokenizer, [`languages`] its
//! grammars, ported from the same upstream source, and [`palette`] the table
//! saying which `--code-*` of the theme registry each token name is painted
//! from.
//!
//! The entry point is [`highlight`], which both the reading view and the
//! editor paint from. A language with no ported grammar comes back as no
//! runs at all, which renders the block plain — what Obsidian does for a
//! fence whose language Prism has not got.
//!
//! A grammar is a `static` of ordered entries: an entry is a token name plus
//! one or more patterns, each a regex source plus flags. The order is
//! load-bearing — `comment` before `string` is why a `//` inside a string is
//! not a comment — so the entries keep the order they were written in, and
//! every entry is `'static` because a compiled regex table is keyed by the
//! grammar's address and lives for the process.
//!
//! The patterns below are Prism's own, ported key for key from
//! `components/prism-*.js`. JavaScript's regex flags become inline groups:
//! `/…/i` is `(?i)…`, `/…/m` is `(?m)…`. Prism's `lookbehind` becomes the
//! first capture group (`LB`), its `greedy` flag matches against the whole
//! text rather than one unclaimed run (`GR`), `inside` nests another grammar,
//! and `alias` adds class names after the type.

pub mod compiled;
pub mod engine;
pub mod grammar;
pub mod languages;
pub mod palette;

pub use engine::{spans, to_html, tokenize, Node, Span, Token};
pub use languages::{grammar as grammar_for, LANGUAGES};
pub use palette::{style_of, Paint, Style};

/// A run of code to paint: a byte range and how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

/// The runs to paint `code` with, for the fence's info string.
///
/// The info string is resolved the way Obsidian resolves it: the first
/// word, lowercased ([[Specification#Observed code block copy behaviour]]),
/// and a name no grammar is ported for gives no runs.
///
/// Runs are returned **flat and in painting order**: a nested token comes
/// after the one that holds it, so a painter that applies them in sequence
/// ends on the innermost, which is how the cascade resolves a nested span.
/// Runs whose style is [`Paint::Normal`] with no emphasis are dropped — they
/// would paint what the block already paints.
pub fn highlight(code: &str, info: Option<&str>) -> Vec<Run> {
    let Some(grammar) = language_of(info).and_then(|n| languages::grammar(&n)) else {
        return Vec::new();
    };
    let nodes = engine::tokenize(code, grammar);
    engine::spans(&nodes)
        .into_iter()
        .map(|s| Run {
            start: s.start,
            end: s.end,
            style: palette::style_of(s.kind, s.alias),
        })
        .filter(|r| r.style.paint != palette::Paint::Normal || r.style.bold || r.style.italic)
        .collect()
}

/// The language a fence's info string names: its first word, lowercased.
///
/// Obsidian takes the first word and lowercases it, and a word holding
/// anything but `[A-Za-z0-9_-]` does not resolve at all — `c#` and `C++`
/// are `language-none` (probed 2026-10-29,
/// [[Specification#Observed code block copy behaviour, Obsidian 1.14.5]]).
pub fn language_of(info: Option<&str>) -> Option<String> {
    let word = info?.split_whitespace().next()?;
    if word.is_empty()
        || !word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some(word.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_when_no_language() {
        let runs = highlight("hello world", None);
        assert!(runs.is_empty(), "no language should produce no runs");
    }

    #[test]
    fn unknown_language_is_plain() {
        let runs = highlight("hello world", Some("notalanguage"));
        assert!(runs.is_empty(), "unknown language should produce no runs");
    }

    #[test]
    fn python_keywords() {
        let runs = highlight("def foo():\n    pass", Some("python"));
        assert!(
            runs.iter().any(|r| r.style.paint == Paint::Keyword),
            "expected a keyword run, got {:?}",
            runs
        );
    }

    #[test]
    fn rust_keywords() {
        let runs = highlight("fn main() {}", Some("rust"));
        assert!(
            runs.iter().any(|r| r.style.paint == Paint::Keyword),
            "expected a keyword run, got {:?}",
            runs
        );
    }

    #[test]
    fn unported_language_is_plain() {
        let runs = highlight("const x = 'hello';", Some("cobol"));
        assert!(runs.is_empty(), "unported language should produce no runs");
    }

    #[test]
    fn javascript_keywords() {
        let runs = highlight("const x = 'hello';", Some("javascript"));
        assert!(
            runs.iter().any(|r| r.style.paint == Paint::Keyword),
            "expected a keyword run, got {:?}",
            runs
        );
    }

    #[test]
    fn every_grammar_compiles() {
        for (_name, _aliases, grammar) in LANGUAGES {
            let _ = compiled::compile(grammar);
        }
    }
}
