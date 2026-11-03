# Silica

Silica is a native knowledge base written in Rust on GPUI. It opens an
Obsidian vault unchanged — same folder, same `.obsidian` config, same
Markdown, same `.canvas` and `.base` files — with no import step, no
conversion, and no second copy of anything. The vault folder is the only
source of truth. Silica is an independent, clean-room implementation and
is not affiliated with Obsidian.

## Status

Reader and writer are done; all five phases have landed, including phase 5
(plugins). `silica-markdown` parses CommonMark (all 652 spec examples)
plus the GitHub extensions Obsidian uses into a single spanned tree with
byte spans, and renders reading-view HTML that matches Obsidian on
1007 of 1009 corpus notes. The app opens a real vault with a three-pane
shell: Files, reading view, Backlinks, Outgoing links, Tags, All
properties and Outline; Search with grouping, sort, copy results, and
vault-wide search & replace; the quick switcher, graph, command palette
and a Settings modal that writes. Notes open in Live Preview or Source
mode with find and replace, multiple cursors, auto-pairing, collapsible
headings and lists, and autosave; `[[`, `![[` and `#` autocomplete
against the vault's files, aliases, headings, tags and block references;
clipboard images and dropped files save to the vault's attachment folder
and insert embeds; frontmatter is an editable typed properties table that
rewrites only the lines it changes. A `.canvas` opens in a canvas view;
a `.pdf` opens in a viewer with toolbar, sidebar and find bar; Mermaid
diagrams render via a pure-Rust engine; audio and video embeds play in
the reading view, remote images load over HTTP, Ctrl+hovering a link
previews the target inline, and file and folder rows drag between folders
to move them; `$x$` and `$$x$$` typeset inline. The main area splits into
tab groups, and `workspace.json` records the tree the way Obsidian does.
Everything is bound through Obsidian's command ids, so a vault's
`hotkeys.json` applies unchanged.

Installed plugins in `.obsidian/plugins/` are classified as native,
compatibility, or refused-with-reason. Compatibility plugins run in a
per-plugin QuickJS host with a shimmed `app`; their commands appear in the
palette and their settings tabs in Settings > Community plugins. A
`silica.json` marker flags a native plugin targeting Silica itself.

> Keep a backup of any vault you open with it. Reading is well covered;
> editing is newer code against files you cannot get back. Until the
> round-trip gate is met, let Silica read a vault that matters and let
> Obsidian write it.

## Benchmarks

Reference machine: Lenovo Legion 5 — CachyOS, AMD Ryzen 5 4600H, 12
threads. Measured with `crates/silica-bench` on a synthetic vault:
20,000 notes, 2,000 attachments, 35.3 MB of Markdown.

| run | Scan | Read | Index | Resolve | Total |
| --- | --- | --- | --- | --- | --- |
| 1 (cold) | 45 ms | 16 ms | 81 ms | 47 ms | 190 ms |
| 2 | 44 ms | 12 ms | 80 ms | 40 ms | 175 ms |
| 3 | 44 ms | 11 ms | 71 ms | 39 ms | 166 ms |

Gate: 190 ms cold against a 2.00 s budget — PASS.

## Crates

| Crate | Responsibility |
| --- | --- |
| `silica` | GPUI application: window, workspace shell, views |
| `silica-vault` | A vault is a folder: scanning, file kinds, `.obsidian` config reads |
| `silica-markdown` | Obsidian Flavored Markdown parsed into one spanned tree, HTML renderer |
| `silica-cache` | Metadata cache and link resolution |
| `silica-suggest` | Autocompletion triggers, candidates, accepted-link writes |
| `silica-templates` | Template placeholders and date formats |
| `silica-formats` | On-disk formats: file kinds, `app.json`, JSON Canvas 1.0, Bases YAML |
| `silica-bases` | The Bases expression language |
| `silica-commands` | 244 command ids and default hotkeys |
| `silica-search` | Search query grammar and matcher |
| `silica-text` | Editor text model: buffer, selections, undo, edit plans |
| `silica-canvas` | Canvas model: viewport, edges, selection, undo |
| `silica-pdf` | PDF rendering: pages, outline, title, page text |
| `silica-math` | TeX math typesetting (KaTeX-compatible) |
| `silica-control` | Control socket protocol (`--control`) and `silica-mcp` |

## Running it

```sh
cargo run --release -p silica -- "/path/to/vault"
cargo run --release -p silica -- docs/silica-demo   # bundled demo vault
cargo run --release -p silica -- "/path/to/vault" --control /run/user/1000/silica.sock
cargo test --workspace
```

`--control` exposes the app on a Unix socket speaking newline-delimited
JSON: open a note, run a command id, send keystrokes or clicks, read back
the window's whole state.

Two Model Context Protocol servers ship with the workspace:

- `silica-mcp` launches Silica and drives it through the control socket,
  so an agent can open notes, run commands, and screenshot the app.
  Register it as `{"mcpServers": {"silica": {"command": "target/release/silica-mcp"}}}`.
- `silica-llm-mcp` talks to a local OpenAI-compatible model server over
  plain HTTP. Its `llm_digest` tool reads a file here and returns only the
  model's summary, so large logs never enter the caller's context.
  `llm_generate` and `llm_models` round it out. Configure it with
  `SILICA_LLM_URL` (default `http://127.0.0.1:11434/v1`); an optional
  `SILICA_LLM_DIGEST_ROOT` confines `llm_digest` to a directory tree.

## Limitations

Factual gaps versus Obsidian, tracked rather than hidden:

- Math uses a KaTeX-compatible engine rather than MathJax; genuinely
  unsupported TeX still shows source in red.
- HTML is drawn where it can be drawn faithfully and left as source where
  it cannot.
- Local video plays without sound or seeking; remote media opens in the
  system player.
- The graph is a full view, not a side pane.
- No file recovery; Obsidian's snapshots live outside the vault.
- Two of 1,009 corpus notes do not match Obsidian's output (degenerate
  delimiter runs, a wikilink inside link text).

Obsidian Sync, Publish and the commercial services are Obsidian's
products, not file formats; Silica has nothing to do with them — any
file-level sync (git, Syncthing, Dropbox) works because files are all
Silica needs.

## Deliberately different

- Frontmatter links are rewritten in place; Silica replaces only the link
  text instead of re-serializing the whole block.
- The command palette lists only runnable commands; the Hotkeys tab still
  lists all 244 ids.
- Default font is Liberation Sans; `appearance.json` fonts override it.

## License

MIT. See [LICENSE](LICENSE).
