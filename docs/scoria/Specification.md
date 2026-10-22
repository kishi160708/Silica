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
| Auto-pairing, as the rows under "Observed auto-pair behavior" record it: `(`, `[` and `{` close unless a word character follows; `"`, `'`, `` ` ``, `*` and `_` close only after a space or the start of the text at end of line; `<`, `~`, `=`, `$` and `%` never close; a second quote or marker skips while a second bracket nests; a typed closer skips only over one the editor planted; Backspace between an empty pair takes both; every pair character but `<` wraps a selection, and the opener and its closer are one undo step | `-text::autopair` |
| Live Preview: markers of a construct are hidden unless the cursor or selection touches it; headings scaled by level, a closed ATX heading's trailing `#` run hidden with the opening one and a setext underline row hidden and laid out at body size; wikilinks show their alias, or their target with each subpath separator rendered as ` > ` (`Note > Heading`, `Note > ^blockid`, a leading `#` dropped), markdown links their text; tasks render as checkboxes that toggle on click; quotes as a bar per level; callouts fold the type into the title; comments and block ids hidden; inline math loses its dollars; a click on a rendered link follows it, Ctrl+click in Source mode too | `::preview`, `::editor` |
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
| JSON Canvas 1.0 fields, z-order by array order, unknown keys at every level; written as Obsidian 1.14.4 writes an edited save: the file's own key order kept with the model's keys appended in its fixed order, groups first, default edge ends dropped, one object per line unless it nests one, tabs, no trailing newline, see [[#Observed canvas write behavior, Obsidian 1.14.4 (installer build 1.13.7)]] | `-formats::canvas`, `-formats/tests/canvas_obsidian.rs` (ratchet), `-text/tests/writer_roundtrip.rs` |
| Canvas view: the dot grid, cards, groups, edges, labels and controls drawn as Obsidian 1.14.4 draws them; pan, zoom, selection, moving, resizing, drawing edges, typing into a card, the menus and the keyboard, saving through the canvas writer; the leaf's `viewState`, see [[#Observed canvas view behavior, Obsidian 1.14.4 (installer build 1.13.7)]] | `::canvas`, `-canvas` |
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

What Obsidian itself does was the open question here, and it is now
observed (see [[Development#Line endings and the byte order mark, 2026-10-18]]).
**Its editor normalizes on save**: every ending becomes `\n` and a UTF-8
byte order mark is dropped.

| Before | After one character is typed |
| --- | --- |
| `# Mixed\nline one\nline two\r\nline three\n` | every line `\n` |
| `# Mixed\r\nline one\r\nline two\nline three\r\n` | every line `\n` |
| `a\r\nb\nc\r\nd\n` | `aX\nb\nc\nd\n` |
| `# All\r\nline one\r\nline two\r\n` — uniform CRLF | every line `\n` |
| `\u{feff}# Bom\nline one\n` | `# Bom\nlineX one\n` — the mark is gone |

A note that is only **opened** is not rewritten: the same mixed file read
back after being opened and left alone is byte-identical. The normalization
belongs to the save, not the load.

This is the one place  deliberately keeps *more* of the file than
Obsidian does.  writes the file's own dominant ending and puts a byte
order mark back, so it never introduces a difference; Obsidian's editor
converts the file the first time it is edited there. A vault used in both
therefore drifts towards LF with no mark, driven by Obsidian, and 
never drives it the other way.

Note that this is the *editor's* save. The frontmatter API is not the same
path and does not normalize: writing a property into a CRLF note leaves
`---\r\ntitle: Hi\nadded: v\n---\r\n`, recorded under the properties
section above.

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
brackets.  does auto-pair `[` per the rows below, so `[[` there gives
`[[]]` with the suggester opening inside; what Obsidian does is still unread.

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
[[Development#The probe batch, 2026-10-18]]), driving the link suggester's
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

Where the id goes depends on the kind, and there are only two shapes. A
block whose text ends a line takes the id **inline**, and a block that
spans lines takes it on a **line of its own** with a blank line either side:

| Kind | Written |
| --- | --- |
| paragraph | `last line ^id` |
| list item | `- alpha item ^id`, and on the parent's own line when it has a nested child |
| heading | `## H\n\n^id\n` |
| table | `\| 1 \| 2 \|\n\n^id\n` |
| fenced code | ` ```\n\n^id\n ` |
| indented code | `    second line\n\n^id\n` |
| callout | `> body line\n\n^id\n` |
| blockquote | `> second line\n\n^id\n` |
| display math | `$$\n\n^id\n` |

A list item inside a blockquote or a callout is not addressed as an item at
all: the row is the quote, and the id goes on its own line after the whole
quote. At the end of a file with no trailing newline, an inline id is
appended and the file still ends without one
(`last paragraph ^id`), while an own-line id brings a trailing newline with
it (`\| 1 \| 2 \|\n\n^id\n`).

A block that already carries an id is linked by it and the file is not
written at all — its bytes and its modification time are both untouched.

**Unprobed:** an ordered list item and a list item at the end of a file,
whose accept steps did not fire; the inline rule is taken from the two
unordered samples that did.

## Observed settings write behavior, Obsidian 1.14.4

Observed with a throwaway plugin in a scratch vault (see
[[Development#The probe batch, 2026-10-18]]), setting each key through
`vault.setConfig` and reading `.obsidian/app.json` and
`.obsidian/appearance.json` back as raw bytes before and after every
change.

**A setting returned to its default is written, not removed.** Every one of
the thirty-eight changes the run made left its key in the file, the
default-valued ones included: `newLinkFormat` back to `shortest`, `theme`
back to `system`, `tabSize` back to `4`, `baseFontSize` back to `16`,
`livePreview` back to `true`, `readableLineLength` back to `true` and
`accentColor` back to `""` all kept their key with the explicit value. A
key absent from the file stays absent until something sets it, and from
then on it is always written. That is what  already does, so no way
of removing a config key is needed.

The values each enum key takes, as written:

| Key | File | Values |
| --- | --- | --- |
| `defaultViewMode` | `app.json` | `preview`, `source` |
| `propertiesInDocument` | `app.json` | `visible`, `hidden`, `source` |
| `newFileLocation` | `app.json` | `root`, `current`, `folder` |
| `newLinkFormat` | `app.json` | `shortest`, `relative`, `absolute` |
| `trashOption` | `app.json` | `system`, `local`, `none` |
| `fileSortOrder` | `app.json` | `alphabetical`, `byModifiedTime` |
| `theme` | `appearance.json` | `moonstone`, `obsidian`, `system` |

`livePreview` is a boolean rather than an enum, and `tabSize` and
`baseFontSize` are written as plain integers (`2`, `4`, `8`, `14`, `16`,
`20`) with no decimal point. A text field writes its string verbatim,
including a font family with a space in it (`"Noto Sans Mono"`), and an
emptied field writes `""` rather than dropping the key.

**Unprobed:** the option *labels* each dropdown shows, and the min, max and
step of the two sliders — the run drove the keys rather than the controls,
so it settled what is written and not what the tab reads.

## Observed canvas write behavior, Obsidian 1.14.4 (installer build 1.13.7)

First seen on 2026-10-18 in the gate vault: a canvas authored outside
Obsidian (2-space pretty-printed, every field on its own line, key order
`id, x, y, width, height, type, text, color`) was opened once and came back
rewritten whole, 6215 bytes down to 4731, every node on one line in the
order `id, type, text, x, y, width, height, color`. Then pinned down on
2026-10-19 over CDP (`tools/obsidian-probe/canvas.mjs`, four runs, raw
results in `tools/obsidian-probe/results/2026-10-19-canvas.json`) with
scratch canvases carrying every node and edge type, every optional key in a
scrambled order, and unknown keys at every level. `-formats::canvas`
reproduces all of it, and `-formats/tests/canvas_obsidian.rs` holds
the files the probe wrote beside what Obsidian wrote back.

**When Obsidian writes.** Opening a canvas writes nothing. Closing it
rewrites the file even when nobody edited it, through the formatter below
but from the data as loaded: keys stay in file order and nothing is added
or dropped. An edit (a node moved, an edge drawn) saves within about two
seconds and rebuilds every object from the model, which is the form below.
`canvas:new-file` creates `Untitled.canvas` as an **empty file** (zero
bytes) and writes `{"nodes":[...],"edges":[]}` once something is in it.

**The formatter.** Tab indentation, no space after `:` or `,`, no trailing
newline. A container is written on one line unless one of its values is
`null`, an object or an array (`typeof v === 'object'`); then every value
goes on its own line one tab deeper and the closing bracket on its own line
at the container's indent. So a plain node or edge is one line, a node that
carries a plugin's nested object is spread over several, and `"edges":[]`
and `{}` stay as written. Numbers and strings are `JSON.stringify`'s: `1e3`
comes back `1000`, `-0` as `0`, `0.5` as `0.5`; `\n`, `\"`, `\\` and `\t`
are escaped, `é` and U+2028 are not.

**Key order on an edited save** — the model's `getData` writes each object
as the keys of the object it was read from, in file order, minus the keys
the model owns, and then the model's keys in a fixed order, each
overwriting in place when the file already had it and appended otherwise:

| Object | Keys the model owns (removed from the file order) | Appended, in this order |
| --- | --- | --- |
| every node | `x`, `y`, `width`, `height`, `color` | `id`, `x`, `y`, `width`, `height`, `color` (when set), `type`, then the type's own keys |
| text node | — | `text` |
| file node | — (`file` and `subpath` keep their place) | `file`, `subpath` |
| link node | — (`url` keeps its place) | `url` |
| group node | `label`, `background` as well (`backgroundStyle` keeps its place) | `label`, `background`, `backgroundStyle` |
| edge | everything but `id` and unknown keys | `id`, `fromNode`, `fromSide`, `toNode`, `toSide`, `fromEnd`, `toEnd`, `color`, `label` |
| the file | — (`nodes` and `edges` keep their place) | `nodes`, `edges` |

Consequences, each observed:

- A node Obsidian made itself has no file order, so it is written `id, x,
  y, width, height, color, type, text` (`file, subpath`; `url`; `label`).
  Read back and saved after an edit, the same node comes out `id, type,
  text, x, y, width, height, color` — Obsidian's own output is not a fixed
  point, which is exactly what the gate vault's canvas showed. The output
  for a file it has read *is* one.
- Unknown keys survive at every level (`pluginTop`, `pluginNode`,
  `pluginEdge` in the probe), in place.
- An edge's `fromEnd: none` and `toEnd: arrow` are defaults and are dropped
  even when the file spelled them out; `fromEnd: arrow` and `toEnd: none`
  are written, after `toSide` and before `color`.
- Reading a file puts the **groups first**, then the rest in file order,
  and the nodes are written in that order; a node created afterwards goes
  on the end in creation order (the probe created text, file, link, group,
  group, text and got them back in that order). A new group gets the label
  `Untitled group`.
- A file with no `edges` key gets one appended after `nodes` on its first
  edited save.

**Unprobed:** a canvas whose node geometry is fractional after a drag at an
odd zoom (the writer formats `0.5` as JavaScript would, but exponent forms
from 1e21 and below 1e-6 are not reproduced), and whether `backgroundStyle`
is written for a group that has no `background`.

## Observed canvas view behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-21 over CDP on a scratch vault
(`tools/obsidian-probe/canvas-view.mjs` for the API, command and CSS
dumps, then `canvas-view-probe-a.mjs`, `-b.mjs` and `-c.mjs` for the
measurements and the input events; raw results in
`tools/obsidian-probe/results/2026-10-21-canvas-view.json`). The fixture
held every node type, the six preset colours and a hex colour, three
groups (one coloured, one with a `cover` background) and seven edges with
labels, both arrow directions and no arrows. `-canvas` carries the
numbers as constants and `::canvas` draws from them.

**The viewport and the leaf.** The canvas is one CSS transform,
`translate(w/2 - x·s, h/2 - y·s) scale(s)` with `s = 2^zoom`: `x` and `y`
are the canvas point at the view's centre and `zoom` is the base-2
logarithm of the scale. Those three are what the leaf records,
`{"type":"leaf","state":{"type":"canvas","state":{"file":"Fixture.canvas","viewState":{"x":123.5,"y":-45.25,"zoom":-0.75}},"icon":"lucide-layout-dashboard","title":"Fixture"}}`,
written on the layout's save. The zoom runs from -4 (scale 1/16) to 1
(scale 2). A plain wheel pans (a 100-unit wheel step moved the view 93
units at scale 1), Shift+wheel pans sideways, Ctrl+wheel zooms by a third
of a doubling per 100 units around the pointer; the middle button drags
the view and so does Space with the left button. Zoom to fit is Shift+1,
zoom to selection Shift+2.

**Chrome.** Top right, 8 px in, four bordered groups of 32 px buttons
(34 px with the 1 px border, radius 4): *Canvas settings*; *Zoom in*,
*Reset zoom*, *Zoom to fit (Shift + 1)*, *Zoom out*; *Undo*, *Redo*;
*Canvas help*. Bottom centre, 16 px up, the card menu: three 48 px
buttons in a 144 by 48 box of radius 4, *Drag to add card*, *Drag to add
note from vault*, *Drag to add media from vault*. Dark theme: group
`#333333` on `#333333`, buttons `#1c1c1c` with `#b3b3b3` glyphs, the card
menu `#282828`; light: white on `#e4e4e4`, glyphs `#5c5c5c`.

**The dot grid.** An SVG pattern of `0.7` px circles in
`--canvas-dot-pattern` (`#333333` dark, `#e4e4e4` light) on
`--canvas-background` (`#1c1c1c`, `#ffffff`), anchored to canvas space.
The pattern is 20 canvas units; it is doubled while it would be smaller
than 20/√2 ≈ 14.14 px on screen (20 px at scale 1 and at 0.5, 14.14 px at
scale 0.177).

**Cards.** Border 1 px plus a 1 px inset ring of the same colour, radius
8, all in canvas units, so they scale with the zoom. A coloured card's
border is its colour at alpha 0.7 and its body is tinted with it at alpha
0.07 over the canvas background; an uncoloured card has a solid
`--canvas-color` border (`#7e7e7e` dark, `#c0c0c0` light) and no tint.
Presets, dark: 1 `#fb464c`, 2 `#e9973f`, 3 `#e0de71`, 4 `#44cf6e`, 5
`#53dfdd`, 6 `#a882ff`; light: `#e93147`, `#ec7500`, `#e0ac00`,
`#08b94e`, `#00bfbc`, `#7852ee`. A hex colour is used as is. Content is
padded 16 units. Selected (`is-focused`): the border solid and a 2 unit
ring of the colour outside it. The resize handles are invisible: 20 unit
strips along each side and 20 unit squares at the corners, which appear
on hover with the four 20 unit connection points at the side midpoints
(no visible dot). Smallest card 50 by 50. A double click on the
background makes a 250 by 60 text card centred on the pointer, unsnapped,
already in edit mode; a file, link or group card made from the menus is
400 by 400; a new group is labelled `Untitled group`.

**Labels.** Everything above a card is drawn in canvas units and
counter-scaled by 1/√scale, so it shrinks slower than the cards. A file
card's name (the page title for a link card) sits at the frame's top
left, 15 units of `--canvas-card-label-color` (`#666666`, `#ababab`),
its bottom 4 units above the frame, at most the card's width, ellipsised;
a text card has none. A group's label is a pill at the same place: 22.5
units of text, padded 4 by 8, radius 4; solid in the group's colour with
black text (white in the light theme) for a coloured group, the default
colour at alpha 0.1 under `--text-muted` for a plain one. An edge's label
is a pill at the curve's midpoint: 20 units of text in 26 of line, padded
6, radius 4, the canvas background behind it, drawn above the cards.

**Edges.** The curve starts 7 units out from the middle of the side and
ends 7 units short of the other; an end without an arrow gets a straight
7 unit stub to the card, an arrow (tip on the card's edge, 10.4 long,
13 wide, pointing in) covers it. The control points sit along the sides'
normals at half the distance between the curve's ends, clamped to
[70, 150] (observed at spans from 46 to 1826 units). Stroke 2/√scale
units (2√scale px on screen), colour the edge's or `--canvas-color`; the
hit area is 24/√scale units wide. A selected edge keeps its stroke.
Dropping a drawn edge on a card records the side nearest the drop point;
dropping it on empty space makes a 250 by 60 text card there and opens
the creation menu (*Add card*, *Add note from vault*, …), and Escape
takes the card and the edge away again.

**Groups.** The body of a group is background: a click there selects
nothing and a drag there is a marquee; the frame and the label select
and drag it. Dragging a group moves the cards wholly inside it (one
straddling the frame stays). Deleting a group deletes only the group.
Creating one around the selection (Ctrl+G, or *Create group* on the
floating menu) appends it to the nodes.

**Selection and keys.** A marquee selects what it crosses (a card half
covered is selected). The view's own scope binds Ctrl+Z, Ctrl+Shift+Z,
Ctrl+Y, Ctrl+A, Shift+1, Shift+2 and the arrows: an arrow nudges the
selection by 20 units, with Shift by 100. Delete and Backspace remove
the selection, Escape ends editing and leaves the card selected, Enter
on a selected card edits it, Ctrl+C puts the selection on the clipboard
as JSON Canvas text and Ctrl+V pastes it (plain text pastes as a new
card), Ctrl+D duplicates. Snapping (grid and objects, both on by default
in the Canvas settings tab) is off while Alt is held. A card emptied
during editing is removed when editing ends; a click outside ends it.

**Menus.** The floating menu above a selection: *Remove*, *Set color*,
*Zoom to selection (Shift + 2)*, then *Edit* for a text or note card;
*Align*, *Edit label*, *Set background* for a group; *Line direction*
(*Nondirectional*, *Unidirectional*, *Bidirectional*) and *Remove label*
or *Edit label* for an edge; *Create group* and *Align* for several
cards. *Set color* opens the default plus the six presets plus a custom
swatch. Right-click on a card: *Zoom to selection*, *Edit*, *Convert to
file...*, separator, *Remove*; on several: *Zoom to selection*, *Create
group*, separator, *Cut*, *Copy*, *Paste*, separator, *Delete*; on an
edge: *Edit label*, *Follow connection*, separator, *Remove*; on the
background: *Add card*, *Add note from vault*, *Add media from vault*,
*Add web page*, *Create group*, separator, *Undo*, separator, *Paste*,
separator, *Snap to grid*, *Snap to objects*, *Read-only*. Ctrl+drag on
the background draws a rectangle and offers *Add card*, *Add note from
vault*, *Add media from vault*, *Add web page*, *Create group* for it.
The commands are `canvas:new-file` (*Create new canvas*),
`canvas:jump-to-group`, `canvas:convert-to-file` (*Convert to file...*)
and `canvas:export-as-image`, none with a default hotkey; the last three
are shown only while a canvas is active.

**Zoomed out.** At zoom -1.7 and below (scale 0.3078; the *Zoom
threshold for hiding card content* setting) the wrapper gets
`mod-zoomed-out`: a text card shows a centred `align-left` icon, a note
card its name at 32 px semibold, an image card keeps its image and a
link card its web view.

**Saving.** An edit is written about 2 s later (2011 ms measured); a
viewport change alone writes nothing to the canvas file.

**'s divergences.** The resize handles and connection points are
drawn (small dots in the card's colour) rather than invisible; a
selected edge gets a translucent halo; an uncoloured selected card uses
the accent for its ring (not observed); a link card shows its address
rather than a web view and its label is the address rather than the
page title; *Align*, *Set background*, *Edit label*, *Add web page*,
*Read-only*, the snapping toggles, the settings and help buttons and
`canvas:export-as-image` are not there; the card menu's buttons are
clicked, not dragged; an edge dropped on empty space makes its card at
once rather than through the creation menu; the editor in a card draws
at the base font size rather than scaled with the zoom; *Convert to
file* names the note after the card's first line rather than asking.

**Unprobed:** the exact fit padding of Shift+1, the zoom step of the
buttons, snap-to-objects' rules, the creation rectangle of a Ctrl+drag,
and the colour of an uncoloured card's selection ring.

## Observed task checkbox behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-18 with a throwaway plugin in a scratch vault (see
[[Development#The probe batch, 2026-10-18]]), clicking each
`input.task-list-item-checkbox` in the **reading view** with `el.click()` and
reading the note back as raw bytes after every click.

**Any status but a space becomes a space.** The note started as
`- [ ] t0`, `- [x] t1`, `- [X] t2`, `- [?] t3`, `- [-] t4`, `- [>] t5`, and
six clicks, one per line, each changed exactly that line and left the other
five byte-identical:

| Clicked | Line before | Line after |
| --- | --- | --- |
| 0 | `- [ ] t0` | `- [x] t0` |
| 1 | `- [x] t1` | `- [ ] t1` |
| 2 | `- [X] t2` | `- [ ] t2` |
| 3 | `- [?] t3` | `- [ ] t3` |
| 4 | `- [-] t4` | `- [ ] t4` |
| 5 | `- [>] t5` | `- [ ] t5` |

So the click is a two-state toggle writing the literal three bytes `[x]` or
`[ ]`, not a cycle through the statuses, and a custom status is **not**
preserved: `[X]` does not come back capitalised, and `[?]`, `[-]` and `[>]`
are all lost.  does exactly this already (`Editor::click_rendered`),
so the apparent data loss is parity, not a defect.

The rendered markup carries the status character even though the click
discards it:

| Source | `li` class | `li` `data-task` | input |
| --- | --- | --- | --- |
| `- [ ] t0` | `task-list-item` | `""` | `<input type="checkbox" class="task-list-item-checkbox">` |
| `- [x] t1` | `task-list-item is-checked` | `x` | same, with `checked` |
| `- [X] t2` | `task-list-item is-checked` | `X` | same, with `checked` |
| `- [?] t3` | `task-list-item is-checked` | `?` | same, with `checked` |
| `- [-] t4` | `task-list-item is-checked` | `-` | same, with `checked` |
| `- [>] t5` | `task-list-item is-checked` | `>` | same, with `checked` |

Every non-space status is `is-checked` and carries the `checked` attribute,
and the `ul` is `contains-task-list`. The two shapes the block parser rejects
rendered as plain text with no input at all, confirming the parser's rule
from the rendered side: `- [ ]Not a task` (no space after the bracket) and
`- [ ]` alone at end of line.

No configuration key governs any of this. `taskStatus`, `taskStatuses`,
`customTaskStatus`, `checkbox`, `checkboxes`, `cycleTaskStatus`,
`alternateCheckbox`, `completedTaskCharacter` and `taskCompletionText` all
read undefined.

Two commands exist for statuses beyond the click, both already in
`-commands/data/commands.txt` with their observed labels:
`editor:toggle-checklist-status` ("Toggle checkbox status") and
`editor:cycle-list-checklist` ("Cycle bullet/checkbox"). Neither was run.

**Unprobed:** what either command does — in particular whether
`editor:cycle-list-checklist` is the path that reaches `[?]`, `[-]` and `[>]`,
which the click cannot. Live Preview: the matrix step returned `null` for all
six rows and read no bytes, so clicking a checkbox in Live Preview is
unmeasured here ('s own Live Preview toggle is covered by
[[Development#The checkbox the padding hid, 2026-10-18]]). Repeated toggling
of one checkbox: the step that tried it reported the DOM reacting with the
file never changing, contradicting the matrix above, so round-tripping a
checkbox is unsettled. Also unprobed: whether a theme or CSS can make a
custom status survive, whether a checkbox inside an embedded note writes to
the embedded file, and what a click does when the note is open and dirty in
another pane.

## Observed heading fold behavior, Obsidian 1.14.4

Observed on 2026-10-18 with a throwaway plugin in a scratch vault, Obsidian
1.14.4 (installer build 1.13.7), folding one 33-line note (`Fold Probe.md`,
450 bytes, frontmatter plus an `#`/`##`/`###` tree, a list and a footnote
definition) and reading the fold record, the DOM and the vault back after
every step. All fourteen steps of the run completed; nothing below rests on a
failed step. Every editor observation was made in **Live Preview** — the view
reported `{"mode":"source","source":false}` and the container class read
`is-live-preview is-folding`. Raw Source mode was never exercised.

A fold is recorded as line ranges, zero-based and inclusive at both ends:
`{"folds":[{"from":11,"to":18}],"lines":33}`. `lines` is the document's line
count including the empty line after the final newline (the 450-byte note
splits into 33 lines).

What `editor:toggle-fold` folded, against the source line numbering:

| Line folded | Range recorded |
| --- | --- |
| `## Section A` (11), next heading of the same level `## Section B` on 19 | `11`-`18` — the trailing blank line 18 included |
| `### Sub A1` (15), the last thing in Section A | `15`-`18` |
| `## Section B` (19), a list in its body | `19`-`26` |
| `## Section C` (27), the last heading | `27`-`32` — to the final line, the `[^1]:` definition on 31 and the empty line 32 included |
| `# H1 Top` (7) | `7`-`32` — the nested `##`/`###` headings inside it included |
| `- list parent` (23) with two indented children | `23`-`25` — the blank line 26 is *not* part of a list fold, unlike a heading fold's trailing blank |
| `PREAMBLEBODY ...` (5), a paragraph before any heading | command returns `true`, nothing folds |
| `---` (0), the frontmatter's opening fence | command returns `true`, nothing folds |

So the extent is the heading line through the line before the next heading of
the same or a higher level, trailing blanks included, and the last heading
folds to the end of the file.

In the editor the folded heading line is still rendered with `…` appended
(element class `cm-foldPlaceholder`); the folded lines are gone from the
rendered line list. The placeholder line still follows Live Preview's
cursor-reveal rule: with the cursor on it the line reads `## Section A…`, and
after a switch away and back it reads `Section A…`. Each foldable line also
carries a zero-width `div.cm-fold-indicator` (inline-block, `position:
relative`) paired with a sibling `div.collapse-indicator.collapse-icon` at
`opacity: 0`, `position: absolute`, `left: -15px` on the heading lines
captured and `-10px` on the list line; the folded line's indicator gains
`is-collapsed`. The capture returned five such pairs for a note with six
foldable lines, plus one unpaired `collapse-indicator` at `left: -22px` above
the first heading, so it is not established to be a complete listing.

**Nothing is written to the note.** The 450 bytes are byte-identical after
the fold and after the whole run, and no file in the vault's listing was
added, changed or removed. The record lives in `localStorage`, under the key
`5c0r1ac0rpu50001-note-fold-Fold Probe.md` — the app id, `-note-fold-`, then
the vault-relative path. Only this one root-level note was observed, so how a
path in a subfolder is spelled in the key is not known. The vault diff the
run took does not cover `.obsidian/` (its file listing holds no dotfiles),
and the probe's own `setConfig` call did rewrite `.obsidian/app.json` during
the run, so "a fold touches no vault byte" is established for the note and
the vault's non-dot files only.

The fold survives leaving the note and coming back in the same session: the
record is written on leaving and the section is still folded on return.

In reading view:

| Trait | Observed |
| --- | --- |
| Sharing | the editor's fold collapses the same section in reading view, and returning to the editor shows it still folded |
| Record shape | merely entering reading view rewrote `{11,18}` to `{11,12}`, before any click; all four reading-view folds took the `{h, h+1}` form — `{7,8}`, `{11,12}`, `{19,20}`, `{27,28}` |
| Back in the editor | the same fold reads at its full extent again (`{11,18}`), so a stored `to` is not authoritative for the editor |
| Collapsed content | absent from the preview DOM, not hidden: the wrapper list runs `el-p` (TOPBODY) → `el-h2 is-collapsed` (Section A) → `el-h2` (Section B), with no wrapper for the section's paragraph, its `### Sub A1` heading or that heading's body |
| Extent | collapsing `# H1 Top` hid everything below it and left the paragraph *above* it visible; `## Section B` hid its body and the list; `## Section C` hid its body and the footnote definition |
| Indicator | `span.heading-collapse-indicator.collapse-indicator.collapse-icon`, the heading element's first child, `position: absolute`, `left: 31.9935px`, `opacity: 0` expanded and `opacity: 1` plus `is-collapsed` collapsed; the glyph is an inline `svg.svg-icon.right-triangle` with path `M3 8L12 17L21 8` |
| Classes | the `<h1>`/`<h2>` itself keeps an empty class; `is-collapsed` lands on the indicator span and on the wrapper div (`el-h2 is-collapsed`) |
| Clicking the indicator | toggles, both ways — clicking an already-collapsed one expanded it |

One click on heading *text* (an `H2`; which heading is not recorded) changed
nothing. That is one observation, not a rule.

The six fold command ids, with the names the registry reported, all queried
by id rather than enumerated:

| Id | Name |
| --- | --- |
| `editor:toggle-fold` | Toggle fold on the current line |
| `editor:fold-all` | Fold all headings and lists |
| `editor:unfold-all` | Unfold all headings and lists |
| `editor:fold-more` | Fold more |
| `editor:fold-less` | Fold less |
| `editor:toggle-fold-properties` | Toggle fold properties in current file |

Each reports `hotkeys: []` for itself. The five editor ones all pass their
check callback with a markdown view focused in Live Preview.

`editor:fold-all` folded everything in one pass, leaving three rendered lines
and the set `{0,0}`, `{2,3}`, `{7,32}`, `{11,18}`, `{15,18}`, `{19,26}`,
`{23,25}`, `{27,32}` — one range per heading and per list parent, plus
`{2,3}`, which covers the indented YAML list under `tags:` on source lines 2
and 3, and `{0,0}`. No range covers the frontmatter block (lines 0-4), so the
frontmatter is not shown to fold. `editor:unfold-all` cleared every heading
and list fold; in three invocations the set ended at exactly `[{0,0}]` each
time. The `{0,0}` entry is unexplained — it is never correlated with hidden
content, and it reaches storage: the run's final record read
`{"folds":[{"from":0,"to":0},{"from":19,"to":20}],"lines":33}`.

With `.obsidian/app.json` holding literally `{}`, `foldHeading` and
`foldIndent` both read `true` and folding worked, so both default to on.
With `foldHeading` set to `false` through the API: the editor DOM query for
fold indicators came back empty (it returned nothing for the list indicator
either, which `foldIndent: true` should still produce), and
`editor:toggle-fold` returned `true` and folded nothing — the step records no
target line, so the toggle is not shown to have been aimed at a heading. In
reading view the three headings captured still carried the indicator span,
computed `display: none` with a zero rect, and a scripted click on that
hidden indicator still collapsed the section: the machinery stays live, the
affordance is hidden.

One incidental: writing `foldHeading` through the API did not reach
`.obsidian/app.json` at once. When the app read `false`, the file still read
`{}`; after restoring the in-app value to `true`, the file read
`{"foldHeading": false}` — the file trailing the in-memory value by one
change. One observation, in a run whose cleanup did not restore the config.

**Unprobed:** what the editor does on a cold load with a record the reading
view wrote (`{h, h+1}`) — the app was never restarted, and merely rendering
the note in preview rewrites the stored record, so a cold editor load after a
reading-view session is the likely case and is unobserved; whether fold state
survives a restart at all, and whether anything outside `localStorage` holds
it — `foldManager` exposes `loadPath`, `savePath`, `load` and `save`, none of
which was traced to a file, and the step's `.obsidian` listing came back
empty from a directory we know holds `app.json`, so it is not evidence of
absence; whether `.obsidian/` is untouched by a fold; what `{0,0}` is;
`editor:fold-more` and `editor:fold-less`, invoked four times between them,
each returning `true` with the fold set and the rendered lines unchanged and
the cursor position unrecorded — a no-op observation, not their semantics;
`editor:toggle-fold-properties`, never invoked; keyboard folding, since
`hotkeys: []` is each command's own report and no key was pressed; folding
with raw Source mode on; `foldIndent: false`; the `localStorage` key for a
note in a subfolder; whether a click on heading text ever toggles a fold in
reading view, and whether any user-visible affordance remains there when
`foldHeading` is `false` (only a synthetic click on the `display: none`
indicator was tried); what navigation the jump step performed — it recorded
only that the cursor ended at line 19 column 0 with the section unfolded in
the live fold set while the saved record still carried `{19,26}`, so "opening
a link into a folded section unfolds it" is not established; whether folding
a heading while another fold is already open removes only its own record (the
one expand observed had no other entry left to leave alone); and folding for
headings inside callouts or blockquotes, setext headings, a heading on the
first line of a file, a file with no trailing newline, and an empty last
section.

## Observed code-block copy behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-18 with a throwaway plugin in a scratch vault: notes
written byte-exactly through the vault API, then every `copy-code-button` in
the rendered container clicked while `navigator.clipboard`, Electron's
`clipboard` and the `copy` event were tapped, and the clipboard read back.
The version numbers are `apiVersion` and `installedVersion`; `appVersion` was
null in this build, so the app version was never captured.

Two counting caveats that the rest of this section already accounts for. The
scanned container held **both** views — it opens with
`div.markdown-source-view … is-live-preview style="display: none"` — so of the
ten `<pre>` elements found in the probe note, nine are reading view and the
tenth is the editor's own live-preview callout widget (a byte-identical
duplicate of the reading-view callout block, with an all-zero rect). Ten
clicks therefore produced nine distinct payloads, not ten. And every click was
dispatched programmatically on a button whose computed `display` was `none`;
the hover → reveal → click path a person takes was never exercised.

The markup:

- Every `<pre>` the Markdown renderer produces carries exactly one
  `<button class="copy-code-button">`, appended as the pre's last child,
  after the `<code>`. Ten of ten pres in the probe note had one.
- `class` is the button's only attribute: no `aria-label`, no `title`, no
  text, no `tabindex` (`tabIndex` reads 0, which is the browser default for
  `<button>`, not an Obsidian choice). Its sole child is an
  `svg.svg-icon.lucide-copy`.
- The renderer adds it, not the reading-view leaf: a detached
  `MarkdownRenderer.render` that was never attached to a view already had a
  button on all three of its pres, the callout block and a bare
  `<pre><code>indented block\n</code>` included.
- Reading view shows no language name anywhere on the block: `::before` and
  `::after` on both `pre` and `code` compute to `content: none`, and the
  flair-element scan came back empty on every pre in both scanned notes.

The four `app.css` rules that matched, plus the two mobile overrides:

| Selector | Declarations |
| --- | --- |
| `.markdown-rendered pre:not(:hover) > button.copy-code-button` | `display: none` |
| `.markdown-rendered button.copy-code-button` | `margin: 6px; padding: 6px 8px; height: auto; background-color: transparent; box-shadow: none; color: var(--text-muted); font-size: var(--font-ui-smaller); font-family: var(--font-interface); position: absolute; top: 0; inset-inline-end: 0` |
| `.markdown-rendered button.copy-code-button:hover` | `background-color: var(--background-modifier-hover)` |
| `.is-mobile .markdown-rendered pre:not(:hover) > button.copy-code-button` | `display: block` |
| `.is-mobile .markdown-rendered button.copy-code-button` | `width: auto` |

So the button is hover-only on desktop and always visible on mobile. The
computed style of an unhovered button is `display: none`, `border-radius:
5px`, `color: rgb(179, 179, 179)`; the containing pre is `position: relative`.

What a click copies. The one intercepted call is
`navigator.clipboard.writeText`, once per click; the Electron tap and the
`copy` listener were installed and their intercept arrays stayed empty (an
absence, not a recorded negative). The payload is the `code` element's plain
text — Prism's `<span class="token …">` markup does not survive — with
**exactly one** trailing newline removed, not a trim:

| Case | DOM text | Copied |
| --- | --- | --- |
| Fence body ending in three newlines | `…tab indented\n\n\n` (79 chars) | `…tab indented\n\n` (78) |
| Ordinary one-line fence | `const CASE_LANG = 1;\n` | `const CASE_LANG = 1;` |
| 4-space indented block | `…four spaces\n…second line\n` (52) | same without the final `\n` (51) |
| Frontmatter `pre` (no trailing newline) | `title: Code Copy Probe\ntags:\n  - probe` (38) | identical (38) |
| Empty fence | `""` | `""` |

Trailing spaces and a leading tab inside the body survive untouched. The
byte claims rest on a deliberate input/output diff: the probe wrote the note
and read it back byte-identical before rendering it.

A CRLF file's code block is LF-only in both the DOM and the clipboard —
`CRLF line one\r\nCRLF line two\r\n` on disk (6 CRs, round-tripped) renders as
`CRLF line one\nCRLF line two\n` and copies as `CRLF line one\nCRLF line two`.
Obsidian normalizes the endings while leaving the file's CRs alone.

One sample of each block shape got a button and copied its own body, all from
a single hand-built note: fence with a language, fence with none, 4-space
indented block, `~~~` fence, an unregistered info string, inside a callout,
inside a blockquote, an empty fence, and the frontmatter block. The
frontmatter case is the odd one: `pre.frontmatter.language-yaml` computes to
`display: none` (the properties widget is shown instead) and still carries a
button, and a synthetic click on it copies the raw YAML. The one inline
`<code>` in the note had no sibling button.

Prism classes on the rendered block:

| Fence | `pre` class | `code` class |
| --- | --- | --- |
| ```` ```js ```` | `language-js` | `language-js is-loaded` |
| `~~~python` | `language-python` | `language-python is-loaded` |
| ```` ```-made-up ```` (unregistered) | `language--made-up` | `language--made-up is-loaded` |
| ```` ``` ```` with no info string | *(none)* | *(none)* |
| 4-space indented block | *(none)* | *(none)* |
| Frontmatter | `frontmatter language-yaml` | — |

Click feedback is icon-only. The SVG is `lucide-check` at 0, 150 and 400 ms
after the click and back to `lucide-copy` at 900, 1500 and 2200 ms. Across
all six samples `className` stays `copy-code-button` and `textContent` stays
empty, and no Notice appeared on any of the twelve clicks the run made.

In the editor there are **two** affordances, not one:

- A plain fence line gets `span.code-block-flair`, `aria-label="Copy"`,
  `contenteditable="false"`, on the `HyperMD-codeblock-begin-bg` line, with
  the text `JavaScript` for a ```` ```js ```` fence. Its rule is
  `.markdown-source-view.mod-cm6 .code-block-flair { position: absolute;
  right: 6px; top: 6px; z-index: 1; display: inline-block; padding:
  var(--size-4-1) var(--size-4-2); border-radius: var(--code-radius);
  font-family: var(--font-interface); font-size: var(--font-ui-smaller);
  color: var(--text-muted); cursor: var(--cursor); }`, plus a `:hover`
  background.
- A code block rendered as an embed widget — the callout — gets a real
  `button.copy-code-button` inside the CodeMirror DOM: one `pre` inside the
  editor, one `cm-embed-block`, and the focusable-element walk lists
  `BUTTON.copy-code-button tabindex=0` immediately after
  `DIV.cm-embed-block cm-callout`.

No command in the registry copies a code block. Filtering the registry, the
ids containing "copy" are `workspace:copy-path`, `workspace:copy-full-path`,
`workspace:copy-url`, `editor:table-row-copy`, `editor:table-col-copy` and
`bases:copy-table`; the ones containing "code" are `editor:toggle-code` and
`editor:insert-codeblock`.

Every reading-view `pre` sampled computed to `white-space: pre-wrap` with
`overflow-x: auto`. This contradicts the "fenced code never wraps" row above
and is a computed value only — the matched-rule dump covered the copy-button
selectors, and the page carried seven stylesheets besides `app.css`.

**Unprobed:** what clicking the editor's `span.code-block-flair` does (it was
never clicked, only its `aria-label` read), and whether the flair's text is a
language display-name table or a one-off — one fence, one language was seen.
Whether Live Preview and source mode differ: both steps read back
`mode: "source"`, and their button counts (10 and 9) were taken over a
container that still held the reading-view DOM. Keyboard activation: the
button is natively focusable, not disabled, and the ten buttons appear in
document order in the focusable walk, but a `focus()` call on a
`display: none` button did not move focus and the synthetic Enter is
self-labelled non-evidence, so whether a real Tab reaches it and Enter copies
is unanswered. The painted geometry and size, since every button rect was
zero under `display: none`. Whether a real pointer interaction behaves like
the synthetic click. Why copying the empty fence left the Electron clipboard
holding the pre-probe text rather than `""` or the previous value — and
relatedly, for nine of the ten clicks only the Electron tap witnessed the
clipboard (`navigatorSkipped: "skipped for time budget"`). Whether the write
reaches the system clipboard independently of the probe: `navigator.clipboard
.writeText` was replaced before every click, so the read-back cannot separate
Obsidian's write from the tap's own forwarding. Whether a code block inside a
transcluded note embed gets a button — no embed rendered in the probe note.
Whether any setting turns the button off: the config scan filtered a
probe-chosen list of eight keys, not an enumeration of Obsidian's settings.
The exact revert delay of the check icon, bounded only between 400 ms and
900 ms. Mobile behavior beyond the two `.is-mobile` rules, nothing having run
with `isMobile` true. Whether the copy text can diverge from the file's bytes
in ways other than line endings — HTML entities, unicode normalization,
zero-width characters. Whether the frontmatter pre's button is reachable by a
person at all, its pre being `display: none`. And which rule sets the
reading-view `pre` to `pre-wrap`.

## Observed raw HTML behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-18 with a throwaway plugin in a scratch vault: a note of raw-HTML constructs opened in reading view and read back, a second note in a subfolder for source-relative `src` values, and 26 single-construct cases from a second probe file. Three limits bound everything below. The run executed its step set twice and the stored values are the second pass, which created nothing — `setup.made` is empty, every probe file already existed. The reading view it measured stopped short: `sectionChildTags` holds 18 entries and ends on the `DIV.el-h2` for "Sup and sub" with no body element after it, so the note's tail never rendered. And the 26 cases come from a different session — its `app://` host hash and its `pic.png` query both differ from the main run's — in a file that records no step name, so the surface those cases rendered into is not stated.

### Images

A raw `<img>` whose `src` resolves to a vault file has the attribute itself rewritten to `app://<36 hex>/<absolute filesystem path>?<13 digits>`. Resolution is Obsidian's link resolution, not path arithmetic:

| Authored in | `src` | Resolves to |
| --- | --- | --- |
| root note | `pic.png`, `./pic.png`, `/pic.png` | `pic.png` |
| root note | `assets/deep.png`, `deep.png` | `assets/deep.png` |
| root note | `pic two.png`, `pic%20two.png` | `pic two.png`, written `pic%20two.png` |
| `html/Nested HTML.md` | `sibling.png`, `./sibling.png`, `html/sibling.png` | `html/sibling.png` |
| `html/Nested HTML.md` | `../pic.png`, `pic.png` | root `pic.png` |
| `html/Nested HTML.md` | `assets/deep.png`, `deep.png` | `assets/deep.png` |

A bare name is resolved like a wikilink target rather than joined to the note's folder: from `html/`, `sibling.png` finds the sibling and `deep.png` finds `assets/deep.png`. Each of those basenames was unique in the vault, so the preference order between a root file and the source's own folder was not exercised.

- The host is 36 hex characters and differs between the two probe sessions (`e53bdfa6…cfc9f`, `4440591f…6345b`) — per session and opaque.
- The `?<13 digits>` query is constant per file within one run (`pic.png` `?1792374870859`, `assets/deep.png` `?1792374954193`, `html/sibling.png` `?1792374954195`), the same value in both notes and both DOMs. `pic.png` carries `?1792375399934` in the other session's file, so as observed it is not a stable property of the file.
- A `src` that resolves to nothing is left verbatim (`missing.png`); the browser then resolves it against `app://obsidian.md/` and the image is broken (`naturalWidth` 0).
- An `http://` `src` is left alone and really fetched (`naturalWidth` 1). The request carries no `Referer` header, which is what the `referrerpolicy` below asks for.
- The reading view adds `referrerpolicy="no-referrer"` to every raw `<img>`, the unresolved and the remote one included. It belongs to raw-HTML imgs specifically: the `<img>` inside a reading-view markdown embed (`span.internal-embed media-embed image-embed is-loaded`) carries only `alt` and `src`.
- Nine consecutive raw `<img>` lines with no blank line between them are one HTML block and render as a single `div.el-img` with the images as direct children — no `<p>`. Raw imgs written inside a paragraph stay in that paragraph's `<p>`.
- Markdown embeds have a different shape, for contrast: reading view wraps the img in `<span alt="md-sub" src="assets/deep.png" class="internal-embed media-embed image-embed is-loaded">` — the unresolved vault path stays on the wrapper and the resolved `app://` URL goes on the inner img — while the editor-side form is `div.internal-embed … tabindex="-1" contenteditable="false"` with `div.image-wrapper`, `div.image-resize-corner` and `div.embed-actions` children.

The editor DOM retained inside the same leaf renders raw HTML through a `span.cm-html-embed` widget whose imgs carry the same resolved `app://` srcs but no `referrerpolicy`. Those rows were read while the leaf was displaying the reading view and are not laid out (zero rects), so they are widget output rather than a measured Live Preview session.

### Anchors

- A raw `<a>` keeps its href exactly as authored — no resolution, no extension handling, no `app://` — and gains `target="_blank" rel="noopener nofollow"`. It gets neither `class="internal-link"` nor `data-href`, even when the href names a note that exists.
- A wikilink in the same paragraph renders `<a data-href="Target Note" href="Target Note" class="internal-link" target="_blank" rel="noopener nofollow">`, so `target` and `rel` are on internal links too in 1.14.4, and `data-href` plus the class are what separate them from a raw anchor.
- Clicking each of five raw anchors (`Target Note`, `Target Note.md`, `assets/deep.png`, `Missing Target`, `#Images block`): no leaf opened (one before and after), the active file stayed `Raw HTML.md`, `window.open` and electron `shell.openExternal` were never called, `location.href` was unchanged, and no vault file was created or removed — not even for the missing target. `defaultPreventedByObsidian` was false on all five, but the probe's own navigation guard also fired and cancelled the default on all five, and nothing records the handler order, so this says what did not happen, not that Obsidian declined the click.

### `<pre>`

- A raw `<pre>` renders as a real `<pre>` inside `div.el-pre`, with an empty `className` and no attributes of Obsidian's own. Its content is literal — `**not bold**` and `[[Target Note]]` stay text, `&` serializes as `&amp;`, two leading spaces survive — but a tag inside it is live markup: `<b>tag inside pre</b>` is an element, and the only `<b>` in the note.
- The copy button belongs to `pre > code`: a raw `<pre><code>` gets `button.copy-code-button` (a lucide-copy svg), a raw `<pre>` with no `<code>` child does not, a fenced block does. Two copy buttons in the note, and two buttons of any kind.
- A fenced block is `pre.language-text > code.language-text.is-loaded[data-line="0"]`; a raw one has neither the class nor `data-line`.
- Wrapping, measured once in a 304 px pane: the three reading-view `<pre>` elements all computed `white-space: pre-wrap; overflow-wrap: break-word; overflow-x: auto; tab-size: 4`, and a 295-character line whose longest unbroken token is 240 characters did not scroll sideways (`scrollWidth == clientWidth == 304`; the block grew to 408 px tall). The same note's two editor-side `<pre>` elements, read from the same DOM, computed `white-space: pre; overflow-x: visible` with zero rects. This disagrees with the "fenced code never wraps" invariant recorded above, but one pane width, an uncaptured theme and an unlaid-out counter-example are not enough to change it.

### Sanitizer, 26 single-construct cases

| Authored | Rendered |
| --- | --- |
| `<span onclick="…">x</span>` | `<span>x</span>` — the handler attribute is dropped, the element stays; `onclickFired` false |
| `<script>window.__probeScript=1</script>after` | `after` — element and script text gone, following text survives; `scriptExecuted` false |
| `<a href="javascript:…">x</a>` | `<a target="_blank" rel="noopener nofollow">x</a>` — no `href` at all; `javascriptHrefFired` false |
| `<a href="https://example.invalid/page">`, `<a href="obsidian://open?vault=x&file=y">` | href kept verbatim (`&` re-escaped as `&amp;` in serialization), `target` and `rel` added |
| `<span class id data-foo title>` | all four attributes kept |
| `<img src="pic.png" width="100" height="50" alt="A">` | `<img alt="A" height="50" width="100" src="app://…" referrerpolicy="no-referrer">` |
| `<iframe src="http://…" sandbox="">` | both kept as written — an author's empty `sandbox` is not replaced |
| `<table><tr><td>c</td></tr></table>` | `<table><tbody><tr><td dir="ltr">c</td></tr></tbody></table>` |
| `start <b>bold never closed` | `<p dir="auto">start <b>bold never closed</b></p>` — closed at the end of its paragraph, no leak into later blocks |
| ins, del, kbd, samp, var, small, big, center, font[color], strike, tt, details, summary, sup, sub | all kept with their content |

- The `style` attribute survives verbatim in all nine style cases, and the computed style shows it taking effect for `color`, `background-color`, `font-size`, `font-weight`, `display:none`, `position:absolute` and `display:inline-block`. Two cases show survival only: `background-image:url(http://…)` left no `backgroundImage` in the computed set and no request reached the probe's server, and `<div style="text-align:center">` was recorded with its `attrs` and `computed` both null.
- Attribute order is rebuilt, not passed through: `class id data-foo title` comes back `title data-foo id class`, and `src width height alt` comes back `alt height width src referrerpolicy`.
- Obsidian stamps direction attributes a matching renderer has to emit too: `<p dir="auto">` on paragraphs, `dir="auto"` on headings (`<h2 data-heading="Anchors" dir="auto">`), `dir="ltr"` on a `<td>` in a raw table.
- Inline HTML is wrapped in `<p dir="auto">`; HTML that opens a CommonMark HTML block — `div`, `center`, `details`, `table`, `iframe`, a lone `<img>` on its own line — gets no paragraph wrapper.
- `<sup>` and `<kbd>` are themed rather than merely structural: sup computes `vertical-align: super` at 12.5 px against a 15 px body, kbd gets `background rgb(35, 35, 35)` at 13.125 px.

The recorded pass left the vault's 74 paths identical before and after, and cleanup detached one markdown leaf. It also attempted no write, so that is not evidence that rendering and clicking are read-only — only that nothing was written.

**Unprobed:** where a raw anchor click ends up, and whether `href="#Images block"` scrolls, since the probe's guard cancelled every navigation; `<details>` in a real note and in-note `<sup>`/`<sub>` — `details.count` is 0 and the sup/sub paragraph was "not found", both consistent with the preview stopping at the last heading rather than with Obsidian omitting them, and the second file's own `supsub` value is null, so sup/sub rests on the one detached case; whether an iframe or a CSS `background-image` in a note actually fetches (no `/san.png` request appears, but that HTTP log is shared across scenarios and those cases ran in the other session); whether the `?<digits>` query is the file mtime, as no file timestamps were captured; whether the pre-wrap measurement is the stock default, since only six config keys were recorded (`attachmentFolderPath`, `newLinkFormat` `shortest`, `useMarkdownLinks` false, `showInlineTitle`, the probe's HTTP base, the version) and none covers theme, snippets, appearance or readable line length — and every link-resolution observation above was made under `useMarkdownLinks: false`, `newLinkFormat: shortest`; an ambiguous bare basename, root file against source folder, since each probe basename was unique; which elements get unwrapped, since all 26 cases used tags that survived and none covered object, embed, form, svg, style, link or meta; raw HTML inside a callout, blockquote, table cell or list item, and a raw HTML block interrupting a paragraph; whether the notes' bytes changed, since only byte counts and a truncated head were read back; and the markup behind the Embeds, Anchors, Pre and Sup-and-sub sections, since the 2367-byte source of `Raw HTML.md` is truncated after 700 bytes in the evidence.

## Observed remote image behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-18 with a throwaway plugin in a scratch vault: a table of
image syntaxes rendered through `MarkdownRenderer` into a detached container,
one note opened in the reading view, and a local HTTP server logging every
request with its headers. Ten of the sixteen requests came from the detached
container — that is the renderer API, not a note anyone opened; the other six
(`rv-inline.png`, `slow`, `404`, `rv-sized.png`, `cache-probe.png`,
`rv-rawhtml.png`) came from the opened note.

Fetching:

- Every `http:` destination in rendered markdown is requested, with no prompt,
  no allowlist and no gate: sixteen `GET`s in a 9.3 s run, one per distinct URL.
- The six images of the opened note were all requested inside 2 ms of each
  other, and no sampled `<img>` carries a `loading` or `decoding` attribute.
- Mounting one URL repeatedly produces no further request: the DOM held four
  elements on that URL (the probe accounts for three of them — two extra mounts
  plus the reading-view one) and the server logged exactly one `GET`, with a
  single `PerformanceResourceTiming` entry. Whether that cache is Obsidian's or
  Chromium's ordinary one cannot be told apart here.
- An image request carries no `Referer`, no `Origin` and no `Cookie`. The
  plugin's own `XMLHttpRequest` in the same run does send
  `Origin: app://obsidian.md`, so the absence is specific to images.
- What it does send: `Sec-Fetch-Dest: image`, `Sec-Fetch-Mode: no-cors`,
  `Sec-Fetch-Site: cross-site`, `Sec-Fetch-Storage-Access: active`,
  `Accept: image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8`,
  `Accept-Encoding: gzip, deflate, br, zstd`, `Accept-Language: en-US`,
  `Connection: keep-alive`, the `sec-ch-ua` trio, and a Chrome user agent
  ending `obsidian/1.13.7 Chrome/150.0.7871.47 Electron/43.1.0`.

What each syntax renders to, and whether it is fetched:

| Source | Emitted | Fetched |
| --- | --- | --- |
| `![alt](http://h/x.png)` | `<img alt="alt" src="http://h/x.png" referrerpolicy="no-referrer">` | yes |
| `![alt\|200x100](http://h/x.png)` | adds `height="100" width="200"`, `alt="alt"` | yes |
| `![alt\|200](http://h/x.png)` | adds `width="200"`, no `height` attribute | yes |
| `![200x100](http://h/x.png)` | `alt="200x100"` *and* `height="100" width="200"` | yes |
| `![alt\|200x100](http://h/x.png "the title")` | adds `title="the title"` | yes |
| `![alt](<http://h/x.png>)` | same as the plain form | yes |
| `![ref alt][rr]`, `[rr]: http://h/x.png "Ref title"` | `<img title="Ref title" alt="ref alt" src=…>` | yes |
| `<img src="http://h/x.png">` (raw HTML) | `<img src=… referrerpolicy="no-referrer">` | yes |
| `![alt](http://h/noext?format=png&x=1)` | loads (`naturalWidth` 1): no extension sniffing | yes |
| `![alt](data:image/png;base64,…)` | inline `<img>`, `referrerpolicy` still added | n/a |
| `![alt\|120x60](data:image/svg+xml;base64,…)` | `height="60" width="120"` honoured | n/a |
| `![[http://h/x.png]]` | `<span class="internal-embed is-loaded file-embed mod-empty-attachment">“http://h/x.png” could not be found.</span>`, no `<img>` | no |
| `![[http://h/x.png\|200]]` | the same span, plus `width="200"` on it | no |
| `![alt](//h/x.png)` | `<img src="//h/x.png">`, resolving to `app://h/x.png` — scheme *and* port dropped — `naturalWidth` 0 | no |
| `![alt](file:///abs/path.svg)` | rewritten to `app://<36 hex>/abs/path.svg`, loads | n/a |
| `![alt](app://local/abs/path.svg)` | left as written, `naturalWidth` 0 | no |
| `![[probe-pic.svg]]` (internal control) | `span.internal-embed.media-embed.image-embed` wrapping `<img src="app://<36 hex>/abs/path.svg?1792374924921">` | n/a |

A URL inside a wikilink embed is therefore not remote content at all: it is an
unresolved file embed, and nothing is requested. The internal embed's query
string is the file's mtime in milliseconds, matching the vault listing exactly
— a cache-buster keyed on modification time.

`referrerpolicy="no-referrer"` is on every markdown-rendered `<img>`, raw-HTML
and `data:` ones included. The internal embed's `<img>` has none
(`referrerPolicy` is `""`).

Attribute order is not canonical, so nothing should depend on it. The
markdown-rendered sized image serialises as `height, width, title, alt, src,
referrerpolicy`; a plain one as `alt, src, referrerpolicy`; the raw-HTML one
written `src, width, height, alt` comes back as `alt, height, width, src,
referrerpolicy`; the internal sized embed as `alt, width, src`. The raw-HTML
case shows the sanitizer rebuilds the element rather than passing it through.

A size hint is stripped from the alt text and applied as attributes. `|200`
sets `width` only and no `height` attribute; the rendered height then follows
the intrinsic ratio — demonstrated by the internal 300x150 embed
(`width="200"` → 200x100), not by the external cases, whose probe image is 1x1.

The opened-note run captured nodes from more than one surface, and they differ:

| | Markup | Computed |
| --- | --- | --- |
| Wrapper surface | `<div class="image-wrapper"><img src=… alt=…><div class="image-resize-corner"></div></div>`, **no** `referrerpolicy` | `display: block`, `width: 100%`, `max-width: 100%` |
| Raw HTML there | `<img>` inside `span.cm-html-embed`, no `referrerpolicy` | `display: inline`, `width: auto`, `max-width: 100%` |
| Reading view | `<div class="el-p"><p dir="auto">Text before <img alt="inline ext" src=… referrerpolicy="no-referrer"> and text after…</p></div>` | not captured |

The wrapper and its resize corner are present around a 404 and around a slow
image that has not loaded, so they do not depend on load state. Which leaf the
wrapper nodes belong to was not established: the view mode was `preview` and
the sampling was document-wide. Computed sizes from the detached render
container are not evidence of layout — it has none of the reading view's CSS,
and every `<img>` in it computes `display: inline`.

Load states, sampled on the wrapper surface:

- A 404 stays a plain `<img>`: `complete` true, `naturalWidth` 0, no class, no
  placeholder element, no alt substitution in the DOM.
- A five-second image is `complete` false with `currentSrc` `""` at 250 ms and
  at 1451 ms, and `complete` true with `naturalWidth` 1 at 6652 ms; its
  resource timing reads 5004.9 ms.
- `https://` to the plain-HTTP probe port never reached the server, but a
  resource-timing entry exists for it (2.7 ms), so the attempt was made and
  failed in transport. This says nothing about how Obsidian treats `https:`.

No remote-content setting surfaced. `vault.getConfig` returned undefined for
ten guessed keys (`showRemoteContent`, `allowRemoteContent`, `remoteContent`,
`remoteImages`, `externalImages`, `loadRemoteImages`, `trustedExternalContent`,
`privacyMode`, `offlineMode`, `imageCache`) in the same call that returned
values for `attachmentFolderPath`, `showInlineTitle` and `strictLineBreaks`,
and `app.json` held only `foldHeading` and `propertiesInDocument`. Of the nine
setting tabs, three were rendered — Interface (8 items), Editor (25), Files and
links (22) — and none of their items concerns remote content. The window's own
CSP meta constrains `style-src` only, so it is not what governs image loading;
`document.cookie`, `document.referrer` and the meta referrer are all empty.

**Unprobed:** which leaf the `image-wrapper` nodes come from, since sampling was
document-wide while the view mode was `preview`; the reading view's own markup
for the slow, 404, sized and raw-HTML images, because the captured
`previewHTML` is truncated (`…[+770 chars]`) before them; layout of any kind —
every sampled rect is 0x0 and the layout capture is null, so whether space is
reserved before an image arrives, and how it flows with surrounding text, is
unmeasured; whether `div.image-resize-corner` resizes a remote image and
whether a drag writes a `|WxH` hint back into the note; lazy loading, since
nothing was scrolled and no image was shown to be offscreen; how `https:` URLs
behave, since the probe server spoke plain HTTP; why `app://local/<abs path>`
failed while `app://<36 hex>/<abs path>` loaded; caching across restarts or to
disk, revalidation, and `Cache-Control`/`ETag` handling — no response headers
are in the capture and no conditional request appears in the log; redirects,
a non-image `Content-Type`, authentication, and very large images, none of
which were requested; what `CachedMetadata` records for an external image, the
run having measured DOM mounts rather than the cache; how a broken image looks
to the eye, no screenshot being taken; and whether a user-facing switch exists,
six setting tabs (General, Appearance, Hotkeys, Keychain, Core plugins,
Community plugins) having been skipped unrendered.

## Observed properties-in-reading-view behavior, Obsidian 1.14.4

Observed on 2026-10-18 with a throwaway plugin in a scratch vault, on Obsidian
1.14.4 (installer build 1.13.7), reading the reading view's DOM, the two
properties panes and `app.metadataTypeManager`, and the vault's files back as
raw bytes. That run read and clicked; it never succeeded in typing into a
property, so what it settled is structure and layout. The editing path was
settled on 2026-10-19 by a CDP-driven run
([[Development#Driving Obsidian over CDP, 2026-10-18]]): real browser-injected
keystrokes and mouse clicks against the same build, with every file read from
disk by the host rather than through the API. Its vault was fresh —
`.obsidian/app.json` written as `{}` and nothing else — holding `Note A.md`
(`Status: draft`, `zebra: alpha-note`, `alpha: 1`, `tags` one/two, `aliases`
Alpha, `count: 3`, `done: true`, `due: 2026-10-18`, `link: "[[Note B]]"`,
`long value`), `Note B.md` (`status:`), `Note C.md` (`STATUS:`), `Dotted.md`
(`book.title`, `book.year`), `Nested.md` (`shelf:` holding a map),
`Past Date.md` (`due: 2020-03-04`), `No Frontmatter.md` and `Plain.txt`; the
Properties and Daily notes core plugins were already on. All thirteen run steps
and both post-restart steps completed. Two things about that session bound
every pixel below: the window was 1024×800, and the File properties pane
opened in step 6 stayed open for the rest of the run, so the main leaf was
680 px wide for the first text edits and 380 px (block width 304) from then
on. Where the two runs disagree the bytes win; where only the plugin run saw
something, it says so.

`propertiesInDocument`, with a ten-property note open in reading mode:

| Value | Reading view | Side panes |
| --- | --- | --- |
| `visible` | the properties table | all 10 rows / all 13 registry rows |
| `source` | the **same table** — the reading view never shows raw YAML | unchanged |
| `hidden` | the block is gone from the rendered text; `div.metadata-container` is still in the DOM, with its text run together | unchanged |

So `source` is an editor-only mode: only the editor swaps the widget for the
YAML. `hidden` drops the block from the document in both views and changes
neither pane. The CDP run adds the mechanism: under `hidden` the preview root
(`div.markdown-preview-view`) loses its `show-properties` class and the
container's computed display is `none`; under `visible` and `source` the class
is present and the container is `block`. `--metadata-display-reading` reads
`block` in all three modes, so the class, not the variable, carries the
switch. The File properties pane kept all 11 rows under every mode. (That
run's block happened to be folded throughout the step, so under `source` it
saw only the 51 px heading; the table itself under `source` rests on the
plugin run.) In all three modes the frontmatter also emits a
`div.el-pre.mod-frontmatter.mod-ui` element holding a
`pre.frontmatter.language-yaml` with `style="display: none"` whose `code` is
the YAML as Prism tokens (`span.token.key.atrule` for each key) — the plugin
run read it as empty because it sampled text, and it never reaches the
rendered text. Under `hidden` the note's `div.mod-header.mod-ui` element holds
only the inline title; under the other two it holds the title and the whole
block.

The write of the setting is **deferred**. Immediately after the probe restored
`propertiesInDocument`, `.obsidian/app.json` still read `{\n  "foldHeading": true\n}`
with no such key; the key appeared only in the final read about 4.8 s later, as
`{\n  "foldHeading": true,\n  "propertiesInDocument": "visible"\n}` —
two-space indent, no trailing newline, written even though `visible` is the
default. The file held one prior key, which also sorts before the new one, so
this run cannot tell insertion order from sorted output; it was also not a
clean baseline, since `foldHeading` flipped from `false` to `true` mid-session
without the scenario touching it. The CDP run's baseline was clean, and it set
each mode with `app.vault.setConfig` (the Settings dropdown was not used):
the file's mtime landed 1001, 1002 and 1003 ms after the three calls, and each
1 s read already held the new bytes — `{\n  "propertiesInDocument": "source"\n}`
(38 bytes), then `"hidden"` (38), then `"visible"` (39, explicit again) — so
the save is debounced about one second. Each save also rewrote
`appearance.json` (`{}`, 2 bytes) with an mtime within 2 ms of `app.json`'s,
though nothing in it changed.

The in-document block, from the reading view's DOM:

```
div.metadata-container[tabindex=-1][data-property-count="10"]
  div.metadata-error-container                      style="display: none;"
  div.metadata-properties-heading[tabindex=0]
    div.collapse-indicator.collapse-icon > svg.right-triangle
    div.metadata-properties-title                   "Properties"
  div.metadata-content > div.metadata-properties
    div.metadata-property[tabindex=0][data-property-key="status"]
      div.metadata-property-key
        span.metadata-property-icon[aria-disabled=false] > svg.lucide-text
        input.metadata-property-key-input[autocapitalize=none]
                                        [enterkeyhint=next][type=text]
                                        [aria-label="Status"]
      div.metadata-property-value[data-property-type="text"] > <widget>
      div.clickable-icon.metadata-property-warning-icon
                      [aria-label="Type mismatch, expected Text"]  display:none
```

- The type is an attribute of the **value wrapper**, `data-property-type`, not
  of the row: the row element carries no type attribute at all.
- Rows follow the file's key order, unsorted. `data-property-key` is the key
  lower-cased; the key input's `aria-label` keeps the file's casing — `Status:`
  in one note gives key `status` labelled "Status", `status:` in another gives
  `status` labelled "status".
- Every row carries the hidden warning icon, the `link` row included. The six
  labels seen are "Type mismatch, expected" Text, Number, Tags, Aliases,
  Checkbox, Date.
- The block's text ends "Add property": the add affordance is inside the
  in-document container too, not only in the sidebar. It is
  `div.metadata-add-button.text-icon-button[tabindex=0]` holding
  `span.text-button-icon` (lucide-plus) and `span.text-button-label`.
- `data-property-count` follows the row count: 10 at the start, 11 once
  `demo` was added.

Value markup by inferred type, from one note's ten properties:

| Type | Value element |
| --- | --- |
| text | `div.metadata-input-longtext[contenteditable=true][placeholder=Empty][spellcheck=true][tabindex=0]` |
| number | `input.metadata-input.metadata-input-number[type=number][inputmode=decimal][step=any][placeholder=Empty]` |
| checkbox | `input.metadata-input-checkbox` |
| date | `input.metadata-input.metadata-input-text.mod-date` |
| tags, aliases | `div.multi-select-container` holding a contenteditable `div.multi-select-input` |

A `text` property whose value is a wikilink (`link: "[[Note B]]"`, widget
`text` in the registry) is drawn instead as a non-editable `div.metadata-link`:
a `div.metadata-link-inner.internal-link[data-href="Note B"][draggable=true]`
with the link text and a `div.metadata-link-flair` holding a pencil
(lucide-pencil). A tag pill is `div.multi-select-pill[tabindex=0]` holding
`div.multi-select-pill-content > span` and a `div.multi-select-pill-remove-button`
(lucide-x). The `date` row also showed a visible 22×22 `clickable-icon`
labelled "Open daily note" (lucide-link) inside its value area, with the
daily-notes core plugin on. It is not tied to today's date: `Past Date.md`'s
`due: 2020-03-04` row drew the same icon at the same size and x.

Key-column icons, one per type: text `lucide-text`, number `lucide-binary`,
checkbox `lucide-check-square`, date `lucide-calendar`, tags `lucide-tags`,
aliases `lucide-forward`. `multitext` is `lucide-list`, seen only on the All
properties row for `cssclasses`; a key whose value is a nested map (`shelf`)
gets `lucide-file-question` there.

The two views and their five commands:

| Id | Name |
| --- | --- |
| `properties:open` | Properties view: Show all properties |
| `properties:open-local` | Properties view: Show file properties |
| `editor:toggle-fold-properties` | Toggle fold properties in current file |
| `markdown:add-metadata-property` | Add file property |
| `markdown:clear-metadata-properties` | Clear file properties |

The last three are `checkCallback` commands; the two view commands are plain
callbacks. The registered view types are `all-properties` and
`file-properties`. `properties:open-local` created a 300 px-wide
`file-properties` leaf in the right split, shrinking the main leaf from 680 to
380 — identically in both runs.

The File properties pane, titled "File properties", draws the rows with **no**
"Properties" heading and ends with
`div.metadata-add-button.text-icon-button` reading "Add property"
(lucide-plus). (The CDP run's dump of the pane while its tab was hidden shows a
`div.metadata-properties-heading` inside the pane's `metadata-container`, so the
heading is in the markup and its absence is a matter of style; whether CSS
hides it was not measured.) Its key inputs are editable in every row. Its
"More options" menu holds only "Split right" (disabled) and "Split down" —
nothing property-specific. For a Markdown note with no frontmatter the pane
shows zero rows and its visible text is just "Add property" (a "No properties
found." element — `div.pane-empty`, `display: none` — exists in the DOM but is
not in that text); for a `.canvas` file the pane's text *is* "No properties
found." with no rows and no "Add property". The view stays bound to the file in
both cases.

**The pane's text drops input-valued widgets.** A note with `status: shipped`,
`zebra: beta-note` and `count: 7` renders pane text
`"shipped\nbeta-note\nAdd property"` — the number lives in `input.value`, as do
checkbox and date. Any fixture built from these strings silently loses them.

**Editing in the reading view**, from the CDP run. Clicking a text value opens
a field in place: `document.activeElement` becomes that row's
`div.metadata-input-longtext` — contenteditable, inside
`.markdown-reading-view`, no sidebar involved — with a collapsed caret where
the click landed: at the end of the 10-character `alpha-note` when the click
fell past its text, at offset 35 of the 97-character `long value` when it fell
on the text. Typing changes the field and nothing else: 200 ms after
` edited` was typed the file still read `zebra: alpha-note`. Clicking the note
body commits: at the 400 ms read the file held `zebra: alpha-note edited`, and
the 5 s read was identical. The line diff against the fixture is exactly one
line — unquoted, `link: "[[Note B]]"` keeps its double quotes, key order,
indentation and the trailing newline unchanged. Nothing in the run shows a
write before blur; the samples are 200 ms after typing and 150 ms before
Escape, so a longer debounce that blur or Escape pre-empts is not excluded.

Keys inside an open value field:

| Key | Field | Focus afterwards | File |
| --- | --- | --- | --- |
| Enter, on the untouched `long value` | no newline; text unchanged | the row's `div.metadata-property`, selection `rangeCount` 0 | unchanged |
| Escape after typing `zzz` | the `zzz` is gone, the committed text is back | the row container, `rangeCount` 0 | unchanged — still `zebra: alpha-note edited` |
| Tab after typing `qqq` | committed | the **next** row's `div.metadata-property` (`alpha`), `rangeCount` 1 — not its key input, not its value, not out of the table | `zebra: alpha-note editedqqq`, one line changed |
| Ctrl+Z twice, focus on that row container | — | stays on `alpha`'s container | unchanged; `editedqqq` still on disk and in the row after a body click |

Enter was only pressed on an untouched field, so it is known not to insert a
newline and to leave the field; that it commits a *modified* value is inferred
from Tab, not observed. Escape on the row container afterwards did nothing
visible. Ctrl+Z was pressed in reading mode with no editor focused: undo from a
property row does not reach a committed edit.

The key input is live in the reading view too. The run's clicks on `count` and
`due` landed on the key column (x 392–516 at that layout: the row's first
`input` is the key input), which turned the step into key renames. End, `9`
and a click elsewhere turned `count: 3` into `count9: 3` on the same line —
value and position kept, on disk at the 400 ms read after the blur. Ctrl+A and
`2026-10-20` in the `due` key input gave the unquoted line
`2026-10-20: 2026-10-18`, again in place, committed by the next click away.
So a UI rename keeps the entry where it is — unlike the delete-then-set rename
through `processFrontMatter` in [[#Observed properties behavior, Obsidian 1.14.4]],
which moves the key to the end. Two oddities on the way: the first click at
the `due` key input's left edge left focus on the *previous* row's
`input.metadata-input-checkbox` (`done`, 16×16 at x 524), and ArrowRight then
ArrowUp on that focused checkbox left it checked and the file at
`done: true`; a second click at the same point reached the `due` key input.
Why focus went there is unobserved.

Removing a tag writes the list back in block form. Hovering the `one` pill and
clicking its remove button dropped the pill at once; after a body click the
file read `tags:\n  - two` with every other line as before — no collapse to
`[two]`. Whether the removal writes on its own or on the blur was not
separated. (The click meant to add a tag left `body` as the active element, no
suggestion popup appeared and the typed `three` reached nothing, so adding is
unobserved.)

**Add property**, clicked in the in-document block, appends a blank row —
`div.metadata-property[data-property-key=""]` with
`span.metadata-property-icon[aria-disabled="true"]` (lucide-text), an empty key
input and a `text` value — focuses its key input and opens
`div.suggestion-container.mod-property-key` over it. The popup lists registry
keys the note does not already have, each a `suggestion-item.mod-complex` with
the type icon in `span.suggestion-flair` and the name in
`div.suggestion-title`, first item `is-selected`: `cssclasses`, `book.title`,
`book.year`, `shelf`, `count`, `due`, `other` — neither alphabetical nor by
count, and the ordering rule was not found. After typing `demo` the popup was
gone (no "create" row; sampled 400 ms later). Enter on the key writes at once:
the file gained `demo:` — bare colon, no `""` — as the last line of the block,
and focus moved into the new row's value field (an empty
`div.metadata-input-longtext`). Typing `hello` and clicking the body rewrote
that line as `demo: hello`; nothing else in the block changed. A second blank
row left with an empty key and clicked away wrote nothing — no `:`, no `""` —
and the row disappeared from the table.

**The type icon's menu.** Clicking the glyph left of a key opens a `div.menu`
at the icon: "Property type" (lucide-info, `has-submenu`, chevron-right); a
group with `data-section="clipboard"` — Cut (lucide-scissors), Copy
(lucide-copy), Paste (lucide-clipboard-check); two consecutive
`div.menu-separator`s; then "Remove" (`is-warning`, lucide-trash-2,
`data-section="danger"`). So a row's remove affordance lives in this menu, and
hovering a row adds nothing to its markup. The submenu was never expanded and
nothing was picked, so the type list, what a retype writes and what Remove
writes stay open. None of the run's edits created `.obsidian/types.json`, and
the type manager's raw map stayed null before and after the restart.

**Folding.** Clicking the heading's `collapse-indicator` folds the block:
`is-collapsed` is added to `div.metadata-container`,
`div.metadata-properties-heading` and the indicator, `div.metadata-content`
goes `display: none` and the container's text shrinks to "Properties".
`editor:toggle-fold-properties` runs in reading view (`executeCommandById`
returned `true` with the preview leaf active) and toggles the same fold: run
once it unfolded, run again it folded. The fold is per file and outlives the
view: Note B opened unfolded, Note A came back folded, and after a scripted
quit and relaunch Note A was restored in preview mode with the block folded.
The record is the one heading folds use
([[#Observed heading fold behavior, Obsidian 1.14.4]]): the `localStorage` key
`<vault id>-note-fold-<path>`, here `e1e76222977ba271-note-fold-Note A.md`,
value `{"folds":[{"from":0,"to":0}],"lines":20}` — the properties fold is the
range `0`–`0`, the opening fence line, and `lines` is 20 for the 19-line note
plus the empty line after its final newline. `app.json` was unchanged 10 s
after the fold (`{}`), and `workspace.json`, though rewritten by the note
switch (5439 → 5458 bytes), holds no fold state. `localStorage` is shared
across vaults and namespaced only by the id: the scratch vault's store also
listed `5c0r1ac0rpu50001-note-fold-Fold Probe.md` and
`5c0r1ac0rpu50001-file-explorer-unfold` from another vault.

At the 380 px main leaf the reading view's rows stayed 29 px tall at a 304 px
block width — key and value did not stack there.

The All properties pane holds one `div.tree-item` per registry key, each with
`div.tree-item-self tappable is-clickable`, a type icon, the display name and a
count. With `sortOrder: "frequency"` the plugin run's order was `tags`6, `STATUS`3,
`title`3, `zebra`3, `count`2, `aliases`1, `alpha`1, `done`1, `due`1, `link`1,
`long value`1, `other`1, `cssclasses`0 — counts merge casings, and
`cssclasses` is listed at zero. Its sort menu is "Property name (A to Z)",
"Property name (Z to A)", a separator, then "Frequency (high to low)"
(`mod-checked`, with the check icon) and "Frequency (low to high)". Its nav
header has "Change sort order" (lucide-sort-asc) and "Show search filter"
(lucide-search); the latter's target is already in the markup as a hidden
`div.search-input-container` holding
`input[type=search][enterkeyhint=search][spellcheck=false][placeholder="Search..."]`
and `div.search-input-clear-button[aria-label="Clear search"]`, so it reveals
an in-pane field rather than a popup. Clicking the `tags` row made the
already-open search leaf the active one at 300 px with 6 results; a
`searchQuery` of `["tags"]` was read back.

The CDP run filled the pane in. The fresh vault's default workspace already
held a hidden `all-properties` leaf in the right split; `properties:open`,
with the right sidebar open at 300 px and the File properties tab active
there, activated it in that same tab group at x 724, 300 px wide — the
sidebar's existing width, not one the command chose. Its tab header is
`div.workspace-tab-header[data-type="all-properties"][aria-label="All properties"]`
(lucide-archive), and its right-click menu is Close, Pin, Link with tab...,
Move to new window, Split right (disabled), Split down. Closing the tab and
running `properties:open` again created a new leaf in the same place. Rows
are flat: `div.tree-item > div.tree-item-self.tappable.is-clickable` (inline
`padding-inline-start: 24px !important`), `div.tree-item-icon`,
`div.tree-item-inner > div.tree-item-inner-text`, `div.tree-item-flair-outer >
span.tree-item-flair` with the count; each row 25 px tall at x 736, 276 px
wide, 27 px apart, and no `tree-item-children` anywhere. Dotted keys
`book.title` / `book.year` are two depth-0 rows with their own types (text,
number); the nested map `shelf` is one depth-0 row with `lucide-file-question`.

Frequency order (18 rows, before the rename): `Status` 3, `zebra` 3,
`2026-10-20` 1, then `aliases`, `alpha`, `book.title`, `book.year`, `count`,
`count9`, `demo`, `done`, `due`, `link`, `long value`, `other`, `shelf`, `tags`
at 1, `cssclasses` 0. Ties run in ascending name order with digits first;
`Status`/`zebra` agree under both case rules, so the tie-break's case
sensitivity stays open. Picking "Property name (A to Z)" (after the rename)
gave `2026-10-20`, `aliases`, `alpha`, `book.title`, `book.year`, `count`,
`count9`, `cssclasses` (0, in place), `demo`, `done`, `due`, `link`,
`long value`, `other`, `shelf`, `state`, `Status` (2), `tags`, `zebra` (3):
case-insensitive (`state` before `Status`), digits first, zero-count keys not
segregated. The choice is view state, not a vault setting: the leaf's
`getViewState().state` went from
`{"sortOrder":"frequency","showSearch":false,"searchQuery":""}` to
`sortOrder: "alphabetical"`, `workspace.json` carried it inside that leaf's
`state` within 6 s and `app.json` did not change; a leaf created later started
at `frequency` again.

Rename: the row's right-click menu is Rename (lucide-edit-3,
`data-section="action"`), "Property type" (lucide-menu, `has-submenu` — the
icon menu's twin uses lucide-info), a separator, "Delete property"
(`is-warning`, lucide-trash-2, `data-section="danger"`). Rename is in-pane: the
`tree-item-self` gains `has-focus is-being-renamed`, its
`div.tree-item-inner-text` becomes `contenteditable="true" spellcheck="true"`
and holds a non-collapsed selection (anchor 0, focus 1 — the whole name or its
first character; the sample cannot tell, and the run pressed Ctrl+A before
typing); no modal and no scope dialog appeared at any point. Typing `state`
and Enter rewrote **only Note A**, `Status: draft` → `state: draft` in place,
one line changed; `Note B.md` (`status:`) and `Note C.md` (`STATUS:`) were
byte-identical before and after. Whether that is current-file scope or a
case-sensitive match on `Status` the data cannot say — both fit, and the
view's `propertyRenameScope` read back only as an object. The reading view's
row followed at once (`state`, `draft`). The pane then listed `zebra` 3,
`Status` 2, …, `state` 1: the merged entry kept the display name `Status` with
no file holding `Status:` any more — the casing is fixed when the key is first
indexed and not recomputed on a rename; after the relaunch the same entry read
`status` 2, indexed afresh from `status:` and `STATUS:`.

Search filter: clicking "Show search filter" shows the
`div.search-input-container` (x 732, 284×30) and focuses its
`input[type=search]`; typing `zeb` filtered the list live to the single
`zebra` 3 row; Escape hid the container, put all 19 rows back and left focus
on `body`.

**Obsidian rewrites `.obsidian` files it did not change.** The fixture wrote
only `app.json`. At launch Obsidian rewrote it byte-identical (`{}`, mtime
moved) and created `appearance.json` (`{}`), `core-plugins.json` (666 bytes)
and `workspace.json`; on the relaunch all four were rewritten with unchanged
sizes (`app.json` …847403 → …880304, `appearance.json` …847403 → …880305,
`core-plugins.json` …761193 → …879846, `workspace.json` …875554 → …880394).
Byte-identical mtimes are not something Obsidian itself provides, so 's
stricter rule is not needed for compatibility — only for not being the thing
that changes a file.

`Plain.txt`: with `showUnsupportedFiles` false the explorer DOM still holds its
`nav-file-title[data-path="Plain.txt"]`, but with a zero rect — hidden — so
the click on it was skipped.

`app.metadataTypeManager` keys every property by its **lower-cased** name and
keeps one display name per key: `Status`, `status` and `STATUS` across three
notes give the single entry mapKey `status`, name "STATUS". Its
`registeredTypeWidgets` are `aliases`, `checkbox`, `date`, `datetime`, `file`,
`folder`, `multitext`, `property`, `number`, `tags`, `text` — there is no
`cssclasses` widget, and `cssclasses` is carried as `multitext`. None of the
inferred widgets is persisted: the manager's own raw types map is null and
`.obsidian/types.json` was absent before and after both runs;
`getPropertyInfo('zebra')` read `{name: "zebra", widget: "text"}` throughout.

Layout, from one session: the sidebar's key column held a fixed width beside
the value at pane widths 300 and 280, and at 180 the key and value **stack**,
each taking the full row width and the row growing from 29 px to 54 px. The
relevant variables are `--metadata-label-width: 9em` (computed 135 px, `flex:
0 0 auto`) and `--metadata-input-longtext-lines: 3`. That session's metadata
variables were visibly customized (`--metadata-property-radius: 0px`, a
`color-mix()` label background, a non-default font stack), so the pixel
constants are this vault's, not necessarily Obsidian's defaults; the stacking
behavior itself is not theme-dependent in any way the run could show.

**Unprobed:** the non-text widgets' writes — the number input (`count`'s
click hit the key column), the checkbox (`done`'s too; arrow keys on a focused
checkbox did not toggle it, but a click on the box itself was never made) and
the date input (quoted or not once changed) — and adding a tag (the click on
the pill input left `body` focused, no suggestion popup appeared and `three`
reached nothing). Enter in a *modified* value field (only an untouched one was
tried). Undo from inside the body editor after a property commit (Ctrl+Z was
pressed on a row container in reading mode). Why the first click at the `due`
key input focused the `done` checkbox, and whether a key rename moves focus
into the row's value. Whether a pill removal writes on its own or on the next
blur. Right-click menus on a row, in the reading view and in the File
properties pane: both CDP right-clicks captured empty menus
(`Input.dispatchMouseEvent` with the right button may not raise `contextmenu`
in Electron), so the only row menu seen is the icon's, and any
reading-vs-sidebar difference is open. "Remove" from that menu and the bytes
after it; the "Property type" submenu's items and order; what
`zebra: alpha-note editedqqq` becomes when retyped as Number; Cut, Copy and
Paste on a row — none was picked. The first shape `types.json` takes from a UI
retype (its API-written shape is in
[[#Observed properties behavior, Obsidian 1.14.4]]). The ordering rule of the
key-suggestion popup, and what it offers on a partial match (it closed on
`demo`, which matched nothing). The Settings dropdown path for
`propertiesInDocument` (the run used `setConfig`). The All properties rename's
scope — current file or case-sensitive match. Whether the File properties
pane's heading element is hidden by CSS. Whether the fold record is written on
unfold as well as on fold (the key was first read after folding), and where
`properties:open` puts its leaf when the right sidebar is collapsed. Pane
geometry on a default theme beyond the row rects above. What clicking
`Plain.txt` does and what the File properties pane shows for it. Whether the
frequency tie-break is case-sensitive and whether keys other than `cssclasses`
list at count 0 (this fixture discriminates neither). Whether `types.json`
entries and the reserved keys `tags`/`aliases` match case-insensitively (no
type assigned, no `Tags:`). Which casing wins when a key is first indexed
(`Status` in the run, `status` after the relaunch with `Status:` gone — "first
file indexed" fits but is unproven). `markdown:add-metadata-property` and
`markdown:clear-metadata-properties` were not run; and the exact string the
search box receives from a property row was not re-read (`["tags"]` from the
plugin run stands).

## Observed auto-pair behavior, Obsidian 1.14.4

Observed on 2026-10-18 with a throwaway plugin in a scratch vault, against
Obsidian 1.14.4 (installer build 1.13.7) — those are the API and installer
versions the plugin read back; the running app version came back `null` and
was not observed. Typing is `document.execCommand("insertText")` on the
CodeMirror contentDOM, the one JS path that reaches the input handler; the
run proved the path first (typing `(` on an empty note gave `()`) and every
typing row took it. **Backspace and undo rows are different**: they are
synthetic, untrusted `KeyboardEvent`s, so they record what CodeMirror's
keymap and Obsidian's handlers did together, with no native editing
fallback behind them.

Both toggles read back `true` when the run opened. The run then flipped
them: all four combinations of `autoPairBrackets` and `autoPairMarkdown`
were typed through on an empty document, and the captured starting values
were restored at the end (nothing was restored from a guessed default).

**The two settings own disjoint character sets**, on an empty document with
the cursor at 0:

| Typed | `autoPairBrackets` | `autoPairMarkdown` | Closes when |
| --- | --- | --- | --- |
| `(` `[` `{` | `()` `[]` `{}` | — | `autoPairBrackets` |
| `"` `'` | `""` `''` | — | `autoPairBrackets` |
| `` ` `` `*` `_` | — | ` `` ` `**` `__` | `autoPairMarkdown` |
| `<` `~` `=` `$` `%` | — | — | never |

Each set follows its own toggle and ignores the other's; with both off
nothing closes. `<`, `~`, `=`, `$` and `%` never close under any
combination, and typing a second one just adds the character (`~~`, `==`,
`$$`, `%%`). The membership claim is bounded by the thirteen characters
probed — no other quote or bracket form was tried.

**A second identical character nests for brackets and skips for the rest.**
`(` `(` gives `(())` with the cursor at ch 2; `"` `"` leaves `""` and only
moves the cursor to ch 2. The same skip holds for `'`, `` ` ``, `*` and
`_`.

**Closing depends on the characters either side of the cursor**, not on the
setting alone. With both settings on, seven characters were typed in five
positions:

| Cursor | `(` `[` `{` | `"` `'` `` ` `` `*` |
| --- | --- | --- |
| `|word` — word char ahead | no (`(word`) | no (`"word`) |
| `don|t` — inside a word | no (`don(t`) | no (`don't`) |
| `word|` — after a word char, end of line | closes (`word()`) | no (`word"`) |
| `say |` — after a space, end of line | closes (`say ()`) | closes (`say ""`) |
| `(|)` — before a `)` | closes (`(())`) | no (`(")`) |

So brackets close unless the next character is a word character, and the
quote and markdown characters were seen to close only where the preceding
character is a space or the start of the document and the line ends at the
cursor. Two mid-line controls sharpen that: in `para| text here` (word char
before, space after) `(` closes to `para() text here` while `*` does not
(`para* text here`), and in `x =| 1` (`=` before, space after) `(` closes
and `*` does not — a preceding `=` suppresses markdown pairing even though
it is not a word character. Two collapsed cursors on `alpha` / `beta` at ch
0 each got a bare `(` for the same reason: `(alpha` / `(beta`.

**A fenced code block and a `$$` math block do not suppress bracket
pairing**: `let x| = 1` inside a ```js fence gives `let x() = 1`, and
`x =| 1` inside `$$` gives `x =() 1`, the same as the plain-paragraph
control.

**Frontmatter follows the same rule, with no YAML special case.** In source
mode at `title: |`, `"`, `'`, `[` and `*` all close (`title: ""`,
`title: ''`, `title: []`, `title: **`), and on the body line `|body text`
`"` does not. In Live Preview the YAML is replaced by the properties widget
and is not CodeMirror-editable text, so these rows are source mode only.

**Typing a closer over an auto-inserted one moves the cursor instead of
inserting.** `(|)` typed `)` leaves `()` with the cursor at ch 2. It still
skips after an unrelated edit — typing `(`, then `q` after the pair, then
coming back inside, still skips. Typing `)` at `x|)`, where the `)` was
planted rather than inserted by the editor, writes `x))`; that document has
no `(` anywhere, so what decides — provenance or an unmatched opener — is
not settled (see Unprobed).

**Backspace between an empty pair deletes both characters**, whether or not
the editor inserted the pair: an auto-inserted `(|)` and a planted `(|)`
both leave an empty document, against a control where `pla|in` loses one
character. With content inside, `(|x)` leaves `x)` — only the opener goes.
`*|*` also leaves an empty document, and `**|` leaves `*`; the run does not
record whether that `**` was typed or planted, so the "whoever inserted it"
half is established for brackets only.

**With a selection, every pair character wraps it** and keeps the selection
over the inner text: `alpha` selected and `(` typed gives `(alpha) beta`
with `alpha` still selected. This includes `~`, `=`, `$` and `%`, which
never close at a bare cursor. `<` is the exception — it replaces the
selection (`< beta`). Wrapping is per range: two selections and one `(`
give `(alpha)` and `(beta)`, both selections kept.

**`[` twice gives `[[]]`** with the cursor at ch 2, and the file suggester
opens there (70 items, four registered suggesters). With `autoPairBrackets`
off the same two keys give `[[` and the suggester still opens, so the popup
does not depend on the closers. Sending Escape left the document `[[]]`,
unchanged.

**One undo removes the opener and its auto closer together**: typing `(`
at `seed|` gives `seed()`, and one undo gives `seed`. After skipping over a
closer the same single undo reaches `seed`, so the skip adds no undo step
of its own. The probe left 600 ms between keystrokes, so each keystroke was
its own CodeMirror history group, and the second undo in each case
re-entered the probe's own `setValue` seeding — those snapshots are noise.

**The closer is ordinary text.** Typing `(` then `a` and running save left
the file reading `(a)` (char codes 40, 97, 41, decimal). That is a decoded
string rather than a byte read; the file's prior content and whether a
trailing newline survived were not recorded.

**Unprobed:** skip-over for any closer but `)`; whether the auto-inserted
pair survives a cursor round trip (the step recording it kept no
before-state); word-boundary behavior for `_`, `~`, `=`, `$`, `%` and `<` (only `(`, `[`,
`{`, `"`, `'`, `` ` ``, `*` were run through the five positions); which
half of the markdown rule fails — preceding whitespace specifically, or end
of line ahead; whether inline code or a blockquote suppresses pairing (a fenced
code block does not; both earlier rows put the cursor before a word
character, which already suppresses it everywhere) and likewise whether any
context suppresses markdown pairing;
whether the "source mode" rows really ran in source mode — they are
byte-identical to the Live Preview rows, but nothing reads the mode back
after the switch, and the Live Preview rows' own validity check (requested
cursor versus `cursorBefore`) cannot be run because the requested positions
were not recorded; whether the `**` in the backspace rows was typed or
planted; whether Escape closes the link popup (no popup state was captured
at that moment); what typing `]` inside `[[|]]` does; whether an abandoned auto
closer is ever removed; how a fast-typed `(` plus following text groups for
undo; whether both toggles are Obsidian's factory defaults (they were read
from the API in a scratch vault, not from a pristine `app.json`); and
behavior under Vim mode, on mobile, or with an IME composition in flight.

## Observed attachment paste behavior, Obsidian 1.14.4

Observed on 2026-10-18 with a throwaway plugin in a scratch vault, on Obsidian
1.14.4 (installer build 1.13.7): synthetic `paste` events carrying a `File`
item were dispatched on the CodeMirror content element, and the vault listing,
the editor text, the bytes written and the bytes on disk were read back around
each one. Twenty-three pastes over seventeen steps, every step reporting `ok`.
The vault's baseline was `attachmentFolderPath: "/"`, `newLinkFormat:
shortest`, `useMarkdownLinks: false`, `alwaysUpdateLinks: false`, and the
naming, collision, format, cursor and undo steps all ran at that baseline, so
their files land at the vault root.

**Naming.** A pasted image is saved as `Pasted image ` + a 14-digit
**local**-time stamp `yyyyMMddHHmmss` + the extension. The run's clock
separated the two readings (local `20261018205534`, UTC `20261019015534`,
`America/Chicago`) and the file took the local form.

Which clipboard items get that name, and which keep their own:

| Clipboard item | Written as |
| --- | --- |
| `image/png`, clipboard name not recorded | `Pasted image 20261018205534.png` |
| `image/png`, clipboard name `image` (no extension) | `Pasted image 20261018205535 1.png` |
| `image/jpeg`, clipboard name `image.jpg` | `Pasted image 20261018205535.jpg` |
| `image/webp`, clipboard name `image.webp` | `image.webp` |
| `image/gif`, clipboard name `image.gif` | `image.gif` |
| `text/plain` file item, clipboard name `notes.txt` | `notes.txt` |

The run does not separate "the item's own name is ignored" from "the item's
name carried no usable extension": the only png whose clipboard name was
recorded was called `image`, and the jpeg's name already ended `.jpg`. Every
one of these, the plain-text file included, was inserted as an embed
(`![[notes.txt]]`).

**Collisions** append a space and an ascending integer before the extension,
and the comparison is against the whole file name, extension included:
`Pasted image 20261018205535.png` and `Pasted image 20261018205535.jpg` both
exist unsuffixed, while a second png in that same second became `Pasted image
20261018205535 1.png`. Three pastes inside one wall-clock second gave ` 1`,
` 2`, ` 3`. `fileManager.getAvailablePathForAttachment` resolves the same way
and continues past existing suffixes (asked for a name whose ` 1`–` 3` were
taken, it returned ` 4`); it returns a free name, a dotted name and a name
with no dot unchanged, and creates no file.

**Bytes.** For all five formats the written byte count equals the source byte
count (png 68/68, jpeg 160/160, webp 44/44, gif 42/42, text 24/24) and the
first sixteen bytes carry each format's own magic (`89 50 4e 47 …`,
`ff d8 ff e0 … JFIF`, `RIFF…WEBP`, `GIF89a`). No hash or byte-for-byte
comparison was recorded and the fixtures are 24–160 bytes, so this is evidence
against re-encoding rather than proof of its absence.

**Insertion.** The embed goes in at the cursor with no padding added on either
side (`BEFORE ![[…]]AFTER`). Over a selection it replaces it (`BEFORE `
selected gives `![[…]] AFTER`). With two cursors one paste writes **one** file
and inserts the same link at both (`one![[…]]\ntwo![[…]]`), leaving two
selections. Right after a paste the note was on disk with exactly the editor's
text — 49 bytes, trailing newline kept.

One undo put the text back to `BEFORE AFTER` in a single step and left the
attachment alone: the file stayed in the vault index through every entry of
the undo trace and was still listed on disk at 68 bytes at the end of the run.
The trace's later entries show another step's buffer, so only the first undo
is clean evidence of the step size.

A clipboard carrying only `text/plain` and no `File` item creates nothing:
`/home/user/pictures/snap.png` was inserted verbatim. That is a path-shaped
clipboard, not an OS file-manager copy.

Every paste that recorded the DOM event came back `defaultPrevented` — the
text-only one that wrote no file included — so the flag does not separate
Obsidian's handler from CodeMirror's own paste handling.

**Where the file lands.** `fileManager.getAvailablePathForAttachment` for each
`attachmentFolderPath` and two source notes (API only, no file written):

| `attachmentFolderPath` | Source note | Path returned |
| --- | --- | --- |
| `""` or `/` | `Target.md`, `Sub/Nested.md` | vault root |
| `Attachments` | `Target.md`, `Sub/Nested.md` | `Attachments/` |
| `./att` | `Target.md` | `att/` |
| `./att` | `Sub/Nested.md` | `Sub/att/` |
| `Sub` | `Target.md`, `Sub/Nested.md` | `Sub/` |
| `Blocker.md` (an existing **file**) | `Target.md`, `Sub/Nested.md` | vault root |

Three settings were confirmed by a real paste, each from a single source note:
`Attachments` from `Target.md` wrote `Attachments/…`, `./att` from
`Sub/Nested.md` wrote `Sub/att/…`, and `Blocker.md` from `Target.md` wrote to
the vault root. The rest of the table is API output only.

**The link a real paste writes**, from `Sub/Nested.md` to a uniquely named
file in `Attachments/`:

| `useMarkdownLinks` | `newLinkFormat` | Inserted |
| --- | --- | --- |
| false | `shortest` | `![[Pasted image 20261018205538 1.png]]` |
| false | `relative` | `![[../Attachments/Pasted image 20261018205539.png]]` |
| false | `absolute` | `![[Attachments/Pasted image 20261018205539 1.png]]` |
| true | `shortest` | `![](Pasted%20image%2020261018205539%202.png)` |
| true | `relative` | `![](../Attachments/Pasted%20image%2020261018205539%203.png)` |

`absolute` carries no leading slash. A markdown link has **empty** link text
and its spaces encoded as `%20`; no name in the run held any other character
whose encoding would show. `app.fileManager.generateMarkdownLink` over the
same file returns these same forms without the leading `!`.

`shortest` is not "the bare name unless the basename is ambiguous". Three real
pastes wrote the same base name into three folders, in this order:
`Attachments/Pasted image 20261018205538.png` from `Target.md` gave
`![[Pasted image 20261018205538.png]]`; then the same name under `Sub/att/`
from `Sub/Nested.md` gave the full path `![[Sub/att/Pasted image
20261018205538.png]]`; then the vault-root copy from `Target.md` gave the bare
name again, with the other two already in place. What the three agree on is
the shortest form that resolves back to *that* file from the source note.

**Notices.** The eight pastes that recorded a `notices` array recorded none —
including the paste whose attachment folder pointed at an existing file, and
the text-only paste — as did the final restore step. The other fifteen pastes
record no notice field at all.

At the end of the run, after the probe restored `attachmentFolderPath` to `/`,
`newLinkFormat` to `shortest` and `useMarkdownLinks` to `false`,
`.obsidian/app.json` held `{"foldHeading": true, "propertiesInDocument":
"visible"}` — none of the three keys present, while `propertiesInDocument` sat
there at its own default. How the restore wrote those three keys is not
recorded, so this does not settle whether setting a key to its default removes
it (see [[Specification#Observed settings write behavior, Obsidian 1.14.4]],
which observed the opposite through `vault.setConfig`).

**Unprobed:** whether a *paste* creates a missing attachment folder — the
folders already existed by the time the real-paste step ran — and whether the
API call is what created them: `Attachments`, `att` and `Sub/att` were folders
afterwards and `att` never received a file, but the run's "before" listing is
a file listing and cannot show a folder was absent. `attachmentFolderPath =
"./"` (Obsidian's "Same folder as current file") was never set, nor a deep
non-existent path. `useMarkdownLinks` with `absolute` was never pasted for
real, only generated by the API. No png with a real, distinct clipboard name
(`photo.png`) was ever pasted, so the rename rule for named png and jpeg items
is unsettled, as is the extension chosen for a `image/jpeg` item named `*.jpeg`
or unnamed. Nothing beyond png, jpeg, webp, gif and `text/plain` was pasted —
no svg, avif, bmp or pdf — and no attachment name contained `#`, `?`, `&`,
parentheses or non-ASCII, so only the space is known to be percent-encoded. A
real OS file-manager copy and drag-and-drop were not exercised; neither were
reading view, a canvas or any non-markdown view. Whether the note's save was
the paste's doing or the probe's is unrecorded, as is whether any further undo
ever removes the attachment, whether a Notice appears for the fifteen pastes
that recorded none, whether the `Pasted image` prefix is localized or
configurable, and what a collision across different folders does.
`alwaysUpdateLinks` was read (`false`) but never varied, and `appVersion` was
null, so only the API version 1.14.4 and installer 1.13.7 identify the build.

## Observed file explorer drag-and-drop and inline rename behavior, Obsidian 1.14.4

Observed on 2026-10-18 with a throwaway plugin in a scratch vault, Obsidian
1.14.4 (installer build 1.13.7), dispatching synthetic `DragEvent`s over real
explorer rows, calling the explorer view's own rename entry points, and reading
the vault back after every step. Every move below is backed by both the
`fileManager.renameFile` call the probe saw and the vault diff it read
afterwards. The run forced `alwaysUpdateLinks` on (it had been off) and left it
on; `newLinkFormat` was `shortest`, `useMarkdownLinks` off.

Every row in the file explorer, file and folder alike, carries
`draggable="true"` — all 37 rows present. The payload the row sets has the keys
`source, type, icon, title, file`:

| Dragged | `type` | `icon` | `title` | `file` |
| --- | --- | --- | --- | --- |
| `Root.md` | `file` | `lucide-file` | `Root` | `Root.md` |
| `Deep` (folder) | `folder` | `lucide-folder-open` | `Deep` | `Deep` |

So a file's `title` is its basename without the extension. Only `.md` files
were ever dragged.

Where a drop lands:

| Dropped on | Result |
| --- | --- |
| a folder row | the file moves into that folder (`Root.md` → `Dest/Root.md`) |
| a file row | the file moves into *that file's parent folder* (`Inbox/Gamma.md` dropped on `Notes/Beta.md` → `Notes/Gamma.md`) |
| the folder it already lives in | nothing: no `renameFile` call, no notice, no change on disk — though the hover highlight and the `Move into “Dest”` label still appear |
| `.nav-files-container` (the scrolling file list) | the file moves to the vault root (`Dest/Root.md` → `Root.md`) |
| a workspace tab header | accepted, but as a different action: the label reads `Open as new tab` and a `.workspace-drop-overlay` element appears |
| `.nav-header`, `.nav-buttons-container`, `.workspace-leaf-content`, `.workspace-drawer-vault-name` | nothing is accepted: no highlight, no new drag action |

The drag ghost is a `.drag-ghost` element holding the item's title plus an
action line, `Move into “Dest”`, in curly quotes. Over the root zone the name
in it was `probe` — which is both the vault's name and its folder name in this
run, so the two are not distinguished. A ghost with no action line is 31 px
tall against 46 px for one that carries one.

The hover highlight is the class `is-being-dragged-over`, and it lands on the
*container*, not on the row that received the event: on the outer
`.tree-item.nav-folder` while `.tree-item-self.nav-folder-title` is unchanged,
and on `.nav-files-container` itself for the root zone. A file row, although it
accepts the drop, takes no highlight anywhere — the ghost's action line is the
only affordance.

A name collision at the destination neither prompts nor refuses. `Notes/Alpha.md`
dropped onto `Dest`, which already held `Alpha.md`, moved in as `Dest/Alpha 1.md`
— space before the digit — leaving `Dest/Alpha.md` untouched and showing no
modal. That is one collision, not a series.

A move that changes the file's basename rewrites the links to it in `shortest`
form and shows a Notice reading exactly `Updated 1 link in 1 file.` — `Linker.md`
went from `- unrelated: [[Notes/Alpha]]` to `- unrelated: [[Alpha 1]]`. With
`alwaysUpdateLinks` on no prompt or modal appears at any point in the run. Moves
that leave the basename alone produced no notice at all, and the note read
immediately after `Root.md` moved to `Dest/Root.md` still said
`- markdown: [Root](Root.md)` and `- wikilink: [[Root]]`.

A folder dragged onto its own descendant (`Deep` onto `Deep/Sub`) did nothing:
no highlight, no action line on the ghost, no drag action recorded, vault
unchanged. No folder was dragged onto a *valid* folder anywhere in the run, so
this is not isolated from "folder drags offer no move under synthetic
`DragEvent`s" — only the vault-unchanged half is certain. The API behind it is
not guarded the same way: `fileManager.renameFile("Deep", "Deep/Sub/Deep")`
fails with a raw `EINVAL: invalid argument, rename '…/Deep' -> '…/Deep/Sub/Deep'`,
and leaves no partial state (`Deep/`, `Deep/Sub/`, `Deep/Sub/Leaf.md` all still
there), while `renameFile("Deep", "Deep")` is a silent no-op.

A collapsed folder hovered continuously during a drag spring-loads open between
the 700 ms and the 800 ms tick (collapsed with 1 child row at 700, expanded with
2 at 800). One run, sampled at 100 ms, with synthetic events.

Inline rename edits the row in place. The row's
`.tree-item-inner.nav-file-title-content` div becomes `contenteditable="true"
spellcheck="true"` and the row gains `is-active has-focus is-being-renamed`:

```
<div class="tree-item-self nav-file-title tappable is-clickable is-active has-focus is-being-renamed" data-path="blurme.md" draggable="true">
  <div class="tree-item-inner nav-file-title-content" contenteditable="true" spellcheck="true">blurme</div>
</div>
```

The editable holds the basename alone and the whole of it is selected when the
editor opens. For a non-markdown file the extension sits outside the editable in
a sibling `.nav-file-tag` div — `cover.png` renders as `cover` + `png` — and is
not part of the editable text.

Blur commits: `blurme.md` became `blurred.md`, extension intact, with the
editable closed afterwards. Escape abandons the edit, and a blur dispatched
after the Escape does not commit the discarded text (`escme.md` survived both).
The probe set the editable's text by a means it did not record, so how a typed
character reaches the commit path (`onFileRenameInput` versus plain
`textContent`) is not established by this run. The explorer view exposes the
whole state machine: `startRenameFile`, `onFileRenameInput`, `onTitleBlur`,
`saveRename`, `exitRename`, `onKeyEnterInRename`, `onKeyEscInRename`,
`onKeyRename`, `displayError`.

The command `workspace:edit-file-title` is present, is named `Rename file`, and
the registry declares F2 with no modifiers as its default (user hotkeys: none).
That is registry data: every F2 keystroke the run actually dispatched did
nothing, and all three inline renames were opened by calling
`view.startRenameFile` directly. Invoked with focus in a CodeMirror editor the
command made a title element editable with its whole basename selected and
renamed nothing by itself — but the two invocations, from the same
`cm-content` focus, produced two different elements: once `.inline-title`
(text `Linker`), once `.view-header-title` (text `Beta`). Which element the
command targets, and on what, is not settled by this run.

Ctrl+click on explorer rows opened files rather than selecting them: markdown
leaves went from 0 to 2, and afterwards the only row carrying an extra class was
`Dest/Keep.md` with `is-active`. The probe's read of the view's selection list
returned null (a failed read, not an observed empty), and the drag payload it
then captured was for a row it had not clicked, so nothing here shows whether a
selection travels with a drag. The API says multi-selection is modelled:
`dragManager` has `dragFiles` alongside `dragFile` and `dragFolder`, the
explorer view has `dragFiles` and `onDeleteSelectedFiles`, and every file and
folder item carries a `selectable` field.

Two recording caveats for anyone re-running this. The recorder double-records:
the one blur rename appears twice in `renameFileCalls` and the one link notice
twice in `notices`, so no count may be read off those arrays — only the text.
And the highlight detector watches class churn document-wide: one capture picked
up an unrelated `search-result-container … is-loading`, so a positive is not
automatically drag-related (the negatives are the stronger evidence). The scratch
vault's `alwaysUpdateLinks` was deliberately left forced on, which will bias a
re-run from the same vault.

**Unprobed:** whether F2 pressed with focus genuinely on an explorer row opens
the in-row editor, and what `workspace:edit-file-title` does with focus in the
explorer rather than the editor (the "explorer" context never took focus — the
active element was `cm-content` in both invocations); whether Enter commits an
inline rename (`onKeyEnterInRename` exists but no Enter was dispatched); what an
inline rename onto a name already taken does, and whether committing an inline
rename on a non-markdown file keeps its extension (the `cover.png` editor was
opened but never committed); inline rename of a *folder*, never opened at all;
whether a drag-move prompts before rewriting links with `alwaysUpdateLinks` off;
whether a markdown-style link's destination is rewritten on a move at all, and
after what delay, since the only read was immediate and the file later moved
back; whether `fileManager.renameFile` auto-numbers or errors on a collision
(the direct API check could not run — the drop had already moved the file); what
a drop on a file row does when the destination folder already holds that name;
whether a folder dropped on a valid, non-descendant folder moves at all under
these synthetic events, the control this run lacks; whether `.view-content`,
and a collapsed folder's children area, are drop zones (the `.view-content`
probe grabbed `.workspace-leaf-content` twice, and a planned `root-fileItem`
probe found no element); whether the explorer multi-selects under any other
modifier and whether `dragManager.dragFiles` can carry several paths; when the
`is-being-dragged-over` and `is-being-dragged` classes are torn down (one step
still carried both after its drop, four did not, and no `dragend` was ever
dispatched); whether dropping between rows reorders or inserts; what a drop on
an editor pane or another vault window does; and whether a drag-move offers an
undo path.

## Observed graph.json write behavior, Obsidian 1.14.4 (installer build 1.13.7)

Observed on 2026-10-18 with a throwaway plugin in a scratch vault (47 nodes, 15
links), polling `.obsidian/graph.json`'s bytes, size and mtime around each
interaction. One run, one vault, no restart; the version is the baseline step's
`appVersion` — the run's own `meta.appVersion` is null.

**Obsidian creates the file; an enabled plugin does not.** At baseline the graph
core plugin was found and enabled, and `.obsidian` held only `app.json`,
`appearance.json`, `community-plugins.json`, `core-plugins.json`,
`workspace.json` and a `plugins/` folder — no `graph.json`. Opening the global
graph view created it, once, with nobody touching the controls. It was present
at the first poll after the view opened, and the poller ran at roughly 120 ms
(14 polls over 1691 ms), so every latency below is a poll-granularity upper
bound, not a measurement.

The created file is a complete twenty-key document — never sparse — in this
order, with these values:

```json
{
  "collapse-filter": true,
  "search": "",
  "showTags": false,
  "showAttachments": false,
  "hideUnresolved": false,
  "showOrphans": true,
  "collapse-color-groups": true,
  "colorGroups": [],
  "collapse-display": true,
  "showArrow": false,
  "textFadeMultiplier": 0,
  "nodeSizeMultiplier": 1,
  "lineSizeMultiplier": 1,
  "collapse-forces": true,
  "centerStrength": 0.518713248970312,
  "repelStrength": 10,
  "linkStrength": 1,
  "linkDistance": 250,
  "scale": 1,
  "close": false
}
```

All four control sections start collapsed. Two-space indentation, one key per
line, no trailing newline — the last byte is `}` and the file is 494 bytes.
Numbers come out the way JavaScript prints them: integers with no decimal point
(`0`, `1`, `10`, `250`) and everything else at full round-trip precision, with
no fixed number of decimal places — `0.518713248970312` at fifteen significant
digits, and `5.0625` for a zoom. Later states measured 503, 498 and 497 bytes.

**The file is a serialization of the graph core plugin's own options object.**
The plugin instance's `options` holds exactly the same twenty keys in exactly
the same order as the file, and its prototype carries `saveOptions`,
`onExternalSettingsChange`, `openGraphView` and `openLocalGraph`. That accounts
for the fixed key order, the whole-document rewrite, and for no `local*` key
ever reaching the file.

Every observed file state carries those same twenty keys in that same order —
the creation write, the writes whose bytes were recorded afterwards, and every
later snapshot. One interaction changes exactly one value and leaves the other
nineteen untouched; nothing is appended and nothing is reordered. Whether a
write is atomic is not observable from size/mtime polling.

What wrote, and what it wrote:

| Interaction | Written | Seen |
| --- | --- | --- |
| Open the global graph view | the whole file, created | first poll (~120 ms grain) |
| Click the `Filters` section header | `collapse-filter` `true` → `false` | one write, ~300 ms after the click |
| Click `Groups` | `collapse-color-groups` `true` → `false` | one write, ~300 ms |
| Click `Display` | `collapse-display` `true` → `false` | one write, ~300 ms |
| Click `Forces` | `collapse-forces` `true` → `false` | one write, ~300 ms |
| Wheel zoom | `scale` `1` → `5.0625`, then a second write `5.0625` → `8` | one write each, see below |
| Click the panel's Close button | `close` `false` → `true` | one write, ~100 ms |
| Open a local graph | nothing | 0 writes in ~2.8 s |
| Detach every graph view | nothing | 0 writes in ~1.2 s |

Only the expand direction of a section header was exercised: every click went
`true` → `false`, and re-collapsing was not tried. The ~300 ms figures are
quantized by the same ~120 ms polling, so they say "a write landed about a third
of a second later", not "the debounce is 300 ms".

**Controls panel.** The panel is `div.graph-controls[data-ignore-swipe="true"]`.
Each section is `div.tree-item.graph-control-section` plus one of `mod-filter`,
`mod-color-groups`, `mod-display`, `mod-forces`, and expanding removes
`is-collapsed` from that div; the header div keeps `tree-item-self
mod-collapsible` unchanged. While a section is collapsed its setting rows are
absent from the DOM, not merely hidden — a query for setting-items under the
collapsed panel returned none. The panel has four
`div.clickable-icon.graph-controls-button`: `mod-close` ("Close"), `mod-open`
("Open graph settings"), `mod-animate` ("Start timelapse animation"),
`mod-reset` ("Restore default settings"). Clicking Close adds `is-close` to
`.graph-controls` and writes `"close": true`; the graph view itself stays open.

**Zoom.** Wheel zoom persists to `scale`, and both values that reached the file
match a wheel-tick target rather than the animating scale: `5.0625` is exactly
1.5⁴, and the second write was exactly `8` at a moment when the renderer read
`7.9319604902379455` at both samples taken around it. The renderer's `scale` was
never sampled at write time, so nothing here shows what the writer reads; what
it does not read is the animated value. The second write's mtime is exactly
2000 ms after the first; a pan drag was dispatched inside that window but moved
nothing (`panX`/`panY` identical before and after), so the second write cannot
be attributed to the drag, to the zoom settling, or to a periodic flush — the
three are indistinguishable in this run. The renderer's `targetScale` read `8`
at every sample after the wheel burst.

`graph.json` carries no pan or offset coordinates in any observed state, and the
key set never grows.

**Local graph.** Opening a local graph wrote nothing to `graph.json`. Its
options live in the workspace leaf state: `type: "localgraph"`, `state.file`
(`"Alpha.md"`), `state.options`. That option set is not `graph.json`'s — it
drops `showOrphans` and adds `localJumps` (`1`), `localBacklinks` (`true`),
`localForelinks` (`true`), `localInterlinks` (`false`). The renderer engine's
own live `options` object is a third, filter-only set: `showTags`,
`showAttachments`, `hideUnresolved`, `showOrphans`, `localFile`, `localJumps`,
`localInterlinks`, `localForelinks`, `localBacklinks`. The `local*` keys exist
in the engine and the leaf state and never appear in the file.

The local graph opened at a point when `graph.json` already held
`collapse-filter: false`, `scale: 8`, `close: true`; its own options read
`collapse-filter: true`, `scale: 1`, `close: false` — the values the file was
created with, and its four control sections were all collapsed while the global
panel's four had been expanded. So a local graph does not start from the current
`graph.json`. Whether it starts from built-in defaults or from a snapshot of the
plugin's options taken earlier in the session cannot be told from one local
graph in one session.

The view types are `graph` (title "Graph view") and `localgraph` (title "Local
graph"), both icon `lucide-git-fork`. `getViewState()` on the global graph leaf
reports an empty `state` object; no write to `workspace.json` was observed, but
the baseline for that file was taken after both leaves were already open.
Detaching every graph view wrote nothing further; the file kept the last state
written while a view was open.

**Unprobed:** whether a filter checkbox (`showTags`, `showAttachments`,
`hideUnresolved`, `showOrphans`) writes at all — the sections were still
collapsed, so no row existed to click; whether a slider writes on `input` or
only on `change`, and which key the probed slider owns — its five synthetic
frames ended on the slider's starting value and fired no `change` event, so the
zero writes prove nothing, and only `min -3 / max 3 / step 0.1` was captured
(that range matches the Display section's text-fade control, which makes
`textFadeMultiplier` the likely owner, unconfirmed); whether toggling a *local*
graph option writes anything — that sub-step silently did not run (`toggled`,
`boxKind` and both `boxClass` fields all null) even though the step reported
success; whether pan persists anywhere — the synthetic drag never moved the
renderer; re-collapsing a section (`false` → `true`); the search box,
`colorGroups` edits, `showArrow`, and the "Restore default settings" and
timelapse buttons, none of which were touched (`search` stayed `""` and
`colorGroups` `[]` throughout); the real debounce and whether writes are
periodic; whether a write is atomic (temp file plus rename) or in place; whether
`8` is a zoom clamp, and what the lower bound is; whether
`onExternalSettingsChange` actually makes Obsidian reload `graph.json` when
something else rewrites it, which the prototype suggests but the run never
tested; whether unknown keys in a hand-written `graph.json` survive, since the
file did not exist at baseline and Obsidian only ever rewrote its own; and
whether a local graph's options ever reach `workspace.json`. Every figure above
comes from a single ~14 s run against one 47-node scratch vault: the file shape
and key order are robust to that, the timings and the saturation at `8` are not.

## Observed search pane behavior, Obsidian 1.14.4

Two runs. The first, on 2026-10-18, used a throwaway plugin in a scratch
vault, on Obsidian 1.14.4 (installer build 1.13.7): the Search view was opened
through the `global-search` plugin's `openGlobalSearch`, then the view object,
its header elements, its result DOM, the menus its surfaces open, `.obsidian/`
and `localStorage` were read back. `Menu.showAtMouseEvent`,
`Menu.showAtPosition`, `Modal`, `navigator.clipboard` and Electron's
`clipboard` were all patched to record anything the pane did. **That run never
changed the sort order and never invoked the copy action.** The 2026-10-19
observations came from a CDP-driven run — real browser-injected keystrokes and
mouse clicks, with file bytes read from disk by the host
([[Development#Driving Obsidian over CDP, 2026-10-18]]) — on the same Obsidian
1.14.4 (installer build 1.13.7) in a fresh scratch vault of twelve fixtures:
eleven `.md` notes containing `needle` (`_needle.md`, `9 needle.md`,
`Apple Pie.md`, `apple.md`, `apple2.md`, `apple10.md`, `kiwi.md`, `mango.md`,
`zebra.md`, `notes/apple.md`, `notes/banana.md`) and `plain.txt`, every file
given a distinct mtime. Three bounds on that run: each sort switch was a
script-dispatched `change` event on the native `<select>` — ArrowDown on the
focused select changed nothing — which reached `setSortOrder` through the
dropdown's `changeCallback`; the copy modal's two selects were set the same way
(its toggle, the count control, the menu items and the buttons were real
clicks); and the query was empty by the time the toggle and keyboard steps ran,
so those steps settled state keys and handler sources, not visible effects.
A sentence below that names no date holds for both runs.

The view:

- Type `search`, icon `lucide-search`, display title `Search`; the command is
  `global-search:open`, "Search: Search in all files".
- Its serialized state is exactly six keys — `query`, `matchingCase`,
  `explainSearch`, `collapseAll`, `extraContext`, `sortOrder`. `getState` reads
  the first three off the view and the last three off the result-DOM object:
  `t.collapseAll=this.dom.collapseAll,t.extraContext=this.dom.extraContext,t.sortOrder=this.dom.sortOrder`.
  Sort order and the two display flags live on the result list, not on the view.
- In a vault where none was ever chosen, `sortOrder` is `alphabetical` and all
  four flags are `false`. A never-searched leaf already serializes all six:
  `.obsidian/workspace.json` read right after launch, before any search, held
  `"query": "", "matchingCase": false, "explainSearch": false, "collapseAll": false, "extraContext": false, "sortOrder": "alphabetical"`
  for the search leaf, in that order — the keys are emitted even at their
  defaults — and the file-explorer leaf uses the same
  `"sortOrder": "alphabetical"` string.
- `setSortOrder` is `function(e){var t=this.dom;t.sortOrder=e,t.changed()}`: it
  assigns `dom.sortOrder` and calls `dom.changed()`, and nothing else.
  `dom.changed()` is a generic debounce wrapper; its inner body was not captured
  in either run.
- The result list is virtualized — the DOM object carries `infinityScroll`,
  `vChildren`, `resultDomLookup`, `focusedItem`, `pusherEl`, `emptyStateEl`,
  `showingEmptyState` and `working`.
- The view's container holds, in order, `search-row`, `search-info-container`,
  `search-params`, `search-results-info` and
  `search-result-container mod-global-search`. The sort control is a native
  `<select class="dropdown">`, not a menu, and it sits in the
  `search-results-info` bar beside the result count — not in the settings
  section, which holds only the three toggles. (The bar's text content is the
  count, the six option labels and the current label once more, the last from
  the hidden `is-measuring` twin select the dropdown component sizes itself
  with.) The view owns `sortOrderDropdown`, `collapseResultsToggle`,
  `extraContextToggle`, `explainSearchToggle`, `filterSectionToggleEl` and
  `searchParamsContainerEl`, with `toggleFilterSection` on the prototype.
  `onPaneMenu` is `function(e,t){}`, an empty body.
- Other prototype surface that bounds the feature: `saveSearch` (with
  `requestSaveSearch` on the instance), `startSearch`/`stopSearch`,
  `setQuery`/`getQuery`, `renderSearchInfo`, `onCopyResultsClick`,
  `onTabHeaderClick`, and the focus-key handlers `onKeyEnterInFocus`,
  `onKeyArrow{Up,Down,Left,Right}InFocus`, `onKeyShowMore{Before,After}`.
- `startSearch` (source, not driven) calls `stopSearch`, empties the result
  list and clears the focused item, compiles the input with the `matchingCase`
  flag (`new NL(n,o,this.matchingCase)`) and returns early when that yields no
  matcher, re-renders the search-info block, feeds each file through
  `addResult`/`removeResult`, sets the count from
  `labelResultCount({count: dom.getMatchCount()})` when its queue stops, and
  ends with `app.workspace.requestSaveLayout()` then `requestSaveSearch()` —
  every query run schedules a layout save and a recent-searches save.

The header holds three elements:

| Element | Class | Icon | Label |
| --- | --- | --- | --- |
| Match case | `input-right-decorator clickable-icon` | `uppercase-lowercase-a` | `Match case` |
| Search settings | `clickable-icon` | `lucide-sliders-horizontal` | `Search settings` |
| Result count | `clickable-icon search-results-result-count` | `lucide-more-horizontal` | none |

A real click on Search settings switches `search-params` from `display: none`
to `display: flex` and marks the icon `is-active`. The section holds exactly
three `setting-item mod-toggle` rows with empty descriptions, top to bottom:
`Collapse results`, `Show more context`, `Explain search terms`. Match case is
not in it: it is the decorator inside the input, and it gains `is-active` when
on.

The count element's text is `<span>7 results</span>` for four files holding 1,
1, 3 and 2 matches: it counts matches, not files, and names no file count. Its
markup is a `clickable-icon` wrapping a `more-options-icon`. On 2026-10-19 it
read `16 results` over eleven rows whose flairs sum to 14; the two files whose
names contain the term (`_needle`, `9 needle`) carry flair 1 each and render
`needle` in their titles inside `search-result-file-matched-text`. That fits
filename hits counting as results without adding to the file's flair — an
inference from one count, since `getMatchCount` was not read. The same text
read `0 results` once the query had been emptied in-session, and an empty
string right after a restart with no search run.

**A real left-click on the count opens a two-item menu**, both items section
`action`: `Copy search results` (`lucide-copy`) and `Bookmark...`
(`lucide-bookmark`). Escape closes it. The first is the control for
`onCopyResultsClick` — see Copying results; the second was never clicked.

Results, for the query `needle`:

- On 2026-10-18 the order was `apple.md`, `notes/banana.md`, `mango.md`,
  `zebra.md` under `sortOrder: alphabetical` — basename order. Path order
  would put `notes/banana.md` last; creation order (ctime 870845, 870851,
  870852, 870854, 870863 for apple, kiwi, mango, banana, zebra) would put
  mango second; the match counts 1, 1, 3, 2 rule out ordering by number of
  matches.
- Rows start expanded (`collapsed: false` on every row) and carry a match
  count: apple 1, banana 1, mango 3, zebra 2. With `extraContext: false`, each
  match rendered as a single short string.
- A write to `kiwi.md` during the 2026-10-18 run (82 bytes before, 32 after —
  overwritten, not appended) put a fifth file in the list within the 451 ms the
  probe waited, `kiwi.md` inserted between `notes/banana.md` and `mango.md`,
  with `sortOrder` unchanged. 451 ms is the wait the probe used, not a measured
  latency.
- Default search is case-insensitive: on 2026-10-19 `kiwi.md` contained only
  `Needle` and was listed with flair 1 for the lowercase query under
  `matchingCase: false`.
- A `.txt` file holding the term is never listed: `plain.txt` was absent in
  every sort order and again in the copy run, while present in the vault on
  disk.
- A real click on a row's collapse icon (`tree-item-icon collapse-icon`,
  `right-triangle`) collapses that row only (`is-collapsed`) and opens no file
  (`getActiveFile()` stayed null). The click meant to re-expand it later landed
  inside the rect the count control occupied, so re-expansion by a second
  click is unobserved. Each match line (`search-result-file-match tappable`)
  carries two hidden hover buttons labelled `Show more context` —
  `search-result-hover-button mod-top` (`lucide-chevron-up`) and `mod-bottom`
  (`lucide-chevron-down`) — so context expands per match, above and below.

**Sort orders.** The `<select>` has exactly six options and index 0 is selected
in a vault that never chose one. The native popup is not in the DOM and was
never opened; the options and labels were read from its `<option>` children.
Paths below drop `.md`.

| Stored value | Label | Order observed for `needle` |
| --- | --- | --- |
| `alphabetical` | File name (A to Z) | `_needle`, `9 needle`, `notes/apple`, `apple`, `Apple Pie`, `apple2`, `apple10`, `notes/banana`, `kiwi`, `mango`, `zebra` |
| `alphabeticalReverse` | File name (Z to A) | `zebra`, `mango`, `kiwi`, `notes/banana`, `apple10`, `apple2`, `Apple Pie`, `notes/apple`, `apple`, `9 needle`, `_needle` |
| `byModifiedTime` | Modified time (new to old) | `zebra`, `mango`, `_needle`, `apple`, `apple2`, `Apple Pie`, `notes/apple`, `kiwi`, `9 needle`, `notes/banana`, `apple10` |
| `byModifiedTimeReverse` | Modified time (old to new) | the exact reverse of `byModifiedTime` |
| `byCreatedTime` | Created time (new to old) | `apple10`, `notes/banana`, `9 needle`, `kiwi`, `notes/apple`, `Apple Pie`, `apple2`, `_needle`, `mango`, `apple`, `zebra` |
| `byCreatedTimeReverse` | Created time (old to new) | `apple`, `zebra`, then the other nine in their `byCreatedTime` order |

What the orders show:

- Alphabetical compares the basename without extension, not the path:
  `notes/apple` sits with `apple`, `notes/banana` between `apple10` and
  `kiwi`. Case is folded (`Apple Pie` between `apple` and `apple2`); digit
  runs compare numerically (`apple2` before `apple10`); a space sorts before a
  digit (`Apple Pie` before `apple2`); a bare prefix sorts first (`apple`
  before both); `_` sorts before `9`, which sorts before letters. Byte order
  is ruled out three times over — it puts `apple10` before `apple2`,
  `9 needle` before `_needle`, and `Apple Pie` first among the apples — and
  the data is consistent with a locale-aware numeric collation.
- `alphabeticalReverse` reversed every pair except the one basename tie:
  `notes/apple` stayed ahead of `apple` in both directions. That ties keep
  their order is likely, not certain — one tie pair, corroborated by the
  created-time groups below. Why `notes/apple` led `apple` on the fresh render
  is not known (the fixtures were written `apple.md` first).
- `byModifiedTime` matched the fixtures' mtime order exactly, newest first;
  its reverse is the exact reverse. No two fixtures shared an mtime.
- Created time put nine files in one group and `apple`/`zebra`, the two
  written first, in an older one; inside a group the rows kept the order they
  had before the switch (the nine stand in their `byModifiedTimeReverse` order
  with `apple` and `zebra` removed, in both directions). The host's `fs.stat`
  has `apple` and `zebra` at birthtime `…566.33` ms, `kiwi` at `…567.58` and
  the rest from `…567.62` to `…568.04`, with one identical `ctimeMs` for all
  twelve. The grouping fits Obsidian keying on the birthtime rounded to whole
  milliseconds — floor would have split `kiwi` from the rest, `ctime` proper
  would have made a single group — but those are the host's stats, not
  Obsidian's own, so this stays likely.

**Changing the sort order re-sorts the existing list without re-running the
query.** With every method on the result DOM and on the view wrapped in
counters, each of the five switches recorded `view.setSortOrder` 1,
`changeFocusedItem` 1 and `setFocusedItem` 1 (absent in one case), and never
`startSearch`, `stopSearch`, `emptyResults`, `addResult` or `removeResult`.
The count stayed `16 results`, though the sampler started 120 ms after the
change, so it is the counters, not the samples, that rule out a transient. The
one row collapsed by hand before the switches (`_needle`) stayed collapsed
through all five orders and after `alphabetical` was restored. Four of the five
switches left one row carrying `has-focus` (`_needle`, `_needle`, `apple`,
none, `zebra`); which row gets it was not worked out.

Where state lives:

- The six keys persist in the search leaf's `state` in
  `.obsidian/workspace.json`, and nowhere else that was looked. With
  `byModifiedTime`, `extraContext` and `explainSearch` set, the file after
  Ctrl+Q held
  `"query": "", "matchingCase": false, "explainSearch": true, "collapseAll": false, "extraContext": true, "sortOrder": "byModifiedTime"`;
  `app.json` stayed `{}` with no sort key at any point; and after a restart the
  dropdown showed `Modified time (new to old)` and the two toggles came back
  enabled. When the write happened is not known: the bytes were read at launch
  and after quit, and `startSearch` ends in `requestSaveLayout()`, so the save
  may well precede the quit.
- Recent searches live in `localStorage` under `<appId>-recent-searches`,
  holding a JSON array of query strings — `["needle"]` on 2026-10-19, the
  vault's only key, and it survived the restart; `["[\"tags\"]"]` on
  2026-10-18, one of six keys for that vault. That lines up with
  `saveSearch`/`requestSaveSearch` and with `bookmarks:bookmark-current-search`.
- `.obsidian/global-search.json` appeared between launch (when `.obsidian/`
  held `app.json`, `appearance.json`, `core-plugins.json`, `workspace.json`)
  and the post-quit listing; no step read its bytes, so what it holds — and
  whether the first search's `requestSaveSearch` wrote it — is unknown.
- The 2026-10-18 vault's `.obsidian/` held `app.json`, `appearance.json`,
  `community-plugins.json`, `core-plugins.json`, `graph.json` and
  `workspace.json` plus a `plugins/` folder — there was no `search.json`.
  `app.json` was `{"foldHeading": true, "propertiesInDocument": "visible"}`,
  with no `searchSortOrder`, `sortOrder` or `searchSort` key; that absence now
  reads as expected, since the sort order is leaf state.
- The 2026-10-18 scratch vault held `Notes/` and `notes/` as two distinct
  folders, so nothing here speaks to folder-name case handling.

**Copying results opens a modal; nothing reaches the clipboard until its button
is pressed.** `onCopyResultsClick` is
`function(e){e.preventDefault(),new x2(this.app,this.dom).open()}`, and a real
click on the menu item left a sentinel clipboard untouched. The modal is titled
`Copy search results`, has a close button, and its content holds, in DOM
order, a `textarea.copy-search-result-textarea` (prefilled, editable,
`rows=2`), three settings rows, and two buttons:

| Control | Label | Description | Values (stored → label) | Default |
| --- | --- | --- | --- | --- |
| toggle | Show path | Display full path of the file rather than just the file name. | — | off |
| select | Link style | Optionally turn each file result in to a link. | `none` → None, `wikilink` → Wikilink, `markdown` → Markdown link | `none` |
| select | List prefix | Optionally add a list item prefix to each file result. | `none` → None, `-` → Dash (-), `*` → Asterisk (*), `num` → Numbered | `none` |

Buttons: `Copy results` (`mod-cta`) and `Done`. The textarea holds one line
per file in the list's order — the list was in default alphabetical order in
this run; copying under another sort was not tried — joined by `\n` with no
trailing newline, no match lines, no indentation; a file with two matches gives
one line. All 24 combinations were read from the textarea over `apple.md`,
`Apple Pie.md`, `notes/banana.md`, `mango.md`; the `notes/banana.md` line per
combination, before any prefix:

| Show path | Link style | Line |
| --- | --- | --- |
| off | None | `banana` |
| off | Wikilink | `[[banana]]` |
| off | Markdown link | `[banana](banana)` |
| on | None | `notes/banana.md` |
| on | Wikilink | `[[notes/banana.md]]` |
| on | Markdown link | `[banana](notes/banana.md)` |

So the name form is the basename without extension, the path form is the
vault-relative path with extension and folder kept, a Markdown link's text is
always the basename and its target is not percent-encoded
(`[Apple Pie](Apple Pie.md)`). The prefix prepends `- `, `* ` or `1. `, `2. `,
… (numbered from 1 in list order) to every line. A real click on
`Copy results` put the textarea text on the system clipboard byte for byte —
`1. [apple](apple.md)\n2. [Apple Pie](Apple Pie.md)\n3. [banana](notes/banana.md)\n4. [mango](mango.md)`,
LF only, no trailing newline, read through Electron's clipboard — showed the
notice `Results copied to your clipboard.`, and left the modal open; Escape
closed it. `Done` was never pressed, and no `paste.md` was created, so the
bytes a paste lands in a note are unobserved.

204 commands are registered. The ones the 2026-10-18 probe's filter surfaced
that touch copying or search are `workspace:copy-path`,
`workspace:copy-full-path`, `workspace:copy-url`, `bases:copy-table`,
`bookmarks:bookmark-current-search` and `global-search:open`. The other ids
were not recorded, so nothing is established about them.

**The four toggles each flip one state key**, clicked for real with an empty
query, and clicking again restores `false`:

| Toggle | Where | State key | Class when on | Also set on the result DOM |
| --- | --- | --- | --- | --- |
| Collapse results | settings section | `collapseAll` | `checkbox-container is-enabled` | `dom.collapseAll` |
| Show more context | settings section | `extraContext` | `checkbox-container is-enabled` | `dom.extraContext` |
| Explain search terms | settings section | `explainSearch` | `checkbox-container is-enabled` | (the probe read only `collapseAll`, `extraContext`, `sortOrder` off the DOM object) |
| Match case | input decorator | `matchingCase` | `input-right-decorator clickable-icon is-active` | — |

Enabling `Explain search terms` moved its own row down 45 px (y 164 at the
click that enabled it, 209 at the click that disabled it) while the other three
rows stayed put, so it renders something above the settings section even with
an empty query — `search-info-container` precedes `search-params`. The probe's
`[class*="explain"]` selector found nothing, so that block is not classed
`explain`. What the other three do to a populated list was not seen.

Context menus:

- 2026-10-18, synthetic right-clicks: a result's file title opens a 13-item
  menu. A match line opens the same menu with "Delete file" dropped — 12 items,
  every callback byte-identical to the file-title one (item 0 is
  `function(){r.app.workspace.openLinkText(t,n,"tab")}` in both). A match row's
  menu acts on the file; nothing in it is match-specific — no copy-match, no
  line target.
- The dump below is the menu's insertion order with each item's `data-section`,
  not the order a reader sees: Obsidian renders a menu grouped by section,
  which is why three titles are fragments that only read under a Copy heading.

| # | Title | Section |
| --- | --- | --- |
| 0 | Open in new tab | `open` |
| 1 | Open to the right | `open` |
| 2 | Rename... | `action` |
| 3 | as Obsidian URL | `info.copy` |
| 4 | Open in default app | `system` |
| 5 | Move file to... | `action` |
| 6 | from vault folder | `info.copy` |
| 7 | from system root | `info.copy` |
| 8 | Show in system explorer | `system` |
| 9 | Open in new window | `open` |
| 10 | Reveal file in navigation | `system` |
| 11 | Bookmark... | `action` |
| 12 | Delete file | `danger` |

- Right-clicking the result container (`search-result-container`) and the
  results children (`search-results-children`) opened no menu at all on
  2026-10-18. The pane's tab header did open one, with exactly one item:
  "Close" (section `close`, icon `lucide-x`, callback
  `function(){return t.leaf.detach()}`). A real right-click on 2026-10-19,
  read 735 ms later, gave the same one item, which removes most of the worry
  that the synthetic click read the menu before it was populated. A real
  right-click on the `workspace-tab-header-spacer` beside the tabs opened
  nothing, and the view has no `nav-header` element in this layout. The two
  empty result surfaces stay weak negatives: on 2026-10-19 there was no empty
  space below the last result to click.
- The 2026-10-19 run could not re-dump the search rows' menus: no rows existed
  when that step ran. Between the copy step and the menu step the rows
  disappeared and the query read empty from then on. The copy step's real
  click had opened one modal; because the clipboard stayed unchanged the
  script then called `onCopyResultsClick` directly and opened a second,
  pressed Escape once, and the next step began by re-running
  `global-search:open` — a stacked modal is the likely cause, but nothing
  recorded the moment the query emptied.
- The Files view's menu for `apple.md`, from a real right-click, in displayed
  order:

| Title | Section | Note |
| --- | --- | --- |
| Open in new tab | `open` | `lucide-file-plus` |
| Open to the right | `open` | `lucide-separator-vertical` |
| Open in new window | `open` | `lucide-picture-in-picture-2` |
| --- | | |
| Make a copy | `action` | `lucide-files` |
| Move file to... | `action` | `lucide-folder-tree` |
| Bookmark... | `action` | `lucide-bookmark` |
| Merge entire file with... | `action` | `lucide-git-merge` |
| --- | | |
| Copy path | none | `has-submenu`, `lucide-clipboard` |
| --- | | |
| Open version history | `view` | `lucide-history` |
| --- | | |
| Open in default app | `system` | `lucide-arrow-up-right` |
| Show in system explorer | `system` | `lucide-arrow-up-right` |
| --- | | two separators in a row |
| --- | | |
| Rename... | `danger` | `lucide-edit-3` |
| Delete | `danger` | `is-warning`, `lucide-trash-2` |

  Against the synthetic search-row dump it has `Rename...` in `danger` rather
  than `action`, `Delete` rather than `Delete file`, a `Copy path` submenu in
  place of the three `info.copy` fragments, no `Reveal file in navigation`,
  and three items the search dump lacks (`Make a copy`,
  `Merge entire file with...`, `Open version history`); whether a search row's
  real menu matches it item for item is still open.

Keyboard, from the handler sources on the view prototype — none was driven
against a populated list:

| Key | Handler |
| --- | --- |
| ArrowDown | `e.defaultPrevented` aside, calls `cE()` (unread) then `dom.changeFocusedItem("forwards")`; no input guard |
| ArrowLeft | nothing while the search input is the active element; else on a file row `setCollapse(true, true)`; on a match row collapses the parent file row and moves focus up to the nearest uncollapsed ancestor; on the list root `setCollapseAll(true)` |
| ArrowRight | same input guard; on the list root `setCollapseAll(false)`; on a file row `setCollapse(false, true)` |
| Enter | `focusedItem.onResultClick(e)` when the focused item is a file row or a match row |

So file rows and match rows are both focusable, and the Ctrl/Shift Enter
variants are decided inside `onResultClick`, which was not read. The driven
sequence (five ArrowDown, ArrowUp, two ArrowLeft, two ArrowRight, Enter,
Ctrl+Enter, Shift+Enter) ran over an empty list: `focusedItem` stayed null,
the input stayed the active element for the arrows, and no Markdown leaf,
popout window or vault file appeared after any Enter — which says nothing was
focused, not that Enter does nothing.

**Unprobed:** the visible effect of each toggle on a populated list — the
`Needle` hit set under Match case (the capitalised query never landed), whether
Collapse results drops match lines, how many lines Show more context adds and
whether they are trimmed, and what Explain search terms renders (only its
layout shift was seen). Keyboard navigation as driven: what the focus ring
moves over, what Enter, Ctrl+Enter and Shift+Enter open and where, and what
`cE()` in the ArrowDown handler is. The real context menu of a result's file
title and of a match line, whether it matches the Files view's menu for the
same file item for item, and the menu — if any — of the empty space below the
last result. Ctrl+C on a focused result row (never pressed; the only keyboard
step around the count menu pressed ArrowDown then Enter and read an unchanged
clipboard). What `Bookmark...` in the count menu does, what `Done` in the copy
modal does, whether the modal's textarea tracks the result list after an edit
or a sort change, and the bytes a paste of the copied text lands in a note.
The contents of `.obsidian/global-search.json` and which action writes it.
What a restored non-empty `query` does on restart (the query was empty at
quit). Whether the sort re-sort moves existing DOM nodes or re-renders rows,
and which row takes `has-focus` after a switch. The tie-break of a fresh list
(only re-sort stability was seen, from one tie pair), mtime ties (none
existed), the exact created-time key (birthtime rounded to milliseconds fits
one grouping of host stats), and the native `<select>` popup itself, which was
never opened — every switch was a synthetic `change` event. Whether a second
click on a row's collapse icon re-expands it, and whether clicking a row's
title collapses or opens.

## Observed shell typography and chrome, Obsidian 1.14.4 (installer build 1.13.7)

Read from a running Obsidian on Linux over CDP on 2026-10-20 with
`tools/obsidian-probe/frame-typography.mjs`, `frame-settings.mjs` and
`frame-setting-key.mjs` (raw runs in `tools/obsidian-probe/results/2026-10-20-frame-*.json`);
every number is a computed style or a bounding box in CSS pixels at zoom 1 in a 1024 by 800 window
on a fresh vault. These are the values `crates//src/theme.rs` and the shell renderers carry.

**The window frame.** `body` carries `is-frameless is-hidden-frameless mod-linux`: on Linux the
default "Window frame style" is **Hidden**. The setting lives in Settings → **Interface** (not
Appearance), labelled "Window frame style", described "Determines the styling of the title bar of
Obsidian windows. Requires a full restart to take effect.", with options `hidden` "Hidden (default)",
`custom` "Obsidian frame" and `native` "Native frame". Choosing any of them wrote **nothing** under
`.obsidian` (`appearance.json` stayed `{}` and `app.vault.config` stayed empty): the choice is kept
outside the vault, so  takes it from `--frame` or `SCORIA_FRAME` and writes no key. Obsidian
1.14 opens Settings in a popout window; its rows are only reachable through
`app.setting.activeTab.containerEl`.

In the hidden frame the title row is **39 px** tall (`.titlebar`, transparent, over the 40 px tab
header containers) and holds, left to right: the left sidebar toggle (`.sidebar-toggle-button.mod-left`,
44 by 39, its icon in a 28 by 24 button padded `4px 6px`), the left dock's tab header, the root tab
header, the right sidebar toggle (36 by 39, at the right end of the root header: left of the right
dock's tab header while the sidebar is open, before the window buttons while it is collapsed; it
never sits in the right dock's own header) and the window buttons `.titlebar-button.mod-minimize`, `.mod-maximize`,
`.mod-close`, each **44 by 39** padded `0 16px`, in a `.titlebar-button-container.mod-right` 132 px
wide in the sidebar colour. `--frame-right-space` is `calc(126px - 0px)` and the root tab header
is padded `0 134px 0 8px` to stay clear of them. The `.titlebar-text` is 13 px, weight 700, muted at
opacity 0.85 and reads "New tab - <vault> - Obsidian 1.14.4"; the hidden frame gives it no box.
`--titlebar-height` is 30 px and `--window-border-radius` is unset; the window's corners are
nevertheless rounded, about 10 px on all four in a screenshot of 2026-10-21 (KDE Wayland, 2x),
so the rounding is the hidden frame's own rather than a theme's.

**The hidden frame's shadow.** A window capture at scale 1 on 2026-10-21 (KDE Wayland, the
surface's own pixels with alpha) shows the content in a transparent margin of **23 px at the left
and right, 15 px above and 46 px below**. The margin holds a shadow that is pure black at a
varying alpha: along a side it rises from 0.016 at the outer edge to 0.16 at the content, along
the top from 0.004 to 0.06, and along the bottom from 0.016 at the outer edge to 0.27 at the
content, which fits a gaussian of sigma 13 at 32 % black offset 12 px downward (CSS
`0 12px 27px rgba(0,0,0,0.32)`). The last pixel before the content is darker than the ramp —
alpha 0.27 at the sides, two pixels of 0.15 and 0.11 at the top — a one pixel outline on the
shadow rather than a border inside the content, whose first pixel is `--background-secondary`.

**Fonts.** `--font-interface` is `'??', '??', '??', ui-sans-serif, -apple-system, BlinkMacSystemFont,
system-ui, "Segoe UI", "Google Sans Flex", Roboto, "Inter Variable", "Inter", …` (the `??` are the
theme and override slots, empty here); `--font-text` is the same without the first slot;
`--font-monospace` is `'??', '??', ui-monospace, SFMono-Regular, "Cascadia Mono", "Roboto Mono",
"DejaVu Sans Mono", "Liberation Mono", Menlo, Monaco, "Consolas", "Source Code Pro", monospace`.
`document.fonts` lists Inter 100–900 normal and italic as **loaded** and Source Code Pro as unloaded:
Inter is what the shell renders in.  bundles Inter's static faces (`crates//assets/fonts`)
and defaults both families to it.

**Sizes.** `--font-text-size` 16 px; `--font-ui-smaller` 12, `--font-ui-small` 13, `--font-ui-medium`
15, `--font-ui-large` 20; `--line-height-normal` 1.5, `--line-height-tight` 1.3; `body` is 15 px at
19.5 px. Headings: `--h1-size` 1.618em, h2 1.462, h3 1.318, h4 1.188, h5 1.076, h6 1em; weights
700, 680, 660, 640, 620, 600; line heights h1 1.2, h2 1.2, h3 1.3; `--bold-weight` is
`calc(400 + 200)`. In a note: `h1` 25.888 px / 31.07 line / weight 700 / letter-spacing −0.388 px
(−0.015em), margin `0 0 16px`; `h2` 23.392 px / 28.07 / 680 / −0.257 px, margin `40px 0 16px`; `p`
16 px / 24 px line, margin `16px 0`; `.inline-title` is an h1 with margin-bottom 12.944 px; the
preview and the editor are padded 32 px (`--file-margins: 32px 32px`, `--file-line-width` 700 px);
a tag is 14 px in a 28 px pill padded `3.5px 9.1px` on a 10 percent accent wash.

**Chrome.** `--header-height` 40 px: every tab header container and the view header are 40 px, the
view header padded `0 12px` with an 8 px gap, its title 13 px, its nav arrows and actions 28 by 24
clickable icons padded `4px 6px` with a 4 px radius, the arrows at opacity 0.4 with nowhere to go.
`--ribbon-width` 44 px: `.workspace-ribbon.mod-left` (classes `workspace-ribbon side-dock-ribbon
mod-primary`) starts 40 px down, padded `8px 4px 12px`, its `.side-dock-actions` a column with a
6 px gap of 28 by 24 buttons whose 16 px icons sit at opacity 0.85; `.side-dock-settings` is at the
bottom. Sidebars: `.mod-left-split` and `.mod-right-split` are **300 px** wide on a fresh vault, the
right one collapsed at first launch; the dock tab header is padded `0 8px`, its tabs 28 by 25.3 with
a 4 px radius, the active one on `--background-modifier-hover`, showing only the view's 16 px icon;
the `.workspace-leaf-resize-handle` is a 3 px strip on the dock's inner edge (`cursor: col-resize`,
a `--divider-color` hairline, `--divider-color-hover` the accent at `--divider-width-hover` 3 px).
The root tab: `.workspace-tab-header` 200 by 34.3 (`--tab-width` 200, `--tab-max-width` 320),
padded `1px 4px 4px`, radius `10px 10px 0 0`, the active one in `--background-primary`; its title
13 px; its close button 20 by 20 padded 2 px with an 8 px radius (`lucide-x`); the new-tab button
24 wide (`lucide-plus`) with margin `0 4px 0 8px`; the tab list button `lucide-chevron-down`.
Inside the tab, `.workspace-tab-header-inner` is the box the 1/4/4 padding leaves: 192 by 29.3,
padded `0 3px 0 6px` with a 2 px gap, radius 4 px (`--tab-radius`). **Hover** paints that inner
box, not the tab: `--background-modifier-hover` (white at 6.7 percent in dark, black at 6.7 percent
in light) on the 4 px-rounded inner box, so the wash sits 1 px below the tab's top and 4 px in from
its sides and bottom, while the tab's own 10 px top corners stay unpainted. The title's colour does
not change on hover (`--tab-text-color-focused`, #b3b3b3, before and after), the close button
appears (`display: none` → `flex`) and, under the pointer itself, draws no wash of its own, only
its cross going from `--text-muted` to `--text-normal`. The active tab shows nothing on hover: its
inner box stays transparent. Between two inactive tabs a divider: the inner box's `::after`, a 1 px
line 15 px tall 8 px below the tab's top, at the tab's **right** edge straddling the gap
(`right: -0.5px`), in `--tab-divider-color` (#3f3f3f dark, #dadada light). The active tab's is
`opacity: 0`, and the one on the tab before the active tab is painted over by the active tab, so
the first gap and the two around the active tab are bare. Hover changes no divider: both lines
beside a hovered tab stay, the wash stopping 4 px short of them. (Read over CDP on 2026-10-22,
`tools/obsidian-probe/results/2026-10-22-tab-hover.json`, both themes; the earlier note here that a
hovered tab hides its divider, and the #555555 measured off a screenshot, were wrong.) The dock
tabs hover the same way: the hovered 28 by 25 tab's inner box takes `--background-modifier-hover`
at the 4 px radius, the active one carries the same wash at rest.

**The strip's right end** (read over CDP on 2026-10-22,
`tools/obsidian-probe/results/2026-10-22-tab-list.json`). After the last tab comes the new-tab
button, `.workspace-tab-header-new-tab`: a 24 by 24 `clickable-icon` padded 4 px around the 16 px
`lucide-plus`, at y 8.66 in the 40 px strip (so centred on the titles rather than on the 34 px
tab), margin `0 4px 0 8px`. Pressing it, or running `workspace:new-tab`, appends an empty "New
tab" at the strip's right end whichever tab was active, and makes it the active tab; a second
press appends another after it (`tools/obsidian-probe/results/2026-10-22-new-tab.json`, with the
second of four tabs active). The empty tab's view offers "New note Ctrl + N", "New base", "New
canvas" and "Go to file Ctrl + O". The last tab's own `::after` divider stays when the tab is inactive, so
a hairline parts the tabs from the button. Then the spacer, then the tab-list button
`.workspace-tab-header-tab-list`, the same 24 px `clickable-icon` around `lucide-chevron-down` at
y 8.66 with margin `0 2px 0 0`, and, with the right sidebar collapsed, the sidebar toggle, a 28 by
24 `clickable-icon` padded `4px 6px` at y 8. Clicking the chevron opens `.menu.mod-tab-list` at the
pointer: `--background-secondary`, 8 px radius, a 0.69 px `--background-modifier-border` border,
three shadows, padded 6 px; rows 24.9 px tall padded `4px 8px`, 4 px radius, an 8 px gap between
the 16 px icon and the 13 px `--text-normal` title, `--background-modifier-hover` under the pointer;
separators 0.69 px with margin `6px -6px`, so they run the menu's full width. The rows: **Stack
tabs** (`lucide-layers`, section `action`; reads **Unstack tabs** once the group is stacked,
`mod-stacked` on `.workspace-tabs`), **Bookmark N tabs...** (`lucide-bookmark`, N the tabs with a
file, absent when none has one; opens the "Add N bookmarks" modal with a Title field whose
placeholder is the current date and time, and Cancel / Save), a separator, **Close all**
(`lucide-x`, section `close`; every tab in the group closes and, in the window's only group, one
empty "New tab" is left active), a separator, then one row per tab (`lucide-file`, section
`tablist`, the tab's title) with `mod-checked` and a trailing check on the group's active tab;
picking a row activates that tab. Escape closes the menu.
Explorer rows: `.nav-file-title` 24.9 px tall padded `4px 8px 4px 24px`, margin-bottom 2 px, radius
8 px, 13 px text, muted until active (then `--text-normal` on the hover wash); `.nav-header` 40 px
padded 8 px with a 2 px gap of 28 by 24 buttons. The status bar is a tab in the window's bottom right
rather than a bar across it: `.status-bar` padded `4px 8px 4px 4px`, min-height 18, the sidebar
colour, a hairline on its left (and top), radius `8px 0 0 0`, 12 px text, items padded `3px 4px`
with a 4 px gap. Icons everywhere are 16 px with a 2 px stroke on Lucide's 24 unit grid (`--icon-s`,
`--icon-m` 16, `--icon-l` 18, `--icon-xs` 14).

**The vault profile.** Settings is not at the bottom of the ribbon: the probe found
`.side-dock-settings` 0 px tall. In 1.14 the left sidebar ends with a footer, the vault profile,
seen on 2026-10-21 in a screenshot of the running app (the docs vault open, dark theme, 2x): a
hairline across the top, then a row the height of a header with, on the left, a `chevrons-up-down`
icon at muted strength and the vault's name in 13 px semibold `--text-normal`, and on the right two
clickable icons, Help (`circle-help`) and Settings (`settings`). The name and its chevrons open the
vault switcher, Help opens the help site, the gear opens Settings. The footer is part of the sidebar,
so it goes with it when the sidebar collapses, and it is there whether or not the ribbon is shown.
Its exact padding and the switcher's hover shape are still to be read over CDP;  draws the row
40 px tall, padded 8 px, its buttons the ribbon's 28 by 24, and runs `app:open-vault` from the name,
`app:open-help` from Help and `app:open-settings` from the gear.

**The vault switcher.** `app:open-vault` ("Manage vaults") and `app:switch-vault` drop a menu up from
the name: every vault this machine has opened, most recent first with a check on the one showing (a
folder that is gone is listed faint and says so when clicked), a hairline, then "Open vault..."
(`app:open-another-vault`, the desktop's folder picker) and "Open sandbox vault"
(`app:open-sandbox-vault`). A click outside closes it. The list is Obsidian's `obsidian.json`
document — `{"vaults": {<16 hex digits>: {"path", "ts", "open"}}}`, `ts` in Unix milliseconds,
`open` on the vault a window shows — written as `.json` under `~/.config/` (Linux),
`%APPDATA%\` (Windows) or `~/Library/Application Support/` (macOS), or
`$SCORIA_CONFIG_DIR`; unknown keys survive a rewrite. Every vault loaded goes on the list except the
sandbox, which is a fresh ` Sandbox <ms>` folder under the temporary directory holding a
`Welcome.md` and an `Another note.md`, opened on `Welcome`. Switching puts the vault that was open
away as quitting would (the note's edits and `workspace.json` written if the session changed them),
closes every tab group, and restores the next vault from its own `workspace.json`. The menu's rows
are 's reading of the screenshot; Obsidian's own vault switcher is still to be probed.

**The Help window.** `app:open-help` and the footer's Help open a window of Obsidian's shape rather
than the browser (seen in a screenshot of 1.14.4, 2026-10-21): the mark, the name, "Version x.y.z",
then a card of rows, each an icon on the left, a title over a line of explanation, and a button on
the right — Obsidian's are the help site (an accent "Visit"), Discord ("Join"), the forum ("Visit")
and the sandbox vault ("Open"). 's four are the manual (, accent "Visit"), the source,
the issue tracker and the sandbox vault; the window is 700 by 860 with the desktop's own frame,
Escape closes it, and opening help again brings the one up forward.

**Icons named in the DOM** (`svg.lucide-*`, so  draws the same Lucide drawings from Lucide's
own repository): ribbon `file-search` (switcher), `git-fork` (graph), `layout-dashboard` (canvas),
`calendar` (daily note), `files` (template), `terminal` (palette), `layout-list` (base); left dock
`folder-closed`, `search`, `bookmark`; right dock `links-coming-in` and `links-going-out`
(Obsidian's own;  draws its own two under those names, Lucide's chain at the top left with a
horizontal arrow beneath it pointing in or out), `tags`, `archive` (All properties), `list` (outline),
in that order; view header `arrow-left`,
`arrow-right`, actions `bookmark` and `more-vertical`; explorer header `edit`, `folder-plus`,
`sort-asc`, `gallery-vertical`, `chevrons-up-down`; search `search`, `list`, `move-vertical`; tags
`folder-tree`; bookmarks `bookmark-plus`, `folder-plus`, `chevrons-down-up`; folder collapse
`right-triangle` and the sidebar toggles `sidebar-toggle-button-icon` are Obsidian's own.

**Palette.** Dark (`theme-dark`): `--background-primary` #1C1C1C, `-primary-alt` #232323,
`-secondary` #282828, `-secondary-alt` #333333, `--background-modifier-border` #333333,
`--background-modifier-hover` `color-mix(in oklch, white 6.7%, transparent)`, `--text-normal`
#dadada, `--text-muted` #b3b3b3, `--text-faint` #666666, `--interactive-accent` `hsl(258, 88%, 66%)`
(the picker shows #8a5cf5), `--text-accent` and `--link-color` `hsl(255, 89.76%, 75.9%)` (computed
rgb(166, 138, 249)), `--tab-text-color` #666666, `--tab-text-color-active` #b3b3b3,
`--tab-background-active` #1C1C1C, `--nav-item-color` #b3b3b3, `--nav-item-color-active` #dadada,
`--icon-color` #b3b3b3, `--divider-color` #333333, `--code-background` #232323, `--text-selection`
a 33 percent accent wash, `--scrollbar-thumb-bg` a 10 percent white wash. Light (`theme-light`):
#ffffff, #fafafa, #f6f6f6, #fcfcfc, border #e4e4e4, hover `black 6.7%`, text #222222, muted
#5c5c5c, faint #ababab, accent and link `hsl(258, 88%, 66%)`, tab text #ababab / #5c5c5c,
selection a 20 percent wash. Radii `--radius-s` 4, `-m` 8, `-l` 12; `--clickable-icon-radius` 4.

**What a sidebar writes.** `workspace.json`'s `left` and `right` nodes carry `"width": 300` on a
fresh vault; dragging the left handle 120 px wrote `"width": 419.50215911865234` (the dock's new
`style="width: 419.502px"`), and the sidebar toggle adds `"collapsed": true` after `width` and
removes it again. `app:toggle-left-sidebar`, `app:toggle-right-sidebar` and `app:toggle-ribbon`
are the commands ("Toggle left sidebar", "Toggle right sidebar", "Toggle ribbon").

## Observed tab group behavior, Obsidian 1.14.4 (installer build 1.13.7)

One CDP-driven run on 2026-10-20 (`tools/obsidian-probe/splits-bookmarks.mjs`,
raw results in `tools/obsidian-probe/results/2026-10-20-splits-bookmarks.json`)
in a fresh scratch vault of four notes (`A.md`, `B.md`, `C.md`, `sub/D.md`),
reading `app.workspace.rootSplit` after every step and `workspace.json` after
`app.workspace.saveLayout()`. Everything below was seen, not read from docs.

The layout:

- `main` is always a `split` with `direction: "vertical"`, even around one
  group: `{"id", "type": "split", "children": [...], "direction"}` in that key
  order. A group is `{"id", "type": "tabs", "children": [leaves]}` plus
  `"currentTab": n` after `children` when the active tab is not the first.
- `workspace:split-vertical` ("Split right", `hasCheck`) on a group inside a
  vertical split inserts a new `tabs` node **right after it** in the same split.
  `workspace:split-horizontal` ("Split down") on a group inside a vertical split
  **replaces the group's node with a `split` of `direction: "horizontal"`** whose
  children are the old group and the new one. The rule is the same both ways:
  a sibling when the enclosing split already runs that way, a wrapping split
  otherwise. Splitting a group inside the horizontal split with "Split right"
  wrapped it in a vertical split, three levels deep.
- The new group holds **a copy of the active leaf**: same file, same `mode` and
  `source`, a fresh leaf id. It becomes the active leaf. Splitting an empty tab
  makes a new group with an `empty` leaf; splitting the graph makes one with a
  `graph` leaf.
- Dragging the divider between two children writes **`"dimension"`** on *both*
  of them: a percentage of the split, `29.411764705882355` / `70.58823529411765`,
  inserted **after `type` and before `children`** (`id, type, dimension,
  children`; for a split, `id, type, dimension, children, direction`). The DOM
  carries it as `flex-grow: 29.4118`. Before any drag no child has the key.
  Splitting a child that has a dimension gives each half of it (`29.41` became
  `14.70` and `14.70`); closing a child rescales the rest to a hundred (`14.70`,
  `70.59` became `17.24`, `82.76`); a split left with one child loses the key.
- Closing a group's last tab (`workspace:close`) **removes the group**, unless it
  is the only group, which keeps an empty leaf. A split left with one child is
  **not collapsed**: the root kept a nested horizontal split around a single
  `tabs` node. A split left with no child goes.
- `workspace:close-tab-group` ("Close this tab group") removes the active group
  with all its tabs. `workspace:close-others-tab-group` ("Close others in tab
  group") closes the **other tabs of the active group**, not other groups.
- After a group closes, the active leaf becomes the **most recently active leaf
  elsewhere** (the one active before the closed group was), not the layout's
  first.
- `editor:focus-left/right/top/bottom` ("Focus on tab group to the left/right",
  "above/below") step to the sibling on that side of the nearest enclosing split
  that runs that way; when the sibling is itself a split, focus lands on the
  group used most recently inside it (from the left column, focus-right went
  back to the bottom-right group that had been active last, not the top-right).
  At an edge nothing happens. All four are `hasCheck`.
- `editor:open-link-in-new-split` ("Open link under cursor to the right",
  default hotkey Mod+Alt+Enter) is "Split right" and then the link opened in
  the new group — a **new** sibling next to the current group, even when a group
  already exists to its right.
- The tabs container of the active group carries `mod-active`; its active tab
  header has `mod-active is-active`, an inactive group's active tab only
  `is-active`. A click in a group's content makes it active.
- A nested layout survives a quit and a relaunch byte for byte: `workspace.json`
  after the restart equalled the file written at quit, and was not rewritten
  within 3 s of the layout being ready.

 implements all of this since 2026-10-20 ([[Development#Tab groups, 2026-10-20]]),
with one graph view shared between groups: a second group showing the graph
says so in place of it.

Since 2026-10-21 a tab header can also be dragged ([[Development#Dragging tabs, 2026-10-21]]).
The rule  uses is **not probed**, it is the common desktop one: dropped on
another tab it goes in front of that tab, on a strip's open space at its end,
and over a group's content the outer band of each side (30 percent of the
shorter side, the nearer edge winning in a corner) makes a new group on that
side the way "Split right" and "Split down" place theirs — a sibling in front
or behind when the enclosing split runs that way, a wrapping split otherwise —
while the middle joins the strip. The dropped tab is the active tab of the
group it landed in, and a group left with no tab closes as it does when its
last tab is closed. Obsidian's zone geometry, highlight and ghost are still
to be observed.

**Unprobed:** the `dimension` a drag writes when only *some* siblings have one;
`workspace:close-others` (without `-tab-group`) across groups; dragging a tab
header into another group or onto a split edge (where the zones start, what is
highlighted, whether a split from a drop halves a `dimension` as "Split right"
does); `workspace:toggle-stacked-tabs`; what "Split right" does from the
sidebar's "More options" menu.

## Observed bookmarks behavior, Obsidian 1.14.4 (installer build 1.13.7)

Same run and vault as [[#Observed tab group behavior, Obsidian 1.14.4 (installer build 1.13.7)]],
with the `bookmarks` core plugin enabled and `.obsidian/bookmarks.json` read
from disk after every command.

The file:

- `{"items": [...]}`, two-space indented, **no trailing newline**. New items are
  appended at the end. It is written within a second of the change.
- Every item has `"type"` first and `"ctime"` (Unix milliseconds) second; what
  follows depends on the type:

| Type | Keys after `type`, `ctime` | Made by |
| --- | --- | --- |
| `file` | `path`; `subpath` (`"#Second heading"` or `"#^b56db2"`) when a heading or block | `bookmarks:bookmark-current-view`, `bookmark-current-heading`, `bookmark-current-section` |
| `search` | `query` | `bookmarks:bookmark-current-search` with the Search pane showing a query |
| `graph` | `title`, `options` (the whole graph config: `collapse-filter`, `search`, `showTags`, ... `close`) | `bookmark-current-view` with the graph active; the default title is `Untitled Graph` |
| `group` | `items` (nested items), `title` | `bookmarks:bookmark-all-tabs`; the default title is the moment formatted `MMMM D, YYYY h:mm A` (`October 20, 2026 1:49 PM`); every file item in it shares one `ctime`, the group's is later |
| `folder` | `path` | added through the plugin API in this run, so the key order Obsidian's own UI writes is unprobed |
| `url` | `url`, `title` | likewise through the API |

- `"title"` is written only when the modal's Title field was filled in; an
  empty field writes no key, and the pane shows the path (`A`,
  `A#Second heading`, `A#^b56db2`), the query, or the group's title.
- `bookmarks:bookmark-current-section` **writes a block id into the note** when
  the paragraph has none: ` ^b56db2` appended to the line (six lowercase
  hex-ish characters, a space before the caret), then bookmarks
  `"subpath": "#^b56db2"`.
- `bookmarks:unbookmark-current-view` ("Remove bookmark for the current file")
  removed **one** plain `file` item for the open file — in this run the one
  inside the group that had been added last, leaving the older top-level one.
  It is a lookup by path, last added wins.
- Running `bookmark-current-view` on a file already bookmarked opens
  **"Edit bookmark"** (with a Remove button) and writes nothing on Save.

The modal, "Add bookmark": a `Path` row (the file, read-only text input holding
`A`, `A#Second heading`, `A#^b56db2`; `Query` for a search), a `Title` row (an
empty input whose placeholder is that same text), a `Bookmark group` row, and
`Cancel` / `Save`. The graph's has no Path row and its Title input starts
filled with `Untitled Graph`. "Bookmark all tabs" opens "Add 3 bookmarks" with
only Title (placeholder the date) and Bookmark group. Enter saves.

The pane (view type `bookmarks`): a `nav-header` of four buttons in order —
"Bookmark the active tab..." (`lucide-bookmark-plus`), "New group"
(`lucide-folder-plus`), "Collapse all" (`lucide-chevrons-down-up`), "Show
search filter" (`lucide-search`) — then one `tree-item` per item in file order.
Icons by type: file `lucide-file`, heading `lucide-heading`, block
`lucide-toy-brick`, search `lucide-search`, graph `lucide-git-fork`, folder
`lucide-folder`, url `lucide-globe-2`; a group is a collapsible row with the
`right-triangle` and its items indented under it. The row for the open file
carries `is-active`.

Commands and names: `bookmarks:open` "Bookmarks: Show bookmarks";
`bookmarks:bookmark-current-view` "Bookmarks: Bookmark..." (check: a file or
graph view is active); `bookmarks:bookmark-current-search` "Bookmarks: Bookmark
current search..." (check); `bookmarks:unbookmark-current-view` "Bookmarks:
Remove bookmark for the current file" (check); `bookmarks:bookmark-current-section`
"Bookmarks: Bookmark block under cursor..." (check); `bookmarks:bookmark-current-heading`
"Bookmarks: Bookmark heading under cursor..." (check); `bookmarks:bookmark-all-tabs`
"Bookmarks: Bookmark all tabs..." (no check). None has a default hotkey.

 implements this since 2026-10-20 ([[Development#Bookmarks, 2026-10-20]]): the file
in these bytes, the seven commands, and the pane less its search filter and
the "Bookmark group" dropdown.

**Unprobed:** what clicking a search or heading bookmark in the pane does (the
clicks dispatched in this run landed on nothing); the item context menu and the
file menu's "Bookmark..." entries (the explorer was hidden behind the Search
pane when they were tried); the order of `folder` and `url` keys as the UI
writes them; the "Bookmark group" dropdown; renaming and dragging items; the
search filter.

## Observed file explorer header behavior, Obsidian 1.14.4 (installer build 1.13.7)

Read from a running Obsidian on Linux over CDP on 2026-10-21 with
`tools/obsidian-probe/explorer-header.mjs` (raw run in
`tools/obsidian-probe/results/2026-10-21-explorer-header.json`), on a scratch
vault of ten files in four folders, created in one order and given
modification times in another so the time sorts could be told apart.

**The header.** `.nav-header` is 40 px tall with 8 px of padding and no
border; inside it `.nav-buttons-container` is a flex row, `justify-content:
center`, `gap: 2px`, `flex-wrap: wrap`. Five `clickable-icon nav-action-button`
of 28 by 24 (padding `4px 6px`, radius 4 px, `--text-muted`), in order, each
with its `aria-label` as the tooltip (`.tooltip` below the button):

| Label | Icon | Does |
| --- | --- | --- |
| New note | `lucide-edit` (Lucide's `square-pen`) | what `file-explorer:new-file` does: `Untitled.md` in the folder `newFileLocation` names (`root` → the vault root, `current` → the open note's folder), opened in a new tab with the inline title focused |
| New folder | `lucide-folder-plus` | `Untitled/` at the vault root, its row in inline rename (`is-being-renamed`, `contenteditable`); Escape keeps the folder under that name |
| Change sort order | `lucide-sort-asc` (`arrow-up-narrow-wide`) | the menu below |
| Auto-reveal current file | `lucide-gallery-vertical` | toggles the leaf's `autoReveal`; the button carries `is-active` while on |
| Expand all | `lucide-chevrons-up-down` | opens every folder at every depth; while any folder is open the same button reads "Collapse all" with `lucide-chevrons-down-up` and closes them all |

A sixth, "Show search filter" (`lucide-search`), is in the DOM with a zero
rect in this build. The last button's label follows the tree, not the last
press: one folder opened by hand turns "Expand all" into "Collapse all".

**The sort menu.** Six items in three pairs separated by `menu-separator`,
the one in force `mod-checked` with a `lucide-check` icon: "File name (A to
Z)", "File name (Z to A)", "Modified time (new to old)", "Modified time (old
to new)", "Created time (new to old)", "Created time (old to new)". The leaf's
`sortOrder` takes `alphabetical`, `alphabeticalReverse`, `byModifiedTime`,
`byModifiedTimeReverse`, `byCreatedTime`, `byCreatedTimeReverse`; it is
written into the file-explorer leaf's `state` in `workspace.json` (key order
`sortOrder`, `autoReveal`, `showSearch`, `searchQuery`) and `app.json` stays
`{}` — the `fileSortOrder` key of [[#Observed settings write behavior, Obsidian 1.14.4]] is not what the
header writes. **Folders keep their name order under every sort**; only the
files of each folder move. Observed, with folders `Archive/old`, `Empty`,
`Projects/{Deep/Inner, Budget, Plan}` and root files `Alpha`, `beta`, `cover.png`,
`Gamma`, `n2`, `n10`:

| `sortOrder` | Root files | Projects |
| --- | --- | --- |
| `alphabetical` | Alpha, beta, cover, Gamma, n2, n10 | Budget, Plan |
| `alphabeticalReverse` | n10, n2, Gamma, cover, beta, Alpha | Plan, Budget |
| `byModifiedTime` | newest first by mtime | newest first |
| `byModifiedTimeReverse` | the exact reverse | the exact reverse |
| `byCreatedTime` | newest first by birth time (`cover.png`, created last, led) | newest first |
| `byCreatedTimeReverse` | the exact reverse | the exact reverse |

**Auto-reveal.** Off in a fresh vault (`autoReveal: false`; `view.autoReveal`
is undefined, the flag lives in the state). With it off, opening
`Projects/Deep/Inner.md` through a link expanded nothing: the tree stayed as
it was and no visible row was `is-active`. `file-explorer:reveal-active-file`
then expanded `Projects` and `Deep` and marked `Inner` active. Clicking the
button turned it on, wrote `autoReveal: true` and **revealed the open file at
once**; opening `Archive/old.md` next expanded `Archive` and marked its row.
Turning it off again left the tree expanded.

 implements this since 2026-10-21 ([[Development#Files header, 2026-10-21]]):
the five buttons with the same Lucide drawings and labels, the sort menu and
its six orders (files by name, mtime or birth time; folders by name), the
leaf's two keys read from and written back into `workspace.json`, and
`FileExplorer::follow`, which highlights the open file and only expands to it
with auto-reveal on. New folder goes through 's prompt modal rather
than inline rename, and the search filter is not built.

**Unprobed:** the computed colour and background of the `is-active` button
( paints the accent on the hover wash); whether a visible row of the
open file is `is-active` with auto-reveal off ( highlights it); the
menu's position relative to the button.

## Observed file explorer keyboard behavior, Obsidian 1.14.4 (installer build 1.13.7)

Read from a running Obsidian on Linux over CDP on 2026-10-21 with
`tools/obsidian-probe/explorer-keyboard.mjs` (raw run in
`tools/obsidian-probe/results/2026-10-21-explorer-keyboard.json`), on a scratch
vault of seven notes in three folders. The pane's `tree` object exposes
`focusedItem`, and one `.tree-item-self` carries `has-focus` for it.

**Getting the keyboard into the tree.** `file-explorer:reveal-active-file`
expands to the open file, marks its row `has-focus` and makes the explorer
the active leaf; `file-explorer:open` makes the explorer the active leaf and
leaves `has-focus` where it was (or nowhere). A click on a row that opens a
file gives the keyboard to the editor (`cm-content` is the active element,
the explorer is not the active leaf, and the arrows then move the caret); a
click on the row of the file that is already open puts `has-focus` on it with
the explorer active; a click on a folder toggles it, makes the explorer
active and sets no `has-focus` — but the keyboard position is that folder:
Down then lands on the row under it.

**The keys**, with `has-focus` on a row and the explorer active:

| Key | On a file | On a folder |
| --- | --- | --- |
| Down / Up | the next / previous visible row, clamped at both ends; the file does not open | the same |
| Enter | opens the file; the keyboard goes to the editor; the row keeps `has-focus` and gains `is-active` | toggles the folder |
| Space | nothing | toggles the folder |
| Right | — (not probed) | collapsed: expands it, ring stays; open: the ring moves to its first child |
| Left | the ring moves to the parent folder, which collapses | open: collapses it, ring stays; collapsed at the root: nothing |
| Home / End / PageUp | nothing | nothing |
| Escape | the keyboard goes to the editor; the row keeps `has-focus` | — |
| F2 | inline rename of the row (`tree-item-inner` becomes the active element, contenteditable) | — |
| Delete | the "Delete file" dialog: "Are you sure you want to delete “Alpha.md”? It will be moved to your system trash." — Cancel / Delete, Delete focused | — |
| a letter, Tab | nothing | — |

Ctrl+Enter on the already-open file put the keyboard in the editor, like Enter.

**The ring.** `.tree-item-self.has-focus`: `box-shadow: 0 0 0 2px rgb(85, 85, 85)`
in the dark theme (`--background-modifier-border-focus`), the text at
`--text-normal`; with `is-active` as well, the hover wash behind it.

 implements this since 2026-10-21 ([[Development#Explorer keyboard, 2026-10-21]]):
an "Explorer" key context on the tree with Up, Down, Left, Right, Enter,
Space, Escape, F2 (the rename prompt) and Delete (the delete prompt); the
ring is a 2 px border; the two commands and the three kinds of click focus as
above; with no ring on a visible row, Down and Up start from the open file's
row, or the first. The socket's `focus` method takes `explorer`.

**Unprobed:** Right on a file; where Down starts with no `focusedItem` at
all; the light theme's ring colour ( uses `#c0c0c0`); whether Up or
Down from a hidden `focusedItem` (its folder since collapsed) starts over.
