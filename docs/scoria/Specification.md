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
| Frontmatter only on line 1, closed by `---` alone (YAML's `...` does not close it), CRLF tolerated; a BOM before the fence is tolerated, which Obsidian 1.14.4 does not do | `-markdown::extract_frontmatter` |
| Wikilink body grammar `path#heading#heading\|alias`, `#^block` | `-markdown::parse_linktext` |
| Wikilinks and embeds are not parsed inside code spans; `[[` needs `]]` on the same line | `-markdown/tests/obsidian.rs` |
| Tags need a non-numeric character; `# Heading` is not a tag; `#` after an alphanumeric is not a tag | `-markdown/tests/obsidian.rs` |
| Block ids: ` ^id` at the end of a paragraph or list item, or `^id` alone on a line after a table, list, or code block | `-markdown/tests/obsidian.rs` |
| Callouts: `> [!type]-` or `+` on the first quoted line; title is the rest of the line; built-in types and aliases | `-markdown::canonical_callout_type` |
| `==highlight==`, `~~strike~~` need exactly two delimiters; `$x$` needs no space inside and no digit after; `%%` comments hide inline and block text | `-markdown/tests/obsidian.rs` |
| Reading-view hooks: `internal-link` with `data-href`, `tag`, `internal-embed` with `src`/`alt`, `callout` with `data-callout`/`data-callout-fold`, `task-list-item` with `data-task`, `data-heading` | `-markdown::render_html` |
| Cache record: links, embeds, tags, headings, sections with Obsidian's type names, list items with Obsidian's `parent` encoding, blocks, footnotes, frontmatter links with key paths | `-cache::index_markdown` |
| Link resolution: extension handling; `/path` from the root only, `./` and `../` relative to the source only, other paths root then relative; an ambiguous basename picks a root file, then the source's folder, then the shortest path, then path order; case-insensitive | `-cache::LinkResolver` |
| Vault-wide cache: `resolvedLinks` and `unresolvedLinks` counts per source (links, embeds, frontmatter links), backlinks per occurrence, tags merged case-insensitively from inline text and the `tags` property (list or comma string, `#` optional), aliases from `aliases` | `-cache::MetadataCache` |
| Reading view from the same tree: embeds resolve to a whole note, a `#Heading` section (to the next heading of the same or higher level) or a `#^id` block, nested four deep; footnotes numbered by first reference; a single newline is a line break unless `strictLineBreaks` | `::reading::model` |
| Reading view tables follow HTML automatic layout: a column is as wide as its widest cell, never narrower than its longest word, and the slack is shared when the note is narrower; fenced code never wraps | `::reading::view` |
| Source editor: the buffer holds `\n`; a save restores the file's BOM and its dominant `\r\n` ending; a file whose endings disagree is written back verbatim until it is edited; autosave two seconds after the last edit, on Ctrl+S, on navigation, on leaving source mode and on close; only the open note is written | `-text::FileText`, `::workspace::Workspace::flush_save` |
| Writer round trips: opening a note and saving it changes no bytes, every editing command undoes back to the file's bytes, typing changes only the bytes at the cursor, a property write stays inside its block, and a config file read and written back is unchanged | `-text/tests/writer_roundtrip.rs` (ratchet) |
| Smart lists: Enter continues quote markers, bullets, numbering (following siblings renumber) and task checkboxes, clears an empty item and outdents an empty nested one; Tab and Shift+Tab indent and outdent list items with a tab or `tabSize` spaces per `useTab`, and the ordered items they unsettle renumber: an item follows its previous sibling at its depth, one with none restarts at 1 | `::lists` |
| Live Preview: markers of a construct are hidden unless the cursor or selection touches it; headings scaled by level; wikilinks show alias or target, markdown links their text; tasks render as checkboxes that toggle on click; quotes as a bar per level; callouts fold the type into the title; comments and block ids hidden; inline math loses its dollars; a click on a rendered link follows it, Ctrl+click in Source mode too | `::preview`, `::editor` |
| Live Preview widgets: a top-level table, math block, or paragraph that is one embed (note, section, block or image) renders in place of its lines through the reading renderer while no selection touches them; the cursor entering the block (arrows, or a click on a table or math widget, at the clicked row) shows its source; links inside an embed widget work | `::reading::model::build_widgets`, `::editor` |
| Find and replace: `editor:open-search` (Mod+F) and `editor:open-search-replace` (Mod+H) open a panel in the editor; literal and case-insensitive by default, with match case and regex toggles (`$&`, `$1`, `$$` in a regex replacement); all matches highlighted; next and previous wrap; replace steps on; replace all is one undo step; Find all (Alt+Enter) selects every match and closes the panel | `::find`, `::editor` |
| Multiple cursors: Find all, Alt+click (again on a cursor removes it), `editor:add-cursor-above` and `below`; typing, deleting, Enter, Tab, cut, copy (joined with newlines) and paste (one line per cursor when the counts match) apply at every cursor as one undo step; movement moves them all, overlapping selections merge; Escape or a plain click keeps only the primary | `::editor` |
| View modes: `defaultViewMode` picks reading view (`preview`) or editing for a note at startup, `livePreview` picks Live Preview or Source for editing; `markdown:toggle-preview` toggles reading and editing, `editor:toggle-source` toggles Live Preview and Source | `::workspace::Workspace::default_mode` |
| Files view: folders before files, names compared case-insensitively, `.md` hidden, unknown kinds hidden unless `showUnsupportedFiles` | `::explorer::FileTree` |
| Search grammar: implicit AND, uppercase `OR` binds looser, `-` negation, `"phrase"` with `\"`, `/regex/flags`, field operators apply to one term or group, `[prop]` forms; explain tree uses Obsidian's labels; `tag:` matches exactly or a parent of a nested tag | `-search::query`, `-search::matcher` |
| Fuzzy search: query characters in order, case-insensitive, word starts and consecutive runs score higher; simple search needs every token as a substring | `-search::fuzzy` |
| Filesystem changes: dot-folders ignored, a removed folder removes its files, a moved-in folder is walked; adding or removing a file re-resolves every link | `-vault::Vault::refresh_path`, `-cache::MetadataCache::reresolve` |
| File operations: new notes are `Untitled`, `Untitled 1`, ... in the `newFileLocation` folder (`root`, `current`, or `newFileFolderPath` when it exists); a `folder/Name` link creates its folders; creating over a path fails ("File already exists."), renaming onto one fails ("Destination file already exists!") but a case-only rename succeeds; `trashOption` `system` (freedesktop trash), `local` (flat in `.trash/`, folders whole, collisions ` 2`, ` 3`) or `none`; defaults `system`, `promptDelete` on, `alwaysUpdateLinks` on | `-vault::ops` |
| Link rewriting on rename or move: every link resolving to the file is regenerated in `newLinkFormat` (shortest falls back to the full path when ambiguous), keeping its style (wikilink, embed, markdown, angle brackets), `.md` and `%20` in markdown destinations, the subpath, and the alias verbatim unless it equals the old basename and not the written path; markdown link text follows the new basename or link path when it was the old one; code, reference links and `[[#Heading]]` untouched; a moved note's own links change only when they would break; frontmatter links replaced in place | `-cache::links` |
| Properties: the block parses into ordered entries with byte spans; a type comes from `types.json`, then the reserved keys `tags` and `aliases`, then the value (string `text`, bare `YYYY-MM-DD` `date`, number `number`, boolean `checkbox`, sequence `multitext`; `cssclasses` is a plain list and an ISO date-time is `text`) | `-formats::properties` |
| Property serialization as Obsidian 1.14.4 writes it: a scalar plain where YAML allows it else double quoted (`"a: b"`, `"  padded  "`, `"42"`, `"[[A]]"`, `""`, `"#a"`), a date bare, a newline as a `\|-` block scalar, a list as a two-space block sequence (the empty list `[]`), null as a bare key, a new property after the last entry, and removing the last property removes the fences | `-formats::properties::render_entry`, `plan` |
| Property edits are surgical: only the entries whose value actually changes are rewritten, so comments, flow sequences, quoting, blank lines and the file's line endings survive — Obsidian re-serializes the whole block and loses them | `-formats::properties::plan`, `-cache/tests/properties_corpus.rs` (ratchet) |
| Properties UI: a foldable block above the note with a type glyph, key and value per row, links and tags inside a value clickable, lists and tags as pills, a value that does not fit its type marked and kept as written; `propertiesInDocument` shows the table (`visible`, the default), nothing (`hidden`) or the raw YAML (`source`) | `::properties`, `::reading::view` |
| Property editing in Live Preview: a click opens a field whose keys the editor routes (Enter commits, Tab commits a key and moves to its value, Escape abandons), a commit is one undo step, a key typed empty removes the property, a rename keeps the entry in place (Obsidian moves it to the end), a new row writes nothing until it is named, and a type chosen from the row menu is recorded in `types.json` | `::properties`, `::editor` |
| Unlinked mentions: whole-word, case-insensitive, names and aliases, longest name wins, not inside `[[ ]]`, `[ ]( )`, inline or fenced code | `::mentions` |
| `graph.json`: filters (`search`, `showTags`, `showAttachments`, `hideUnresolved`, `showOrphans`), `colorGroups` with `{query, color: {a, rgb}}`, display and force keys, `scale` and `close`; `collapse-*` keys round-trip | `-formats::config::GraphConfig` |
| Config files are sparse and optional; unknown keys round-trip | `-formats::config`, `-vault::Vault::read_config` |
| Writing a config file changes only what the value changed: the new value is merged into the file's own JSON, so key order, number spelling (`1` not `1.0`, and more digits than an `f32` field holds), and keys the type does not serialize all survive | `-vault::Vault::write_config` |
| `core-plugins.json` map and legacy array shapes | `-formats::config::CorePlugins` |
| JSON Canvas 1.0 fields, z-order by array order, tab-indented output | `-formats::canvas` |
| Bases top-level keys and recursive filters | `-formats::base` |
| Command ids, palette names and default hotkeys as Obsidian 1.14.4 registers them; `hotkeys.json` entries replace the defaults and an empty list unbinds; labels read `Ctrl + Shift + F` | `-commands` |
| Command palette: fuzzy over names, pinned ids from `command-palette.json` first, then this session's recent commands, then registration order; commands that cannot run are not listed | `::palette` |
| Settings tabs, rows and defaults as Obsidian 1.14.4 shows them; `switcher.json`, `command-palette.json` and plugin manifests round-trip | `::settings`, `-formats::config` |
| Settings toggles write: a click sets the key Obsidian writes in `app.json`, `appearance.json`, `switcher.json`, `core-plugins.json` or `community-plugins.json`, changing nothing else in the file, and the running app follows what it can (readable line length, smart lists, indentation, properties in document, strict line breaks, show all file types); a vault with no `core-plugins.json` gets the whole default map rather than a one-key file | `::settings`, `::workspace::Workspace::change_setting` |
| Each settings row says how far it gets in : untagged means the app follows it at once, "on restart" the theme and fonts, "not in  yet" a value written only for Obsidian to read. A  divergence: Obsidian has no such tags, and shows every setting as live | `::settings::Honored` |

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

Line endings and the byte order mark, from the corpus and a 1700-note real
vault rather than from Obsidian's source:

- A file's endings are not always uniform. One note in the wild carried a
  single `\r\n` among 4430 `\n` lines, so "the file used CRLF" has to mean
  the dominant ending, not the presence of one.
-  therefore writes the dominant ending on every line once a file is
  edited, and writes a mixed file back verbatim while nothing has changed.
  **Unprobed:** what Obsidian itself writes for such a file. Worth a probe
  note in the scratch vault before the writer milestone closes.

Not reproduced (two corpus notes): nested runs such as `****a****` and
`==== four ==x===y==`, and a wikilink inside markdown link text, which splits
the link in Obsidian.

## Observed properties behavior, Obsidian 1.14.4

Observed with a throwaway plugin in a scratch vault (see
[[Development#Properties, 2026-10-18]]), driving
`app.fileManager.processFrontMatter`, `app.metadataTypeManager` and the three
property commands, then reading the files back byte for byte.

The serializer:

- A `processFrontMatter` call that mutates **nothing** still rewrites the block:
  `# a comment` is deleted, `tags: [one, two]` becomes a two-space block
  sequence, and a blank line inside the block is dropped. A block already in
  that canonical form round-trips byte-identical, so the canonical form is
  what  writes for a value it changes — and only for that value.
- Scalars are plain where YAML allows it and double quoted otherwise:
  `k: "a: b"`, `k: "a #b"`, `k: "  padded  "`, `k: ""`, `k: "42"`,
  `k: "[[Target]]"`, `k: "#a"`. `say "hi"`, `Ä ö — ✓`, `https://example.com`
  and a 200-character run all stay plain and unwrapped. A date is written
  bare (`d: 2026-10-18`).
- A value containing a newline becomes `k: |-` with the lines indented two
  spaces. A nested mapping is written with two-space indentation.
- Lists are block sequences (`k:\n  - a\n  - b`), one `- ` per item, each item
  quoted by the scalar rules. The empty list is the one flow form, `k: []`.
- Null is the bare key (`k:`); a JavaScript `undefined` writes no key at all.
- A new property goes after the last entry. Deleting the last property deletes
  the fences too: `---\na: 1\n---\nbody\n` becomes `body\n`.
- Renaming through `processFrontMatter` (delete then set) moves the key to the
  end of the block.  renames in place instead, changing fewer bytes.
- Duplicate keys make the whole block fail to parse ("Map keys must be
  unique"), so such a note has no properties in Obsidian's cache at all.

Types, from `registeredTypeWidgets`:

- The registry is `text`, `multitext` (shown as "List"), `number`,
  `checkbox`, `date`, `datetime`, `tags`, `aliases`, plus `file`, `folder`
  and `property` for Bases views. There is **no** `cssclasses` type.
- `tags` and `aliases` are `reservedKeys` of their widgets, so those keys keep
  their type whatever the value is.
- Inference from the value: a string is `text` except a bare `YYYY-MM-DD`,
  which is `date`; a YAML number is `number`, a boolean `checkbox`, a sequence
  `multitext`; null, a nested mapping and a block scalar are all `text`.
  An ISO *date-time* string (`2026-10-18T09:30:00`) infers as `text`, not
  `datetime` — `datetime` is only ever assigned.
- `types.json` is written in insertion order, seeded with `aliases: aliases`,
  `cssclasses: multitext` and `tags: tags` before any assigned key, with
  two-space indentation and no trailing newline.

Commands and settings:

- `markdown:add-metadata-property` ("Add file property") writes nothing to the
  file; it opens an empty row in the UI, and the property appears only once it
  is named.
- `markdown:clear-metadata-properties` ("Clear file properties") removes the
  whole block, leaving the body alone.
- `propertiesInDocument` defaults to `visible` with the key absent from
  `app.json` (confirmed through `vault.getConfig`).
- The properties block is the same editable widget in reading view as in Live
  Preview; the heading reads "Properties" and folds.

Where  deliberately differs, because Obsidian loses bytes:

- **CRLF.** Writing a property into a CRLF block gives mixed endings
  (`---\r\ntitle: Hi\nadded: v\n---\r\n`).  writes the file's own
  ending throughout.
- **Byte order mark.** Obsidian does not recognise frontmatter after a BOM: it
  prepends a *second* block, leaving
  `---\nadded: v\n---\n\u{feff}---\ntitle: Hi\n---\n`.  reads the block
  through the BOM and edits it in place.
- **`...`** does not close a properties block in Obsidian, and no longer does
  in  either; adding a property to `---\na: 1\n...\n` prepends a second
  block there too.

## Observed autocompletion behavior, Obsidian 1.14.4

Observed with a throwaway plugin in a scratch vault (see
[[Development#Autocompletion, 2026-10-18]]), driving the four
`app.workspace.editorSuggest.suggests` instances directly: `onTrigger` for
every shape of line, `getSuggestions` for what each offers, and the popup's
own `scope` key handlers for what accepting one writes.

The popup binds `ArrowUp`, `ArrowDown`, `PageUp`, `PageDown`, `Home`, `End`,
`Ctrl+P`, `Ctrl+N`, `Enter` and `Escape`; the link suggester also binds
`Tab`, `Shift+Enter`, `#` and `^`. All four have a limit of 100 items.

What triggers, scanning back from the cursor on the cursor's line only:

- **Links.** The last `[[` before the cursor with no `]` after it. The query is
  everything from there to the cursor and keeps its spaces, `#`, `^`, `|` and
  `/`: `[[Alpha#first` is one query, `Alpha#first`, which the suggester
  splits itself. `[[[al` takes the last `[[`. A cursor inside `[[al]]` still
  triggers; one after the `]]` does not. `![[` is the same suggester, the
  query starting after the third bracket.
- **Tags.** A `#` at the start of the line or after whitespace, through the
  cursor; `word#inl` does not trigger. The query excludes the `#` but the
  range to replace includes it.
- Nothing suppresses either one: both fire inside a code span, inside a fenced
  code block, inside `%%comments%%` and inside frontmatter. The link
  suggester wins when both would fire, being first in the list.
- `[text](` offers nothing. There is no markdown-link path completion.

What each offers:

- An **empty** `[[` query lists every file in the vault plus every alias,
  ordered by the file's modification time, newest first — not by recency of
  opening, which is what the quick switcher uses. A non-empty query is fuzzy
  matched and an alias scores just below its file.
- The list is `metadataCache.getLinkSuggestions()`: files and aliases, and
  **no unresolved link targets**, which is the other way the link suggester
  differs from the quick switcher. A query nothing matches offers one item
  with no file, which links the text as typed.
- `[[Note#` lists that note's headings in document order, the H1 included,
  unfiltered while the query after the `#` is empty. `[[#` lists the
  *current* note's headings.
- `[[Note#^` lists every block of the note — headings as `## First Heading`
  with their markers, paragraphs as their text with any `^id` stripped.
- `[[Note|` lists that note's aliases; with text after the `|` it offers that
  text as the display text.
- `#` lists every tag in the vault alphabetically, parent segments
  (`project` for `project/alpha`) as their own items, without the `#`.

What accepting one writes, with `newLinkFormat: shortest` and wikilinks:

| Typed | Accepted | Written |
| --- | --- | --- |
| `[[beta no` | the file | `[[Beta Note]]` |
| `[[al` | `Sub/Alpha` | `[[Sub/Alpha\|Alpha]]` |
| `[[alpha ali` | the alias | `[[Alpha\|Alpha Alias]]` |
| `[[Alpha\|disp` | the typed text | `[[Alpha\|disp]]` |
| `[[Alpha#first` | the heading | `[[Alpha#First Heading]]` |
| `[[Alpha#^` | a block with no id | `[[Alpha#^07fe38]]` |
| `[[pi` | `pic.png` | `[[pic.png]]` |
| `![[pi` | `pic.png` | `![[pic.png]]` |
| `[[No Such Note` | the typed text | `[[No Such Note]]`, and no file is created |
| `#inl` | the tag | `#inline-tag ` |

- The display text after `|` is added exactly when the path part contains a
  `/`, and only for markdown targets: `[[Sub/Alpha|Alpha]]` and
  `[[../Beta Note|Beta Note]]`, but `[[../pic.png]]` with none.
- The path part is `fileToLinktext` in the vault's `newLinkFormat`, so the
  same note is `[[Gamma]]` from inside `Sub`, `[[Sub/Gamma|Gamma]]` from the
  root under `absolute`, and `[[../Beta Note|Beta Note]]` under `relative`.
- A **heading** pick is the exception twice over: the path is left exactly as
  it was typed rather than reformatted, and no display text is ever added.
  `[[Sub/Gamma#` becomes `[[Sub/Gamma#Gamma Heading]]` even though
  `shortest` would have written the path as `Gamma`.
- A closing `]]` already after the cursor is consumed rather than doubled, so
  accepting inside `[[be|]]` leaves `[[Beta Note]]` and not four brackets.
- Accepting a **block** with no id of its own writes the id into the target
  note. The id is six lowercase hex characters (twelve of twelve samples),
  appended to a paragraph inline (`para one ^3a3815`) and written on a line
  of its own after a heading (`## First Heading\n\n^07fe38\n`). An id
  already there is reused and the target is not touched.

With `useMarkdownLinks` the same picks write markdown links: the display text
is the basename, the alias, or the heading text, the destination is the link
path with `.md` kept for notes, and only spaces are percent-encoded —
`[Beta Note](Beta%20Note.md)`, `[Alpha Alias](Alpha.md)`,
`[First Heading](Alpha.md#First%20Heading)`, `[pic](pic.png)`,
`![Beta Note](Beta%20Note.md)`. A pick with no file behind it writes an
**empty** display text: `[](No%20Such%20Note%20At%20All)`, and a block
reference writes `[](Alpha.md#^07fe38)`; the cursor then lands between the
`[` and the `]`, whichever key accepted it, since that is the part left to
fill in.

The three accept keys differ, and the difference is the point:

- **Enter** writes the whole link and leaves the cursor after the `]]`; on a
  tag it writes a trailing space (`#inline-tag `) and sits after it.
- **Tab** writes the same link but leaves the cursor *inside*, before the
  `]]`, so a `#heading` or `|alias` can follow; on a tag it writes no
  trailing space. With `useMarkdownLinks` there is nowhere to stand, and Tab
  lands where Enter does.
- **Shift+Enter** ignores the selected item and links the text as typed:
  `[[beta no` becomes `[[beta no]]`. The tag suggester does not bind it.

**Unprobed:** whether the `#` and `^` keys the link popup binds do anything
beyond typing the character, and whether typing `[[` auto-pairs the closing
brackets (`autoPairBrackets`, which  does not implement either way).

## Observed table editing behavior, Obsidian 1.14.4

Observed with a throwaway plugin in a scratch vault (see
[[Development#Table editing, 2026-10-18]]), running each of the sixteen
table commands against eight shapes of table with the cursor in two
different cells, then reading the note back. The commands' own
`editorCheckCallback` only passes in Live Preview with the editor focused —
in raw source mode it never does, so **Obsidian's table commands are a Live
Preview feature**; the 186 runs where the check passed are what is recorded
here.

Any table command **rewrites the whole table**, not the row or column it
was asked about. The written form is the same every time:

- Every row gets outer pipes, even when the table was written without them,
  and one space either side of each cell's text.
- A column is as wide as its widest cell, and never narrower than three,
  which is what the delimiter needs. Cells are padded to that width.
- The delimiter fills the column width: `---`, and with an alignment
  `:--`, `:-:` and `--:`, growing to `:-----`, `:----:` and `-----:`.
- Padding follows the column's alignment: left and none pad on the right,
  right pads on the left, and centre splits the difference with the odd
  space on the right (`B` in a six-wide centred column is `  B   `).
- A row with too few cells is filled out with empty ones.
- Cell text is kept as written, escapes included: `x \| y` stays one cell.
  A `|` inside a code span is **not** protected — `` `a|b` `` is two cells,
  `` `a `` and `` b` `` — which is what the parser already does.

What each command does, with the cursor in some cell:

| Command | Effect |
| --- | --- |
| `table-row-before`, `table-row-after` | an empty row above or below |
| `table-row-up`, `table-row-down` | swap with the row the other side; the first body row swaps with the **header**, delimiter staying put |
| `table-row-copy` | the row again, directly below |
| `table-row-delete` | the row; deleting the header promotes the next row into it |
| `table-col-before`, `table-col-after` | an empty column either side, alignment `None` |
| `table-col-left`, `table-col-right` | swap with the column the other side, alignment travelling with it |
| `table-col-copy` | the column again, directly to the right, alignment included |
| `table-col-delete` | the column and its alignment |
| `table-col-align-left/center/right` | the column's alignment, and every cell in it re-padded |

`editor:insert-table` is the one command that is not about an existing
table: on an empty line it writes a blank line and then a two-by-two table
of empty three-wide cells, leaving the cursor in the first header cell.

```
|     |     |
| --- | --- |
|     |     |
```

Editing a cell is the opposite: **ordinary typing inside a table re-pads
nothing**. A change to one cell's text leaves every other byte of the table
as it was, and it stays that way when the cursor moves to another cell and
when it leaves the table altogether — `| 1 | 2 |` edited to `| WIDE | 2 |`
stays ragged until a table command runs. So the padded form is what the
commands write, not an invariant Obsidian maintains.

The widget itself is a real `table.table-editor` inside a `.table-wrapper`,
each cell holding a `.table-cell-wrapper`; the cells are not
`contenteditable` themselves, so the table stays drawn as a grid while the
cursor is inside it rather than unfolding to its source the way other
widgets do.

**Unprobed:** where Obsidian leaves the cursor after each command (the
columns it reported came from an unfocused editor and disagreed with
themselves), and whether a column's width counts characters or display
columns for CJK and emoji.

## Observed block reference behavior, Obsidian 1.14.4

Observed with a throwaway plugin in a scratch vault (see
[[Development#Phase 3 probe, 2026-10-18]]), driving the link suggester's
`onTrigger`/`getSuggestions` and the popup's own Enter handler over a note
holding one of every block kind, then reading the target note's bytes back
as an escaped string and a hex dump.

`[[Note#^` offers one row per block, in document order. The kinds, with the
label each row reads for the probe note:

| Node | Row reads |
| --- | --- |
| heading | `# Probe Heading` — the markers are kept |
| paragraph | `A plain paragraph with no id.` |
| table | `A B \n1 2` — cells joined by spaces, rows by a newline |
| code (fenced) | `let fenced = 1;` — the fence and its language are dropped |
| code (indented) | `indented code line` |
| callout | `A callout titlecallout body line` — title and body run together |
| blockquote | `a blockquote\nsecond quote line` |
| math | `x = y + 1` — the `$$` fences are dropped |
| listItem | one row per item, nested items getting their own row as well |

A thematic break is **not** offered, and neither is the frontmatter block.
A list item inside a blockquote is offered as the *blockquote*, and one
inside a callout as the *callout*: the quote is the block, not the item.
`[[#^` lists the blocks of the note being edited. A path that resolves to
nothing (`[[NoSuchNote#^`) and one that resolves to an attachment
(`[[pic.png#^`) each offer a single row of type `none` rather than an empty
list.

Accepting a row whose block has no id of its own writes one into the target
note. The id is six lowercase hex characters — 180 generated in a row were
180 distinct values — and where it goes depends on the kind:

- **A paragraph** takes ` ^id` appended to its last line, *verbatim*, after
  whatever is already there. A line ending in a hard break keeps it and the
  id goes after: `Ends in two spaces.  ` becomes
  `Ends in two spaces.   ^id`, and `Ends in a backslash\` becomes
  `Ends in a backslash\ ^id`.
- **A heading** takes the id on a line of its own, with a blank line either
  side, and the bytes after it are normalized: the heading keeps its text,
  then `\n\n^id\n`, then what followed with its leading run of newlines
  collapsed to one. Every shape agrees:

| Before | After |
| --- | --- |
| `## H` (no trailing newline) | `## H\n\n^id\n` |
| `## H\n` | `## H\n\n^id\n\n` |
| `## H\n\nbody\n` | `## H\n\n^id\n\nbody\n` |
| `## H\nbody\n` | `## H\n\n^id\n\nbody\n` |
| `## H\n### H2\nbody\n` | `## H\n\n^id\n\n### H2\nbody\n` |

  The last two are the ones worth noticing: Obsidian inserts a blank line
  that was not there, so the id is a paragraph of its own rather than a
  second line of the following block.

A block that already carries an id is linked to by that id and the target
note is not touched at all.

Where  deliberately differs, because Obsidian loses bytes:

- **CRLF.** Writing an id into a CRLF note gives the inserted lines LF
  endings and leaves the rest CRLF: `# Crlf\r\n\r\npara` becomes
  `# Crlf\n\n^id\n\r\n\r\npara`, a mixed file.  writes the file's own
  dominant ending throughout, as it already does for properties.

**Unprobed:** the byte-for-byte placement for a table, a fenced or indented
code block, a callout, a blockquote, a display-math block and a list item —
the step that would have shown them typed into the very note it was reading
and spoiled its own fixture.  offers those rows only when the block
already has an id until a second run settles them.
