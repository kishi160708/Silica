//! Live Preview's view of one line: the source with formatting markers
//! hidden or replaced, except in constructs the cursor or selection
//! touches, which show their source. Pure functions over one line and its
//! highlight spans, so the editor shapes exactly what these return and
//! maps clicks back through the same pieces.

use _markdown::highlight::{HighlightKind, HighlightSpan};
use std::borrow::Cow;
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

/// Offset inside an ATX heading's text where its closing `#` sequence
/// begins, whitespace included, when it has one. The parser drops the same
/// run when it takes the heading's text, so Live Preview hides it.
fn atx_closing(text: &str) -> Option<usize> {
    let kept = text.trim_end_matches([' ', '\t']);
    let stripped = kept.trim_end_matches('#');
    if stripped.len() == kept.len() {
        return None;
    }
    if stripped.is_empty() {
        Some(0)
    } else if stripped.ends_with([' ', '\t']) {
        Some(stripped.trim_end_matches([' ', '\t']).len())
    } else {
        None
    }
}

/// Block decorations and font scale of a line, from its spans.
pub fn line_block(spans: &[HighlightSpan]) -> (f32, Decor) {
    let mut scale = 1.0;
    let mut decor = Decor::default();
    for s in spans {
        match s.kind {
            // A setext underline is the heading's markup, so the row it
            // sits on keeps the body size.
            HighlightKind::Heading(h) if !h.underline => {
                scale = heading_scale(h.level);
            }
            HighlightKind::QuoteMarker => decor.quote_depth += 1,
            HighlightKind::ThematicBreak => decor.rule = true,
            HighlightKind::CodeBlock => decor.code_block = true,
            HighlightKind::Frontmatter => decor.frontmatter = true,
            _ => {}
        }
    }
    (scale, decor)
}

/// Whether a selection or cursor at `touch` (line-relative) reveals the
/// construct at `range`: it does when the two meet, ends included.
pub fn touches(touch: &[Range<usize>], range: &Range<usize>) -> bool {
    touch
        .iter()
        .any(|t| t.start <= range.end && t.end >= range.start)
}

/// Builds the Live Preview text of `line`. `touch` holds the parts of the
/// selections on this line (collapsed cursors included), line-relative;
/// any construct one of them touches keeps its source.
pub fn display_line(line: &str, spans: &[HighlightSpan], touch: &[Range<usize>]) -> DisplayLine {
    let (scale, decor) = line_block(spans);
    let mut hides: Vec<(Range<usize>, Cow<'static, str>, HighlightKind)> = Vec::new();
    for (ix, span) in spans.iter().enumerate() {
        let revealed = touches(touch, &span.range);
        if revealed {
            continue;
        }
        let r = span.range.clone();
        let Some(t) = line.get(r.clone()) else {
            continue;
        };
        let k = span.kind;
        let both_ends =
            |n: usize,
             open: &[&str],
             hides: &mut Vec<(Range<usize>, Cow<'static, str>, HighlightKind)>| {
                if t.len() >= 2 * n && open.iter().any(|o| t.starts_with(o)) && t.ends_with(&t[..n])
                {
                    hides.push((r.start..r.start + n, "".into(), k));
                    hides.push((r.end - n..r.end, "".into(), k));
                }
            };
        match k {
            HighlightKind::Heading(h) if h.underline => {
                hides.push((r.clone(), "".into(), k));
            }
            HighlightKind::Heading(_) => {
                let hashes = t.bytes().take_while(|&b| b == b'#').count();
                let ws = t[hashes..]
                    .bytes()
                    .take_while(|&b| b == b' ' || b == b'\t')
                    .count();
                if hashes > 0 && ws > 0 {
                    hides.push((r.start..r.start + hashes + ws, "".into(), k));
                    let after = r.start + hashes + ws;
                    if let Some(close) = atx_closing(&t[hashes + ws..]) {
                        hides.push((after + close..r.end, "".into(), k));
                    }
                }
            }
            HighlightKind::Strong => both_ends(2, &["**", "__"], &mut hides),
            HighlightKind::Emphasis => both_ends(1, &["*", "_"], &mut hides),
            HighlightKind::Strikethrough => both_ends(2, &["~~"], &mut hides),
            HighlightKind::Highlight => both_ends(2, &["=="], &mut hides),
            HighlightKind::Code => {
                let n = t.bytes().take_while(|&b| b == b'`').count();
                if n > 0 && t.len() >= 2 * n && t.ends_with(&t[..n]) {
                    hides.push((r.start..r.start + n, "".into(), k));
                    hides.push((r.end - n..r.end, "".into(), k));
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
                            hides.push((r.start..r.start + open + p + 1, "".into(), k));
                        }
                        _ => {
                            hides.push((r.start..r.start + open, "".into(), k));
                            // A subpath reads `Note > Heading`, the label the
                            // reading view already gives it; a link into this
                            // note has no path, so its leading `#` just goes.
                            for (i, _) in inner.match_indices('#') {
                                let at = r.start + open + i;
                                let with = if i == 0 { "" } else { " > " };
                                hides.push((at..at + 1, with.into(), k));
                            }
                        }
                    }
                    hides.push((r.end - 2..r.end, "".into(), k));
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
                        hides.push((r.start..r.start + open, "".into(), k));
                        hides.push((r.start + close..r.end, "".into(), k));
                    }
                }
            }
            HighlightKind::Url => {
                if t.starts_with('<') && t.ends_with('>') && t.len() >= 2 {
                    hides.push((r.start..r.start + 1, "".into(), k));
                    hides.push((r.end - 1..r.end, "".into(), k));
                }
            }
            HighlightKind::Comment | HighlightKind::QuoteMarker | HighlightKind::ThematicBreak => {
                hides.push((r.clone(), "".into(), k));
            }
            HighlightKind::BlockId => {
                // Take the space before the id along with it.
                let start = if r.start > 0 && line.as_bytes()[r.start - 1] == b' ' {
                    r.start - 1
                } else {
                    r.start
                };
                hides.push((start..r.end, "".into(), k));
            }
            HighlightKind::ListMarker => {
                // The parser accepts one to four spaces or a tab between
                // the marker and the checkbox, and all of it is markup.
                let task_after = spans.iter().skip(ix + 1).find(|s| {
                    s.kind == HighlightKind::Task
                        && s.range.start >= r.end
                        && line
                            .get(r.end..s.range.start)
                            .is_some_and(|gap| gap.bytes().all(|b| b == b' ' || b == b'\t'))
                });
                match task_after {
                    // A task shows its checkbox in place of the bullet.
                    Some(task) => hides.push((r.start..task.range.start, "".into(), k)),
                    None if t.len() == 1 && matches!(t, "-" | "*" | "+") => {
                        hides.push((r.clone(), "•".into(), k));
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
                hides.push((r.clone(), glyph.into(), k));
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
                            hides.push((r.start + open..r.start + end, name.into(), k));
                        } else {
                            let ws = t[end..].bytes().take_while(|&b| b == b' ').count();
                            hides.push((r.start + open..r.start + end + ws, "".into(), k));
                        }
                    }
                }
            }
            HighlightKind::Footnote => {
                // `[^ref]` and inline `^[text]` both lose their brackets.
                if (t.starts_with("[^") || t.starts_with("^[")) && t.ends_with(']') && t.len() >= 3
                {
                    hides.push((r.start..r.start + 2, "".into(), k));
                    hides.push((r.end - 1..r.end, "".into(), k));
                }
            }
            HighlightKind::Math => {
                // `$x$` and `$$x$$` lose their dollars; a block's lines are
                // a widget when it is not being edited, so only the
                // delimiters on this line go.
                let n = t.bytes().take_while(|&b| b == b'$').count().min(2);
                if n > 0 && t.len() >= 2 * n && t.ends_with(&t[..n]) {
                    hides.push((r.start..r.start + n, "".into(), k));
                    hides.push((r.end - n..r.end, "".into(), k));
                }
            }
            HighlightKind::CodeBlock
            | HighlightKind::Frontmatter
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
        text.push_str(&with);
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
        render_at(src, 0, touch)
    }

    fn render_at(src: &str, at: usize, touch: Option<Range<usize>>) -> DisplayLine {
        let doc = _markdown::parse(src);
        let lines = highlight_lines(&doc);
        let line = src.lines().nth(at).unwrap_or("");
        let touch: Vec<Range<usize>> = touch.into_iter().collect();
        display_line(line, &lines[at], &touch)
    }

    #[test]
    fn inline_markers_hide_unless_touched() {
        assert_eq!(
            render("a **b** *c* ~~d~~ ==e== `f`", None).text,
            "a b c d e f"
        );
        assert_eq!(render("**b** and __c__", None).text, "b and c");
        assert_eq!(render("so $x^2$ and $$y$$", None).text, "so x^2 and y");
        assert_eq!(render("so $x^2$ and", Some(4..4)).text, "so $x^2$ and");
        // The cursor right after the bold reveals it; elsewhere not.
        assert_eq!(render("**b** *c*", Some(5..5)).text, "**b** c");
        assert_eq!(render("**b** *c*", Some(0..0)).text, "**b** c");
        assert_eq!(render("**b** *c*", Some(6..6)).text, "b *c*");
    }

    #[test]
    fn links_show_their_text() {
        assert_eq!(
            render("[[Note]] [[Note|alias]] [[Note#H]]", None).text,
            "Note alias Note > H"
        );
        assert_eq!(render("![[pic.png]]", None).text, "pic.png");
        assert_eq!(render("![[Note#H]]", None).text, "Note > H");
        // A block id keeps its caret, and a link into this note has no
        // path to separate from.
        assert_eq!(
            render("[[Note#A#B]] [[Note#^id]] [[#Local]]", None).text,
            "Note > A > B Note > ^id Local"
        );
        assert_eq!(render("[[Note#H]]", Some(0..0)).text, "[[Note#H]]");
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
        // Every padding the parser accepts between bullet and checkbox.
        assert_eq!(render("-  [x] padded", None).text, "☑ padded");
        assert_eq!(render("-    [ ] four", None).text, "☐ four");
        assert_eq!(render("-\t[x] tabbed", None).text, "☑ tabbed");
        assert_eq!(render("> -  [ ] quoted", None).text, "☐ quoted");
        let q = render("> > deep", None);
        assert_eq!(q.text, "deep");
        assert_eq!(q.decor.quote_depth, 2);
        assert_eq!(render("> [!note] Title", None).text, "Title");
        // An empty title shows the capitalised type, foldable or not.
        assert_eq!(render("> [!tip]", None).text, "Tip");
        assert_eq!(render("> [!warning]-", None).text, "Warning");
        assert_eq!(render("> [!tip]", Some(0..0)).text, "> [!tip]");
        let hr = render("---\n", None);
        assert_eq!(hr.text, "");
        assert!(hr.decor.rule);
        assert_eq!(
            render("text %%hidden **x**%% end ^id", None).text,
            "text  end"
        );
    }

    #[test]
    fn closed_atx_headings_drop_their_trailing_hashes() {
        assert_eq!(render("## Title ##", None).text, "Title");
        assert_eq!(render("## Title ##   ", None).text, "Title");
        assert_eq!(
            render("## Closing ## sequence ##", None).text,
            "Closing ## sequence"
        );
        assert_eq!(render("## ##", None).text, "");
        // Not a closing sequence: no space before the run.
        assert_eq!(render("## Title#", None).text, "Title#");
        assert_eq!(render("## Title ##", Some(0..0)).text, "## Title ##");
    }

    #[test]
    fn a_setext_underline_is_markup() {
        let src = "Setext title\n============\n";
        let title = render_at(src, 0, None);
        assert_eq!(title.text, "Setext title");
        assert_eq!(title.scale, heading_scale(1));
        let underline = render_at(src, 1, None);
        assert_eq!(underline.text, "");
        assert_eq!(underline.scale, 1.0);
        // The cursor on the underline brings it back.
        assert_eq!(render_at(src, 1, Some(0..0)).text, "============");
        let h2 = render_at("Setext two\n---\n", 1, None);
        assert_eq!(h2.text, "");
        assert_eq!(h2.scale, 1.0);
        // A title that is itself a run of the underline character: the
        // parser marks which row is the markup, so the title row is still
        // a heading and only the row below it goes.
        let title = render_at("==\n==\n", 0, None);
        assert_eq!(title.text, "==");
        assert_eq!(title.scale, heading_scale(1));
        let underline = render_at("==\n==\n", 1, None);
        assert_eq!(underline.text, "");
        assert_eq!(underline.scale, 1.0);
        // The same for the level 2 shape, whose title reads as a break.
        let title = render_at("--\n--\n", 0, None);
        assert_eq!(title.text, "--");
        assert_eq!(title.scale, heading_scale(2));
        assert_eq!(render_at("--\n--\n", 1, None).text, "");
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
