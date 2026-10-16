//! The syntax tree produced by [`crate::parse`].
//!
//! Every node carries the byte [`Span`] of the source it was parsed from so
//! the editor can decorate text, the cache can record Obsidian `position`s,
//! and the reading view can render, all from one tree.

use crate::span::Span;
use std::borrow::Cow;
use std::collections::HashMap;

/// A parsed note.
#[derive(Debug, Clone, PartialEq)]
pub struct Document<'a> {
    pub source: &'a str,
    pub blocks: Vec<Block<'a>>,
    /// Link reference definitions by normalized label (see [`normalize_label`]).
    pub link_definitions: HashMap<String, LinkDefinition<'a>>,
    /// Footnote definition labels in order of first definition.
    pub footnote_labels: Vec<&'a str>,
}

/// `[label]: destination "title"`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkDefinition<'a> {
    pub label: Cow<'a, str>,
    pub destination: Cow<'a, str>,
    pub title: Option<Cow<'a, str>>,
    pub span: Span,
}

/// A block-level node.
#[derive(Debug, Clone, PartialEq)]
pub struct Block<'a> {
    pub kind: BlockKind<'a>,
    /// From the first byte of the block to the end of its last line (newline excluded).
    pub span: Span,
    /// `^id` attached to this block.
    pub id: Option<BlockId<'a>>,
}

/// A block identifier such as `^abc-12`; `span` covers the caret and the id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockId<'a> {
    pub id: &'a str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind<'a> {
    /// YAML properties block at the top of the file.
    Yaml {
        yaml: &'a str,
    },
    Paragraph {
        content: Vec<Inline<'a>>,
    },
    Heading {
        level: u8,
        content: Vec<Inline<'a>>,
        setext: bool,
    },
    ThematicBreak,
    CodeBlock {
        /// `None` for an indented code block.
        fence: Option<Fence>,
        /// The info string after the opening fence, unescaped.
        info: Option<Cow<'a, str>>,
        /// Block content with indentation and fences removed, each line newline-terminated.
        content: Cow<'a, str>,
    },
    HtmlBlock {
        /// Raw lines, newline-terminated, written to the output verbatim.
        content: Cow<'a, str>,
    },
    BlockQuote {
        children: Vec<Block<'a>>,
    },
    /// `> [!type]- Title` followed by a quoted body.
    Callout {
        /// The type as written, for example `TIP`; see [`crate::canonical_callout_type`].
        callout_type: &'a str,
        fold: Option<Fold>,
        /// Inline content of the title; empty when the header has no title.
        title: Vec<Inline<'a>>,
        children: Vec<Block<'a>>,
    },
    List {
        kind: ListKind,
        tight: bool,
        items: Vec<ListItem<'a>>,
    },
    Table {
        alignments: Vec<Alignment>,
        header: Vec<TableCell<'a>>,
        rows: Vec<Vec<TableCell<'a>>>,
    },
    /// `$$ ... $$` display math; `content` excludes the delimiters.
    Math {
        content: Cow<'a, str>,
    },
    /// `%% ... %%` spanning whole lines; never rendered.
    Comment {
        content: &'a str,
    },
    FootnoteDefinition {
        label: &'a str,
        children: Vec<Block<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fence {
    /// `` ` `` or `~`.
    pub marker: u8,
    pub length: usize,
    /// Indentation of the opening fence, stripped from every content line.
    pub indent: usize,
}

/// `-` collapsed by default, `+` expanded by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// `-`, `*` or `+`.
    Bullet { marker: u8 },
    /// `1.` or `1)`.
    Ordered { start: u64, delimiter: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem<'a> {
    pub span: Span,
    /// The bullet or number including its delimiter.
    pub marker: Span,
    /// `Some(' ')` for `[ ]`, `Some('x')` for `[x]`, any single character otherwise.
    pub task: Option<char>,
    pub children: Vec<Block<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell<'a> {
    pub span: Span,
    pub content: Vec<Inline<'a>>,
}

/// An inline node.
#[derive(Debug, Clone, PartialEq)]
pub struct Inline<'a> {
    pub kind: InlineKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineKind<'a> {
    /// Literal text with backslash escapes and entities decoded.
    Text(Cow<'a, str>),
    SoftBreak,
    HardBreak,
    /// `` `code` `` with the content normalized as CommonMark specifies.
    Code(Cow<'a, str>),
    /// Raw inline HTML, emitted verbatim.
    Html(Cow<'a, str>),
    Emphasis(Vec<Inline<'a>>),
    Strong(Vec<Inline<'a>>),
    Strikethrough(Vec<Inline<'a>>),
    /// `==text==`
    Highlight(Vec<Inline<'a>>),
    Link {
        destination: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
        kind: LinkKind,
        children: Vec<Inline<'a>>,
    },
    Image {
        destination: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
        /// The alt text as inline content.
        children: Vec<Inline<'a>>,
    },
    /// `[[body]]` or `![[body]]`; `body` is the raw text between the brackets
    /// (with `\|` unescaped inside tables), to be split with [`crate::parse_linktext`].
    Wikilink {
        body: Cow<'a, str>,
        embed: bool,
    },
    /// `#name`, without the `#`.
    Tag(&'a str),
    /// `$x$` (`display` false) or `$$x$$` (`display` true), delimiters excluded.
    Math {
        content: Cow<'a, str>,
        display: bool,
    },
    /// `%%text%%`, delimiters excluded.
    Comment(Cow<'a, str>),
    /// `[^label]`
    FootnoteReference(Cow<'a, str>),
    /// `^[text]`
    InlineFootnote(Vec<Inline<'a>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// `[text](dest)`
    Inline,
    /// `[text][label]`, `[text][]` or `[text]`
    Reference,
    /// `<https://example.com>`
    Autolink,
    /// A bare `https://` or `www.` URL (GFM autolink extension).
    BareUrl,
}

/// Normalizes a link label for case-insensitive lookup: trims, collapses
/// internal whitespace, and case-folds (lower, upper, lower approximates
/// Unicode case folding well enough that `ß` and `SS` match).
pub fn normalize_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut pending_space = false;
    for c in label.trim().chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            for l in c.to_lowercase() {
                for u in l.to_uppercase() {
                    out.extend(u.to_lowercase());
                }
            }
        }
    }
    out
}

impl<'a> Block<'a> {
    /// Child blocks of container kinds; empty for leaves.
    pub fn children(&self) -> &[Block<'a>] {
        match &self.kind {
            BlockKind::BlockQuote { children }
            | BlockKind::Callout { children, .. }
            | BlockKind::FootnoteDefinition { children, .. } => children,
            _ => &[],
        }
    }
}

/// Walks every inline in `inlines` depth first.
pub fn walk_inlines<'a, 'b>(inlines: &'b [Inline<'a>], f: &mut impl FnMut(&'b Inline<'a>)) {
    for inline in inlines {
        f(inline);
        match &inline.kind {
            InlineKind::Emphasis(children)
            | InlineKind::Strong(children)
            | InlineKind::Strikethrough(children)
            | InlineKind::Highlight(children)
            | InlineKind::Link { children, .. }
            | InlineKind::Image { children, .. }
            | InlineKind::InlineFootnote(children) => walk_inlines(children, f),
            _ => {}
        }
    }
}

/// Walks every block in `blocks` depth first, including list items' and
/// table cells' contents through `f` receiving each block in document order.
pub fn walk_blocks<'a, 'b>(blocks: &'b [Block<'a>], f: &mut impl FnMut(&'b Block<'a>)) {
    for block in blocks {
        f(block);
        match &block.kind {
            BlockKind::BlockQuote { children }
            | BlockKind::Callout { children, .. }
            | BlockKind::FootnoteDefinition { children, .. } => walk_blocks(children, f),
            BlockKind::List { items, .. } => {
                for item in items {
                    walk_blocks(&item.children, f);
                }
            }
            _ => {}
        }
    }
}

/// Collects the plain text of inline content, as Obsidian does for heading
/// text and link display text: formatting is dropped, code and text kept.
pub fn plain_text(inlines: &[Inline<'_>]) -> String {
    let mut out = String::new();
    push_plain_text(inlines, &mut out);
    out
}

fn push_plain_text(inlines: &[Inline<'_>], out: &mut String) {
    for inline in inlines {
        match &inline.kind {
            InlineKind::Text(t) | InlineKind::Code(t) => out.push_str(t),
            InlineKind::SoftBreak | InlineKind::HardBreak => out.push(' '),
            InlineKind::Html(h) => out.push_str(h),
            InlineKind::Emphasis(c)
            | InlineKind::Strong(c)
            | InlineKind::Strikethrough(c)
            | InlineKind::Highlight(c)
            | InlineKind::Link { children: c, .. }
            | InlineKind::Image { children: c, .. }
            | InlineKind::InlineFootnote(c) => push_plain_text(c, out),
            InlineKind::Wikilink { body, .. } => {
                let parsed = crate::parse_linktext(body);
                out.push_str(parsed.alias.unwrap_or(parsed.path));
            }
            InlineKind::Tag(t) => {
                out.push('#');
                out.push_str(t);
            }
            InlineKind::Math { content, .. } => out.push_str(content),
            InlineKind::Comment(_) => {}
            InlineKind::FootnoteReference(l) => {
                out.push_str("[^");
                out.push_str(l);
                out.push(']');
            }
        }
    }
}
