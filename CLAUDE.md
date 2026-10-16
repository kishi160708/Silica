# 

Native Rust/GPUI clone of Obsidian's desktop app, targeting byte-level vault compatibility.

## Ground rules

- The vault folder is the only source of truth. Never write a file  did not change; preserve
  line endings, BOMs, and unknown JSON/YAML keys (`#[serde(flatten)] extra` everywhere).
- One Markdown parser (`-markdown`) feeds the editor, reading view, cache, and link rewriting.
  Do not add a second tokenizer for any of those.
- Formats and behaviors are specified in `docs//Specification.md`. When the spec and Obsidian disagree, test
  against a real vault and fix the spec.
- Command ids, config keys, and file names must match Obsidian's exactly (`-commands/data/commands.txt`,
  `-formats/src/config.rs`). Add new ones only with the observed string.
- Clean room: implement from the spec and public docs. Do not paste code, CSS, or icons from
  Obsidian's bundle.

## Layout

```
crates/           GPUI app (bin)
crates/-vault     folder scanning, .obsidian config
crates/-markdown  OFM primitives
crates/-cache     CachedMetadata, LinkResolver
crates/-formats   FileKind, config types, canvas, base
crates/-commands  command registry, default hotkeys
crates/-bench     spike benchmarks: synthetic vault generator, index timer (bin)
docs/             Obsidian vault: spec and design notes (dogfooded by )
```

## Commands

On NixOS run these inside `nix-shell` (`shell.nix` supplies gpui's X11, Wayland, and Vulkan libraries).
gpui is a git dependency pinned to a Zed release commit (see the note in `Cargo.toml`); the crates.io
release is stale and crashes on NVIDIA. Bump the pin on its own, one release at a time, and run the
app on a real GPU before committing.

```
cargo test --workspace            # all library tests (fast)
cargo check -p              # app, pulls gpui (slow first time)
cargo run --release -p  -- <vault> [--note <path>]      # open a vault (release: the editor reparses on every edit)
cargo run --release -p  -- <vault> --synthetic 50000 --stress   # editor spike: self-driven edits, prints timings
cargo run --release -p -bench -- generate <dir>   # synthetic 20k-note vault
cargo run --release -p -bench -- index <vault>    # time scan/read/index/resolve, check the 2 s gate
cargo fmt --all && cargo clippy --workspace --all-targets
```

## Conventions

- Vault-relative paths use `/` and keep original case; comparisons are case-insensitive.
- Positions are zero-based line/column plus byte offset, like Obsidian's `Pos`.
- Tests that need a vault build one in a `tempfile::tempdir()`; never depend on the developer's vault.
- `crates/-markdown/tests/corpus` holds Obsidian's own rendering of a 1000-note vault; the
  corpus test and the CommonMark suite are ratchets. When Obsidian and the parser disagree, add a
  probe note to `tests/corpus/vault/edge`, re-render with the plugin in that vault, and match the
  observed behavior (list it in `docs//Specification.md`).
- `docs//` is an Obsidian vault: write notes with wikilinks and frontmatter, and keep
  `.obsidian/` committed except `workspace.json`.
