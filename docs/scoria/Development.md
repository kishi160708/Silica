---
tags:
  - /dev
---
# Development

## Environment

The reference development machine runs NixOS on Wayland with an NVIDIA GPU. The GPUI app links
libxcb and libxkbcommon and loads Wayland, Vulkan, and GL at run time, so run cargo commands that
touch the `` crate inside `nix-shell` (see `shell.nix` at the repository root).

> [!warning] Pinned dependencies
> `gpui` and `gpui_platform` are git dependencies pinned to a Zed release commit; the crates.io
> release crashes on NVIDIA. `libc` is pinned in `Cargo.lock` because an old `xattr` in gpui's tree
> needs a constant newer libc removed. Both pins are explained in the root `Cargo.toml`.

## Commands

```
cargo test --workspace
cargo run --release -p  -- <vault>                      # Files, reading view, Backlinks/Outline/Tags
cargo run --release -p  -- <vault> --note "Name#Heading" # open a note, scrolled to a heading or ^block
cargo run --release -p  -- <vault> --search "tag:#x OR y"  # open with the Search tab filled (for screenshots)
cargo run --release -p  -- <vault> --switcher "spc"        # open with the quick switcher up (for screenshots)
cargo run --release -p  -- <vault> --pane outgoing          # right dock tab: backlinks, outgoing, outline, tags
cargo run --release -p  -- <vault> --graph [local]          # open on the graph view
cargo run --release -p  -- <vault> --palette "open"         # open with the command palette up
cargo run --release -p  -- <vault> --settings hotkeys       # open Settings on a tab (general, appearance, editor, ...)
cargo run --release -p  -- <vault> --note X --source        # open a note in Source mode (--reading for reading view; the default is the vault's defaultViewMode)
cargo run --release -p  -- <vault> --note X --stress 40     # self-driven edits on that note, then autosave (use a scratch vault)
cargo run --release -p  -- <vault> --note X --source --type '[[al'          # type at the cursor: opens the autocompletion popup for a screenshot
cargo run --release -p  -- <vault> --note X --source --type '[[al' --accept down,enter   # and press the popup's keys (enter, tab, shift-enter, up, down)
cargo run --release -p  -- <vault> --note X --find word     # open with the find panel up and the query matched
cargo run --release -p  -- <vault> --note X --prompt rename  # open the rename prompt (move, delete, folder) for screenshots
cargo run --release -p  -- <vault> --note X --rename "New"   # rename the note at startup, rewriting links (use a scratch vault)
cargo run --release -p -bench -- generate <dir>
cargo run --release -p -bench -- index <vault>
```

In the app: click a file or a link to open it, Ctrl+P opens the command
palette, Ctrl+E toggles reading view and editing (Live Preview unless
`livePreview` is off), `editor:toggle-source` from the palette switches Live
Preview and Source mode, Ctrl+F and Ctrl+H find and replace in the note,
Ctrl+Alt+Left and Right walk the tab's history,
Ctrl+O opens the quick switcher, Ctrl+Shift+F the Search tab, Ctrl+G the
graph (again to return), Ctrl+, opens Settings, Ctrl+S saves (edits also save
on their own two seconds after the last keystroke), Ctrl+N creates a note,
F2 renames the open one, Ctrl+Q quits. A note opens in the vault's
`defaultViewMode` (editing unless it says `preview`). Every shortcut except
Ctrl+Q is an Obsidian command bound from its default hotkey, so the vault's
`hotkeys.json` rebinds or unbinds it. Right-click a file or folder in Files
for new note, new folder, rename, move, copy and delete; clicking a link to a
note that does not exist, or pressing Enter in the switcher on a name nothing
matches, creates it. In the graph: wheel zooms, drag pans, drag a node to
move it, click a node to open it, Ctrl+0 refits the view.

## Releases

`.github/workflows/release.yml` and `.forgejo/workflows/release.yml` both fire on a `v*` tag. Cut a
release by bumping `version` under `[workspace.package]` in the root `Cargo.toml`, committing that,
then tagging and pushing to both remotes:

```
git tag -a v0.1.0 -m " 0.1.0"
git push origin v0.1.0 && git push forgejo v0.1.0
```

Either workflow refuses to build when the tag and the workspace version disagree, since the archive
is named from the tag and the binary reports the manifest version. GitHub builds
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` — one archive per
target, plus `SHA256SUMS` over all of them — and publishes with `gh release create`. The homelab
Forgejo runner has only the Linux container (see [[#Environment]] and `ci.yml`), so it
builds the x86_64 Linux archive and creates the release through the instance API with the token
Forgejo Actions injects for the run. Release notes on both are the commit subjects since the
previous tag. A tag with a hyphen in it, `v0.2.0-rc1`, is published as a prerelease.

Both builds pass `--locked`, so the pinned `gpui` revision and `libc` come from `Cargo.lock`.

> [!warning] The Windows and macOS binaries are untested
> CI only runs `cargo check -p ` on those platforms and the release job is the first thing
> that links them; neither has been started on a real GPU. Nothing is code-signed or notarized, so
> macOS Gatekeeper and Windows SmartScreen will both object. Linux x86_64 is the only build that
> runs on the reference machine.

## Reader (phase 2)

Started 2026-10-16, after the three spike gates passed. Gate: a daily user can
browse their real vault with no missing content. Passed 2026-10-16, see
[[#Reader gate, 2026-10-16]].

| Reader item | Status |
| --- | --- |
| Open a vault | done: scanned and indexed on a background thread, `.obsidian` config read |
| Files view | done: folder tree, folders first, collapse state, auto-reveal, attachments shown, unknown kinds hidden |
| Reading view | done for text content; see the list below |
| Backlinks | done: linked mentions with the source line, unlinked mentions (names and aliases in plain text outside links and code) |
| Outgoing links | done: resolved and unresolved links of the note, plus unlinked mentions of other notes in it |
| File watching | done: changed, added, removed and moved files re-index in place, links re-resolve, panes and search refresh, an open note reloads keeping its scroll position |
| Outline | done: click scrolls to the heading |
| Tags | done: vault-wide counts merged across inline and frontmatter tags |
| Search | done: full query grammar with Obsidian's explain labels, results grouped per file with context, click opens at the line; sort options and copy results not yet |
| Quick switcher | done: fuzzy over names, aliases and paths, recent files first, unresolved targets shown as not created; Enter on one creates the note |
| Graph | done: global and local graphs, filters, color groups, forces and display settings from `graph.json`; zoom, pan, drag, hover, click opens |
| Command palette | done: Obsidian's command names and hotkeys, pinned commands from `command-palette.json`, this session's recent commands first; lists only the commands  can run |
| Settings, read-only | done: General, Appearance, Interface, Editor, Files and links, Hotkeys, Core plugins, Community plugins and the Quick switcher and Command palette plugin tabs, every value from `.obsidian/`; nothing is written |

## Writer (phase 3)

Started 2026-10-17. Gate: the vault round-trips byte-identical after a week
of mixed use in both apps.

| Writer item | Status |
| --- | --- |
| Source mode | done, see [[#Source editor, 2026-10-16]] |
| Live Preview | done, see [[#Live Preview, 2026-10-16]] |
| File operations and link rewriting | done, see [[#File operations, 2026-10-17]] |
| Tables (editing) | not started: widgets render, cells are not editable |
| Properties | done, see [[#Properties, 2026-10-18]] |
| Autocompletion | links, headings, aliases, embeds and tags, see [[#Autocompletion, 2026-10-18]]; block references are not offered yet |
| Templates, daily notes, file recovery | not started |
| Writing settings back | started: every Settings toggle writes, see [[#Settings writing, 2026-10-18]]; dropdowns, text fields and sliders are still read-only |
| Round-trip harness | done, see [[#Round-trip harness, 2026-10-18]] |

### Autocompletion, 2026-10-18

Typing `[[`, `![[` or `#` opens a popup over the editor. The rows are the
vault's files, their aliases, a note's headings and the vault's tags;
Enter, Tab and Shift+Enter accept one three different ways.

What Obsidian does here was observed rather than guessed. Its four
`app.workspace.editorSuggest.suggests` instances are drivable from a
throwaway plugin (see [[obsidian-behavior-probes]] in the session notes and
[[#File operations, 2026-10-17]] for the method): `onTrigger` was called for
37 shapes of line, `getSuggestions` for what each offers, and the popup's
own `scope` key handlers for what every accept key writes, across both link
styles, all three `newLinkFormat` values and a source note in a subfolder.
Three runs; the results are written up in
[[Specification#Observed autocompletion behavior, Obsidian 1.14.4]] and
encoded row for row in `-suggest/tests/obsidian_rows.rs`.

What the probes settled that guessing would have got wrong:

- **Tab is not Enter.** Enter finishes the link and leaves the cursor after
  the `]]`; Tab leaves it *inside*, so a `#heading` or `|alias` can follow
  what was just accepted. Shift+Enter ignores the selected row and links the
  text as typed. On a tag, Enter writes a trailing space and Tab does not.
- **A heading pick keeps the path as typed** and never takes display text:
  `[[Sub/Gamma#` stays `Sub/Gamma` where `shortest` would have written
  `Gamma`. Every other pick is reformatted to the vault's `newLinkFormat`.
- **Display text follows the path, not the name.** `|Alpha` is added exactly
  when the path part carries a `/`, and only for notes, so `[[Sub/Alpha|Alpha]]`
  but `[[../pic.png]]`.
- **An empty `[[` is ordered by modification time**, newest first — not by
  recency of opening, which is what the quick switcher uses. The list is also
  files and aliases only: the switcher offers unresolved link targets, this
  does not.
- Nothing suppresses the trigger: it fires inside a code span, a fenced
  block, a `%%comment%%` and frontmatter alike. That is why the trigger is a
  scan back along one line rather than a parse.

The code is a new crate, `-suggest`, free of gpui like `-text`:
`trigger` (what the line asks for), `suggestions` (what the vault offers)
and `insert` (the bytes accepting one writes, reusing
`_cache::links::link_path` so there is one link formatter). The GPUI
side is `/src/suggest.rs`, a list anchored under the query, and about
sixty lines in the editor that hand the popup the keys it owns while it is
open. `VaultFile` grew an `mtime` for the ordering above; the metadata was
already being read for `size`.

The Writer gate's harness covers the new write path: accepting a suggestion
at five places in every corpus note replaces the query and nothing else, and
undoes back to the file's bytes — over the real vault too.

Ranking is 's own fuzzy matcher (the one behind the quick switcher), so
the order within a query can differ from Obsidian's; what each row *writes*
is what matches.

**Not done:** block references. `[[Note#^` offers nothing rather than
offering a row it cannot write, because accepting a block with no id of its
own means writing a six-character id into the *target* note — inline on a
paragraph, on its own line after a heading. That is a write into a second
file and wants its own change. `Ctrl+P`/`Ctrl+N`, which Obsidian's popup also
binds to the selection, are left to 's own bindings.

### Settings writing, 2026-10-18

Every toggle in the Settings modal writes. A click clones the config the
control reads, flips its one field and emits it; `Workspace::change_setting`
writes the file through `Vault::write_config` — which merges into the file's
own JSON, so only that key moves — stores it in the `VaultState` and pushes
what the running app can change into the open views. The banner now says
which controls write; dropdowns, text fields and sliders are still read-only.

What a toggle writes is a `Flag`: the key Obsidian writes, what the key
means when it is absent, and the accessors for it. The control's state is
read through the same flag that a click writes, so the two cannot drift
apart, and a test walks every flag of every file to check that toggling
writes that key, flips it, and touches nothing else. The one control
Obsidian words backwards ("Use [[Wikilinks]]" is `useMarkdownLinks`
negated) carries an `invert` bit rather than its own code path.

The config a toggle changes has to be mutable while the vault is open, so
`app`, `appearance`, `core_plugins`, `community_plugins` and `switcher` moved
behind locks in `VaultState`, as `property_types` already was, and are read
through accessors.

Applied on the spot, from `Workspace::apply_app_config`: readable line
length, smart lists, indent with tabs or spaces, properties in document and
strict line breaks (the editor rebuilds its widgets), plus show-all-file-types
in Files. The rest (inline title, folding, line numbers, RTL, spellcheck,
auto-pairing, Vim mode, native menus, the ribbon, tab title bars) is written
for Obsidian to honor;  does not implement those yet.

Which is which is now on the row. Every item carries an `Honored`: nothing
on a setting  follows as you click it, "on restart" for the theme and
the fonts (the `Theme` global is built once from `appearance.json` at
startup), and "not in  yet" for everything written only for Obsidian
to read — the whole Core plugins tab included, since 's own Files,
Search and Graph do not turn off with the plugins that own them in Obsidian.
The vault is shared, so writing a setting  ignores is worth doing;
letting a click look like it changed something here is not. A test walks
every tab and asserts that exactly seven toggles go untagged, so
implementing one of the tagged settings means dropping its tag in the same
change.

Two rules came out of the shapes of the files themselves rather than a probe:
a vault with no `core-plugins.json` gets a whole map of every plugin at its
default, because a one-key file would read as "everything else off", and the
legacy array shape keeps its shape. **Unprobed:** whether Obsidian removes a
key when a setting goes back to its default.  writes the explicit
value, so toggling off and on again can leave a key Obsidian would not have
written (the value is the one Obsidian would read either way).

### Round-trip harness, 2026-10-18

The Writer gate — the vault round-trips byte-identical after a week of mixed
use — now has a harness:
`crates/-text/tests/writer_roundtrip.rs`, over the 1000-note corpus and
both `.obsidian` sets in the repository. Four properties, one per shape of
writing:

- opening a note and saving it changes no bytes;
- every editing command undoes back to the file's bytes (a scripted session
  of typing, Enter, Tab, Shift+Tab, Backspace, Delete, Ctrl+Backspace,
  paste, typing over a selection and cutting a line, at five places in every
  note, wound all the way back);
- an edit is local: typing changes the bytes at the cursor and nothing else,
  and a property write stays inside its block;
- a config file read and written back is unchanged.

Invariants are checked after every step, not just at the end: the line index
is the one the text implies, every cursor sits on a character boundary, and
what a save would write reads back as the text in hand. Setting
`SCORIA_ROUNDTRIP_VAULT` to a vault's path runs the same round trips over it
(nothing is written — the edits happen in a buffer in memory), which is how
the gate's real vault is checked without a test depending on one.

To make the editor testable the text model moved out of the GPUI view into
`crates/-text`: `TextBuffer` (the text, its line index, the selections,
the undo history and one method per editing command), `FileText`, the
`lists`, `find` and edit-plan modules, all free of gpui.
`crates//src/editor.rs` keeps the shaping, scrolling, decorations, find
panel and properties table and holds a `TextBuffer`; its actions are now
`buf.<command>()` followed by `after_edit`. Nothing changed in behavior —
the suite and a 400-step stress run on a 20k-line note are the evidence.

Writing the harness found three bugs on the spot:

- **A stray `\r\n` rewrote a whole file.** `FileText` took any `\r\n` as
  "this file uses CRLF", so a note from a real vault with one such line
  among 4430 Unix ones would have had every line ending rewritten by the
  first save. The ending is now the dominant one, and a file whose endings
  disagree is kept verbatim and written back unchanged until it is edited.
- **`core-plugins.json` and `hotkeys.json` were sorted maps.** Obsidian
  writes both in its own order, so the first save would have reordered every
  line. Both are `IndexMap` now, like `types.json`.
- **A struct cannot reproduce a config file.** Serializing `GraphConfig`
  gave Obsidian's `graph.json` back with its keys in declaration order, `1.0`
  where the file said `1`, `0.51871324` where it said `0.518713248970312`
  (an `f32` field) and no `colorGroups` at all (skipped when empty).
  `Vault::write_config` now merges the new value into the file's own JSON:
  a key whose value did not change keeps its bytes and its place, a changed
  one is replaced where it stands, a new one is appended, and a key the type
  does not serialize is left alone.

Not covered here, by design: link rewriting on rename
(`-cache/tests/rename_corpus.rs`) and the properties planner
(`-cache/tests/properties_corpus.rs`) have their own corpus ratchets;
this harness drives the writer's path into them rather than their internals.

### Properties, 2026-10-18

The frontmatter block is now a typed, editable properties table in Live
Preview and a read-only one in the reading view.
`-formats::properties` is the model: it scans the YAML into ordered
entries with byte spans, reads each value with `serde_yaml`, infers types,
serializes values the way Obsidian writes them, and plans edits as byte
ranges. `-cache::note_properties` is the one place that joins it to
`extract_frontmatter`. `::properties` turns entries into display rows
and holds the field-editing rules as pure functions; `::reading::view`
draws the rows (read-only, or editable when Live Preview passes a
`PropertiesHost`) and `::editor` owns the open field, routes keys into
it and applies a commit.

The rules came from Obsidian 1.14.4 directly, with the same throwaway-plugin
method as the file operations: `-probe` in a scratch vault drove
`processFrontMatter` through about forty scenarios, dumped
`registeredTypeWidgets` and the inferred and assigned type of every value
shape, ran the three property commands, and read every file back byte for
byte. What it showed is listed in [[Specification#Observed properties
behavior, Obsidian 1.14.4]] and encoded in the tests. The headline:

- Obsidian rewrites the **whole** block on any property write. A
  `processFrontMatter` call that mutates nothing still deletes YAML comments,
  turns `tags: [one, two]` into a block sequence and drops blank lines. A
  block already in that canonical form round-trips byte-identical.
- So  writes Obsidian's canonical form, but only for the entries whose
  value actually changes. `plan` compares *values*, not rendered text, which
  is what keeps a flow sequence and its quoting when the value is unchanged;
  everything else in the block keeps its bytes.

`crates/-cache/tests/properties_corpus.rs` is the ratchet over the
1005 corpus notes that have frontmatter: rewriting every value with itself
plans no edit, every entry span really covers its key and stays inside the
block, a real edit leaves the bytes before and after the entry identical,
clearing leaves exactly the body (plus a BOM where there is one), and adding
a property appends one line and reorders nothing. Writing the ratchet found
two bugs on the spot: bare dates were being quoted on rewrite, and an
unchanged flow list was being reflowed.

The UI: a foldable "Properties" heading, then a row per property with its
type glyph, key and value. Lists, tags and aliases draw as pills; a
wikilink value draws as the link (its alias, or the target without a
subpath) and follows on click; a tag item is clickable whether or not it is
written with `#`; a checkbox draws a box; an empty value shows a
placeholder; a value that does not fit its type is marked and kept as
written. In Live Preview a click on a key or value opens a field — the
editor routes typing, the arrows, Home, End, Backspace and Delete into it,
Enter commits, Tab commits a key and moves to its value, Escape abandons —
and a commit is one undo step through `apply_edits`. The row icon opens the
type menu, the `×` removes the property, `+ Add property` opens an unnamed
row that writes nothing until it is named. The three commands
(`markdown:add-metadata-property`, `markdown:clear-metadata-properties`,
`editor:toggle-fold-properties`) are runnable, and `propertiesInDocument`
picks the table, nothing or the raw YAML.

Choosing a type writes `.obsidian/types.json`, the first config file 
writes: `Vault::write_config` serializes with two-space indentation and no
trailing newline, as Obsidian does, and `PropertyTypes` keeps insertion order
(an `IndexMap`) and its unknown keys so the file does not churn.

Two parser corrections fell out of the probe. YAML's `...` does not close a
properties block in Obsidian, and no longer does in `extract_frontmatter`.
Obsidian also does not recognise frontmatter after a byte order mark and
prepends a second block when a property is added;  keeps reading
through the BOM and edits the block in place, which is a deliberate
divergence from a case where Obsidian loses the original block. Obsidian
likewise leaves mixed line endings when it writes into a CRLF block; 
writes the file's own ending.

Not yet: editing properties in the reading view (Obsidian allows it there
too), drag-to-reorder, a calendar picker for `date` and `datetime`, value
autocompletion from other notes, and the Properties core-plugin side panes
(`properties:open`, `properties:open-local`).

### File operations, 2026-10-17

New note (Ctrl+N, `file-explorer:new-file`, the Files menu, a click on a
link nobody has created, Enter in the switcher), new folder, rename (F2,
`workspace:edit-file-title`), move (`file-explorer:move-file`, a fuzzy
folder picker), make a copy, and delete (`app:delete-file`) exist, each
through a small prompt modal (`crates//src/prompt.rs`) or the
explorer's right-click menu. `-vault::ops` does the disk work and
updates the file and folder lists (the vault now tracks folders, so empty
ones show in Files); `-cache::links` plans the link rewrites;
`VaultState` in `workspace.rs` runs both and brings the index up to date
before the watcher's events arrive (they then find nothing to change).

The rules were taken from Obsidian 1.14.4 directly: a throwaway plugin in a
scratch vault (`-probe`, kept in the session's scratch space, not the
repository) renamed files through `fileManager.renameFile` under every
`newLinkFormat`, resolved links through `getFirstLinkpathDest`, created and
trashed files, and dumped the resulting notes. What it showed, now encoded
in the two modules' tests:

- Every link that resolved to the renamed file is regenerated with the
  vault's `newLinkFormat` whatever form it had (`[[a]]`, `[[A.md]]`,
  `[[Sub/A]]`, `[A](./A.md)`, `[A](/A.md)` all become `[[A2]]` or
  `[A2](A2.md)`); a link whose written path already matches is left byte
  for byte, so moving a note with a unique name changes nothing. Shortest
  format falls back to the full path when the basename is ambiguous.
- The style survives: wikilink or markdown, embed or link, angle-bracketed
  destination or not. Markdown destinations keep `.md` and encode only
  spaces (`Ä%20(b)%20100%.md`). The subpath is kept; the alias is kept
  verbatim, except that an alias equal to the old basename and different
  from the written path follows the new basename (`[[Sub/S|S]]` becomes
  `[[S2|S2]]`, `[[S|S]]` becomes `[[S2|S]]`). A markdown link's text follows
  the new basename when it was the old one, or the new link path when it was
  the old destination without `.md`.
- Code spans and fenced code are untouched; reference-style links and their
  definitions are not updated; `[[#Heading]]` self links are not touched.
- Links inside a moved note are regenerated only when they would stop
  resolving (`[B](../B.md)` after the note moved into a subfolder).
- Frontmatter links update. Obsidian re-serializes the whole YAML block when
  it does (`aliases: [Bee]` became a block list);  replaces the link
  text in place instead, keeping the file's bytes.
- `newFileLocation`: a new note is `Untitled.md`, `Untitled 1.md`, ... at
  the root, in the open note's folder, or in `newFileFolderPath` (falling
  back to the root when that folder is missing); a `folder/Name` link
  creates the folders. Creating over an existing path fails with "File
  already exists.", renaming onto one with "Destination file already
  exists!"; a case-only rename succeeds.
- `trashOption` defaults to `system` (freedesktop trash on Linux, with a
  `.trashinfo` record; the vault's `.trash` is the fallback elsewhere);
  `local` moves a file flat into `.trash/` and a folder with its contents,
  numbering collisions ` 2`, ` 3`. `promptDelete` defaults to on.
- `alwaysUpdateLinks` defaults to on; off asks first ("Update links?" with a
  "Don't update" button), as Obsidian's dialog does.

The same probe corrected link resolution: with duplicate basenames Obsidian
picks a file at the vault root first, then one in the source's own folder,
then the shortest path, then path order (`[[Zed]]` from `Long/Way/Down/C.md`
is `Zed.md`, not the sibling `Long/Way/Down/Zed.md`; `[[X]]` there is the
sibling `Long/Way/Down/X.md` over `Mid/X.md`). `./` and `../` paths resolve
relative to the source only, `/path` from the root only. The spec's
"same folder first" rule was wrong and is fixed in [[Specification]].

`crates/-cache/tests/rename_corpus.rs` renames every note of the
1000-note corpus, moves its folders under every format and renames its
attachments, and checks that every resolved link still resolves to the same
file, no link appears or disappears, and nothing outside the links changes.
Not yet: drag and drop in Files, inline title editing, attachment
pasting, the system trash on Windows and macOS.

### Reader gate, 2026-10-16

Checked against a real 10-note vault (project notes with wikilinks and
aliases, tables in nine of them, fenced ASCII diagrams, inline code, ordered
and bulleted lists, two `.canvas` files) by opening every note and the lower
sections of the long ones. Every block rendered with its content; backlinks,
outgoing links and unlinked mentions matched the text. Two fidelity bugs
found and fixed on the way: table columns were all equal, which broke long
identifiers mid-word (columns now follow HTML's automatic table layout, see
below), and fenced code wider than the note wrapped instead of scrolling.

Still not rendered, none of it present in that vault: MathJax (math shows as
its source), HTML tags (shown as text), remote images, Canvas and Bases
files (Parity phase), and the local graph beyond depth one.

### Command palette and Settings, 2026-10-16

`-commands` now carries the command names (`data/names.txt`) and
default hotkeys as Obsidian 1.14.4 registers them, captured from the running
app with a throwaway plugin that dumped `app.commands` and the settings
tabs; the ids gained the nineteen 1.14.4 additions. `CommandRegistry`
keeps registration order, applies `hotkeys.json` (an entry replaces the
defaults, an empty list unbinds) and turns a binding into a label
(`Ctrl + Shift + F`) or a GPUI keystroke. At startup the workspace binds a
`RunCommand { id }` action for every command it can run, so the shell has
no hard-coded shortcuts left; `Workspace::run_command` dispatches by id.

The palette (`crates//src/palette.rs`) shows only runnable commands,
the way Obsidian hides commands whose check callback fails: palette,
switcher, search, graph and local graph, reading view toggle, back and
forward, settings, sidebar toggles, Files, reveal, Backlinks, Outgoing
links, Outline and Tags. Everything else stays visible in the Hotkeys tab.

Settings (`crates//src/settings.rs`) is a modal with Obsidian's
navigation and setting rows (name, description, control). Controls are
drawn in their current state from `app.json`, `appearance.json`,
`core-plugins.json`, `community-plugins.json` and the plugin manifests,
`switcher.json` and `command-palette.json`; the Hotkeys tab filters all,
assigned, unassigned and assigned-by-me over the registry and marks custom
bindings. A banner says the view is read-only; writing settings waits for
the writer milestone because Obsidian rewrites the whole file on every
change. Not shown: settings whose `app.json` key is unobserved (default file
to open, delete attachments, Mermaid, quick font size, zoom, frame style,
URI callbacks), the Keychain tab, and options of other core plugins.

### Reading view, 2026-10-16

`crates//src/reading/model.rs` turns the parser tree into an owned
render tree (styled runs with clickable ranges, resolved embeds, footnotes in
reference order, frontmatter as properties); `view.rs` draws it with a lazily
measured `list`, one item per top-level block, so a 50k-line note lays out
only what is on screen. Clicks on wikilinks, markdown links, tags and footnote
references go back to the workspace, which resolves them with the vault
cache: `[[Note#Heading]]` and `[[Note#^id]]` scroll to the target block.

Rendered: headings, paragraphs with soft breaks, emphasis, strong, strike,
highlight, inline code, inline math (raw), tags, images (`![[pic.png|200x100]]`
sizes), embedded notes, sections and blocks to a depth of four, lists with
numbers and task checkboxes, block quotes, callouts with fold state and
Obsidian's type colors, tables with column alignment, code blocks with a
language label, display math (raw), raw HTML (as text), footnotes.

Tables lay their columns out the way an HTML table's automatic layout does:
each column wants its widest cell and needs at least its longest word (both
measured with the real font), the table is as wide as the sum when that
fits the note, and otherwise the slack between minimum and wish is shared
in proportion. The view records its painted width for this and re-lays out
when it changes. Code blocks never wrap; long lines scroll sideways.

Not yet: Live Preview, MathJax, HTML rendering, remote images, checkbox
toggling, heading folding, copy buttons, PDF export.

### Graph view, 2026-10-16

`crates//src/graph/layout.rs` is the force simulation: Barnes-Hut
repulsion over a quadtree, springs along links at `linkDistance`, a weak
pull to the center, simulated annealing that stops when cool; drags reheat
it. `view.rs` builds nodes and edges from the cache on the background
executor (notes, optional attachments, unresolved targets and tags, orphans
on or off, the `search` filter and `colorGroups` from `graph.json` evaluated
with the search grammar), steps the layout in the background, and paints
through a canvas: one stroked path for the edges, circles for nodes, labels
once zoomed past a threshold that grows with node count. The local graph
keeps the current note and its depth-1 neighbors; opening another note
re-centers it. The view auto-fits until the user zooms or pans.

Not yet: depth beyond one and the local-graph side pane, arrows, the
timelapse animation, screenshot export, writing display changes back to
`graph.json`.

### File watching and mentions, 2026-10-16

`notify` watches the vault root; the workspace drains the event channel every
300 ms, so a burst of writes becomes one batch. `Vault::refresh_path` turns
each path into added, modified or removed files (walking a moved-in folder,
dropping everything under a removed one, ignoring dot-folders), then
`VaultState::apply` re-reads and re-indexes the notes among them under write
locks and calls `MetadataCache::reresolve` when files appeared or vanished,
because links elsewhere may now resolve. Unlinked mentions use one regex
alternation over every note name and alias (`mentions::Finder`), skipping
matches inside wikilinks, markdown links, inline code and fenced code.

### Search and quick switcher, 2026-10-16

`crates/-search` implements the query grammar once: implicit AND,
uppercase `OR`, parentheses, `-` negation, quoted phrases with `\"`,
`/regex/flags`, the `file:` `path:` `content:` `tag:` `line:` `block:`
`section:` `task:` `task-todo:` `task-done:` `match-case:` `ignore-case:`
operators, and `[prop]`, `[prop:value]`, `[prop:null]`, `[prop:true]`,
`[prop:>n]` properties. `Query::explain` prints the tree with the labels
Obsidian's "Explain search terms" shows, which the tests check. Matching is
regex-based (case-insensitive unless `match-case:`) and returns byte ranges
for highlighting; scoped operators evaluate per line, block, section or task.
The Search tab runs the compiled query over the indexed note texts on the
background executor with rayon and lists results per file. The fuzzy scorer
(`fuzzy_match`) rewards word starts, consecutive runs and short candidates;
the switcher ranks file names, aliases and paths with it. The text field is a
single-line `TextInput` painted through a canvas element with GPUI's input
handler, so IME works.

> [!note] Fonts
> The pinned gpui does not pick weights out of a variable font, so with Noto
> Sans (one `NotoSans.ttf` for every weight) nothing renders bold. The default
> text and interface font is Liberation Sans, which ships static Bold faces;
> `appearance.json` fonts override it.

## Spike gates

The [[Specification]] sets three gates before any reader or writer work starts.

| Gate | Target | Status |
| --- | --- | --- |
| Editor prototype | 50k-line note with wikilink and heading decorations | passed, see below |
| Parser conformance | 1000-note HTML diff corpus at 99 percent | passed: 1007/1009 notes against Obsidian 1.13.7, plus CommonMark 652/652 |
| Index performance | 20k-file vault indexed cold in under 2 s | measured, see below |

### Source editor, 2026-10-16

`crates//src/editor.rs` is a plain-text Markdown editor over one `String`
with a line index. A custom GPUI element shapes and paints only the visible
lines; decorations come from `_markdown::highlight`, recomputed on a
background thread after each edit (one parse in flight, one queued) and
swapped in when ready, so typing never waits for the parser. Input goes
through GPUI's input handler scoped to the cursor line, so IME composition
works; everything else is an action.

Grown from the spike prototype the same day: soft wrap at the view width
(capped at the readable line length and centered, like Obsidian), a
selection (shift with every movement key, drag, double-click word,
triple-click line, Ctrl+A), word movement and deletion, undo and redo with
CodeMirror's grouping (consecutive typing or deleting within half a second
is one step), clipboard copy, cut and paste (the whole line when nothing is
selected), and the note's text font with the monospace font for code. The
scroll position is a line plus a pixel offset into it rather than an
absolute pixel, so wrapped-row heights are only measured near the viewport
and a 50k-line note never needs a full layout pass; Page Up and Down,
revealing the cursor and clamping at the end all walk rows from that anchor.

Saving: the buffer holds `\n`; the file's BOM and `\r\n` endings are
remembered at load and put back on write, so a save changes nothing but the
edited bytes (verified with `--stress` on a BOM-and-CRLF note in a scratch
vault). Edits save two seconds after the last keystroke, on Ctrl+S
(`editor:save-file`), when another note opens, when the view switches to
reading mode (which then rebuilds from the saved text), and when the window
closes. The watcher's reload of the open note is skipped when the file is
what the editor last wrote, keeps the cursor when the note changed
underneath an unedited buffer, and leaves unsaved edits alone otherwise.

```
cargo run --release -p  -- docs/ --synthetic 50000          # open a generated 50k-line note
cargo run --release -p  -- docs/ --synthetic 50000 --stress  # 600 self-driven edits, then a report
```

Stress run on the reference machine with soft wrap on, 50,001 lines (4.3 MB,
244k decorated spans; the synthetic note is far denser in syntax than a real
one):

| Measure | Result |
| --- | --- |
| Frames painted during 600 edit, selection, undo and navigation steps in 7 s | 607 |
| Shaping the visible lines, per frame | 0.17 ms mean, 1.95 ms max |
| Full reparse plus decoration pass (background) | 222 ms mean |
| Edit to refreshed decorations | 244 ms |

The frame path is the gate: scrolling and typing stay far under a frame
budget regardless of note size. Decoration latency scales with note length
because the whole note is reparsed; a real 50k-line note of ordinary prose is
about half this size, and incremental reparse of the edited block is the
planned follow-up. Not yet: auto-pair of brackets and markdown markers, and
Ctrl+click page previews.

### Find and replace, 2026-10-16

Ctrl+F (`editor:open-search`) opens Obsidian's search panel at the top
right of the editor, switching to editing first; Ctrl+H
(`editor:open-search-replace`) adds the replace row, which a toggle on the
panel also shows. The query starts as the selection when it sits on one
line. Matching is case-insensitive and literal (`crates//src/find.rs`,
capped at 20k matches); every match is tinted, the selected one more so,
and the panel counts them. Typing selects the first match from the cursor
on, Enter and the arrows step forward and back with wrap-around, Enter in
the replace field or the Replace button replaces the selected match and
steps on, Replace all does every match in one undo step, Escape closes.
Matches refresh after every edit. `--find q` opens a note with the panel
up, for screenshots.

### Smart lists and the scrollbar, 2026-10-16

Enter with `smartIndentList` continues what the line started: its quote
markers, its bullet or the next number, and an empty checkbox after a task;
the rest of a split item moves to the new line and the ordered items that
follow renumber in the same undo step. Enter on an empty item clears it, or
outdents it when nested. Shift+Enter is a plain newline. Tab indents a list
item (or every selected line) by one step, a tab or `tabSize` spaces per
`useTab`, and Shift+Tab removes one. `crates//src/lists.rs` holds the
prefix parser and the rules as pure functions with tests.

The vertical scrollbar is line-based like the scroll position: the thumb
sits at the first visible line's share of the note and is as tall as the
visible lines' share, so it never needs the height of every line. Dragging
it or clicking the track scrolls to that line.

### Live Preview, 2026-10-16

Live Preview is the same editor and buffer with a per-line display
transform (`crates//src/preview.rs`): from the line's highlight spans
it hides markers or substitutes glyphs, and returns the display text with a
piece map between source and display offsets. Every construct the cursor
or selection touches keeps its source, so a line's markers appear as the
cursor enters them and go when it leaves, per construct rather than per
line, as in Obsidian. The editor shapes the display text and maps clicks,
the cursor, the selection and IME bounds through the pieces; Source mode is
the identity transform.

Rendered today: heading markers hidden and the line scaled (Obsidian's h1
to h6 sizes at a tighter line height, so rows now have per-line heights
and all scroll math walks those), bold, italic, strikethrough, highlight and
inline code markers, wikilinks shown as their alias or target, markdown
links and images as their text, autolink brackets, footnote brackets,
comments and block ids hidden, bullets as dots, task markers as checkboxes
(click toggles them without moving the cursor), quote markers as a left bar
per level, callout type blocks folded into the title, thematic breaks as a
rule, and code blocks and frontmatter on a band. Clicking a rendered link
follows it (Ctrl+click in both editing modes). Tables, embeds, images and
math showed their source with decorations until the widgets below.

`crates/-markdown::highlight` gained a `QuoteMarker` span per `>` so
the transform knows where each level's marker sits. The 300-step stress run
on a 530-line note in Live Preview shaped in 0.07 ms per frame; the 50k-line
Source mode gate is unchanged at 0.17 ms.

### Regex search, multiple cursors and block widgets, 2026-10-17

The find panel gained Obsidian's two toggles, `Aa` (match case) and `.*`
(regex), both off by default; `crates//src/find.rs` now compiles the
query with the `regex` crate (a literal query is escaped), skips empty
matches and treats a pattern that does not parse as no results. A regex
replacement expands `$&`, `$1` to `$9` and `$$`. Find all (the button, or
Alt+Enter in the query) turns every match into a selection, closes the panel
and focuses the editor.

The editor keeps extra selections beside the primary one (`extra`, sorted and
disjoint); `cursors()` and `set_cursors()` move between that and a plain
list, merging selections that overlap. Typing, Backspace, Delete, word
deletion, Enter (a plain newline when there are several cursors), Tab, cut,
copy and paste go through `edit_all`, one replacement per selection in one
undo step; the history records the selections either side, so undo restores
all cursors, and a typing step joins the previous group when it starts
exactly where that one ended. Movement runs the same per-cursor target
function for every selection; `add_cursor` (the `editor:add-cursor-above`
and `below` commands) puts one a row above the topmost or below the
bottommost, Alt+click adds or removes one, Escape and a plain click
collapse to the primary. The status bar counts cursors. IME composition
still addresses the primary cursor only; the others are shifted past its
edit.

Tab and Shift+Tab with smart lists on now renumber: `lists::renumber_block`
walks the list block around the changed lines and gives every ordered item
from the first changed line down its previous sibling's number plus one,
or 1 when it has none (so an indented item starts a nested list at 1 and
the items it left behind close the gap). The edits are planned in pre-edit
coordinates (`indent_plan`) so the indentation and the new numbers are one
undo step.

Live Preview renders tables, math blocks and embed-only paragraphs as
widgets. The background parse also builds them (`reading::model::build_widgets`,
with the vault as embed source, so embedded notes and images resolve), and
the editor folds a widget's lines into one entry whose height is the
rendered element's; `height_of` returns that height for the first line and
zero for the rest, so the scroll math is unchanged. The widget element
comes from the reading view's `Renderer`, which no longer depends on the
`ReadingView` entity but takes link and fold handlers, and is laid out as a
child element inside the editor's custom element (the editor entity is
leased for the frame so the app is at hand). Any selection touching the
block unfolds it: arrowing into a table or math block shows its source, as
does clicking one (at the clicked row); an embed widget keeps its clicks so
links inside work. Inline math drops its dollars like other inline markers.
Math is shown as its TeX source, centered and in the accent color, as in the
reading view; there is no formula renderer yet. `--find-all q` opens a note
with every match selected, for screenshots.

### Parser, 2026-10-16

`-markdown` now parses CommonMark, GFM tables, strikethrough, task lists,
footnotes and bare URLs, and Obsidian's wikilinks, embeds, tags, block ids,
callouts, highlights, comments and math into one tree with byte spans. All 652
examples of the CommonMark 0.31.2 spec render byte-identical HTML
(`tests/commonmark.rs`, a ratchet that must not go down).

The Obsidian side of the gate is `tests/corpus.rs`: `tests/corpus/vault` is a
vault of 1000 generated notes (`-bench generate --notes 1000`) and nine
hand-written edge-case notes, and `tests/corpus/expected` holds the HTML
Obsidian 1.13.7 produced for each through the `-corpus` plugin that
ships inside the vault. Both sides are reduced to a canonical token stream
(no copy buttons, icons, MathJax output or `dir` attributes) before
comparison. 1007 of 1009 notes match; the two left are pathological
delimiter runs and a wikilink inside markdown link text. The observed
behaviors are listed in [[Specification]].

To regenerate the expected HTML after changing the corpus: set the corpus
vault as the open vault in `~/.config/obsidian/obsidian.json` (or open it from
Obsidian's vault switcher), create an empty `.-autorun` file in the vault,
start Obsidian, and trust the plugin when asked the first time. The plugin
writes `../expected`, then closes its window.

### Index benchmark, 2026-10-15

Synthetic vault from `-bench generate`: 20,000 notes, 2,000 attachments, 35 MB of Markdown.
Page cache was warm; cold numbers need the cache dropped first.

| Threads | Scan | Read | Index | Resolve | Total |
| --- | --- | --- | --- | --- | --- |
| 128 | 78 ms | 46 ms | 24 ms | 32 ms | 180 ms |
| 1 | 77 ms | 144 ms | 369 ms | 137 ms | 726 ms |

Folder scanning is the largest fixed cost and the stage to watch as the indexer grows.

### Index benchmark, 2026-10-16

Same vault, with the cache built from the full parser tree instead of a line
scanner. Warm page cache.

| Threads | Scan | Read | Index | Resolve | Total |
| --- | --- | --- | --- | --- | --- |
| 128 | 75 ms | 19 ms | 11 ms | 34 ms | 139 ms |
| 1 | 76 ms | 116 ms | 563 ms | 140 ms | 895 ms |

The full parser costs about 200 ms more than the old line scanner on one
thread (35 MB in 563 ms, roughly 60 MB/s) and is invisible with the thread
pool. The gate still passes cold with margin; the parser is not the bottleneck.
