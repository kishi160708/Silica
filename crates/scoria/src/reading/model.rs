//! Reading-view model: an owned render tree built from the shared parser's
//! [`Document`], so the view can hold it across frames without borrowing the
//! note text. Mirrors what `_markdown::render_html` emits (the same
//! tree, the same footnote numbering, the same embed rules) but as data a
//! GPUI element tree can be built from: paragraphs become runs of styled
//! text with clickable ranges, embeds are resolved and inlined recursively,
//! and footnote definitions are gathered at the end.

use _formats::properties::Property;
use _formats::FileKind;
use _markdown::{
    canonical_callout_type, html::has_scheme, html::percent_decode, parse, plain_text, Alignment,
    Block, BlockKind, Document, Fold, Inline, InlineKind, LinkKind, ListItem, ListKind, Subpath,
};
use std::ops::Range;
use std::path::PathBuf;

/// Embedded notes nest at most this deep; Obsidian stops at a similar depth.
pub const MAX_EMBED_DEPTH: usize = 4;

/// What the model builder needs from the vault to resolve embeds.
pub trait EmbedSource {
    /// Resolves a link body written in `source_path` to a vault path.
    fn resolve(&self, linktext: &str, source_path: &str) -> Option<String>;
    /// Reads a markdown file's text.
    fn read(&self, path: &str) -> Option<String>;
    /// Absolute path of a vault file, for images.
    fn absolute(&self, path: &str) -> PathBuf;
}

/// Formatting that applies to a run of text; several may apply at once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub highlight: bool,
    pub code: bool,
    pub link: bool,
    pub tag: bool,
    pub math: bool,
    pub footnote: bool,
    pub html: bool,
}

impl InlineStyle {
    pub fn is_plain(&self) -> bool {
        *self == InlineStyle::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// A wikilink body or a vault-relative markdown link destination,
    /// resolved by the workspace relative to the note it appears in.
    Internal(String),
    External(String),
    Tag(String),
    /// Jump to footnote number `n` at the bottom of the note.
    Footnote(usize),
}

/// One line-wrapped run of inline content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichText {
    /// Unique within one model, for GPUI element ids.
    pub id: usize,
    pub text: String,
    /// Non-overlapping styled byte ranges in order; gaps are plain.
    pub runs: Vec<(Range<usize>, InlineStyle)>,
    pub links: Vec<(Range<usize>, LinkTarget)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Text(RichText),
    Embed(Embed),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Embed {
    pub kind: EmbedKind,
    /// The link body as written, for opening the target.
    pub link: String,
    pub alt: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmbedKind {
    Image(PathBuf),
    Note {
        path: String,
        /// `Note`, `Note > Heading`, or `Note > ^id`.
        title: String,
        blocks: Vec<RBlock>,
    },
    /// The target file does not exist.
    Missing,
    /// The target exists but  cannot render it inline yet.
    Unsupported(FileKind),
    TooDeep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RItem {
    pub task: Option<char>,
    pub number: Option<u64>,
    pub children: Vec<RBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RBlock {
    /// The properties block. The entries are the parsed frontmatter with
    /// their byte spans; the renderer resolves each key's type against the
    /// vault's `types.json` and builds the rows, because the same block is
    /// drawn read-only in the reading view and editable in Live Preview.
    Properties(Vec<Property>),
    Paragraph(Vec<Segment>),
    Heading {
        level: u8,
        text: RichText,
        plain: String,
    },
    ThematicBreak,
    Code {
        /// Unique within one model, for GPUI element ids.
        id: usize,
        language: Option<String>,
        text: String,
    },
    Html(String),
    Quote(Vec<RBlock>),
    Callout {
        /// Canonical type, one of Obsidian's built-ins.
        kind: &'static str,
        fold: Option<bool>,
        title: RichText,
        children: Vec<RBlock>,
    },
    List {
        tight: bool,
        items: Vec<RItem>,
    },
    Table {
        alignments: Vec<Alignment>,
        header: Vec<RichText>,
        rows: Vec<Vec<RichText>>,
        /// The bytes each cell stands for, the header row first, so Live
        /// Preview can edit a cell in place.
        spans: Vec<Vec<Range<usize>>>,
    },
    Math(String),
    /// `(number, body)` in reference order, rendered after a rule.
    Footnotes(Vec<(usize, Vec<RBlock>)>),
}

/// A built note.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model {
    pub blocks: Vec<RBlock>,
    /// Source byte offset where each top-level block starts, for mapping
    /// cache positions (outline, backlinks) to list items.
    pub starts: Vec<usize>,
}

impl Model {
    /// Index of the top-level block containing byte `offset`.
    pub fn block_at(&self, offset: usize) -> Option<usize> {
        if self.starts.is_empty() {
            return None;
        }
        Some(match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        })
    }
}

/// Rendering settings that come from `app.json`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildOptions {
    /// Obsidian's "Strict line breaks": a single newline inside a paragraph
    /// is a space rather than a line break.
    pub strict_line_breaks: bool,
}

/// Builds the model for `text`, the content of `source_path`, with the default settings.
#[cfg(test)]
pub fn build(source_path: &str, text: &str, vault: &dyn EmbedSource) -> Model {
    build_with(source_path, text, vault, BuildOptions::default())
}

/// Builds the model for `text`, the content of `source_path`.
pub fn build_with(
    source_path: &str,
    text: &str,
    vault: &dyn EmbedSource,
    opts: BuildOptions,
) -> Model {
    let doc = parse(text);
    let mut b = Builder {
        vault,
        source_path,
        text: doc.source,
        opts,
        depth: 0,
        next_id: 0,
        footnotes: FootnoteState::default(),
    };
    b.footnotes.definitions = collect_definitions(&doc.blocks);
    let starts = doc.blocks.iter().map(|b| b.span.start).collect();
    let mut blocks = b.blocks(&doc.blocks, false);
    if let Some(section) = b.footnote_section(&doc) {
        blocks.push(section);
    }
    Model { blocks, starts }
}

/// A top-level block Live Preview renders in place of its source lines:
/// a table, a math block, or a paragraph that is one embed.
#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    /// Byte span of the block in the note (newline after it excluded).
    pub span: Range<usize>,
    pub block: RBlock,
}

/// The blocks of `doc` (the content of `source_path`) that Live Preview
/// shows as widgets, built with ids unique among them.
pub fn build_widgets(
    source_path: &str,
    doc: &Document<'_>,
    vault: &dyn EmbedSource,
    opts: BuildOptions,
) -> Vec<Widget> {
    let mut b = Builder {
        vault,
        source_path,
        text: doc.source,
        opts,
        depth: 0,
        next_id: 0,
        footnotes: FootnoteState::default(),
    };
    b.footnotes.definitions = collect_definitions(&doc.blocks);
    let mut out = Vec::new();
    for block in &doc.blocks {
        let widget = match &block.kind {
            // The properties block is a widget so Live Preview can draw it as
            // an editable table instead of raw YAML.
            BlockKind::Yaml { .. } => true,
            BlockKind::Table { .. } | BlockKind::Math { .. } => true,
            BlockKind::Paragraph { content } => match content.as_slice() {
                [one] => matches!(
                    &one.kind,
                    InlineKind::Wikilink { embed: true, .. }
                        | InlineKind::Image {
                            kind: LinkKind::Inline,
                            ..
                        }
                ),
                _ => false,
            },
            _ => false,
        };
        if !widget {
            continue;
        }
        if let Some(built) = b.block(block, false) {
            out.push(Widget {
                span: block.span.start..block.span.end,
                block: built,
            });
        }
    }
    out
}

/// Builds the model of an embedded note or section with the given nesting.
fn build_embedded(
    source_path: &str,
    doc: &Document<'_>,
    blocks: &[Block<'_>],
    depth: usize,
    next_id: &mut usize,
    vault: &dyn EmbedSource,
    opts: BuildOptions,
) -> Vec<RBlock> {
    let mut b = Builder {
        vault,
        source_path,
        text: doc.source,
        opts,
        depth,
        next_id: *next_id,
        footnotes: FootnoteState::default(),
    };
    b.footnotes.definitions = collect_definitions(&doc.blocks);
    let mut out = b.blocks(blocks, false);
    if let Some(section) = b.footnote_section(doc) {
        out.push(section);
    }
    *next_id = b.next_id;
    out
}

#[derive(Default)]
struct FootnoteState {
    /// Labels present in the document (first definition wins).
    definitions: Vec<String>,
    /// Labels in order of first reference; index + 1 is the number.
    numbers: Vec<String>,
    /// Inline footnotes keyed by generated label.
    inline: Vec<(String, RichText)>,
}

fn collect_definitions(blocks: &[Block<'_>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    _markdown::walk_blocks(blocks, &mut |b| {
        if let BlockKind::FootnoteDefinition { label, .. } = &b.kind {
            if !out.iter().any(|l| l == label) {
                out.push(label.to_string());
            }
        }
    });
    out
}

struct Builder<'v> {
    vault: &'v dyn EmbedSource,
    source_path: &'v str,
    /// The whole note, so the properties block can carry file byte spans.
    text: &'v str,
    opts: BuildOptions,
    depth: usize,
    next_id: usize,
    footnotes: FootnoteState,
}

impl<'v> Builder<'v> {
    fn id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn blocks(&mut self, blocks: &[Block<'_>], tight: bool) -> Vec<RBlock> {
        blocks.iter().filter_map(|b| self.block(b, tight)).collect()
    }

    fn block(&mut self, block: &Block<'_>, _tight: bool) -> Option<RBlock> {
        Some(match &block.kind {
            BlockKind::Yaml { .. } => RBlock::Properties(
                _cache::note_properties(self.text)
                    .map(|p| p.entries)
                    .unwrap_or_default(),
            ),
            BlockKind::Paragraph { content } => RBlock::Paragraph(self.segments(content)),
            BlockKind::Heading { level, content, .. } => RBlock::Heading {
                level: *level,
                text: self.rich(content),
                plain: plain_text(content).trim().to_string(),
            },
            BlockKind::ThematicBreak => RBlock::ThematicBreak,
            BlockKind::CodeBlock { info, content, .. } => RBlock::Code {
                id: self.id(),
                language: info
                    .as_deref()
                    .and_then(|i| i.split_whitespace().next())
                    .map(str::to_string),
                text: content.trim_end_matches('\n').to_string(),
            },
            BlockKind::HtmlBlock { content } => RBlock::Html(content.trim_end().to_string()),
            BlockKind::BlockQuote { children } => RBlock::Quote(self.blocks(children, false)),
            BlockKind::Callout {
                callout_type,
                fold,
                title,
                children,
            } => {
                let title = if title.is_empty() {
                    let mut chars = callout_type.chars();
                    let shown: String = chars
                        .next()
                        .map(|f| {
                            f.to_uppercase()
                                .chain(chars.map(|c| c.to_ascii_lowercase()))
                                .collect()
                        })
                        .unwrap_or_default();
                    RichText {
                        id: self.id(),
                        text: shown,
                        ..RichText::default()
                    }
                } else {
                    self.rich(title)
                };
                RBlock::Callout {
                    kind: canonical_callout_type(callout_type),
                    fold: fold.map(|f| f == Fold::Collapsed),
                    title,
                    children: self.blocks(children, false),
                }
            }
            BlockKind::List { kind, tight, items } => RBlock::List {
                tight: *tight,
                items: self.items(kind, items, *tight),
            },
            BlockKind::Table {
                alignments,
                header,
                rows,
            } => RBlock::Table {
                alignments: alignments.clone(),
                header: header.iter().map(|c| self.rich(&c.content)).collect(),
                rows: rows
                    .iter()
                    .map(|r| r.iter().map(|c| self.rich(&c.content)).collect())
                    .collect(),
                spans: std::iter::once(header)
                    .chain(rows.iter())
                    .map(|r| r.iter().map(|c| c.span.start..c.span.end).collect())
                    .collect(),
            },
            BlockKind::Math { content } => RBlock::Math(content.trim_matches('\n').to_string()),
            BlockKind::Comment { .. } | BlockKind::FootnoteDefinition { .. } => return None,
        })
    }

    fn items(&mut self, kind: &ListKind, items: &[ListItem<'_>], tight: bool) -> Vec<RItem> {
        let mut number = match kind {
            ListKind::Ordered { start, .. } => Some(*start),
            ListKind::Bullet { .. } => None,
        };
        items
            .iter()
            .map(|item| {
                let n = number;
                if let Some(n) = number.as_mut() {
                    *n += 1;
                }
                RItem {
                    task: item.task,
                    number: n,
                    children: self.blocks(&item.children, tight),
                }
            })
            .collect()
    }

    /// A paragraph's content split into text runs and block-level embeds.
    fn segments(&mut self, content: &[Inline<'_>]) -> Vec<Segment> {
        let mut out = Vec::new();
        let mut acc = RichBuilder::default();
        for inline in content {
            match &inline.kind {
                InlineKind::Wikilink { body, embed: true } => {
                    self.flush(&mut acc, &mut out);
                    let embed = self.wikilink_embed(body);
                    out.push(Segment::Embed(embed));
                }
                InlineKind::Image {
                    destination,
                    kind: LinkKind::Inline,
                    children,
                    ..
                } if !has_scheme(destination) => {
                    self.flush(&mut acc, &mut out);
                    let alt = plain_text(children);
                    let alt = alt.split('|').next().unwrap_or("").trim_end().to_string();
                    let dest = percent_decode(destination).into_owned();
                    let embed = self.embed(&dest, alt, None, None);
                    out.push(Segment::Embed(embed));
                }
                _ => self.inline(inline, &mut acc),
            }
        }
        self.flush(&mut acc, &mut out);
        out
    }

    fn flush(&mut self, acc: &mut RichBuilder, out: &mut Vec<Segment>) {
        let text = std::mem::take(acc).finish(self.id());
        // Whitespace between two embeds would render as an empty line.
        if !text.text.trim().is_empty() {
            out.push(Segment::Text(text));
        }
    }

    fn rich(&mut self, content: &[Inline<'_>]) -> RichText {
        let mut acc = RichBuilder::default();
        for inline in content {
            self.inline(inline, &mut acc);
        }
        acc.finish(self.id())
    }

    fn inline(&mut self, inline: &Inline<'_>, acc: &mut RichBuilder) {
        match &inline.kind {
            InlineKind::Text(t) => acc.push(t),
            InlineKind::SoftBreak if self.opts.strict_line_breaks => acc.push(" "),
            InlineKind::SoftBreak | InlineKind::HardBreak => acc.push("\n"),
            InlineKind::Code(c) => acc.styled(|s| s.code = true, |a| a.push(c)),
            InlineKind::Html(h) => acc.styled(|s| s.html = true, |a| a.push(h)),
            InlineKind::Emphasis(c) => self.wrap(acc, c, |s| s.italic = true),
            InlineKind::Strong(c) => self.wrap(acc, c, |s| s.bold = true),
            InlineKind::Strikethrough(c) => self.wrap(acc, c, |s| s.strike = true),
            InlineKind::Highlight(c) => self.wrap(acc, c, |s| s.highlight = true),
            InlineKind::Link {
                destination,
                kind,
                children,
                ..
            } => {
                let internal = matches!(kind, LinkKind::Inline | LinkKind::Reference)
                    && _markdown::html::is_vault_destination(destination);
                let target = if internal {
                    LinkTarget::Internal(percent_decode(destination).into_owned())
                } else {
                    LinkTarget::External(destination.to_string())
                };
                let start = acc.len();
                if *kind == LinkKind::Autolink {
                    let shown = plain_text(children);
                    let shown = shown.strip_prefix("mailto:").unwrap_or(&shown).to_string();
                    acc.styled(|s| s.link = true, |a| a.push(&shown));
                } else {
                    self.wrap(acc, children, |s| s.link = true);
                }
                acc.link(start, target);
            }
            InlineKind::Image {
                destination,
                children,
                ..
            } => {
                // External or reference-style images: shown as a link until
                // remote images load.
                let start = acc.len();
                let alt = plain_text(children);
                let shown = if alt.is_empty() {
                    destination.to_string()
                } else {
                    alt
                };
                acc.styled(|s| s.link = true, |a| a.push(&shown));
                acc.link(start, LinkTarget::External(destination.to_string()));
            }
            InlineKind::Wikilink { body, embed } => {
                let link = _markdown::parse_linktext(body);
                let target = _markdown::link_target(body).to_string();
                let shown = match link.alias {
                    Some(a) => a.to_string(),
                    None => wikilink_display(&link),
                };
                let start = acc.len();
                if *embed {
                    acc.push("![[");
                    acc.push(&shown);
                    acc.push("]]");
                } else {
                    acc.styled(|s| s.link = true, |a| a.push(&shown));
                    acc.link(start, LinkTarget::Internal(target));
                }
            }
            InlineKind::Tag(name) => {
                let start = acc.len();
                acc.styled(
                    |s| s.tag = true,
                    |a| {
                        a.push("#");
                        a.push(name);
                    },
                );
                acc.link(start, LinkTarget::Tag(name.to_string()));
            }
            InlineKind::Math { content, .. } => {
                acc.styled(|s| s.math = true, |a| a.push(content));
            }
            InlineKind::Comment(_) => {}
            InlineKind::FootnoteReference(label) => {
                if self
                    .footnotes
                    .definitions
                    .iter()
                    .any(|l| l == label.as_ref())
                {
                    let n = self.footnote_number(label);
                    self.footnote_ref(acc, n);
                } else {
                    acc.push(label);
                }
            }
            InlineKind::InlineFootnote(children) => {
                let label = format!("inline-{}", self.footnotes.inline.len() + 1);
                let body = self.rich(children);
                self.footnotes.inline.push((label.clone(), body));
                let n = self.footnote_number(&label);
                self.footnote_ref(acc, n);
            }
        }
    }

    fn wrap(&mut self, acc: &mut RichBuilder, children: &[Inline<'_>], f: fn(&mut InlineStyle)) {
        let saved = acc.style;
        f(&mut acc.style);
        for c in children {
            self.inline(c, acc);
        }
        acc.style = saved;
    }

    fn footnote_number(&mut self, label: &str) -> usize {
        if let Some(i) = self.footnotes.numbers.iter().position(|l| l == label) {
            return i + 1;
        }
        self.footnotes.numbers.push(label.to_string());
        self.footnotes.numbers.len()
    }

    fn footnote_ref(&mut self, acc: &mut RichBuilder, n: usize) {
        let start = acc.len();
        acc.styled(|s| s.footnote = true, |a| a.push(&format!("[{n}]")));
        acc.link(start, LinkTarget::Footnote(n));
    }

    fn footnote_section(&mut self, doc: &Document<'_>) -> Option<RBlock> {
        if self.footnotes.numbers.is_empty() {
            return None;
        }
        let labels = self.footnotes.numbers.clone();
        let mut out = Vec::new();
        for (i, label) in labels.iter().enumerate() {
            let n = i + 1;
            let body =
                if let Some((_, text)) = self.footnotes.inline.iter().find(|(l, _)| l == label) {
                    vec![RBlock::Paragraph(vec![Segment::Text(text.clone())])]
                } else {
                    let mut found = None;
                    _markdown::walk_blocks(&doc.blocks, &mut |b| {
                        if found.is_none() {
                            if let BlockKind::FootnoteDefinition { label: l, children } = &b.kind {
                                if l == label {
                                    found = Some(children);
                                }
                            }
                        }
                    });
                    match found {
                        Some(children) => self.blocks(children, false),
                        None => Vec::new(),
                    }
                };
            out.push((n, body));
        }
        Some(RBlock::Footnotes(out))
    }

    // ----- embeds --------------------------------------------------------------

    fn wikilink_embed(&mut self, body: &str) -> Embed {
        let link = _markdown::parse_linktext(body);
        let target = _markdown::link_target(body);
        let mut width = None;
        let mut height = None;
        let mut alt = None;
        if let Some(a) = link.alias {
            for part in a.split('|').map(str::trim) {
                match size_hint(part) {
                    Some((w, h)) => {
                        width = Some(w);
                        height = h;
                    }
                    None if alt.is_none() && !part.is_empty() => alt = Some(part.to_string()),
                    None => {}
                }
            }
        }
        let alt = alt.unwrap_or_else(|| wikilink_display(&link));
        self.embed(target, alt, width, height)
    }

    fn embed(&mut self, link: &str, alt: String, width: Option<f32>, height: Option<f32>) -> Embed {
        let parsed = _markdown::parse_linktext(link);
        let resolved = self.vault.resolve(link, self.source_path);
        let kind = match resolved {
            None => EmbedKind::Missing,
            Some(path) => match FileKind::from_path(&path) {
                FileKind::Image => EmbedKind::Image(self.vault.absolute(&path)),
                FileKind::Markdown => {
                    if self.depth + 1 >= MAX_EMBED_DEPTH {
                        EmbedKind::TooDeep
                    } else {
                        self.note_embed(&path, parsed.subpath.as_ref(), &wikilink_display(&parsed))
                    }
                }
                other => EmbedKind::Unsupported(other),
            },
        };
        Embed {
            kind,
            link: link.to_string(),
            alt,
            width,
            height,
        }
    }

    fn note_embed(&mut self, path: &str, subpath: Option<&Subpath<'_>>, title: &str) -> EmbedKind {
        let Some(text) = self.vault.read(path) else {
            return EmbedKind::Missing;
        };
        let doc = parse(&text);
        let selected: Vec<Block<'_>> = match subpath {
            None => doc
                .blocks
                .iter()
                .filter(|b| !matches!(b.kind, BlockKind::Yaml { .. }))
                .cloned()
                .collect(),
            Some(Subpath::Headings(names)) => match section(&doc.blocks, names) {
                Some(s) => s.to_vec(),
                None => return EmbedKind::Missing,
            },
            Some(Subpath::Block(id)) => match block_by_id(&doc.blocks, id) {
                Some(b) => vec![b.clone()],
                None => return EmbedKind::Missing,
            },
        };
        let mut next_id = self.next_id;
        let blocks = build_embedded(
            path,
            &doc,
            &selected,
            self.depth + 1,
            &mut next_id,
            self.vault,
            self.opts,
        );
        self.next_id = next_id;
        let title = if title.is_empty() {
            path.trim_end_matches(".md").to_string()
        } else {
            title.to_string()
        };
        EmbedKind::Note {
            path: path.to_string(),
            title,
            blocks,
        }
    }
}

/// Accumulates text with a style stack into one [`RichText`].
#[derive(Default)]
struct RichBuilder {
    text: String,
    style: InlineStyle,
    runs: Vec<(Range<usize>, InlineStyle)>,
    links: Vec<(Range<usize>, LinkTarget)>,
}

impl RichBuilder {
    fn len(&self) -> usize {
        self.text.len()
    }

    fn push(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(s);
        let end = self.text.len();
        if self.style.is_plain() {
            return;
        }
        if let Some((range, style)) = self.runs.last_mut() {
            if *style == self.style && range.end == start {
                range.end = end;
                return;
            }
        }
        self.runs.push((start..end, self.style));
    }

    fn styled(&mut self, f: impl FnOnce(&mut InlineStyle), body: impl FnOnce(&mut RichBuilder)) {
        let saved = self.style;
        f(&mut self.style);
        body(self);
        self.style = saved;
    }

    fn link(&mut self, start: usize, target: LinkTarget) {
        let end = self.text.len();
        if end > start {
            self.links.push((start..end, target));
        }
    }

    fn finish(self, id: usize) -> RichText {
        RichText {
            id,
            text: self.text,
            runs: self.runs,
            links: self.links,
        }
    }
}

/// `200` or `200x100` → `(width, Some(height))`.
fn size_hint(s: &str) -> Option<(f32, Option<f32>)> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit() || b == b'x') {
        return None;
    }
    let mut parts = s.split('x');
    let w: f32 = parts.next()?.parse().ok()?;
    let h = match parts.next() {
        Some(h) => Some(h.parse().ok()?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((w, h))
}

/// Display text of a wikilink without an alias: `Note`, `Note > Heading`, `Note > ^id`.
fn wikilink_display(link: &_markdown::LinkText<'_>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !link.path.is_empty() {
        parts.push(link.path.to_string());
    }
    match &link.subpath {
        Some(Subpath::Headings(hs)) => parts.extend(hs.iter().map(|h| h.to_string())),
        Some(Subpath::Block(b)) => parts.push(format!("^{b}")),
        None => {}
    }
    parts.join(" > ")
}

fn heading_matches(block: &Block<'_>, name: &str) -> Option<u8> {
    if let BlockKind::Heading { level, content, .. } = &block.kind {
        if plain_text(content).trim().eq_ignore_ascii_case(name.trim()) {
            return Some(*level);
        }
    }
    None
}

/// The blocks under a heading path (`#A#B`): from the matched heading to
/// the next heading of the same or a higher level.
fn section<'b, 'a>(blocks: &'b [Block<'a>], names: &[&str]) -> Option<&'b [Block<'a>]> {
    let mut range: &[Block<'a>] = blocks;
    for name in names {
        let start = range
            .iter()
            .position(|b| heading_matches(b, name).is_some())?;
        let level = heading_matches(&range[start], name)?;
        let end = range[start + 1..]
            .iter()
            .position(|b| matches!(&b.kind, BlockKind::Heading { level: l, .. } if *l <= level))
            .map(|i| start + 1 + i)
            .unwrap_or(range.len());
        range = &range[start..end];
    }
    Some(range)
}

fn block_by_id<'b, 'a>(blocks: &'b [Block<'a>], id: &str) -> Option<&'b Block<'a>> {
    let mut found = None;
    _markdown::walk_blocks(blocks, &mut |b| {
        if found.is_none() && b.id.is_some_and(|i| i.id.eq_ignore_ascii_case(id)) {
            found = Some(b);
        }
    });
    if found.is_some() {
        return found;
    }
    // A list item id lives on the item's first paragraph in the cache but
    // the block that carries it in the tree is the paragraph itself, which
    // `walk_blocks` visits; nothing more to do here.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use _formats::properties::Value as PropertyValue;
    use std::collections::HashMap;

    struct Stub {
        files: HashMap<&'static str, &'static str>,
    }

    impl EmbedSource for Stub {
        fn resolve(&self, linktext: &str, _source: &str) -> Option<String> {
            let target = _markdown::parse_linktext(linktext).path;
            let target = if target.is_empty() { "Self" } else { target };
            self.files
                .keys()
                .find(|k| {
                    k.trim_end_matches(".md")
                        .eq_ignore_ascii_case(target.trim_end_matches(".md"))
                })
                .map(|k| k.to_string())
        }
        fn read(&self, path: &str) -> Option<String> {
            self.files.get(path).map(|s| s.to_string())
        }
        fn absolute(&self, path: &str) -> PathBuf {
            PathBuf::from("/vault").join(path)
        }
    }

    fn stub() -> Stub {
        Stub {
            files: HashMap::from([
                ("Self.md", "root"),
                (
                    "Other.md",
                    "---\ntitle: x\n---\n# A\nfirst\n## B\nsecond ^blk\n# C\nthird",
                ),
                ("Loop.md", "![[Loop]]"),
                ("pic.png", ""),
            ]),
        }
    }

    fn para_text(block: &RBlock) -> &RichText {
        match block {
            RBlock::Paragraph(segs) => match &segs[0] {
                Segment::Text(t) => t,
                other => panic!("not text: {other:?}"),
            },
            other => panic!("not a paragraph: {other:?}"),
        }
    }

    #[test]
    fn runs_merge_and_links_cover_their_text() {
        let m = build(
            "Self.md",
            "a **b *c*** [[Note|n]] #tag `x` https://e.com",
            &stub(),
        );
        let t = para_text(&m.blocks[0]);
        assert_eq!(t.text, "a b c n #tag x https://e.com");
        let bold = InlineStyle {
            bold: true,
            ..InlineStyle::default()
        };
        let bold_italic = InlineStyle {
            bold: true,
            italic: true,
            ..InlineStyle::default()
        };
        assert_eq!(t.runs[0], (2..4, bold));
        assert_eq!(t.runs[1], (4..5, bold_italic));
        assert!(t
            .runs
            .iter()
            .any(|(r, s)| s.link && &t.text[r.clone()] == "n"));
        assert!(t
            .runs
            .iter()
            .any(|(r, s)| s.code && &t.text[r.clone()] == "x"));
        assert_eq!(
            t.links,
            vec![
                (6..7, LinkTarget::Internal("Note".into())),
                (8..12, LinkTarget::Tag("tag".into())),
                (15..28, LinkTarget::External("https://e.com".into())),
            ]
        );
    }

    #[test]
    fn soft_breaks_become_newlines_and_comments_vanish() {
        let m = build("Self.md", "one\ntwo %%hidden%% three", &stub());
        assert_eq!(para_text(&m.blocks[0]).text, "one\ntwo  three");
        let strict = BuildOptions {
            strict_line_breaks: true,
        };
        let m = build_with("Self.md", "one\ntwo  \nthree", &stub(), strict);
        assert_eq!(para_text(&m.blocks[0]).text, "one two\nthree");
    }

    #[test]
    fn embeds_split_paragraphs_and_resolve_recursively() {
        let m = build(
            "Self.md",
            "before ![[pic.png|200x100]] ![[Other#B]] ![[Other#^blk]] ![[Nope]] after",
            &stub(),
        );
        let RBlock::Paragraph(segs) = &m.blocks[0] else {
            panic!()
        };
        assert_eq!(segs.len(), 6);
        match &segs[1] {
            Segment::Embed(Embed {
                kind: EmbedKind::Image(p),
                width,
                height,
                ..
            }) => {
                assert_eq!(p, &PathBuf::from("/vault/pic.png"));
                assert_eq!((*width, *height), (Some(200.0), Some(100.0)));
            }
            other => panic!("{other:?}"),
        }
        match &segs[2] {
            Segment::Embed(Embed {
                kind: EmbedKind::Note { title, blocks, .. },
                ..
            }) => {
                assert_eq!(title, "Other > B");
                assert!(matches!(&blocks[0], RBlock::Heading { plain, .. } if plain == "B"));
                assert_eq!(para_text(&blocks[1]).text, "second");
                assert_eq!(blocks.len(), 2, "section stops before # C");
            }
            other => panic!("{other:?}"),
        }
        match &segs[3] {
            Segment::Embed(Embed {
                kind: EmbedKind::Note { blocks, .. },
                ..
            }) => assert_eq!(para_text(&blocks[0]).text, "second"),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            &segs[4],
            Segment::Embed(Embed {
                kind: EmbedKind::Missing,
                ..
            })
        ));
        assert_eq!(para_text(&segs[5].as_text()).text, " after");
    }

    impl Segment {
        fn as_text(&self) -> RBlock {
            match self {
                Segment::Text(t) => RBlock::Paragraph(vec![Segment::Text(t.clone())]),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn recursive_embeds_stop_at_the_depth_limit() {
        let m = build("Loop.md", "![[Loop]]", &stub());
        let mut depth = 0;
        let mut blocks = &m.blocks;
        loop {
            let RBlock::Paragraph(segs) = &blocks[0] else {
                panic!()
            };
            match &segs[0] {
                Segment::Embed(Embed {
                    kind: EmbedKind::Note { blocks: inner, .. },
                    ..
                }) => {
                    depth += 1;
                    blocks = inner;
                }
                Segment::Embed(Embed {
                    kind: EmbedKind::TooDeep,
                    ..
                }) => break,
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(depth, MAX_EMBED_DEPTH - 1);
    }

    #[test]
    fn footnotes_number_by_first_reference_and_collect_at_the_end() {
        let m = build(
            "Self.md",
            "x[^b] y^[inline] z[^a] again[^b] none[^zz]\n\n[^a]: A text\n[^b]: B text",
            &stub(),
        );
        let t = para_text(&m.blocks[0]);
        assert_eq!(t.text, "x[1] y[2] z[3] again[1] nonezz");
        assert_eq!(
            t.links.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>(),
            vec![
                LinkTarget::Footnote(1),
                LinkTarget::Footnote(2),
                LinkTarget::Footnote(3),
                LinkTarget::Footnote(1)
            ]
        );
        let RBlock::Footnotes(notes) = m.blocks.last().unwrap() else {
            panic!("{:?}", m.blocks.last())
        };
        assert_eq!(notes.len(), 3);
        assert_eq!(para_text(&notes[0].1[0]).text, "B text");
        assert_eq!(para_text(&notes[1].1[0]).text, "inline");
        assert_eq!(para_text(&notes[2].1[0]).text, "A text");
    }

    #[test]
    fn properties_lists_and_callouts() {
        let m = build(
            "Self.md",
            "---\ntitle: T\ntags: [a, b]\n---\n> [!TIP]- Fold me\n> body\n\n1. one\n2. [x] two\n\n- [ ] task",
            &stub(),
        );
        let RBlock::Properties(entries) = &m.blocks[0] else {
            panic!("{:?}", m.blocks[0])
        };
        assert_eq!(
            entries.iter().map(|e| e.key.as_str()).collect::<Vec<_>>(),
            vec!["title", "tags"]
        );
        assert_eq!(entries[0].value, PropertyValue::Text("T".into()));
        assert_eq!(
            entries[1].value,
            PropertyValue::List(vec!["a".into(), "b".into()])
        );
        match &m.blocks[1] {
            RBlock::Callout {
                kind, fold, title, ..
            } => {
                assert_eq!(*kind, "tip");
                assert_eq!(*fold, Some(true));
                assert_eq!(title.text, "Fold me");
            }
            other => panic!("{other:?}"),
        }
        match &m.blocks[2] {
            RBlock::List { items, .. } => {
                assert_eq!(items[0].number, Some(1));
                assert_eq!(items[1].number, Some(2));
                assert_eq!(items[1].task, Some('x'));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(m.starts.len(), 4);
        assert_eq!(m.block_at(0), Some(0));
        assert_eq!(m.block_at(m.starts[2] + 3), Some(2));
    }

    #[test]
    fn callout_without_title_shows_its_type() {
        let m = build("Self.md", "> [!warning]\n> text", &stub());
        match &m.blocks[0] {
            RBlock::Callout { title, fold, .. } => {
                assert_eq!(title.text, "Warning");
                assert_eq!(*fold, None);
            }
            other => panic!("{other:?}"),
        }
    }
}
