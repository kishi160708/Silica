//! A grammar with its regexes compiled, memoised per grammar.
//!
//! The grammars in [`crate::languages`] are pure data, because a `static`
//! that holds any interior mutability cannot be const-promoted — so the
//! compiled form lives here instead, built on first use and kept for the
//! life of the process.
//!
//! A grammar is keyed by its address, which also settles the recursion:
//! Prism's grammars refer to themselves (a Nix interpolation holds Nix, a
//! JavaScript template string holds JavaScript), so a `Compiled` keeps its
//! children as the *data* pointer and resolves each through this table when
//! the engine descends into it. Compiling eagerly would not terminate.

use crate::grammar::{Grammar, Pattern};
use fancy_regex::Regex;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// One pattern, compiled.
pub struct CPattern {
    /// `None` when the source did not compile. A bad port then matches
    /// nothing rather than panicking, and `every_pattern_compiles` catches
    /// it in the tests instead.
    pub re: Option<Regex>,
    pub flags: u8,
    pub alias: &'static [&'static str],
    pub inside: Option<&'static Grammar>,
}

impl CPattern {
    pub fn lookbehind(&self) -> bool {
        self.flags & crate::grammar::LB != 0
    }

    pub fn greedy(&self) -> bool {
        self.flags & crate::grammar::GR != 0
    }

    /// The first match at or after `from`, as `(start, end)` byte offsets
    /// into `text`, with Prism's lookbehind group already taken off the
    /// front.
    pub fn find(&self, text: &str, from: usize) -> Option<(usize, usize)> {
        let re = self.re.as_ref()?;
        if from > text.len() || !text.is_char_boundary(from) {
            return None;
        }
        let caps = re.captures_from_pos(text, from).ok()??;
        let whole = caps.get(0)?;
        let (mut start, end) = (whole.start(), whole.end());
        if self.lookbehind() {
            // Prism takes the first group's length off the front of the
            // match; a group that did not participate takes nothing.
            if let Some(g) = caps.get(1) {
                start += g.end() - g.start();
            }
        }
        Some((start, end))
    }
}

/// A grammar's entries, in order, compiled. `rest` is already folded in at
/// the end, which is where Prism's `tokenize` splices it.
pub struct Compiled {
    pub entries: Vec<(&'static str, Vec<CPattern>)>,
}

fn compile_pattern(p: &'static Pattern) -> CPattern {
    CPattern {
        re: Regex::new(&p.source.concat()).ok(),
        flags: p.flags,
        alias: p.alias,
        inside: p.inside,
    }
}

fn build(grammar: &'static Grammar) -> Compiled {
    Compiled {
        entries: grammar
            .entries()
            .map(|(kind, patterns)| (kind, patterns.iter().map(compile_pattern).collect()))
            .collect(),
    }
}

type Table = RwLock<HashMap<usize, &'static Compiled>>;

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// `grammar` with its patterns compiled, built once and kept.
///
/// Leaked on purpose: there are as many of these as the crate has
/// grammars, they live as long as the process would want them, and a
/// `&'static` is what lets the engine recurse without reference counting.
pub fn compile(grammar: &'static Grammar) -> &'static Compiled {
    let key = grammar as *const Grammar as usize;
    if let Some(found) = table().read().ok().and_then(|t| t.get(&key).copied()) {
        return found;
    }
    // Built outside the lock: `build` compiles regexes, and another thread
    // asking for the same grammar meanwhile should wait on neither.
    let fresh: &'static Compiled = Box::leak(Box::new(build(grammar)));
    let mut table = match table().write() {
        Ok(t) => t,
        // A poisoned lock means another thread panicked mid-insert; the
        // table is still a valid map, so carry on with it.
        Err(poisoned) => poisoned.into_inner(),
    };
    table.entry(key).or_insert(fresh)
}
