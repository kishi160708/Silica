//! Splitting one note into several, one note per heading: the note
//! composer's "split file".
//!
//! Obsidian's split cuts a note at a heading the user picks: the text above
//! the heading stays where it was, and the heading with everything under it
//! becomes a note of its own. Silica goes one step further down that road
//! and cuts at every heading of the same level from the picked one on, so a
//! note written as a pile of `##` sections comes apart into one note per
//! section rather than into two halves. A heading *deeper* than the picked
//! one stays inside its section, and a heading *shallower* than it ends the
//! split: whatever follows such a heading is not part of what was picked,
//! so it stays in the note it came from.
//!
//! The text is taken as lines, each keeping its own terminator, so a CRLF
//! note keeps its endings on both sides of the cut and a note with no final
//! newline keeps having none until the last line of the last section.

/// One heading a note has: its level, its text and the line it sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// How many `#`s the heading is written with, 1 to 6.
    pub level: u8,
    /// The heading's text, without the `#`s and the spaces around them.
    pub text: String,
    /// The `\n`-ended line index the heading sits on, counting from 0.
    pub line: usize,
}

/// Where a note was cut at the heading on `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    /// The lines above the cut: what stays in the note it came from.
    pub head: String,
    /// One note per heading of the cut heading's level, from the cut on.
    pub sections: Vec<Section>,
    /// What the split left behind below the last section: the lines from
    /// the first heading shallower than the cut's, or empty.
    pub tail: String,
}

/// One note the split makes: a heading and everything under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading this note is named after.
    pub heading: Heading,
    /// The heading's line and everything under it, up to the next section.
    pub text: String,
}

impl Split {
    /// The note that stays: what was above the cut, and whatever the split
    /// left behind below it, in the order they were written.
    pub fn kept(&self) -> String {
        let mut out = String::with_capacity(self.head.len() + self.tail.len());
        out.push_str(&self.head);
        out.push_str(&self.tail);
        out
    }

    /// The headings the sections took out, for the link rewrites that
    /// repoint every `[[note#heading]]` at the note that took it in.
    pub fn moved(&self) -> Vec<&str> {
        self.sections
            .iter()
            .map(|s| s.heading.text.as_str())
            .collect()
    }
}

/// The note's headings, in the order they appear.
pub fn headings(text: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    if text.is_empty() {
        return out;
    }
    for (line, raw) in text.split_inclusive('\n').enumerate() {
        if let Some((level, heading)) = heading_of(raw) {
            out.push(Heading {
                level,
                text: heading.to_string(),
                line,
            });
        }
    }
    out
}

/// Cuts `text` at the heading on `line`: everything above it stays, every
/// heading of its level from there on takes its own note, and the first
/// heading shallower than it ends the split. A line with no heading on it
/// cuts nothing, which is what a caller that was given a stale line gets.
pub fn split_at(text: &str, line: usize) -> Split {
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').collect()
    };
    let Some((level, _)) = lines.get(line).and_then(|raw| heading_of(raw)) else {
        return Split {
            head: text.to_string(),
            sections: Vec::new(),
            tail: String::new(),
        };
    };
    let mut sections = Vec::new();
    let mut start = line;
    let mut tail_at = lines.len();
    for at in line + 1..lines.len() {
        let Some((other, _)) = heading_of(lines[at]) else {
            continue;
        };
        if other < level {
            // * Shallower: the note's own outline goes back up a level, so
            //   this is the note's structure rather than the picked
            //   section's. It stays.
            tail_at = at;
            break;
        }
        if other == level {
            sections.push(section(&lines, start, at));
            start = at;
        }
    }
    sections.push(section(&lines, start, tail_at));
    Split {
        head: lines[..line].concat(),
        sections,
        tail: lines[tail_at..].concat(),
    }
}

/// The note a heading becomes: its text with the characters a file name may
/// not carry replaced by spaces, trimmed, and cut to a length every
/// filesystem takes; `Untitled` when nothing is left of it.
///
/// The vault validates a name when the note is created, so this is the
/// shape that keeps a heading like `What now?` from being refused there
/// rather than a second, stricter rule.
pub fn note_name(heading: &str) -> String {
    let cleaned: String = heading
        .chars()
        .map(|c| {
            if matches!(
                c,
                '*' | '"' | '\\' | '/' | '<' | '>' | ':' | '|' | '?' | '\n' | '\r' | '\t'
            ) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    let name = if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    };
    // * A component of 255 bytes is the shortest limit of the filesystems
    //   Silica runs on, and cutting on a char boundary is what keeps a
    //   multi-byte heading from being split mid-character.
    let mut end = name.len().min(255);
    while end > 0 && !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].trim_end().to_string()
}

/// `line[start..end]` as one note's text, with a final newline put back
/// when the note it came from did not end in one.
fn section(lines: &[&str], start: usize, end: usize) -> Section {
    let mut text: String = lines[start..end].concat();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let raw = lines[start];
    // * The heading this section opens with is on `start`; the split only
    //   ever calls this with a heading there.
    let (level, heading) = heading_of(raw).unwrap_or((1, ""));
    Section {
        heading: Heading {
            level,
            text: heading.to_string(),
            line: start,
        },
        text,
    }
}

/// The `#`s already on `line` with the space after them, and the heading's
/// level and text; `None` for a line that is not a heading. A line inside a
/// code fence is not skipped here: the caller is a buffer, not a rendered
/// note, and a `#` opening a fenced block is a rare enough shape that the
/// note it makes is still what the user picked.
fn heading_of(line: &str) -> Option<(u8, &str)> {
    let body = line.trim_end_matches(['\n', '\r']);
    let bytes = body.as_bytes();
    let mut at = 0;
    while at < bytes.len() && (bytes[at] == b' ' || bytes[at] == b'\t') {
        at += 1;
    }
    let start = at;
    while at < bytes.len() && bytes[at] == b'#' && at - start < 6 {
        at += 1;
    }
    if at == start || at >= bytes.len() || bytes[at] != b' ' {
        return None;
    }
    Some(((at - start) as u8, body[at + 1..].trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_are_found_in_order_with_their_lines() {
        let text = "# One\n\nbody\n\n## Two\n\n### Deep\n\n## Three\n";
        let found = headings(text);
        assert_eq!(
            found,
            vec![
                Heading {
                    level: 1,
                    text: "One".into(),
                    line: 0
                },
                Heading {
                    level: 2,
                    text: "Two".into(),
                    line: 4
                },
                Heading {
                    level: 3,
                    text: "Deep".into(),
                    line: 6
                },
                Heading {
                    level: 2,
                    text: "Three".into(),
                    line: 8
                },
            ]
        );
    }

    #[test]
    fn a_hash_with_no_space_or_a_tag_is_not_a_heading() {
        assert!(headings("#tag\n").is_empty());
        assert!(headings("###\n").is_empty());
        assert_eq!(headings("  ## Indented\n").len(), 1);
    }

    #[test]
    fn splitting_at_a_heading_keeps_what_is_above_it() {
        let text = "# Top\n\nintro\n\n## Two\n\nbody two\n\n## Three\n\nbody three\n";
        let split = split_at(text, 4);
        assert_eq!(split.head, "# Top\n\nintro\n\n");
        assert_eq!(split.tail, "");
        assert_eq!(split.sections.len(), 2);
        assert_eq!(split.sections[0].heading.text, "Two");
        assert_eq!(split.sections[0].text, "## Two\n\nbody two\n\n");
        assert_eq!(split.sections[1].text, "## Three\n\nbody three\n");
        assert_eq!(split.kept(), "# Top\n\nintro\n\n");
    }

    #[test]
    fn a_deeper_heading_stays_inside_its_section() {
        let text = "## Two\n\nbody\n\n### Deep\n\ndeep body\n\n## Three\n";
        let split = split_at(text, 0);
        assert_eq!(split.sections.len(), 2);
        assert_eq!(
            split.sections[0].text,
            "## Two\n\nbody\n\n### Deep\n\ndeep body\n\n"
        );
        assert_eq!(split.sections[1].text, "## Three\n");
    }

    #[test]
    fn a_shallower_heading_ends_the_split_and_stays_behind() {
        let text = "## Two\n\nbody\n\n# Back\n\nrest\n";
        let split = split_at(text, 0);
        assert_eq!(split.sections.len(), 1);
        assert_eq!(split.tail, "# Back\n\nrest\n");
        assert_eq!(split.kept(), "# Back\n\nrest\n");
    }

    #[test]
    fn the_first_line_of_a_note_can_be_the_cut() {
        let text = "## Two\n\nbody\n";
        let split = split_at(text, 0);
        assert_eq!(split.head, "");
        assert_eq!(split.kept(), "");
        assert_eq!(split.sections.len(), 1);
    }

    #[test]
    fn a_line_with_no_heading_on_it_cuts_nothing() {
        let text = "# One\nbody\n";
        let split = split_at(text, 1);
        assert_eq!(split.head, text);
        assert!(split.sections.is_empty());
    }

    #[test]
    fn a_section_without_a_final_newline_gets_one() {
        let text = "## Two\nbody";
        let split = split_at(text, 0);
        assert_eq!(split.sections[0].text, "## Two\nbody\n");
    }

    #[test]
    fn crlf_endings_stay_on_both_sides_of_the_cut() {
        let text = "# Top\r\n\r\n## Two\r\nbody\r\n";
        let split = split_at(text, 2);
        assert_eq!(split.head, "# Top\r\n\r\n");
        assert_eq!(split.sections[0].text, "## Two\r\nbody\r\n");
    }

    #[test]
    fn a_note_name_drops_what_a_file_name_refuses() {
        assert_eq!(note_name("Two"), "Two");
        assert_eq!(note_name(" What now? "), "What now");
        assert_eq!(note_name("a/b:c*d"), "a b c d");
        assert_eq!(note_name("   "), "Untitled");
        assert_eq!(note_name(""), "Untitled");
    }

    #[test]
    fn a_note_name_never_runs_past_the_length_a_component_takes() {
        let long = "é".repeat(300);
        let name = note_name(&long);
        assert!(name.len() <= 255, "{} bytes", name.len());
    }
}
