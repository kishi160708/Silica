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
| Parser conformance | 1000-note HTML diff corpus at 99 percent | not started |
| Index performance | 20k-file vault indexed cold in under 2 s | measured, see below |

### Index benchmark, 2026-10-15

Synthetic vault from `-bench generate`: 20,000 notes, 2,000 attachments, 35 MB of Markdown.
Page cache was warm; cold numbers need the cache dropped first.

| Threads | Scan | Read | Index | Resolve | Total |
| --- | --- | --- | --- | --- | --- |
| 128 | 78 ms | 46 ms | 24 ms | 32 ms | 180 ms |
| 1 | 77 ms | 144 ms | 369 ms | 137 ms | 726 ms |

Folder scanning is the largest fixed cost and the stage to watch as the indexer grows.
