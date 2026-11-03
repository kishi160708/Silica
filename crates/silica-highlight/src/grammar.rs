//! A Prism grammar, in Rust.
//!
//! Prism's grammars are plain JavaScript objects: an **ordered** map from a
//! token name to one pattern or a list of them, each a regex plus four
//! flags. The order is load-bearing — `comment` before `string` is why a
//! `//` inside a string is not a comment — so the entries keep the order
//! they were written in, and every entry is `'static` because the compiled
//! regex table is keyed by the grammar's address and lives for the process.
//!
//! The flags are Prism's own:
//!
//! - [`LB`] (`lookbehind`) — not a regex lookbehind. The pattern's **first
//!   capture group** is context that must match but is not part of the
//!   token, which is how Prism writes a lookbehind without needing one.
//! - [`GR`] (`greedy`) — the pattern matches against the whole text rather
//!   than one unclaimed run, so it may reach across tokens already made.
//!   Strings and comments are greedy, which is how they win over what is
//!   inside them.
//! - `inside` — a grammar for the matched text, giving nested tokens.
//! - `alias` — extra class names after the type (`boolean important`).
//!
//! A grammar is written with the four constructors below, which are `const`
//! so a language is a `static` and costs nothing until it is used:
//!
//! ```ignore
//! pub static JSON: Grammar = Grammar::new(&[
//!     e("property", &[pf(&[STRING, r"(?=\s*:)"], LB | GR)]),
//!     e("number", &[p(&[r"-?\b\d+(?:\.\d+)?(?:e[+-]?\d+)?\b"])]),
//! ]);
//! ```

/// `lookbehind`: the first capture group is context, not token.
pub const LB: u8 = 1;
/// `greedy`: match against the whole text, across tokens already made.
pub const GR: u8 = 2;

/// One pattern of a grammar entry.
pub struct Pattern {
    /// The source, in JavaScript's regex syntax, as the pieces it is built
    /// from — Prism composes several of these out of shared fragments, and
    /// a slice keeps a port able to do the same.
    ///
    /// A grammar is pure data: a `static` holding any interior mutability
    /// cannot be const-promoted at all, so the compiled regex lives in
    /// [`crate::compiled`], which memoises one `Compiled` per grammar.
    pub source: &'static [&'static str],
    pub flags: u8,
    pub alias: &'static [&'static str],
    pub inside: Option<&'static Grammar>,
}

impl std::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pattern")
            .field("source", &self.source())
            .field("flags", &self.flags)
            .field("alias", &self.alias)
            .finish_non_exhaustive()
    }
}

impl Pattern {
    /// The pattern's source, joined.
    pub fn source(&self) -> String {
        self.source.concat()
    }

    pub fn lookbehind(&self) -> bool {
        self.flags & LB != 0
    }

    pub fn greedy(&self) -> bool {
        self.flags & GR != 0
    }
}

/// An entry of a grammar: a token name and the patterns tried for it, in
/// order.
pub struct Entry {
    pub kind: &'static str,
    pub patterns: &'static [Pattern],
}

/// An ordered map from token name to patterns.
///
/// `rest` is Prism's own escape hatch: a grammar that ends with another
/// grammar's entries appended. Prism splices it in before matching, and so
/// does [`Grammar::entries`].
pub struct Grammar {
    pub entries: &'static [Entry],
    pub rest: Option<&'static Grammar>,
}

impl Grammar {
    pub const fn new(entries: &'static [Entry]) -> Grammar {
        Grammar {
            entries,
            rest: None,
        }
    }

    /// A grammar that ends with `rest`'s entries, which is Prism's `rest`.
    pub const fn with_rest(entries: &'static [Entry], rest: &'static Grammar) -> Grammar {
        Grammar {
            entries,
            rest: Some(rest),
        }
    }

    /// Every `(kind, patterns)` in order, `rest` last, which is where
    /// Prism's `tokenize` puts it.
    pub fn entries(&self) -> impl Iterator<Item = (&'static str, &'static [Pattern])> + '_ {
        self.entries.iter().map(|e| (e.kind, e.patterns)).chain(
            self.rest
                .into_iter()
                .flat_map(|r| r.entries.iter().map(|e| (e.kind, e.patterns))),
        )
    }
}

// ---- the four constructors ------------------------------------------------

/// An entry: a token name and its patterns.
pub const fn e(kind: &'static str, patterns: &'static [Pattern]) -> Entry {
    Entry { kind, patterns }
}

/// A plain pattern.
pub const fn p(source: &'static [&'static str]) -> Pattern {
    Pattern {
        source,
        flags: 0,
        alias: &[],
        inside: None,
    }
}

/// A pattern with flags: `LB`, `GR` or both.
pub const fn pf(source: &'static [&'static str], flags: u8) -> Pattern {
    Pattern {
        source,
        flags,
        alias: &[],
        inside: None,
    }
}

/// A pattern with an alias, which goes in the class list after the type.
pub const fn pa(
    source: &'static [&'static str],
    flags: u8,
    alias: &'static [&'static str],
) -> Pattern {
    Pattern {
        source,
        flags,
        alias,
        inside: None,
    }
}

/// A pattern whose match is tokenized again by `inside`.
pub const fn pi(source: &'static [&'static str], flags: u8, inside: &'static Grammar) -> Pattern {
    Pattern {
        source,
        flags,
        alias: &[],
        inside: Some(inside),
    }
}

/// A pattern with both an alias and an `inside`.
pub const fn pai(
    source: &'static [&'static str],
    flags: u8,
    alias: &'static [&'static str],
    inside: &'static Grammar,
) -> Pattern {
    Pattern {
        source,
        flags,
        alias,
        inside: Some(inside),
    }
}
