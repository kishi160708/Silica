# 

 is a native knowledge base written in Rust on [GPUI](https://gpui.rs), built to open
[Obsidian](https://obsidian.md) vaults unchanged: same folder, same `.obsidian` configuration,
same Markdown dialect, same `.canvas` and `.base` files. It is an independent implementation
and is not affiliated with Obsidian.

## Status

Parser foundation. `-markdown` parses CommonMark (all 652 spec examples), the GitHub
extensions Obsidian uses, and Obsidian's own syntax into one tree with byte spans, and renders
it to reading-view HTML; the metadata cache is built from that tree. The application opens a
window with the three-pane shell and reports the vault it was given.

```
cargo run -p  -- "C:\path\to\your\vault"
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
behavior, file formats, and public documentation, lives in the `docs/` vault.

## Crates

| Crate | Responsibility |
| --- | --- |
| `` | GPUI application: window, workspace shell, views |
| `-vault` | A vault is a folder: scanning, file kinds, `.obsidian` config reads |
| `-markdown` | Obsidian Flavored Markdown: CommonMark + GFM + Obsidian syntax parsed into one spanned tree, HTML renderer |
| `-cache` | Metadata cache (`CachedMetadata`) and link resolution with Obsidian's rules |
| `-formats` | On-disk formats: file kinds, `app.json` and friends, JSON Canvas 1.0, Bases YAML |
| `-commands` | The 196 command ids and default hotkeys so vault `hotkeys.json` applies unchanged |

## Roadmap

1. Spike: GPUI editor prototype, OFM parser conformance corpus, vault index performance.
2. Reader: browse, search, graph, backlinks on a real vault.
3. Writer: Live Preview, properties, link rewriting on rename.
4. Parity: canvas, bases, PDF, themes subset, URI and CLI.
5. Ecosystem: native plugin SDK.

## License

MIT. See [LICENSE](LICENSE).
