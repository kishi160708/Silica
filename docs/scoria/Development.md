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
cargo run --release -p  -- <vault> --note X --find word     # open with the find panel up and the query matched
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
on their own two seconds after the last keystroke), Ctrl+Q quits. A note
opens in the vault's `defaultViewMode` (editing unless it says `preview`). Every shortcut except Ctrl+Q is an Obsidian command bound from its
default hotkey, so the vault's `hotkeys.json` rebinds or unbinds it. In the
graph: wheel zooms, drag pans, drag a node to move it, click a node to open
it, Ctrl+0 refits the view.

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
| Quick switcher | done: fuzzy over names, aliases and paths, recent files first, unresolved targets shown as not created; creating notes waits for the writer |
| Graph | done: global and local graphs, filters, color groups, forces and display settings from `graph.json`; zoom, pan, drag, hover, click opens |
| Command palette | done: Obsidian's command names and hotkeys, pinned commands from `command-palette.json`, this session's recent commands first; lists only the commands  can run |
| Settings, read-only | done: General, Appearance, Interface, Editor, Files and links, Hotkeys, Core plugins, Community plugins and the Quick switcher and Command palette plugin tabs, every value from `.obsidian/`; nothing is written |

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
planned follow-up. Not yet: multiple cursors (and the find panel's Find
all), regex and match-case search, auto-pair of brackets and markdown
markers, renumbering on indent, and Ctrl+click page previews.

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
math show their source with decorations.

`crates/-markdown::highlight` gained a `QuoteMarker` span per `>` so
the transform knows where each level's marker sits. The 300-step stress run
on a 530-line note in Live Preview shaped in 0.07 ms per frame; the 50k-line
Source mode gate is unchanged at 0.17 ms.

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
