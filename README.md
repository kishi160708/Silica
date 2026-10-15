# 

 is a native knowledge base written in Rust on [GPUI](https://gpui.rs), built to open
[Obsidian](https://obsidian.md) vaults unchanged: same folder, same `.obsidian` configuration,
same Markdown dialect, same `.canvas` and `.base` files. It is an independent implementation
and is not affiliated with Obsidian.

## Status

Skeleton. The workspace compiles and the format, parser, cache, and vault crates have tests;
the application opens a window with the three-pane shell and reports the vault it was given.

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

The full functional specification this project implements, derived from a static analysis of
Obsidian 1.12.7, lives in `docs/`.

## Crates

| Crate | Responsibility |
| --- | --- |
| `` | GPUI application: window, workspace shell, views |
| `-vault` | A vault is a folder: scanning, file kinds, `.obsidian` config reads |
| `-markdown` | Obsidian Flavored Markdown primitives: frontmatter, wikilinks, embeds, tags, block ids, callouts |
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
