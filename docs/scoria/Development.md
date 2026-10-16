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
cargo run --release -p -bench -- generate <dir>
cargo run --release -p -bench -- index <vault>
```

In the app: click a file or a link to open it, Ctrl+E toggles reading view
and source mode, Ctrl+Alt+Left and Right walk the tab's history, Ctrl+O opens
the quick switcher, Ctrl+Shift+F the Search tab, Ctrl+Q quits.

## Reader (phase 2)

Started 2026-10-16, after the three spike gates passed. Gate: a daily user can
browse their real vault with no missing content.

| Reader item | Status |
| --- | --- |
| Open a vault | done: scanned and indexed on a background thread, `.obsidian` config read |
| Files view | done: folder tree, folders first, collapse state, auto-reveal, attachments shown, unknown kinds hidden |
| Reading view | done for text content; see the list below |
| Backlinks | done: linked mentions with the source line; unlinked mentions not yet |
| Outline | done: click scrolls to the heading |
| Tags | done: vault-wide counts merged across inline and frontmatter tags |
| Search | done: full query grammar with Obsidian's explain labels, results grouped per file with context, click opens at the line; sort options and copy results not yet |
| Quick switcher | done: fuzzy over names, aliases and paths, recent files first, unresolved targets shown as not created; creating notes waits for the writer |
| Graph | not started (`resolvedLinks` is the edge list) |
| Settings, read-only | theme, fonts, base font size, readable line length, strict line breaks, show unsupported files |

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

Not yet: Live Preview, MathJax, HTML rendering, remote images, column widths
in tables (every column is equal), checkbox toggling, heading folding, copy
buttons, PDF export.

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

### Editor prototype, 2026-10-16

`crates//src/editor.rs` is a plain-text Markdown editor over one `String`
with a line index. A custom GPUI element shapes and paints only the visible
lines; decorations come from `_markdown::highlight`, recomputed on a
background thread after each edit (one parse in flight, one queued) and
swapped in when ready, so typing never waits for the parser. Input goes
through GPUI's input handler scoped to the cursor line, so IME composition
works; navigation keys are actions.

```
cargo run --release -p  -- docs/ --synthetic 50000          # open a generated 50k-line note
cargo run --release -p  -- docs/ --synthetic 50000 --stress  # 600 self-driven edits, then a report
```

Stress run on the reference machine, 50,001 lines (4.3 MB, 244k decorated
spans; the synthetic note is far denser in syntax than a real one):

| Measure | Result |
| --- | --- |
| Frames painted during 600 edit and navigation steps in 7 s | 616 |
| Shaping the visible lines, per frame | 0.48 ms mean, 1.52 ms max |
| Full reparse plus decoration pass (background) | 226 ms mean |
| Edit to refreshed decorations | 234 ms |

The frame path is the gate: scrolling and typing stay far under a frame
budget regardless of note size. Decoration latency scales with note length
because the whole note is reparsed; a real 50k-line note of ordinary prose is
about half this size, and incremental reparse of the edited block is the
planned follow-up. Not in the prototype yet: selections, wrapping, undo,
saving, and Live Preview rendering of the decorated constructs.

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
