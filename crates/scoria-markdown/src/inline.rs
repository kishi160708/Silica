//! Inline scanner for Obsidian-only syntax: wikilinks, embeds, tags, block ids.
//!
//! Code spans (`` `...` ``) are skipped, as Obsidian does. This scanner is
//! deliberately line-agnostic; callers decide which blocks (code fences,
//! comments) to exclude before handing text in.

/// Byte range into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline<'a> {
    /// `[[text]]` or `![[text]]`; `text` is the raw body between the brackets.
    Link {
        text: &'a str,
        embed: bool,
        span: Span,
    },
    /// `#tag`, including nested `#a/b`; `name` excludes the `#`.
    Tag { name: &'a str, span: Span },
    /// ` ^id` at the end of a line; `id` excludes the `^`.
    BlockId { id: &'a str, span: Span },
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

fn is_block_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// Skips a code span starting at `i` (which points at a backtick) and returns
/// the index just past its closing run, or past the opening run if unterminated.
fn skip_code_span(bytes: &[u8], i: usize) -> usize {
    let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
    let open_end = i + run;
    let mut j = open_end;
    while j < bytes.len() {
        if bytes[j] == b'`' {
            let r = bytes[j..].iter().take_while(|&&b| b == b'`').count();
            if r == run {
                return j + r;
            }
            j += r;
        } else {
            j += 1;
        }
    }
    open_end
}

pub fn scan_inlines(src: &str) -> Vec<Inline<'_>> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                i = skip_code_span(bytes, i);
            }
            b'[' if i + 1 < bytes.len() && bytes[i + 1] == b'[' => {
                let embed = i > 0 && bytes[i - 1] == b'!';
                let body_start = i + 2;
                if let Some(rel) = src[body_start..].find("]]") {
                    let text = &src[body_start..body_start + rel];
                    if !text.contains('\n') && !text.contains("[[") {
                        let start = if embed { i - 1 } else { i };
                        let end = body_start + rel + 2;
                        out.push(Inline::Link {
                            text,
                            embed,
                            span: Span { start, end },
                        });
                        i = end;
                        continue;
                    }
                }
                i += 2;
            }
            b'#' => {
                let prev_ok =
                    i == 0 || !src[..i].ends_with(|c: char| c.is_alphanumeric() || c == '#');
                let rest = &src[i + 1..];
                let len = rest
                    .chars()
                    .take_while(|&c| is_tag_char(c))
                    .map(char::len_utf8)
                    .sum::<usize>();
                let name = &rest[..len];
                // A tag needs at least one non-numeric character; `# Heading` has an empty name.
                if prev_ok && !name.is_empty() && name.chars().any(|c| !c.is_ascii_digit()) {
                    out.push(Inline::Tag {
                        name,
                        span: Span {
                            start: i,
                            end: i + 1 + len,
                        },
                    });
                    i += 1 + len;
                    continue;
                }
                i += 1;
            }
            b'^' if i > 0 && bytes[i - 1] == b' ' => {
                let rest = &src[i + 1..];
                let len = rest.chars().take_while(|&c| is_block_id_char(c)).count();
                let after = &rest[len..];
                if len > 0
                    && (after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n"))
                {
                    out.push(Inline::BlockId {
                        id: &rest[..len],
                        span: Span {
                            start: i,
                            end: i + 1 + len,
                        },
                    });
                    i += 1 + len;
                    continue;
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_embeds_and_tags() {
        let src = "See [[Note#H|alias]] and ![[img.png|200]] with #tag/nested here";
        let found = scan_inlines(src);
        assert_eq!(found.len(), 3);
        assert!(matches!(
            found[0],
            Inline::Link {
                text: "Note#H|alias",
                embed: false,
                ..
            }
        ));
        assert!(matches!(
            found[1],
            Inline::Link {
                text: "img.png|200",
                embed: true,
                ..
            }
        ));
        assert!(matches!(
            found[2],
            Inline::Tag {
                name: "tag/nested",
                ..
            }
        ));
    }

    #[test]
    fn code_spans_are_skipped() {
        let found = scan_inlines("`[[not a link]]` but [[real]] and `` #x `` #y");
        assert_eq!(found.len(), 2);
        assert!(matches!(found[0], Inline::Link { text: "real", .. }));
        assert!(matches!(found[1], Inline::Tag { name: "y", .. }));
    }

    #[test]
    fn numeric_only_is_not_a_tag_and_headings_are_not_tags() {
        assert!(scan_inlines("#123").is_empty());
        assert!(scan_inlines("# Heading").is_empty());
        assert_eq!(scan_inlines("#1a").len(), 1);
    }

    #[test]
    fn block_id_only_at_line_end() {
        let found = scan_inlines("para ^abc-1\nnext ^x y");
        assert_eq!(found.len(), 1);
        assert!(matches!(found[0], Inline::BlockId { id: "abc-1", .. }));
    }
}
