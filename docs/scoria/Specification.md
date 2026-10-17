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
| Obsidian 1.13.7 reading view: 1007 of 1009 corpus notes render the same canonical HTML | `-markdown/tests/corpus.rs` (ratchet; vault and expected HTML under `tests/corpus`) |
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
| Vault-wide cache: `resolvedLinks` and `unresolvedLinks` counts per source (links, embeds, frontmatter links), backlinks per occurrence, tags merged case-insensitively from inline text and the `tags` property (list or comma string, `#` optional), aliases from `aliases` | `-cache::MetadataCache` |
| Reading view from the same tree: embeds resolve to a whole note, a `#Heading` section (to the next heading of the same or higher level) or a `#^id` block, nested four deep; footnotes numbered by first reference; a single newline is a line break unless `strictLineBreaks` | `::reading::model` |
| Reading view tables follow HTML automatic layout: a column is as wide as its widest cell, never narrower than its longest word, and the slack is shared when the note is narrower; fenced code never wraps | `::reading::view` |
| Source editor: the buffer holds `\n`; a save restores the file's BOM and `\r\n` endings; autosave two seconds after the last edit, on Ctrl+S, on navigation, on leaving source mode and on close; only the open note is written | `::editor::FileText`, `::workspace::Workspace::flush_save` |
| Smart lists: Enter continues quote markers, bullets, numbering (following siblings renumber) and task checkboxes, clears an empty item and outdents an empty nested one; Tab and Shift+Tab indent and outdent list items with a tab or `tabSize` spaces per `useTab` | `::lists` |
| Live Preview: markers of a construct are hidden unless the cursor or selection touches it; headings scaled by level; wikilinks show alias or target, markdown links their text; tasks render as checkboxes that toggle on click; quotes as a bar per level; callouts fold the type into the title; comments and block ids hidden; a click on a rendered link follows it, Ctrl+click in Source mode too | `::preview`, `::editor` |
| Find and replace: `editor:open-search` (Mod+F) and `editor:open-search-replace` (Mod+H) open a panel in the editor; literal, case-insensitive matches, all highlighted; next and previous wrap; replace steps on; replace all is one undo step | `::find`, `::editor` |
| View modes: `defaultViewMode` picks reading view (`preview`) or editing for a note at startup, `livePreview` picks Live Preview or Source for editing; `markdown:toggle-preview` toggles reading and editing, `editor:toggle-source` toggles Live Preview and Source | `::workspace::Workspace::default_mode` |
| Files view: folders before files, names compared case-insensitively, `.md` hidden, unknown kinds hidden unless `showUnsupportedFiles` | `::explorer::FileTree` |
| Search grammar: implicit AND, uppercase `OR` binds looser, `-` negation, `"phrase"` with `\"`, `/regex/flags`, field operators apply to one term or group, `[prop]` forms; explain tree uses Obsidian's labels; `tag:` matches exactly or a parent of a nested tag | `-search::query`, `-search::matcher` |
| Fuzzy search: query characters in order, case-insensitive, word starts and consecutive runs score higher; simple search needs every token as a substring | `-search::fuzzy` |
| Filesystem changes: dot-folders ignored, a removed folder removes its files, a moved-in folder is walked; adding or removing a file re-resolves every link | `-vault::Vault::refresh_path`, `-cache::MetadataCache::reresolve` |
| Unlinked mentions: whole-word, case-insensitive, names and aliases, longest name wins, not inside `[[ ]]`, `[ ]( )`, inline or fenced code | `::mentions` |
| `graph.json`: filters (`search`, `showTags`, `showAttachments`, `hideUnresolved`, `showOrphans`), `colorGroups` with `{query, color: {a, rgb}}`, display and force keys, `scale` and `close`; `collapse-*` keys round-trip | `-formats::config::GraphConfig` |
| Config files are sparse and optional; unknown keys round-trip | `-formats::config`, `-vault::Vault::read_config` |
| `core-plugins.json` map and legacy array shapes | `-formats::config::CorePlugins` |
| JSON Canvas 1.0 fields, z-order by array order, tab-indented output | `-formats::canvas` |
| Bases top-level keys and recursive filters | `-formats::base` |
| Command ids, palette names and default hotkeys as Obsidian 1.14.4 registers them; `hotkeys.json` entries replace the defaults and an empty list unbinds; labels read `Ctrl + Shift + F` | `-commands` |
| Command palette: fuzzy over names, pinned ids from `command-palette.json` first, then this session's recent commands, then registration order; commands that cannot run are not listed | `::palette` |
| Settings tabs, rows and defaults as Obsidian 1.14.4 shows them; `switcher.json`, `command-palette.json` and plugin manifests round-trip | `::settings`, `-formats::config` |

## Parser notes

The parser follows the CommonMark two-phase design: a line-by-line block pass
with a stack of open containers (lazy continuation, tab stops, list item
indentation), then an inline pass per paragraph, heading, callout title, and
table cell using the delimiter stack for emphasis and the bracket stack for
links. Obsidian's constructs are extra block starts and inline scanners in the
same two passes, each behind a flag in `Options` so the CommonMark suite runs
with them off.

Decisions taken from observation that the spec text does not spell out:

- A table's header row must be a one-line paragraph followed by the delimiter
  row, and the header or delimiter row must contain a pipe. A table cannot
  interrupt a longer paragraph, and a row line without a pipe ends the table.
- A paragraph that is only `^id` attaches the id to the previous block at the
  same level, which is how Obsidian ids tables and code blocks. A ` ^id` at
  the end of a callout header ids the callout.
- An unterminated `$$` or `%%` block runs to the end of the file. `$$x$$` or
  `%%x%%` closed on one line stays inline inside a paragraph.
- Footnote references to undefined labels render as the bare label; inline
  footnotes `^[text]` are numbered in order of appearance after referenced
  definitions.
- Markdown links count as vault links in the cache when the destination has
  no URL scheme; the destination is percent-decoded.

## Observed Obsidian behavior

Every item below was seen in Obsidian 1.13.7's output for the corpus under
`crates/-markdown/tests/corpus` and is reproduced by the parser, most
behind `Options::quirks` so the CommonMark suite still runs clean.

Blocks:

- A single newline inside a paragraph renders as `<br>` (strict line breaks off).
- A setext underline only applies to a one-line paragraph; after a longer
  paragraph `---` is a thematic break.
- `- [ ]` with nothing after the bracket is plain text, not a task.
- A list marker indented four or more columns inside a list is a sibling
  item; CommonMark would make it a lazy continuation line.
- A `>` line directly after a list item's text stays inside that item.
- The callout type is trimmed, so `[! bad]` is a `bad` callout; `[!tip]Title`
  without a space is an ordinary quote.
- Frontmatter renders as a hidden `pre.frontmatter`; `<script>` is removed;
  unknown elements are unwrapped; raw `<a>` gains `target` and `rel`.

Inlines:

- `#` starts a tag after whitespace or right after another inline construct
  (`[[link]]#tag`, `` `code`#tag ``, `**b**#tag`, `#a#b`), not after a letter
  or `(`. Tags may contain emoji and any non-ASCII letters.
- `~~~x~~~` and `===x===` delimit with their first two characters; the third
  stays inside. `***x***` nests strong outside em.
- A single `_` cannot close before a letter or digit, so `snake_case_name`
  stays plain while `a_b_ ` emphasizes `b`.
- A backtick run without a closer retries one backtick shorter.
- Legacy entities decode without a semicolon (`&copy` is ©).
- One-letter URL schemes autolink (`<a://b>`); bare `https://`, `http://`,
  `www.` and email addresses link wherever they appear, even mid-word; a
  trailing `"` stays in the URL while `?!.,:*_~` and unbalanced `)` do not.
- `[[Note\|alias]]` is the alias form: the target is the text before the first
  `|` with a trailing backslash dropped. Inside code spans `\|` stays literal.
- An inline `%%comment%%` does not cross a line break.
- `$...$` closes at a `$` preceded by a non-space and not followed by a digit.
- `![alt](path)` without a scheme is an internal embed whose `alt` is the raw
  alt text up to a `|`; `![alt][ref]` is always an `<img>`. For `![[x|a|200]]`
  the alt is the first alias part that is not a size hint, else the display
  text (`Note > Heading`).
- `[[Note#H]]` displays as `Note > H`; an alias adds `aria-label`.

Not reproduced (two corpus notes): nested runs such as `****a****` and
`==== four ==x===y==`, and a wikilink inside markdown link text, which splits
the link in Obsidian.
