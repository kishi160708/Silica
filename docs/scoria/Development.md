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
cargo run -p  -- <vault>
cargo run --release -p -bench -- generate <dir>
cargo run --release -p -bench -- index <vault>
```

## Spike gates

The [[Specification]] sets three gates before any reader or writer work starts.

| Gate | Target | Status |
| --- | --- | --- |
| Editor prototype | 50k-line note with wikilink and heading decorations | not started |
| Parser conformance | 1000-note HTML diff corpus at 99 percent | CommonMark suite at 652/652; Obsidian corpus not started |
| Index performance | 20k-file vault indexed cold in under 2 s | measured, see below |

### Parser, 2026-10-16

`-markdown` now parses CommonMark, GFM tables, strikethrough, task lists,
footnotes and bare URLs, and Obsidian's wikilinks, embeds, tags, block ids,
callouts, highlights, comments and math into one tree with byte spans. All 652
examples of the CommonMark 0.31.2 spec render byte-identical HTML
(`tests/commonmark.rs`, a ratchet that must not go down). The Obsidian
constructs are covered by `tests/obsidian.rs`; the next step for this gate is
a corpus of real notes rendered by Obsidian to diff against.

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
