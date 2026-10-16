# Functional specification

The full specification is maintained as a living document:

- Obsidian Functional Specification (Obsidian 1.12.7, derived from observing its behavior, file
  formats, and public documentation):
  https://claude.ai/code/artifact/6bc126ce-788f-484a-8213-c6eec1f99245

Sections: purpose and evidence basis; architecture; vault and filesystem model; Obsidian Flavored
Markdown; metadata cache and link resolution; editor; workspace shell; the 35 core plugins; Canvas
and Bases formats; search query language; settings, hotkeys and theming; main-process services, IPC
and URI protocol; community plugin API; out-of-scope services; feasibility and phased plan; appendices
with the command inventory and bundle evidence.

## Invariants this repository enforces today

| Invariant | Where |
| --- | --- |
| Known file kinds and extension tables | `-formats::FileKind` |
| Forbidden file-name characters `* " \ / < > : \| ?` and no leading dot | `-formats::validate_file_name` |
| One parser for editor, reading view, cache, and link rewriting; every node carries its byte span | `-markdown::parse` |
| CommonMark 0.31.2: all 652 spec examples render byte-identical HTML | `-markdown/tests/commonmark.rs` (ratchet) |
| GFM tables (`\|` does not split a cell, even in code), strikethrough, task items with any single character, footnotes, bare URLs | `-markdown::Options` |
| Frontmatter only on line 1, BOM and CRLF tolerated | `-markdown::extract_frontmatter` |
| Wikilink body grammar `path#heading#heading\|alias`, `#^block` | `-markdown::parse_linktext` |
| Wikilinks and embeds are not parsed inside code spans; `[[` needs `]]` on the same line | `-markdown/tests/obsidian.rs` |
| Tags need a non-numeric character; `# Heading` is not a tag; `#` after an alphanumeric is not a tag | `-markdown/tests/obsidian.rs` |
| Block ids: ` ^id` at the end of a paragraph or list item, or `^id` alone on a line after a table, list, or code block | `-markdown/tests/obsidian.rs` |
| Callouts: `> [!type]-` or `+` on the first quoted line; title is the rest of the line; built-in types and aliases | `-markdown::canonical_callout_type` |
| `==highlight==`, `~~strike~~` need exactly two delimiters; `$x$` needs no space inside and no digit after; `%%` comments hide inline and block text | `-markdown/tests/obsidian.rs` |
| Reading-view hooks: `internal-link` with `data-href`, `tag`, `internal-embed` with `src`/`alt`, `callout` with `data-callout`/`data-callout-fold`, `task-list-item` with `data-task`, `data-heading` | `-markdown::render_html` |
| Cache record: links, embeds, tags, headings, sections with Obsidian's type names, list items with Obsidian's `parent` encoding, blocks, footnotes, frontmatter links with key paths | `-cache::index_markdown` |
| Link resolution: extension handling, root then relative, basename with same-folder then shortest path, case-insensitive | `-cache::LinkResolver` |
| Config files are sparse and optional; unknown keys round-trip | `-formats::config`, `-vault::Vault::read_config` |
| `core-plugins.json` map and legacy array shapes | `-formats::config::CorePlugins` |
| JSON Canvas 1.0 fields, z-order by array order, tab-indented output | `-formats::canvas` |
| Bases top-level keys and recursive filters | `-formats::base` |
| Command ids and default hotkeys | `-commands` |

## Parser notes

The parser follows the CommonMark two-phase design: a line-by-line block pass
with a stack of open containers (lazy continuation, tab stops, list item
indentation), then an inline pass per paragraph, heading, callout title, and
table cell using the delimiter stack for emphasis and the bracket stack for
links. Obsidian's constructs are extra block starts and inline scanners in the
same two passes, each behind a flag in `Options` so the CommonMark suite runs
with them off.

Decisions taken from observation that the spec text does not spell out:

- A table's header row is the last line of the preceding paragraph; earlier
  lines stay a paragraph. The header or delimiter row must contain a pipe.
- A paragraph that is only `^id` attaches the id to the previous block at the
  same level, which is how Obsidian ids tables and code blocks.
- An unterminated `$$` or `%%` block runs to the end of the file.
- Footnote references to undefined labels stay literal text; inline footnotes
  `^[text]` are numbered in order of appearance after referenced definitions.
- Markdown links count as vault links in the cache when the destination has
  no URL scheme; the destination is percent-decoded.
