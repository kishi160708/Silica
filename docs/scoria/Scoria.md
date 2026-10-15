---
aliases:
  - Home
tags:
  - /meta
---
# 

 is a native Rust/GPUI knowledge base that opens [Obsidian](https://obsidian.md) vaults
unchanged. This folder is itself an Obsidian vault: open `docs/` in Obsidian, and once  can
browse a vault, open it in  too. Anything the two apps disagree on here is a bug or a spec gap.

## Notes

- [[Specification]] — what the app implements and where the full specification lives.
- [[Development]] — machine setup, build commands, pinned dependencies, and benchmark results.

## Rules for this vault

- Write notes the way Obsidian would: wikilinks, YAML frontmatter, `#tags`, callouts.
- Keep `.obsidian/` committed except `workspace.json`, so the vault exercises 's config reader.
- Project rules for contributors and agents stay in the repository root `CLAUDE.md`.
