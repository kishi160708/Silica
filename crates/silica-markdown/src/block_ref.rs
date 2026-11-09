//! Block references: a Logseq-style `^id` on a block, and `[[note#^id]]` /
//! `[[note#Heading]]` wikilinks that point at a block, resolved to the block
//! they name. Resolution returns a [`BlockSlice`]: the block plus its byte
//! span in the document source, which the reading view turns into a jump
//! target (an `id` attribute on the block's HTML, an `href="#^id"` anchor on
//! the reference).
//!
//! A `[[note#^id]]` wikilink's body is already parsed by [`crate::parse_linktext`]
//! into `Subpath::Block`, and `[[note#Heading]]` into `Subpath::Headings`; this
//! module turns those (or a bare `^id`) into a resolved slice by walking
//! [`Document::blocks`].
//!
//! Cross-note references (a non-empty `[[path#...]]`) cannot be resolved from
//! a single [`Document`]: the caller resolves the note's path first
//! (silica-cache's `LinkResolver`), then this module resolves the block inside
//! the target note's document.

use crate::ast::{Block, BlockId, BlockKind, Document};
use crate::linktext::Subpath;
use crate::parse_linktext;
use crate::plain_text;
use crate::span::Span;
/// A reference to a single block, before it is resolved against a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockRef<'a> {
    /// A bare Logseq-style `^id`, resolved inside the current note.
    BlockId { id: &'a str },
    /// A `[[path#^id]]` wikilink to a block. `path` is empty for `[[#^id]]`;
    /// a non-empty path names another note and must be routed to that note's
    /// document before this slice is applied.
    Wikilink { path: &'a str, block_id: &'a str },
    /// A `[[path#Heading]]` or `[[path#H1#H2]]` wikilink to a heading block.
    /// `path` is empty for links inside the current note.
    Heading {
        path: &'a str,
        headings: Vec<&'a str>,
    },
}

/// The resolved location of a block reference.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockSlice<'a> {
    /// The referenced block.
    pub block: &'a Block<'a>,
    /// Byte span of the whole block in [`Document::source`].
    pub span: Span,
    /// The block's own `^id`, if it carries one.
    pub id: Option<BlockId<'a>>,
}

/// Parses a wikilink body into a block reference.
///
/// `[[note#^abc]]` → `Wikilink { path: "note", block_id: "abc" }`;
/// `[[#^abc]]` → `Wikilink { path: "", block_id: "abc" }`;
/// `[[note#Head]]` → `Heading { path: "note", headings: ["Head"] }`.
/// Returns `None` when the body is not a block reference (a plain `[[note]]`).
pub fn block_ref_from_wikilink(body: &str) -> Option<BlockRef<'_>> {
    let parsed = parse_linktext(body);
    match parsed.subpath {
        Some(Subpath::Block(block_id)) => Some(BlockRef::Wikilink {
            path: parsed.path,
            block_id,
        }),
        Some(Subpath::Headings(headings)) if !headings.is_empty() => Some(BlockRef::Heading {
            path: parsed.path,
            headings,
        }),
        _ => None,
    }
}

/// Resolves a block reference within a single document, returning the block
/// slice, or [`None`] when no block carries that id/heading or the reference
/// names another note.
pub fn resolve_block<'a>(
    doc: &'a Document<'a>,
    reference: &BlockRef<'a>,
) -> Option<BlockSlice<'a>> {
    match reference {
        BlockRef::BlockId { id } => find_block_by_id(&doc.blocks, id).map(slice),
        BlockRef::Wikilink { path: "", block_id } => {
            find_block_by_id(&doc.blocks, block_id).map(slice)
        }
        BlockRef::Wikilink { .. } => None, // cross-note: resolve the note first
        BlockRef::Heading { path: "", headings } => find_heading(&doc.blocks, headings).map(slice),
        BlockRef::Heading { .. } => None, // cross-note: resolve the note first
    }
}

/// Resolve a bare `^id` (Logseq style) within `doc`.
pub fn resolve_block_id<'a>(doc: &'a Document<'a>, id: &'a str) -> Option<BlockSlice<'a>> {
    resolve_block(doc, &BlockRef::BlockId { id })
}

fn slice<'a>(block: &'a Block<'a>) -> BlockSlice<'a> {
    BlockSlice {
        block,
        span: block.span,
        id: block.id,
    }
}

fn find_block_by_id<'a>(blocks: &'a [Block<'a>], id: &str) -> Option<&'a Block<'a>> {
    for block in blocks {
        if block.id.as_ref().map(|bid| bid.id) == Some(id) {
            return Some(block);
        }
        if let Some(found) = descend(block, id) {
            return Some(found);
        }
    }
    None
}

fn descend<'a>(block: &'a Block<'a>, id: &str) -> Option<&'a Block<'a>> {
    match &block.kind {
        BlockKind::BlockQuote { children }
        | BlockKind::Callout { children, .. }
        | BlockKind::FootnoteDefinition { children, .. } => find_block_by_id(children, id),
        BlockKind::List { items, .. } => {
            for item in items {
                if let Some(found) = find_block_by_id(&item.children, id) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Resolves a heading subpath (`#H1` or `#H1#H2`) to the heading block that
/// names it. Headings nest: `#H1#H2` names an `H2` inside `H1`'s section, so
/// the walk keeps a stack of open heading sections by level and matches the
/// wanted chain against it, returning the first match in document order.
fn find_heading<'a>(blocks: &'a [Block<'a>], wanted: &[&'a str]) -> Option<&'a Block<'a>> {
    let mut stack: Vec<(u8, String)> = Vec::new();
    let mut headings: Vec<&Block<'a>> = Vec::new();
    collect_headings(blocks, &mut headings);
    for h in headings {
        let BlockKind::Heading { level, content, .. } = &h.kind else {
            continue;
        };
        let text = plain_text(content);
        while let Some((last_level, _)) = stack.last() {
            if *last_level >= *level {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push((*level, text));
        if stack.len() >= wanted.len()
            && stack[stack.len() - wanted.len()..]
                .iter()
                .map(|(_, t)| t.as_str())
                .eq(wanted.iter().copied())
        {
            return Some(h);
        }
    }
    None
}

fn collect_headings<'a, 'b>(blocks: &'b [Block<'a>], out: &mut Vec<&'b Block<'a>>) {
    for block in blocks {
        if matches!(&block.kind, BlockKind::Heading { .. }) {
            out.push(block);
        }
        match &block.kind {
            BlockKind::BlockQuote { children }
            | BlockKind::Callout { children, .. }
            | BlockKind::FootnoteDefinition { children, .. } => collect_headings(children, out),
            BlockKind::List { items, .. } => {
                for item in items {
                    collect_headings(&item.children, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(src: &str) -> Document<'_> {
        crate::parse(src)
    }

    #[test]
    fn block_ref_from_wikilink_parses_block_subpath() {
        assert_eq!(
            block_ref_from_wikilink("note#^abc123"),
            Some(BlockRef::Wikilink {
                path: "note",
                block_id: "abc123",
            })
        );
        assert_eq!(
            block_ref_from_wikilink("#^abc123"),
            Some(BlockRef::Wikilink {
                path: "",
                block_id: "abc123",
            })
        );
        assert_eq!(
            block_ref_from_wikilink("note#Head|alias"),
            Some(BlockRef::Heading {
                path: "note",
                headings: vec!["Head"],
            })
        );
        // A plain link is not a block reference.
        assert_eq!(block_ref_from_wikilink("note"), None);
    }

    #[test]
    fn resolve_bare_block_id() {
        let src = "One.\n\nTwo ^x1\n\nThree.";
        let d = doc(src);
        let slice = resolve_block_id(&d, "x1").expect("x1 should resolve");
        assert_eq!(slice.id.map(|bid| bid.id), Some("x1"));
        assert_eq!(slice.span, Span::new(6, 13));
        assert_eq!(slice.span.of(src), "Two ^x1");
    }

    #[test]
    fn resolve_wikilink_empty_path_matches_current_note() {
        let src = "One.\n\nTwo ^x1\n\nThree.";
        let d = doc(src);
        let slice = resolve_block(
            &d,
            &BlockRef::Wikilink {
                path: "",
                block_id: "x1",
            },
        )
        .expect("[[#^x1]] should resolve in the current note");
        assert_eq!(slice.id.map(|bid| bid.id), Some("x1"));
        assert_eq!(slice.span, Span::new(6, 13));
    }

    #[test]
    fn resolve_cross_note_wikilink_is_none() {
        let src = "Two ^x1\n";
        let d = doc(src);
        assert!(resolve_block(
            &d,
            &BlockRef::Wikilink {
                path: "other",
                block_id: "x1",
            },
        )
        .is_none());
        assert!(resolve_block(
            &d,
            &BlockRef::Heading {
                path: "other",
                headings: vec!["Head"],
            },
        )
        .is_none());
    }

    #[test]
    fn resolve_unknown_id_is_none() {
        let src = "Two ^x1\n";
        let d = doc(src);
        assert!(resolve_block_id(&d, "nope").is_none());
    }

    #[test]
    fn resolve_nested_block_in_list() {
        let src = "- one ^l1\n- two\n";
        let d = doc(src);
        let slice = resolve_block_id(&d, "l1").expect("l1 should resolve");
        assert_eq!(slice.id.map(|bid| bid.id), Some("l1"));
        assert!(!slice.span.is_empty());
        assert!(slice.span.of(src).contains("one"));
    }

    #[test]
    fn resolve_heading_by_text() {
        let src = "# Alpha\n\n## Beta\n\n## Gamma\n";
        let d = doc(src);
        let h = resolve_block(
            &d,
            &BlockRef::Heading {
                path: "",
                headings: vec!["Beta"],
            },
        )
        .expect("Beta should resolve");
        assert_eq!(
            plain_text(match &h.block.kind {
                BlockKind::Heading { content, .. } => content,
                _ => unreachable!(),
            }),
            "Beta"
        );
        // Nested chain resolves against the section stack: #Alpha#Beta names
        // the Beta heading inside Alpha's section, and #Alpha#Gamma names
        // Gamma (a sibling section inside the same Alpha section). Unknown
        // headings miss.
        for wanted in [vec!["Alpha", "Beta"], vec!["Alpha", "Gamma"]] {
            let h = resolve_block(
                &d,
                &BlockRef::Heading {
                    path: "",
                    headings: wanted.clone(),
                },
            )
            .unwrap_or_else(|| panic!("{wanted:?} should resolve"));
            assert_eq!(
                plain_text(match &h.block.kind {
                    BlockKind::Heading { content, .. } => content,
                    _ => unreachable!(),
                }),
                wanted.last().copied().unwrap()
            );
        }
        assert!(resolve_block(
            &d,
            &BlockRef::Heading {
                path: "",
                headings: vec!["Delta"],
            },
        )
        .is_none());
        assert!(resolve_block(
            &d,
            &BlockRef::Heading {
                path: "",
                headings: vec!["Alpha", "Delta"],
            },
        )
        .is_none());
    }

    #[test]
    fn renders_block_reference_in_reading_view() {
        use crate::render_html;
        let src = "One.\n\nTwo ^x1\n\nSee [[#^x1]] and [[note#^y2|alias]].";
        let out = render_html(&doc(src));
        // The bare block reference renders ^id and links to the fragment so the
        // reading view can jump to the block it names.
        assert!(out.contains(
            r##"<a data-href="#^x1" href="#^x1" class="internal-link" target="_blank" rel="noopener nofollow">^x1</a>"##,
        ));
        // Cross-note block reference keeps its written target and shows the alias.
        assert!(out.contains(
            r##"<a data-tooltip-position="top" aria-label="note#^y2" data-href="note#^y2" href="note#^y2" class="internal-link" target="_blank" rel="noopener nofollow">alias</a>"##,
        ));
        // The id is not part of the paragraph's rendered text.
        let rendered = render_html(&doc("Two ^x1"));
        assert!(!rendered.contains("^x1"));
        assert_eq!(rendered.trim_end(), "<p>Two</p>");
    }
}
