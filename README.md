# 

 is a native knowledge base written in Rust on [GPUI](https://gpui.rs), built to open
[Obsidian](https://obsidian.md) vaults unchanged: same folder, same `.obsidian` configuration,
same Markdown dialect, same `.canvas` and `.base` files. It is an independent implementation
and is not affiliated with Obsidian.

## Status

Reader done, Writer in progress. `-markdown` parses CommonMark (all 652 spec examples), the
GitHub extensions Obsidian uses, and Obsidian's own syntax into one tree with byte spans, and renders
it to reading-view HTML that matches Obsidian 1.13.7 on 1007 of 1009 corpus notes; the metadata
cache, the editor and the link rewriter are all built from that tree. The application opens a real
vault with the three-pane shell: Files, reading view, Backlinks, Outgoing links, Outline, Tags,
Search, the quick switcher, the graph, the command palette and a Settings modal whose toggles write. Notes
open in Live Preview or Source mode with find and replace, multiple cursors and autosave; `[[`,
`![[` and `#` autocomplete against the vault's files, aliases, headings and tags; new note,
rename, move, copy and delete rewrite links the way Obsidian does, and frontmatter is an editable,
typed properties table that rewrites only the lines it changes. Everything is bound through
Obsidian's command ids, so a vault's `hotkeys.json` applies unchanged.

```
cargo run --release -p  -- "C:\path\to\your\vault"
cargo run --release -p  -- docs/ --synthetic 50000 --stress   # editor spike with a report
cargo test --workspace
```

## Compatibility tiers

| Tier | Goal | Plan |
| --- | --- | --- |
| 1 | Vault and format compatibility (no conversion, no data loss) | In scope, first |
| 2 | Core feature parity with Obsidian's app and 35 core plugins | In scope |
| 3 | Theme compatibility | Variable-level subset of the 1047 CSS custom properties |
| 4 | Community plugin compatibility | New native plugin API; existing JS plugins are out of scope |

The full functional specification this project implements, derived from observing Obsidian 1.12.7's
behavior, file formats, and public documentation, lives in the `docs/` vault and is published
at [.](https://.).

## Crates

| Crate | Responsibility |
| --- | --- |
| `` | GPUI application: window, workspace shell, views |
| `-vault` | A vault is a folder: scanning, file kinds, `.obsidian` config reads |
| `-markdown` | Obsidian Flavored Markdown: CommonMark + GFM + Obsidian syntax parsed into one spanned tree, HTML renderer |
| `-cache` | Metadata cache (`CachedMetadata`) and link resolution with Obsidian's rules |
| `-suggest` | Autocompletion: what a line triggers, what the vault offers, and the link accepting one writes |
| `-formats` | On-disk formats: file kinds, `app.json` and friends, JSON Canvas 1.0, Bases YAML |
| `-commands` | The 196 command ids and default hotkeys so vault `hotkeys.json` applies unchanged |

## Roadmap

| Phase | Gate | Status |
| --- | --- | --- |
| 1. Spike | 50k-line editor, parser conformance corpus, 2 s vault index | Done |
| 2. Reader | A daily user browses their real vault with no missing content | Done: Files, reading view, Backlinks, Outgoing links, Outline, Tags, Search, quick switcher, graph, command palette, read-only Settings, file watching |
| 3. Writer | The vault round-trips byte-identical after a week of mixed use in both apps | In progress: Source mode, Live Preview, find and replace, file operations with link rewriting, properties, autocompletion, the round-trip harness and the Settings toggles are done; table editing, block references, templates, daily notes and file recovery are next |
| 4. Parity | Core plugin parity | Not started: canvas, bases, PDF, math, themes subset, URI and CLI |
| 5. Ecosystem | Native plugin SDK | Not started |

Progress notes, gates and the per-item status live in `docs//Development.md`.

## Releases

Pushing a `v*` tag builds the `` binary on both forges and attaches the archives with a
`SHA256SUMS` file:

- [GitHub releases](https://github.com/lw-xiong//releases) — Linux x86_64, Windows x86_64, macOS arm64
- [git. releases](https://git.///releases) — Linux x86_64

There is no installer: unpack the archive and run ` /path/to/vault`. The binaries are
unsigned, and only the Linux build has been run on a real GPU — see `docs//Development.md`.

## License

MIT. See [LICENSE](LICENSE).
