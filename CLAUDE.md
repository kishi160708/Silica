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
crates/           GPUI app (bin): workspace shell, Files view, reading view, source editor
crates/-vault     folder scanning, .obsidian config
crates/-markdown  OFM primitives
crates/-cache     CachedMetadata, LinkResolver
crates/-formats   FileKind, config types, canvas, base
crates/-commands  command registry, default hotkeys
crates/-search    search query grammar and matcher, fuzzy search
crates/-text      editor text model: buffer, selections, undo, edit plans (no gpui)
crates/-suggest   autocompletion: triggers, candidates, the link an accepted row writes (no gpui)
crates/-canvas    canvas model: viewport math, edge geometry, hit tests, selection, edits with undo (no gpui)
crates/-bench     spike benchmarks: synthetic vault generator, index timer (bin)
docs/             Obsidian vault: spec and design notes (dogfooded by )
docs/img                brand art; _v3_fixed.svg is the logo every other copy is cut from
docs/img/app            README screenshots and the tab recording, captured over the control socket
site                    : the informational site and the manual (static, node)
```

## Commands

On NixOS run these inside `nix-shell` (`shell.nix` supplies gpui's X11, Wayland, and Vulkan libraries).
The `justfile` wraps every command below (`just` lists them); on macOS it points `CARGO_TARGET_DIR` at
`~/.cache/cargo-target/`, because a repo on a quarantined network share makes Gatekeeper prompt
for every build script written there.
gpui is a git dependency pinned to a Zed release commit (see the note in `Cargo.toml`); the crates.io
release is stale and crashes on NVIDIA. Bump the pin on its own, one release at a time, and run the
app on a real GPU before committing.

```
cargo test --workspace            # all library tests (fast)
SCORIA_ROUNDTRIP_VAULT=<vault> cargo test -p -text --test writer_roundtrip   # Writer gate round trips, over a real vault too
cargo check -p              # app, pulls gpui (slow first time)
cargo run --release -p  -- <vault> [--note "Name#Heading"]   # open a vault (release: the editor reparses on every edit)
cargo run --release -p  -- <vault> --frame native   # the desktop's title bar instead of the hidden frame (SCORIA_FRAME=native does the same)
cargo run --release -p  -- <vault> --synthetic 50000 --stress   # editor spike: self-driven edits, prints timings
cargo run --release -p  -- <vault> --note X --source --type '[[al' [--accept enter]  # open the autocompletion popup (and press its keys) for a screenshot
cargo run --release -p -bench -- generate <dir>   # synthetic 20k-note vault
cargo run --release -p -bench -- index <vault>    # time scan/read/index/resolve, check the 2 s gate
cargo fmt --all && cargo clippy --workspace --all-targets

cd site && npm install && npm run build   #  -> site/dist/ (exits 1 on a dead link)
cd site && npm run serve                  # the same, served on http://localhost:4321/
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
- The logo lives in three places — `docs/img/_v3_fixed.svg` (the master), the app's
  `crates//assets/logo.svg` and the site's `site/sites//assets/`. Change the master
  and re-cut the copies; never edit one of the copies on its own.
- `site/` vendors the builder from the `` repository byte for byte, bar the two
  differences `site/README.md` records. Port fixes across whole rather than merging.
