//! Syntax highlighting spans for editors, derived from the parsed tree.
//!
//! [`highlight_lines`] returns, for every source line, the byte ranges inside
//! that line that belong to a construct worth decorating: headings, links,
//! tags, code, emphasis, and so on. Spans nest (a tag inside a heading, a
//! link inside bold text); [`flatten`] turns one line's spans into
//! non-overlapping segments with every kind that applies, which is what a
//! text layout with per-run styles needs.

use crate::ast::*;
use crate::span::{LineIndex, Span};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    /// Heading line of the given level, marker included.
    Heading(u8),
    /// `[[link]]`
    Wikilink,
    /// `![[embed]]`
    Embed,
    /// `[text](url)`, `![alt](src)`, reference links.
    Link,
    /// Autolinks and bare URLs.
    Url,
    /// `#tag`
    Tag,
    /// Inline code span.
    Code,
    /// Fenced or indented code block.
    CodeBlock,
    Emphasis,
    Strong,
    Strikethrough,
    /// `==highlight==`
    Highlight,
    /// `%%comment%%` and comment blocks.
    Comment,
    /// `$math$` and math blocks.
    Math,
    /// YAML properties block.
    Frontmatter,
    /// `^block-id`
    BlockId,
    /// The `> [!type] Title` line of a callout.
    Callout,
    /// Bullet or number of a list item.
    ListMarker,
    /// `[ ]`, `[x]` task marker.
    Task,
    ThematicBreak,
    /// Raw HTML, inline or block.
    Html,
    /// Footnote references and inline footnotes.
    Footnote,
}

/// A highlighted byte range within one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub range: Range<usize>,
    pub kind: HighlightKind,
}

/// Per-line highlight spans, in order of start offset; outer spans come
/// before the spans they contain.
pub fn highlight_lines(doc: &Document<'_>) -> Vec<Vec<HighlightSpan>> {
    let lines = LineIndex::new(doc.source);
    let mut global: Vec<(Span, HighlightKind)> = Vec::new();
    collect_blocks(doc.source, &doc.blocks, &lines, &mut global);
    global.sort_by(|a, b| a.0.start.cmp(&b.0.start).then(b.0.end.cmp(&a.0.end)));

    let mut out: Vec<Vec<HighlightSpan>> = vec![Vec::new(); lines.line_count()];
    for (span, kind) in global {
        if span.is_empty() {
            continue;
        }
        let first = lines.pos(span.start).line as usize;
        let last = lines.pos(span.end.saturating_sub(1).max(span.start)).line as usize;
        for (line, line_spans) in out.iter_mut().enumerate().take(last + 1).skip(first) {
            let ls = lines.line_start(line);
            let le = if line + 1 < lines.line_count() {
                lines.line_start(line + 1) - 1
            } else {
                doc.source.len()
            };
            let start = span.start.max(ls) - ls;
            let end = span.end.min(le).max(ls) - ls;
            if start < end {
                line_spans.push(HighlightSpan {
                    range: start..end,
                    kind,
                });
            }
        }
    }
    out
}

/// Splits one line's nested spans into non-overlapping segments, each with
/// the kinds that apply, outermost first. Gaps between segments are plain text.
pub fn flatten(spans: &[HighlightSpan]) -> Vec<(Range<usize>, Vec<HighlightKind>)> {
    let mut bounds: Vec<usize> = spans
        .iter()
        .flat_map(|s| [s.range.start, s.range.end])
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut out = Vec::new();
    for w in bounds.windows(2) {
        let (a, b) = (w[0], w[1]);
        let kinds: Vec<HighlightKind> = spans
            .iter()
            .filter(|s| s.range.start <= a && s.range.end >= b)
            .map(|s| s.kind)
            .collect();
        if !kinds.is_empty() {
            out.push((a..b, kinds));
        }
    }
    out
}

fn line_range_at(src: &str, lines: &LineIndex, offset: usize) -> Span {
    let line = lines.pos(offset).line as usize;
    let start = lines.line_start(line);
    let end = if line + 1 < lines.line_count() {
        lines.line_start(line + 1) - 1
    } else {
        src.len()
    };
    Span::new(start, end)
}

fn collect_blocks(
    src: &str,
    blocks: &[Block<'_>],
    lines: &LineIndex,
    out: &mut Vec<(Span, HighlightKind)>,
) {
    for block in blocks {
        if let Some(id) = block.id {
            out.push((id.span, HighlightKind::BlockId));
        }
        match &block.kind {
            BlockKind::Yaml { .. } => out.push((block.span, HighlightKind::Frontmatter)),
            BlockKind::Paragraph { content } => collect_inlines(content, out),
            BlockKind::Heading { level, content, .. } => {
                out.push((block.span, HighlightKind::Heading(*level)));
                collect_inlines(content, out);
            }
            BlockKind::ThematicBreak => out.push((block.span, HighlightKind::ThematicBreak)),
            BlockKind::CodeBlock { .. } => out.push((block.span, HighlightKind::CodeBlock)),
            BlockKind::HtmlBlock { .. } => out.push((block.span, HighlightKind::Html)),
            BlockKind::BlockQuote { children } => collect_blocks(src, children, lines, out),
            BlockKind::Callout {
                title, children, ..
            } => {
                out.push((
                    line_range_at(src, lines, block.span.start),
                    HighlightKind::Callout,
                ));
                collect_inlines(title, out);
                collect_blocks(src, children, lines, out);
            }
            BlockKind::List { items, .. } => {
                for item in items {
                    out.push((item.marker, HighlightKind::ListMarker));
                    if item.task.is_some() {
                        let start = item.marker.end + 1;
                        let end = (start + 3).min(src.len());
                        if src.get(start..end).is_some_and(|s| s.starts_with('[')) {
                            out.push((Span::new(start, end), HighlightKind::Task));
                        }
                    }
                    collect_blocks(src, &item.children, lines, out);
                }
            }
            BlockKind::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    collect_inlines(&cell.content, out);
                }
            }
            BlockKind::Math { .. } => out.push((block.span, HighlightKind::Math)),
            BlockKind::Comment { .. } => out.push((block.span, HighlightKind::Comment)),
            BlockKind::FootnoteDefinition { children, .. } => {
                collect_blocks(src, children, lines, out)
            }
        }
    }
}

fn collect_inlines(inlines: &[Inline<'_>], out: &mut Vec<(Span, HighlightKind)>) {
    for inline in inlines {
        let kind = match &inline.kind {
            InlineKind::Text(_) | InlineKind::SoftBreak | InlineKind::HardBreak => None,
            InlineKind::Code(_) => Some(HighlightKind::Code),
            InlineKind::Html(_) => Some(HighlightKind::Html),
            InlineKind::Emphasis(c) => {
                collect_inlines(c, out);
                Some(HighlightKind::Emphasis)
            }
            InlineKind::Strong(c) => {
                collect_inlines(c, out);
                Some(HighlightKind::Strong)
            }
            InlineKind::Strikethrough(c) => {
                collect_inlines(c, out);
                Some(HighlightKind::Strikethrough)
            }
            InlineKind::Highlight(c) => {
                collect_inlines(c, out);
                Some(HighlightKind::Highlight)
            }
            InlineKind::Link { kind, children, .. } => {
                collect_inlines(children, out);
                Some(match kind {
                    LinkKind::Autolink | LinkKind::BareUrl => HighlightKind::Url,
                    _ => HighlightKind::Link,
                })
            }
            InlineKind::Image { children, .. } => {
                collect_inlines(children, out);
                Some(HighlightKind::Link)
            }
            InlineKind::Wikilink { embed: true, .. } => Some(HighlightKind::Embed),
            InlineKind::Wikilink { .. } => Some(HighlightKind::Wikilink),
            InlineKind::Tag(_) => Some(HighlightKind::Tag),
            InlineKind::Math { .. } => Some(HighlightKind::Math),
            InlineKind::Comment(_) => Some(HighlightKind::Comment),
            InlineKind::FootnoteReference(_) => Some(HighlightKind::Footnote),
            InlineKind::InlineFootnote(c) => {
                collect_inlines(c, out);
                Some(HighlightKind::Footnote)
            }
        };
        if let Some(kind) = kind {
            out.push((inline.span, kind));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn spans_are_per_line_and_nested() {
        let src = "# Title #t\n\n**bold [[Link]]** and `code`\n- [ ] task ^id\n";
        let doc = parse(src);
        let lines = highlight_lines(&doc);
        assert_eq!(lines.len(), 5);
        let kinds = |l: usize| -> Vec<(Range<usize>, HighlightKind)> {
            lines[l].iter().map(|s| (s.range.clone(), s.kind)).collect()
        };
        assert_eq!(
            kinds(0),
            vec![
                (0..10, HighlightKind::Heading(1)),
                (8..10, HighlightKind::Tag)
            ]
        );
        assert_eq!(
            kinds(2),
            vec![
                (0..17, HighlightKind::Strong),
                (7..15, HighlightKind::Wikilink),
                (22..28, HighlightKind::Code),
            ]
        );
        assert_eq!(
            kinds(3),
            vec![
                (0..1, HighlightKind::ListMarker),
                (2..5, HighlightKind::Task),
                (11..14, HighlightKind::BlockId),
            ]
        );
        let flat = flatten(&lines[2]);
        assert_eq!(
            flat,
            vec![
                (0..7, vec![HighlightKind::Strong]),
                (7..15, vec![HighlightKind::Strong, HighlightKind::Wikilink]),
                (15..17, vec![HighlightKind::Strong]),
                (22..28, vec![HighlightKind::Code]),
            ]
        );
    }

    #[test]
    fn multi_line_constructs_split_per_line() {
        let src = "```rs\nlet x;\n```\n*a\nb*\n";
        let lines = highlight_lines(&parse(src));
        assert!(lines[0..3]
            .iter()
            .all(|l| l.iter().any(|s| s.kind == HighlightKind::CodeBlock)));
        assert_eq!(lines[3][0].range, 0..2);
        assert_eq!(lines[4][0].range, 0..2);
        assert_eq!(lines[4][0].kind, HighlightKind::Emphasis);
    }
}
