//! Obsidian Flavored Markdown (OFM) primitives.
//!
//! This crate owns the *one* parser that every other subsystem must share:
//! editor decorations, reading view, the metadata cache, and link rewriting
//! all read the same tokens so that edge cases cannot diverge.
//!
//! Only the Obsidian-specific layer lives here (frontmatter, `[[links]]`,
//! `![[embeds]]`, `#tags`, `^block-ids`, callout headers). CommonMark/GFM
//! block structure will be layered on top in a later milestone.

pub mod frontmatter;
pub mod inline;
pub mod linktext;

pub use frontmatter::{extract_frontmatter, FrontMatter};
pub use inline::{scan_inlines, Inline, Span};
pub use linktext::{parse_linktext, LinkText, Subpath};

/// Block-level constructs the metadata cache records as `sections`.
/// Mirrors Obsidian's `SectionCache.type` vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionKind {
    Paragraph,
    Heading,
    List,
    Code,
    Blockquote,
    Table,
    ThematicBreak,
    Yaml,
    Html,
    FootnoteDefinition,
    Callout,
    Math,
    Comment,
}

/// Callout types shipped with Obsidian and their aliases. Unknown types render as `note`.
pub fn canonical_callout_type(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "note" => "note",
        "abstract" | "summary" | "tldr" => "abstract",
        "info" => "info",
        "todo" => "todo",
        "tip" | "hint" | "important" => "tip",
        "success" | "check" | "done" => "success",
        "question" | "help" | "faq" => "question",
        "warning" | "caution" | "attention" => "warning",
        "failure" | "fail" | "missing" => "failure",
        "danger" | "error" => "danger",
        "bug" => "bug",
        "example" => "example",
        "quote" | "cite" => "quote",
        _ => "note",
    }
}

/// Parses a callout header such as `[!tip]- Title` (the leading `> ` already stripped).
/// Returns `(type, fold, title)`; fold is `Some(true)` for `-` (collapsed), `Some(false)` for `+`.
pub fn parse_callout_header(line: &str) -> Option<(&str, Option<bool>, &str)> {
    let rest = line.trim_start().strip_prefix("[!")?;
    let close = rest.find(']')?;
    let ty = &rest[..close];
    if ty.is_empty() || ty.contains(char::is_whitespace) {
        return None;
    }
    let after = &rest[close + 1..];
    let (fold, after) = match after.as_bytes().first() {
        Some(b'-') => (Some(true), &after[1..]),
        Some(b'+') => (Some(false), &after[1..]),
        _ => (None, after),
    };
    Some((ty, fold, after.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_header_variants() {
        assert_eq!(
            parse_callout_header("[!tip]- Title"),
            Some(("tip", Some(true), "Title"))
        );
        assert_eq!(
            parse_callout_header("[!warning]+"),
            Some(("warning", Some(false), ""))
        );
        assert_eq!(
            parse_callout_header("[!note] Plain"),
            Some(("note", None, "Plain"))
        );
        assert_eq!(parse_callout_header("[! bad]"), None);
        assert_eq!(parse_callout_header("not a callout"), None);
    }

    #[test]
    fn callout_aliases_fold_to_canonical() {
        assert_eq!(canonical_callout_type("TLDR"), "abstract");
        assert_eq!(canonical_callout_type("cite"), "quote");
        assert_eq!(canonical_callout_type("custom"), "note");
    }
}
