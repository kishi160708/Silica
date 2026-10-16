//! Obsidian's search query language, implemented once for the Search view,
//! graph filters, bookmarked searches and `query` code blocks.
//!
//! [`parse`] turns a query string into a [`Query`] tree whose [`Query::explain`]
//! output uses the labels Obsidian shows under "Explain search terms", which
//! is the acceptance test for the grammar. [`Search`] compiles a tree (regexes
//! included) and matches it against a [`SearchFile`], returning the byte
//! ranges to highlight. [`fuzzy_match`] and [`simple_search`] are the
//! `prepareFuzzySearch` and `prepareSimpleSearch` counterparts used by the
//! quick switcher and pickers.

pub mod fuzzy;
pub mod matcher;
pub mod query;

pub use fuzzy::{fuzzy_match, simple_search, FuzzyMatch};
pub use matcher::{Search, SearchError, SearchFile, TaskLine};
pub use query::{parse, Compare, Field, Query};
