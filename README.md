<p align="center">
  <img src="docs/img/_v3_fixed.svg" alt="" width="132" height="109">
</p>

<h1 align="center"></h1>

<p align="center"><strong>Your vault, opened natively.</strong></p>

<p align="center">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-E8590C?style=flat-square&logo=rust&logoColor=white">
  <img alt="Linux, Windows, macOS" src="https://img.shields.io/badge/Linux%20%7C%20Windows%20%7C%20macOS-2b2b2b?style=flat-square">
  <img alt="MIT licensed" src="https://img.shields.io/badge/license-MIT-2b2b2b?style=flat-square">
  <img alt="Early alpha, v0.2.0" src="https://img.shields.io/badge/status-early%20alpha%20·%20v0.2.0-f2545b?style=flat-square">
  <a href="https://github.com/lw-xiong//actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status///ci.yml?branch=main&style=flat-square&label=CI"></a>
</p>

<p align="center">
  <a href="#status">Status</a> ·
  <a href="#the-window">The window</a> ·
  <a href="#tabs-splits-and-panes">Tabs and splits</a> ·
  <a href="#search-and-writing">Search and writing</a> ·
  <a href="#compatibility-tiers">Compatibility</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#releases">Releases</a> ·
  <a href="https://">Manual</a>
</p>

 is a native knowledge base written in Rust on [GPUI](https://gpui.rs), built to open
[Obsidian](https://obsidian.md) vaults unchanged: same folder, same `.obsidian` configuration,
same Markdown dialect, same `.canvas` and `.base` files. There is no import step, no conversion
and no second copy of anything. It is an independent implementation and is not affiliated with
Obsidian.

<img src="docs/img/app/workspace.png" alt="The  window: the Files pane on the left, a note in reading view in the middle with a properties table, a callout and a code block, the Outline pane on the right, all inside a frameless window.">

*The three-pane shell on this repository's own vault, `docs/`, where the specification lives:
a typed properties table, a callout, a code block and the note's outline. Obsidian's hidden window
frame, its palette, its type scale and its Lucide icons, all read off a running copy rather than
guessed at.*

## Status

Reader done, Writer in progress. `-markdown` parses CommonMark (all 652 spec examples), the
GitHub extensions Obsidian uses, and Obsidian's own syntax into one tree with byte spans, and renders
it to reading-view HTML that matches Obsidian 1.13.7 on 1007 of 1009 corpus notes; the metadata
cache, the editor and the link rewriter are all built from that tree. The application opens a real
vault with the three-pane shell: Files, reading view, Backlinks, Outgoing links, Outline, Tags,
Search, the quick switcher, the graph, the command palette and a Settings modal that writes. Notes
open in Live Preview or Source mode with find and replace, multiple cursors and autosave; `[[`,
`![[` and `#` autocomplete against the vault's files, aliases, headings, tags and block references; Obsidian's
sixteen table commands add, move and align rows and columns and tables are edited cell by cell; new note,
rename, move, copy and delete rewrite links the way Obsidian does, and frontmatter is an editable,
typed properties table that rewrites only the lines it changes. The main area splits into tab groups
(Split right, Split down, draggable dividers, the focus commands) and `workspace.json` records the
tree the way Obsidian does, and the Bookmarks pane reads and writes `bookmarks.json` in Obsidian's bytes. Everything is bound through
Obsidian's command ids, so a vault's `hotkeys.json` applies unchanged.

> **Keep a backup of any vault you open with it.** Reading a vault is well covered; editing is
> newer code against files you cannot get back. Until the round-trip gate is met, let  read
> a vault that matters and let Obsidian write it.

## The window

The window has no desktop title bar: it draws Obsidian's hidden frame itself — the sidebar toggles,
the dock tab headers, the tab strip and the window buttons all live in one 40 px row, and the empty
parts of that row drag, zoom and menu the window. `--frame native` asks the desktop for its own
frame instead. The palette, the type scale, the 40 px headers, the 44 px ribbon and the 300 px
sidebars are the values Obsidian computes for its own shell, read over CDP from a running copy and
written down in the specification; the icons are Lucide's own drawings, which is what Obsidian names
in its DOM.

## Tabs, splits and panes

<img src="docs/img/app/tabs.gif" alt="Opening notes in new tabs with the quick switcher, then splitting the main area right so two notes sit side by side.">

*A new tab, the quick switcher's fuzzy match, then Split right — two tab groups with their own tab
strips. The tree is written back to `workspace.json` in the shape Obsidian writes.*

## Search and writing

<img src="docs/img/app/search.png" alt="The Search pane with the query 'line endings', showing 341 results grouped by note with the matches highlighted.">

*Search takes Obsidian's query grammar, groups hits by note with a count, and highlights every
match in context.*

<img src="docs/img/app/autocomplete.png" alt="Typing two square brackets in Live Preview opens a completion popup listing matching notes.">

*`[[` in Live Preview completes against the vault's files, aliases, headings, tags and block
references; the accepted row writes the link in the style the vault already uses.*

## Running it

```
cargo run --release -p  -- "C:\path\to\your\vault"
cargo run --release -p  -- docs/ --synthetic 50000 --stress   # editor spike with a report
cargo run --release -p  -- "/path/to/vault" --control /run/user/1000/.sock
cargo run --release -p  -- "/path/to/vault" --frame native   # the desktop's own title bar instead of 's hidden frame
cargo test --workspace
```

`--control` exposes the running app on a Unix socket speaking newline-delimited
JSON: open a note, run a command id, send keystrokes or clicks, and read back a
dump of the window's whole state. `-mcp` wraps that protocol as a Model
Context Protocol server over stdio, so an agent can launch , drive it and
screenshot it — the screenshots and the recording above were made that way. Both
are described in `docs//Development.md`.

## Compatibility tiers

| Tier | Goal | Plan |
| --- | --- | --- |
| 1 | Vault and format compatibility (no conversion, no data loss) | In scope, first |
| 2 | Core feature parity with Obsidian's app and 35 core plugins | In scope |
| 3 | Theme compatibility | Variable-level subset of the 1047 CSS custom properties |
| 4 | Community plugin compatibility | New native plugin API; existing JS plugins are out of scope |

The full functional specification this project implements, derived from observing Obsidian 1.12.7's
behavior, file formats, and public documentation, lives in the `docs/` vault and is published
at [](https://), whose site is built from `site/` in this repository.

## Crates

| Crate | Responsibility |
| --- | --- |
| `` | GPUI application: window, workspace shell, views |
| `-vault` | A vault is a folder: scanning, file kinds, `.obsidian` config reads |
| `-markdown` | Obsidian Flavored Markdown: CommonMark + GFM + Obsidian syntax parsed into one spanned tree, HTML renderer |
| `-cache` | Metadata cache (`CachedMetadata`) and link resolution with Obsidian's rules |
| `-suggest` | Autocompletion: what a line triggers, what the vault offers, and the link accepting one writes |
| `-templates` | Template placeholders and the moment.js date formats they take |
| `-formats` | On-disk formats: file kinds, `app.json` and friends, JSON Canvas 1.0, Bases YAML |
| `-commands` | The 196 command ids and default hotkeys so vault `hotkeys.json` applies unchanged |

## Roadmap

| Phase | Gate | Status |
| --- | --- | --- |
| 1. Spike | 50k-line editor, parser conformance corpus, 2 s vault index | Done |
| 2. Reader | A daily user browses their real vault with no missing content | Done: Files, reading view, Backlinks, Outgoing links, Outline, Tags, Search, quick switcher, graph, command palette, read-only Settings, file watching |
| 3. Writer | The vault round-trips byte-identical after a week of mixed use in both apps | In progress: Source mode, Live Preview, find and replace, file operations with link rewriting, properties, autocompletion, table editing, block references, templates, daily notes, tab groups, bookmarks, the round-trip harness and the Settings toggles are done; and every Settings control Obsidian offers as one writes; file recovery is out of phase 3 |
| 4. Parity | Core plugin parity | Not started: canvas, bases, PDF, math, themes subset, URI and CLI |
| 5. Ecosystem | Native plugin SDK | Not started |

Progress notes, gates and the per-item status live in `docs//Development.md`.

## Releases

Pushing a `v*` tag builds the `` binary on both forges and attaches the archives with a
`SHA256SUMS` file:

- [GitHub releases](https://github.com/lw-xiong//releases) — Linux x86_64, Windows x86_64, macOS arm64
- [git. releases](https://git.///releases) — Linux x86_64

There is no installer: unpack the archive and run ` /path/to/vault`. Linux x86_64 is the
build that runs on the reference machine; the Windows build opens a vault on Windows 11 and draws
the hidden frame there too. The macOS build has never been started. Nothing is code-signed for
macOS or Windows — see `docs//Development.md`.

Verify a download: `sha256sum -c --ignore-missing SHA256SUMS` checks the archive against the
checksums the workflow wrote; `gh attestation verify <archive> --repo /` checks GitHub's
build provenance, which ties the archive to the workflow run and the commit it was built from. When
a release carries `SHA256SUMS.asc`, `gpg --verify SHA256SUMS.asc SHA256SUMS` checks the maintainer's
signature. That key is `69BF AA44 7DAC E6C3 EF02  6CBA 4EA6 7A61 88DA 92BD`, `
<@outlook.com>`, which `gpg --keyserver keys.openpgp.org --recv-keys 4EA67A6188DA92BD`
fetches; the release notes repeat the fingerprint that signed each release. Security problems go through
[SECURITY.md](SECURITY.md), not the issue tracker.

## License

MIT. See [LICENSE](LICENSE).
