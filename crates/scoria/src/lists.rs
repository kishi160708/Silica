//! Smart lists and quotes for the editor: what Enter continues, what Tab
//! and Shift+Tab indent, and how ordered items renumber. Pure functions
//! over one line's text so they are testable without a window.
//!
//! A line is read as `indent` `quotes` `list_indent` `marker` `task` `rest`,
//! for example `  > > - [x] done`: two spaces of outer indent, two quote
//! markers, no list indent, a `-` bullet, a checked task and `done`.

use std::ops::Range;

/// The prefix of one line that Enter would continue.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prefix {
    /// Whitespace before the first quote marker (or list marker).
    pub indent: Range<usize>,
    /// `> ` markers, verbatim, possibly nested; empty when the line is not quoted.
    pub quotes: Range<usize>,
    /// Whitespace between the quotes and the list marker.
    pub list_indent: Range<usize>,
    /// The list marker without its trailing space; `None` on a plain line.
    pub marker: Option<Marker>,
    /// `[ ]` or `[x]` after the marker, with its trailing space.
    pub task: Option<Range<usize>>,
    /// Where the item's own text starts.
    pub content: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    Bullet {
        range: Range<usize>,
        ch: char,
    },
    Ordered {
        range: Range<usize>,
        number: u64,
        delimiter: char,
    },
}

impl Marker {
    fn range(&self) -> Range<usize> {
        match self {
            Marker::Bullet { range, .. } | Marker::Ordered { range, .. } => range.clone(),
        }
    }
}

/// Parses the continuable prefix of `line` (one line, no newline).
pub fn parse_prefix(line: &str) -> Prefix {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let indent = 0..i;
    let quotes_start = i;
    loop {
        if i < b.len() && b[i] == b'>' {
            i += 1;
            if i < b.len() && b[i] == b' ' {
                i += 1;
            }
            // Up to three spaces may precede a nested marker.
            let mut j = i;
            let mut spaces = 0;
            while j < b.len() && b[j] == b' ' && spaces < 3 {
                j += 1;
                spaces += 1;
            }
            if j < b.len() && b[j] == b'>' {
                i = j;
                continue;
            }
        }
        break;
    }
    let quotes = quotes_start..i;
    let mut indent = indent;
    let mut list_start = i;
    if quotes.is_empty() {
        // No quote: the leading whitespace is the list's own nesting.
        list_start = 0;
        i = indent.end;
        indent = 0..0;
    }
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let list_indent = list_start..i;
    let mut prefix = Prefix {
        indent,
        quotes,
        list_indent: list_indent.clone(),
        marker: None,
        task: None,
        content: i,
    };
    // A marker needs a space (or the end of the line) after it.
    let marker = if i < b.len() && matches!(b[i], b'-' | b'*' | b'+') {
        if i + 1 == b.len() || b[i + 1] == b' ' {
            Some(Marker::Bullet {
                range: i..i + 1,
                ch: b[i] as char,
            })
        } else {
            None
        }
    } else {
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() && j - i < 9 {
            j += 1;
        }
        if j > i
            && j < b.len()
            && matches!(b[j], b'.' | b')')
            && (j + 1 == b.len() || b[j + 1] == b' ')
        {
            Some(Marker::Ordered {
                range: i..j + 1,
                number: line[i..j].parse().unwrap_or(0),
                delimiter: b[j] as char,
            })
        } else {
            None
        }
    };
    let Some(marker) = marker else {
        // No list: the whitespace belongs to the line's indentation.
        if prefix.quotes.is_empty() {
            prefix.indent = 0..list_indent.end;
        }
        prefix.list_indent = list_start..list_start;
        prefix.content = if prefix.quotes.is_empty() {
            prefix.indent.end
        } else {
            list_start
        };
        return prefix;
    };
    i = marker.range().end;
    if i < b.len() && b[i] == b' ' {
        i += 1;
    }
    // `[ ] ` or `[x] ` with any single character inside.
    if i + 3 <= b.len()
        && b[i] == b'['
        && b[i + 2] == b']'
        && (i + 3 == b.len() || b[i + 3] == b' ')
    {
        let mut end = i + 3;
        if end < b.len() && b[end] == b' ' {
            end += 1;
        }
        prefix.task = Some(i..end);
        i = end;
    }
    prefix.marker = Some(marker);
    prefix.content = i;
    prefix
}

/// What Enter does at byte `col` of `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Continuation {
    /// Insert a newline followed by this prefix (the indentation alone on a
    /// plain line, the quote and list markers on a continued item).
    Insert(String),
    /// The item is empty: replace `range` of the line with `with` (clears
    /// the marker, or outdents a nested item) instead of adding a line.
    Clear { range: Range<usize>, with: String },
}

/// Smart-list behaviour for Enter at `col`: continues the marker, numbering
/// and checkbox, and clears or outdents an empty item.
pub fn continuation(line: &str, col: usize) -> Continuation {
    let p = parse_prefix(line);
    if col < p.content {
        // Before the marker: only the leading whitespace carries over.
        let mut ws = line[p.indent.clone()].to_string();
        if p.quotes.is_empty() {
            ws.push_str(&line[p.list_indent.clone()]);
        }
        return Continuation::Insert(ws);
    }
    let mut prefix = String::new();
    prefix.push_str(&line[p.indent.clone()]);
    prefix.push_str(&line[p.quotes.clone()]);
    let Some(marker) = &p.marker else {
        // A bare quote line continues the quote; an empty one ends it.
        if !p.quotes.is_empty() && line[p.content..].trim().is_empty() {
            return Continuation::Clear {
                range: p.indent.start..line.len(),
                with: String::new(),
            };
        }
        return Continuation::Insert(prefix);
    };
    if line[p.content..].trim().is_empty() {
        // Empty item: a nested one outdents, a top-level one loses its marker.
        if !p.list_indent.is_empty() {
            let unit = outdent_width(&line[p.list_indent.clone()]);
            let cut = p.list_indent.start..p.list_indent.start + unit;
            return Continuation::Clear {
                range: cut,
                with: String::new(),
            };
        }
        return Continuation::Clear {
            range: p.list_indent.start..line.len(),
            with: String::new(),
        };
    }
    prefix.push_str(&line[p.list_indent.clone()]);
    match marker {
        Marker::Bullet { ch, .. } => {
            prefix.push(*ch);
            prefix.push(' ');
        }
        Marker::Ordered {
            number, delimiter, ..
        } => {
            prefix.push_str(&(number + 1).to_string());
            prefix.push(*delimiter);
            prefix.push(' ');
        }
    }
    if p.task.is_some() {
        prefix.push_str("[ ] ");
    }
    Continuation::Insert(prefix)
}

/// Bytes of one indentation step to remove from the start of `ws`: a tab,
/// or up to four spaces.
pub fn outdent_width(ws: &str) -> usize {
    let b = ws.as_bytes();
    if b.first() == Some(&b'\t') {
        return 1;
    }
    b.iter().take(4).take_while(|&&c| c == b' ').count()
}

/// The indentation unit for Tab (`useTab`, `tabSize`).
pub fn indent_unit(use_tab: bool, tab_size: u8) -> String {
    if use_tab {
        "\t".to_string()
    } else {
        " ".repeat(tab_size.max(1) as usize)
    }
}

/// Where an indentation step goes on this line: after the quote markers
/// for a list item, at the line start otherwise.
pub fn indent_point(line: &str) -> usize {
    let p = parse_prefix(line);
    if p.marker.is_some() {
        p.list_indent.start
    } else {
        0
    }
}

/// The range an outdent removes from this line, if there is anything to remove.
pub fn outdent_range(line: &str) -> Option<Range<usize>> {
    let p = parse_prefix(line);
    let ws = if p.marker.is_some() {
        p.list_indent.clone()
    } else {
        p.indent.clone()
    };
    let n = outdent_width(&line[ws.clone()]);
    (n > 0).then(|| ws.start..ws.start + n)
}

/// Ordered items that follow an item on `lines[from]` at the same depth,
/// each with the number it should carry, counting from `next`. Deeper
/// items are skipped; a blank line, a bullet or a shallower line ends the
/// list. Returns `(line index, marker range, new number)` for items whose
/// number is wrong.
pub fn renumber_following<'a>(
    lines: impl Iterator<Item = &'a str>,
    reference: &str,
    mut next: u64,
) -> Vec<(usize, Range<usize>, u64)> {
    let r = parse_prefix(reference);
    let depth = (
        &reference[r.indent.clone()],
        &reference[r.quotes.clone()],
        &reference[r.list_indent.clone()],
    );
    let mut out = Vec::new();
    for (ix, line) in lines.enumerate() {
        if line.trim().is_empty() {
            break;
        }
        let p = parse_prefix(line);
        let here = (
            &line[p.indent.clone()],
            &line[p.quotes.clone()],
            &line[p.list_indent.clone()],
        );
        if here.0 != depth.0 || here.1 != depth.1 {
            break;
        }
        if here.2.len() > depth.2.len() {
            continue;
        }
        if here.2 != depth.2 {
            break;
        }
        match p.marker {
            Some(Marker::Ordered { range, number, .. }) => {
                if number != next {
                    out.push((ix, range.start..range.end - 1, next));
                }
                next += 1;
            }
            _ => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cont(line: &str) -> Continuation {
        continuation(line, line.len())
    }

    #[test]
    fn bullets_numbers_and_tasks_continue() {
        assert_eq!(cont("- item"), Continuation::Insert("- ".into()));
        assert_eq!(cont("* item"), Continuation::Insert("* ".into()));
        assert_eq!(cont("3. item"), Continuation::Insert("4. ".into()));
        assert_eq!(cont("3) item"), Continuation::Insert("4) ".into()));
        assert_eq!(cont("- [x] done"), Continuation::Insert("- [ ] ".into()));
        assert_eq!(cont("  - nested"), Continuation::Insert("  - ".into()));
        assert_eq!(cont("\t1. nested"), Continuation::Insert("\t2. ".into()));
    }

    #[test]
    fn quotes_continue_and_combine_with_lists() {
        assert_eq!(cont("> quoted"), Continuation::Insert("> ".into()));
        assert_eq!(cont("> > deep"), Continuation::Insert("> > ".into()));
        assert_eq!(cont("> - item"), Continuation::Insert("> - ".into()));
        assert_eq!(cont("> [!note] Title"), Continuation::Insert("> ".into()));
    }

    #[test]
    fn plain_lines_keep_their_indentation_only() {
        assert_eq!(cont("text"), Continuation::Insert(String::new()));
        assert_eq!(cont("    code"), Continuation::Insert("    ".into()));
        assert_eq!(cont("-not a list"), Continuation::Insert(String::new()));
        assert_eq!(cont("1.5 not a list"), Continuation::Insert(String::new()));
    }

    #[test]
    fn empty_items_clear_or_outdent() {
        assert_eq!(
            cont("- "),
            Continuation::Clear {
                range: 0..2,
                with: String::new()
            }
        );
        assert_eq!(
            cont("- [ ] "),
            Continuation::Clear {
                range: 0..6,
                with: String::new()
            }
        );
        assert_eq!(
            cont("    - "),
            Continuation::Clear {
                range: 0..4,
                with: String::new()
            }
        );
        assert_eq!(
            cont("\t- "),
            Continuation::Clear {
                range: 0..1,
                with: String::new()
            }
        );
        assert_eq!(
            cont("> - "),
            Continuation::Clear {
                range: 2..4,
                with: String::new()
            }
        );
        assert_eq!(
            cont("> "),
            Continuation::Clear {
                range: 0..2,
                with: String::new()
            }
        );
    }

    #[test]
    fn enter_before_the_marker_does_not_continue() {
        assert_eq!(
            continuation("- item", 0),
            Continuation::Insert(String::new())
        );
        assert_eq!(
            continuation("  - item", 1),
            Continuation::Insert("  ".into())
        );
        // Right after the marker's space the item counts as continued.
        assert_eq!(continuation("- item", 2), Continuation::Insert("- ".into()));
    }

    #[test]
    fn indent_and_outdent_points() {
        assert_eq!(indent_point("- a"), 0);
        assert_eq!(indent_point("> - a"), 2);
        assert_eq!(indent_point("text"), 0);
        assert_eq!(outdent_range("    - a"), Some(0..4));
        assert_eq!(outdent_range("\t\t- a"), Some(0..1));
        assert_eq!(outdent_range("> \t- a"), Some(2..3));
        assert_eq!(outdent_range("- a"), None);
        assert_eq!(outdent_range("  text"), Some(0..2));
        assert_eq!(indent_unit(true, 4), "\t");
        assert_eq!(indent_unit(false, 2), "  ");
    }

    #[test]
    fn following_ordered_items_renumber() {
        let lines = ["2. b", "   - nested", "3. c", "9. d", "", "7. e"];
        let fixes = renumber_following(lines.iter().copied(), "1. a", 2);
        assert_eq!(fixes, vec![(3, 0..1, 4)]);
        let fixes = renumber_following(lines.iter().copied(), "1. a", 3);
        assert_eq!(fixes, vec![(0, 0..1, 3), (2, 0..1, 4), (3, 0..1, 5)]);
        // A bullet or a shallower line ends the run.
        let lines = ["  2. b", "- c"];
        assert_eq!(
            renumber_following(lines.iter().copied(), "  1. a", 2),
            vec![]
        );
    }
}
