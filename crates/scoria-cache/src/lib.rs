//! Metadata cache.
//!
//! `index_markdown` turns one note into a `CachedMetadata` record that mirrors
//! Obsidian's public `CachedMetadata` shape. `LinkResolver` holds the vault-wide
//! indexes needed to turn link text into a file path with Obsidian's rules.

pub mod resolver;

pub use resolver::LinkResolver;

use _markdown::{extract_frontmatter, scan_inlines, Inline, Span};
use std::collections::BTreeMap;

/// Line and column are zero-based; `offset` is a byte offset into the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Loc {
    pub start: Pos,
    pub end: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCache {
    /// Parsed target (`Note#Heading`), without the alias.
    pub link: String,
    /// The full original text including brackets.
    pub original: String,
    pub display_text: Option<String>,
    pub position: Loc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagCache {
    /// Includes the leading `#`.
    pub tag: String,
    pub position: Loc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingCache {
    pub heading: String,
    pub level: u8,
    pub position: Loc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCache {
    pub id: String,
    pub position: Loc,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CachedMetadata {
    pub links: Vec<LinkCache>,
    pub embeds: Vec<LinkCache>,
    pub tags: Vec<TagCache>,
    pub headings: Vec<HeadingCache>,
    pub blocks: BTreeMap<String, BlockCache>,
    pub frontmatter: Option<serde_yaml::Value>,
    pub frontmatter_position: Option<Loc>,
}

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(src: &str) -> LineIndex {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    fn pos(&self, offset: usize) -> Pos {
        let line = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        Pos {
            line: line as u32,
            col: (offset - self.starts[line]) as u32,
            offset,
        }
    }

    fn loc(&self, span: Span) -> Loc {
        Loc {
            start: self.pos(span.start),
            end: self.pos(span.end),
        }
    }
}

/// Returns `(fence_char, fence_len)` if the line opens or closes a fenced code block.
fn fence(line: &str) -> Option<(u8, usize)> {
    let t = line.trim_start();
    let b = t.as_bytes();
    let c = *b.first()?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let n = b.iter().take_while(|&&x| x == c).count();
    (n >= 3).then_some((c, n))
}

/// Builds the metadata record for one markdown file.
pub fn index_markdown(src: &str) -> CachedMetadata {
    let lines = LineIndex::new(src);
    let mut meta = CachedMetadata::default();

    let mut body_start = 0;
    if let Some(fm) = extract_frontmatter(src) {
        meta.frontmatter = serde_yaml::from_str::<serde_yaml::Value>(fm.yaml)
            .ok()
            .filter(|v| v.is_mapping());
        meta.frontmatter_position = Some(lines.loc(Span {
            start: fm.start,
            end: fm.end,
        }));
        body_start = fm.end;
    }

    // Walk lines: track fenced code blocks, record headings, collect prose ranges.
    let mut in_fence: Option<(u8, usize)> = None;
    let mut offset = body_start;
    let mut prose: Vec<(usize, usize)> = Vec::new();
    for line in src[body_start..].split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let content = line.trim_end_matches(['\n', '\r']);
        if let Some((c, n)) = fence(content) {
            match in_fence {
                Some((fc, fnn)) if fc == c && n >= fnn => in_fence = None,
                Some(_) => {}
                None => in_fence = Some((c, n)),
            }
            continue;
        }
        if in_fence.is_some() {
            continue;
        }
        let hashes = content.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&hashes) && content[hashes..].starts_with([' ', '\t']) {
            let text = content[hashes..].trim().trim_end_matches('#').trim();
            meta.headings.push(HeadingCache {
                heading: text.to_string(),
                level: hashes as u8,
                position: lines.loc(Span {
                    start: line_start,
                    end: line_start + content.len(),
                }),
            });
            continue;
        }
        prose.push((line_start, line_start + content.len()));
    }

    for (start, end) in prose {
        for inline in scan_inlines(&src[start..end]) {
            match inline {
                Inline::Link { text, embed, span } => {
                    let parsed = _markdown::parse_linktext(text);
                    let link = match text.find('|') {
                        Some(i) => text[..i].trim().to_string(),
                        None => text.trim().to_string(),
                    };
                    let abs = Span {
                        start: start + span.start,
                        end: start + span.end,
                    };
                    let entry = LinkCache {
                        link,
                        original: src[abs.start..abs.end].to_string(),
                        display_text: parsed.alias.map(str::to_string),
                        position: lines.loc(abs),
                    };
                    if embed {
                        meta.embeds.push(entry);
                    } else {
                        meta.links.push(entry);
                    }
                }
                Inline::Tag { name, span } => meta.tags.push(TagCache {
                    tag: format!("#{name}"),
                    position: lines.loc(Span {
                        start: start + span.start,
                        end: start + span.end,
                    }),
                }),
                Inline::BlockId { id, span } => {
                    meta.blocks.insert(
                        id.to_string(),
                        BlockCache {
                            id: id.to_string(),
                            position: lines.loc(Span {
                                start: start + span.start,
                                end: start + span.end,
                            }),
                        },
                    );
                }
            }
        }
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\ntags: [x]\naliases:\n  - Alt\n---\n# Title\n\nSee [[Other#Part|there]] and ![[pic.png]] #tag ^blk1\n\n```\n[[not indexed]] #no\n```\n\n## Second\n";

    #[test]
    fn indexes_everything_outside_code() {
        let m = index_markdown(NOTE);
        assert_eq!(m.headings.len(), 2);
        assert_eq!(m.headings[0].heading, "Title");
        assert_eq!(m.headings[1].level, 2);
        assert_eq!(m.links.len(), 1);
        assert_eq!(m.links[0].link, "Other#Part");
        assert_eq!(m.links[0].display_text.as_deref(), Some("there"));
        assert_eq!(m.links[0].original, "[[Other#Part|there]]");
        assert_eq!(m.embeds.len(), 1);
        assert_eq!(m.tags.len(), 1);
        assert_eq!(m.tags[0].tag, "#tag");
        assert!(m.blocks.contains_key("blk1"));
        let fm = m.frontmatter.unwrap();
        assert_eq!(fm["aliases"][0], "Alt");
    }

    #[test]
    fn positions_are_line_and_column_accurate() {
        let m = index_markdown("a\nbb [[c]]\n");
        let p = m.links[0].position;
        assert_eq!((p.start.line, p.start.col), (1, 3));
        assert_eq!((p.end.line, p.end.col), (1, 8));
    }
}
