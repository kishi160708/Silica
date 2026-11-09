//! Inverted term index for vault-wide search.
//!
//! `TermIndex` maps each lowercased term to the set of file paths that
//! contain it. `run_search` uses it to narrow the candidate set before
//! running the full `Search` matcher, so a query like `foo bar` only
//! touches notes that contain both terms instead of scanning every note
//! in the vault.
//!
//! The index is maintained incrementally: `insert` adds a path's terms,
//! `remove` drops a path entirely, and `rebuild` reconstructs from scratch.
//! Tokenization splits on Unicode whitespace and punctuation, lowercases,
//! and drops terms shorter than two characters.

use std::collections::{HashMap, HashSet};

/// `term` → set of vault-relative paths containing it.
pub struct TermIndex {
    map: HashMap<String, HashSet<String>>,
}

impl Default for TermIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TermIndex {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Add `path`'s terms to the index. Replaces any previous entry for
    /// the same path (call `remove` first if you want to avoid a transient
    /// duplicate during a modify).
    pub fn insert(&mut self, path: &str, text: &str) {
        let terms = tokenize(text);
        for term in terms {
            self.map.entry(term).or_default().insert(path.to_string());
        }
    }

    /// Remove `path` from every term's posting list.
    pub fn remove(&mut self, path: &str) {
        let to_drop: Vec<String> = self
            .map
            .iter_mut()
            .filter_map(|(term, paths)| {
                paths.remove(path);
                if paths.is_empty() {
                    Some(term.clone())
                } else {
                    None
                }
            })
            .collect();
        for term in to_drop {
            self.map.remove(&term);
        }
    }

    /// Paths that contain *every* term in `terms` (AND semantics). An empty
    /// term list matches nothing — a search with no terms should return no
    /// results, matching Obsidian's `startSearch` behavior.
    pub fn candidates(&self, terms: &[&str]) -> Vec<String> {
        if terms.is_empty() {
            return Vec::new();
        }
        let mut result: Option<HashSet<&str>> = None;
        for term in terms {
            let paths = match self.map.get(*term) {
                Some(p) => p.iter().map(|s| s.as_str()).collect(),
                None => return Vec::new(),
            };
            result = Some(match result {
                None => paths,
                Some(prev) => prev.intersection(&paths).copied().collect(),
            });
        }
        result
            .map(|s| s.into_iter().map(String::from).collect())
            .unwrap_or_default()
    }

    /// Number of distinct terms in the index.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Split `text` into searchable terms: runs of alphanumeric characters,
/// lowercased, length >= 2. Single characters and pure punctuation are
/// noise; they never help narrow a search and only bloat the index.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut cur: Vec<char> = Vec::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            let t: String = cur.drain(..).collect();
            if t.len() >= 2 {
                terms.push(t);
            }
        }
    }
    if !cur.is_empty() {
        let t: String = cur.drain(..).collect();
        if t.len() >= 2 {
            terms.push(t);
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_and_lowercases() {
        assert_eq!(
            tokenize("Hello, WORLD! foo-bar 42 x"),
            vec!["hello", "world", "foo", "bar", "42"]
        );
    }

    #[test]
    fn tokenize_drops_short_terms() {
        assert_eq!(tokenize("a I x at in"), vec!["at", "in"]);
    }

    #[test]
    fn index_insert_and_candidates() {
        let mut idx = TermIndex::new();
        idx.insert("a.md", "alpha beta gamma");
        idx.insert("b.md", "beta delta");
        idx.insert("c.md", "alpha gamma");

        let found = idx.candidates(&["alpha", "gamma"]);
        assert_eq!(found, vec!["a.md", "c.md"]);

        let found = idx.candidates(&["delta"]);
        assert_eq!(found, vec!["b.md"]);

        let found = idx.candidates(&["zeta"]);
        assert!(found.is_empty());

        let found = idx.candidates(&[]);
        assert!(found.is_empty());
    }

    #[test]
    fn remove_drops_path() {
        let mut idx = TermIndex::new();
        idx.insert("a.md", "alpha beta");
        idx.insert("b.md", "beta gamma");
        idx.remove("a.md");

        let found = idx.candidates(&["alpha"]);
        assert!(found.is_empty());
        let found = idx.candidates(&["beta"]);
        assert_eq!(found, vec!["b.md"]);
    }
}
