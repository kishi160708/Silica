//! Live Preview's view of one line: the source with formatting markers
//! hidden or replaced, except in constructs the cursor or selection
//! touches, which show their source. Pure functions over one line and its
//! highlight spans, so the editor shapes exactly what these return and
//! maps clicks back through the same pieces.

use _markdown::highlight::{HighlightKind, HighlightSpan};
use std::ops::Range;

/// One run of the display text: copied from the source, or substituted
/// for a source range (a hidden marker is a substitution of nothing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Copy {
        display: usize,
        source: usize,
        len: usize,
    },
    Sub {
        display: usize,
        source: Range<usize>,
        len: usize,
        /// The construct that was rendered, for styling and clicks.
        kind: HighlightKind,
    },
}

/// Block decorations a line carries in Live Preview, painted by the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Decor {
    /// Quote nesting depth: one bar per level.
    pub quote_depth: u8,
    /// The line is a thematic break: draw a rule instead of text.
    pub rule: bool,
    /// Inside a fenced or indented code block.
    pub code_block: bool,
    /// Inside the frontmatter.
    pub frontmatter: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayLine {
    pub text: String,
    /// In display order, covering the whole display text.
    pub pieces: Vec<Piece>,
    /// Font size multiplier (headings).
    pub scale: f32,
    pub decor: Decor,
}

impl DisplayLine {
    /// The source line unchanged (source mode, or nothing to hide).
    pub fn verbatim(text: &str, scale: f32, decor: Decor) -> DisplayLine {
        DisplayLine {
            text: text.to_string(),
            pieces: vec![Piece::Copy {
                display: 0,
                source: 0,
                len: text.len(),
            }],
            scale,
            decor,
        }
    }

    #[cfg(test)]
    pub fn is_verbatim(&self) -> bool {
        matches!(self.pieces.as_slice(), [Piece::Copy { .. }])
    }

    /// Display offset of source byte `s` (the start of a substitution when
    /// `s` falls inside one, its end when `s` is at its end).
    pub fn to_display(&self, s: usize) -> usize {
        for p in &self.pieces {
            match p {
                Piece::Copy {
                    display,
                    source,
                    len,
                } => {
                    if s >= *source && s <= source + len {
                        return display + (s - source);
                    }
                }
                Piece::Sub {
                    display,
                    source,
                    len,
                    ..
                } => {
                    if s >= source.start && s < source.end {
                        return *display;
                    }
                    if s == source.end {
                        return display + len;
                    }
                }
            }
        }
        self.text.len()
    }

    /// Source offset of display byte `d`; a point inside a substitution
    /// maps to its start (its end past the midpoint). At a boundary the
    /// later piece wins, so a click before text lands after a hidden
    /// opening marker.
    pub fn to_source(&self, d: usize) -> usize {
        let mut last_end = 0;
        for p in self.pieces.iter().rev() {
            match p {
                Piece::Copy {
                    display,
                    source,
                    len,
                } => {
                    if d >= *display && d <= display + len {
                        return source + (d - display);
                    }
                    last_end = last_end.max(source + len);
                }
                Piece::Sub {
                    display,
                    source,
                    len,
                    ..
                } => {
                    if d >= *display && d < display + len {
                        return if d - display < len.div_ceil(2) {
                            source.start
                        } else {
                            source.end
                        };
                    }
                    last_end = last_end.max(source.end);
                }
            }
        }
        last_end
    }

    /// The substitution under display byte `d`, if any.
    pub fn sub_at(&self, d: usize) -> Option<(Range<usize>, HighlightKind)> {
        self.pieces.iter().find_map(|p| match p {
            Piece::Sub {
                display,
                source,
                len,
                kind,
            } if d >= *display && d < display + len => Some((source.clone(), *kind)),
            _ => None,
        })
    }
}

/// Obsidian's heading sizes relative to the body (`--h1-size` and so on).
pub fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 1.802,
        2 => 1.602,
        3 => 1.424,
        4 => 1.266,
        5 => 1.125,
        _ => 1.0,
    }
}

/// Block decorations and font scale of a line, from its spans.
pub fn line_block(spans: &[HighlightSpan]) -> (f32, Decor) {
    let mut scale = 1.0;
    let mut decor = Decor::default();
    for s in spans {
        match s.kind {
            HighlightKind::Heading(level) => scale = heading_scale(level),
            HighlightKind::QuoteMarker => decor.quote_depth += 1,
            HighlightKind::ThematicBreak => decor.rule = true,
            HighlightKind::CodeBlock => decor.code_block = true,
            HighlightKind::Frontmatter => decor.frontmatter = true,
            _ => {}
        }
    }
    (scale, decor)
}

/// Builds the Live Preview text of `line`. `touch` is the part of the
/// selection on this line (a collapsed cursor included), line-relative;
/// any construct it touches keeps its source.
pub fn display_line(
    line: &str,
    spans: &[HighlightSpan],
    touch: Option<Range<usize>>,
) -> DisplayLine {
    let (scale, decor) = line_block(spans);
    let mut hides: Vec<(Range<usize>, &str, HighlightKind)> = Vec::new();
    for (ix, span) in spans.iter().enumerate() {
        let revealed = touch
            .as_ref()
            .is_some_and(|t| t.start <= span.range.end && t.end >= span.range.start);
        if revealed {
            continue;
        }
        let r = span.range.clone();
        let Some(t) = line.get(r.clone()) else {
            continue;
        };
        let k = span.kind;
        let both_ends =
            |n: usize, open: &[&str], hides: &mut Vec<(Range<usize>, &str, HighlightKind)>| {
                if t.len() >= 2 * n && open.iter().any(|o| t.starts_with(o)) && t.ends_with(&t[..n])
                {
                    hides.push((r.start..r.start + n, "", k));
                    hides.push((r.end - n..r.end, "", k));
                }
            };
        match k {
            HighlightKind::Heading(_) => {
                let hashes = t.bytes().take_while(|&b| b == b'#').count();
                let ws = t[hashes..]
                    .bytes()
                    .take_while(|&b| b == b' ' || b == b'\t')
                    .count();
                if hashes > 0 && ws > 0 {
                    hides.push((r.start..r.start + hashes + ws, "", k));
                }
            }
            HighlightKind::Strong => both_ends(2, &["**", "__"], &mut hides),
            HighlightKind::Emphasis => both_ends(1, &["*", "_"], &mut hides),
            HighlightKind::Strikethrough => both_ends(2, &["~~"], &mut hides),
            HighlightKind::Highlight => both_ends(2, &["=="], &mut hides),
            HighlightKind::Code => {
                let n = t.bytes().take_while(|&b| b == b'`').count();
                if n > 0 && t.len() >= 2 * n && t.ends_with(&t[..n]) {
                    hides.push((r.start..r.start + n, "", k));
                    hides.push((r.end - n..r.end, "", k));
                }
            }
            HighlightKind::Wikilink | HighlightKind::Embed => {
                let open = if t.starts_with("![[") {
                    3
                } else if t.starts_with("[[") {
                    2
                } else {
                    0
                };
                if open > 0 && t.ends_with("]]") && t.len() >= open + 2 {
                    let inner = &t[open..t.len() - 2];
                    match inner.find('|') {
                        // `[[target|alias]]` shows the alias.
                        Some(p) if p + 1 < inner.len() => {
                            hides.push((r.start..r.start + open + p + 1, "", k));
                        }
                        _ => hides.push((r.start..r.start + open, "", k)),
                    }
                    hides.push((r.end - 2..r.end, "", k));
                }
            }
            HighlightKind::Link => {
                let open = if t.starts_with("![") {
                    2
                } else if t.starts_with('[') {
                    1
                } else {
                    0
                };
                if open > 0 {
                    // The `]` closing the text, counting nested brackets.
                    let mut depth = 0usize;
                    let mut close = None;
                    for (i, b) in t.bytes().enumerate().skip(open) {
                        match b {
                            b'[' => depth += 1,
                            b']' if depth == 0 => {
                                close = Some(i);
                                break;
                            }
                            b']' => depth -= 1,
                            _ => {}
                        }
                    }
                    if let Some(close) = close {
                        hides.push((r.start..r.start + open, "", k));
                        hides.push((r.start + close..r.end, "", k));
                    }
                }
            }
            HighlightKind::Url => {
                if t.starts_with('<') && t.ends_with('>') && t.len() >= 2 {
                    hides.push((r.start..r.start + 1, "", k));
                    hides.push((r.end - 1..r.end, "", k));
                }
            }
            HighlightKind::Comment | HighlightKind::QuoteMarker | HighlightKind::ThematicBreak => {
                hides.push((r.clone(), "", k));
            }
            HighlightKind::BlockId => {
                // Take the space before the id along with it.
                let start = if r.start > 0 && line.as_bytes()[r.start - 1] == b' ' {
                    r.start - 1
                } else {
                    r.start
                };
                hides.push((start..r.end, "", k));
            }
            HighlightKind::ListMarker => {
                let task_after = spans
                    .iter()
                    .skip(ix + 1)
                    .find(|s| s.kind == HighlightKind::Task && s.range.start == r.end + 1);
                match task_after {
                    // A task shows its checkbox in place of the bullet.
                    Some(task) => hides.push((r.start..task.range.start, "", k)),
                    None if t.len() == 1 && matches!(t, "-" | "*" | "+") => {
                        hides.push((r.clone(), "•", k));
                    }
                    None => {}
                }
            }
            HighlightKind::Task => {
                let glyph = if t.as_bytes().get(1) == Some(&b' ') {
                    "☐"
                } else {
                    "☑"
                };
                hides.push((r.clone(), glyph, k));
            }
            HighlightKind::Callout => {
                // `> [!type]- Title`: the type block goes, an empty title
                // shows the type instead.
                if let Some(open) = t.find("[!") {
                    if let Some(close) = t[open..].find(']') {
                        let mut end = open + close + 1;
                        if matches!(t.as_bytes().get(end), Some(b'+' | b'-')) {
                            end += 1;
                        }
                        let title_empty = t[end..].trim().is_empty();
                        if title_empty {
                            let ty = &t[open + 2..open + close];
                            let mut name: String =
                                ty.chars().take(1).flat_map(char::to_uppercase).collect();
                            name.push_str(ty.get(1..).unwrap_or(""));
                            let name: &'static str = Box::leak(name.into_boxed_str());
                            hides.push((r.start + open..r.start + end, name, k));
                        } else {
                            let ws = t[end..].bytes().take_while(|&b| b == b' ').count();
                            hides.push((r.start + open..r.start + end + ws, "", k));
                        }
                    }
                }
            }
            HighlightKind::Footnote => {
                // `[^ref]` and inline `^[text]` both lose their brackets.
                if (t.starts_with("[^") || t.starts_with("^[")) && t.ends_with(']') && t.len() >= 3
                {
                    hides.push((r.start..r.start + 2, "", k));
                    hides.push((r.end - 1..r.end, "", k));
                }
            }
            HighlightKind::CodeBlock
            | HighlightKind::Frontmatter
            | HighlightKind::Math
            | HighlightKind::Html
            | HighlightKind::Tag => {}
        }
    }
    if hides.is_empty() {
        return DisplayLine::verbatim(line, scale, decor);
    }
    hides.sort_by(|a, b| a.0.start.cmp(&b.0.start).then(b.0.end.cmp(&a.0.end)));
    let mut text = String::with_capacity(line.len());
    let mut pieces = Vec::new();
    let mut at = 0;
    for (range, with, kind) in hides {
        if range.start < at
            || range.end > line.len()
            || !line.is_char_boundary(range.start)
            || !line.is_char_boundary(range.end)
        {
            // Inside an earlier hide (a comment swallows its contents).
            continue;
        }
        if range.start > at {
            pieces.push(Piece::Copy {
                display: text.len(),
                source: at,
                len: range.start - at,
            });
            text.push_str(&line[at..range.start]);
        }
        pieces.push(Piece::Sub {
            display: text.len(),
            source: range.clone(),
            len: with.len(),
            kind,
        });
        text.push_str(with);
        at = range.end;
    }
    if at < line.len() {
        pieces.push(Piece::Copy {
            display: text.len(),
            source: at,
            len: line.len() - at,
        });
        text.push_str(&line[at..]);
    }
    DisplayLine {
        text,
        pieces,
        scale,
        decor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use _markdown::highlight::highlight_lines;

    fn render(src: &str, touch: Option<Range<usize>>) -> DisplayLine {
        let doc = _markdown::parse(src);
        let lines = highlight_lines(&doc);
        let line = src.lines().next().unwrap_or("");
        display_line(line, &lines[0], touch)
    }

    #[test]
    fn inline_markers_hide_unless_touched() {
        assert_eq!(
            render("a **b** *c* ~~d~~ ==e== `f`", None).text,
            "a b c d e f"
        );
        assert_eq!(render("**b** and __c__", None).text, "b and c");
        // The cursor right after the bold reveals it; elsewhere not.
        assert_eq!(render("**b** *c*", Some(5..5)).text, "**b** c");
        assert_eq!(render("**b** *c*", Some(0..0)).text, "**b** c");
        assert_eq!(render("**b** *c*", Some(6..6)).text, "b *c*");
    }

    #[test]
    fn links_show_their_text() {
        assert_eq!(
            render("[[Note]] [[Note|alias]] [[Note#H]]", None).text,
            "Note alias Note#H"
        );
        assert_eq!(render("![[pic.png]]", None).text, "pic.png");
        assert_eq!(
            render("[text](https://x.y) ![alt](a.png)", None).text,
            "text alt"
        );
        assert_eq!(
            render("<https://x.y> and https://z", None).text,
            "https://x.y and https://z"
        );
        assert_eq!(
            render("see[^1] and ^[inline]", None).text,
            "see1 and inline"
        );
    }

    #[test]
    fn block_markers_render() {
        let h = render("## Title", None);
        assert_eq!(h.text, "Title");
        assert_eq!(h.scale, heading_scale(2));
        assert_eq!(render("- item", None).text, "• item");
        assert_eq!(render("1. item", None).text, "1. item");
        assert_eq!(render("- [ ] open", None).text, "☐ open");
        assert_eq!(render("- [x] done", None).text, "☑ done");
        let q = render("> > deep", None);
        assert_eq!(q.text, "deep");
        assert_eq!(q.decor.quote_depth, 2);
        assert_eq!(render("> [!note] Title", None).text, "Title");
        assert_eq!(render("> [!tip]", None).text, "Tip");
        let hr = render("---\n", None);
        assert_eq!(hr.text, "");
        assert!(hr.decor.rule);
        assert_eq!(
            render("text %%hidden **x**%% end ^id", None).text,
            "text  end"
        );
    }

    #[test]
    fn offsets_map_both_ways() {
        let d = render("a **b** c", None);
        assert_eq!(d.text, "a b c");
        assert_eq!(d.to_display(0), 0);
        assert_eq!(d.to_display(2), 2); // start of `**`
        assert_eq!(d.to_display(4), 2); // after `**`
        assert_eq!(d.to_display(5), 3);
        assert_eq!(d.to_display(9), 5);
        assert_eq!(d.to_source(2), 4);
        assert_eq!(d.to_source(3), 7);
        assert_eq!(d.to_source(5), 9);
        let t = render("- [ ] open", None);
        assert_eq!(t.sub_at(0), Some((2..5, HighlightKind::Task)));
        assert_eq!(t.to_source(4), 6);
        assert!(render("plain", None).is_verbatim());
    }
}
