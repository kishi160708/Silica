# Contributing

Silica is an independent, clean-room implementation, not affiliated with Obsidian
or any other note app. Compatibility is with Obsidian's *formats*: the vault
folder, `.obsidian` configuration, command ids, hotkeys, all of which are
observable facts about files, never derived code. These rules keep it that way.

## Ground rules

1. **The vault folder is the only source of truth.** Never write a file Silica
   did not change. Preserve line endings, BOMs, and unknown JSON/YAML keys
   (`#[serde(flatten)] extra` everywhere).
2. **One Markdown parser** (`silica-markdown`) feeds the editor, reading view,
   cache, and link rewriting. Never add a second tokenizer for any of those.
3. **Clean room.** Implement from the spec, public docs, and observed file
   formats. Do not paste code, CSS, or icons from another product, and do not
   reference other implementations of this kind of product in commit messages,
   doc comments, or file content. Silica is presented as its own project, so
   name what the code *does*, not where it came from. (Scrubbing this later
   means rewriting history and force-pushing.)
4. **Command ids, config keys, and file names match Obsidian's exactly.** New
   ids only with the observed string; Silica's own additions use a `silica:`
   prefix so they can never collide with a future Obsidian id.
5. **`docs/silica/` is the specification.** When the spec and observed Obsidian
   behaviour disagree, probe the real thing and fix the spec.
6. **Destructive work needs a second look:** anything that rewrites a user's
   files beyond an explicit edit (vault-wide link rewrites, snapshots,
   migrations) ships with its behaviour written down in the commit body and the
   README's "Deliberately different" section.

## Before you open a pull request

From the repo root, all of these must pass:

```sh
cargo check --workspace
cargo test --workspace          # GUI e2e runs when a display exists
cargo clippy --workspace --all-targets   # no new warnings in touched files
rustfmt --edition 2021 --check           # touched files only, never tree-wide
cargo audit
```

Two traps that cost reviewers a round trip:

- **No tree-wide `cargo fmt`.** `cargo fmt --all` reformats a dozen unrelated
  files and buries the real diff. Check your touched files only.
- **The GUI tests are real gates.** `crates/silica/tests/e2e.rs` launches the
  app on a temp vault and drives it over the control socket. Run them with
  `-- --test-threads=1 --nocapture` and read the output; a launch that dies
  writes its panic to `silica-e2e.log` in the temp vault. The writer round-trip
  gate takes a real vault: `SILICA_ROUNDTRIP_VAULT=<vault> cargo test -p
  silica-text --test writer_roundtrip`.

Tests build their vaults in `tempfile::tempdir()`; never depend on a
developer's own vault. A feature is done when the running app produces the
expected output, not when unit tests pass: drive it over `--control` and read
`state` back.

## Commit style

Lowercase `feat(scope):` / `fix(scope):` / `perf(scope):` subjects with a prose
body explaining *why*. Deliberate omissions and simplifications go in the body,
not silently skipped. Non-trivial logic leaves one runnable check behind.

## Releases (maintainers)

One release commit per batch: bump `version` in `Cargo.toml`
(`[workspace.package]`) and the badge in `README.md` together, then push. CI
(test matrix on Linux/Windows/macOS, the xvfb GUI job, and the audit job) runs
on every push; a red pipeline on `main` is fixed before the next push.
