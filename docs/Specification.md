# Functional specification

The full specification is maintained as a living document:

- Obsidian Functional Specification (Obsidian 1.12.7, derived from observing its behavior, file
  formats, and public documentation):
  https://claude.ai/code/artifact/6bc126ce-788f-484a-8213-c6eec1f99245

Sections: purpose and evidence basis; architecture; vault and filesystem model; Obsidian Flavored
Markdown; metadata cache and link resolution; editor; workspace shell; the 35 core plugins; Canvas
and Bases formats; search query language; settings, hotkeys and theming; main-process services, IPC
and URI protocol; community plugin API; out-of-scope services; feasibility and phased plan; appendices
with the command inventory and bundle evidence.

## Invariants this repository enforces today

| Invariant | Where |
| --- | --- |
| Known file kinds and extension tables | `-formats::FileKind` |
| Forbidden file-name characters `* " \ / < > : \| ?` and no leading dot | `-formats::validate_file_name` |
| Frontmatter only on line 1, BOM and CRLF tolerated | `-markdown::extract_frontmatter` |
| Wikilink body grammar `path#heading#heading|alias`, `#^block` | `-markdown::parse_linktext` |
| Tags need a non-numeric character; `# Heading` is not a tag; code spans are skipped | `-markdown::scan_inlines` |
| Block ids only at end of line after a space | `-markdown::scan_inlines` |
| Callout types and aliases | `-markdown::canonical_callout_type` |
| Link resolution: extension handling, root then relative, basename with same-folder then shortest path, case-insensitive | `-cache::LinkResolver` |
| Config files are sparse and optional; unknown keys round-trip | `-formats::config`, `-vault::Vault::read_config` |
| `core-plugins.json` map and legacy array shapes | `-formats::config::CorePlugins` |
| JSON Canvas 1.0 fields, z-order by array order, tab-indented output | `-formats::canvas` |
| Bases top-level keys and recursive filters | `-formats::base` |
| Command ids and default hotkeys | `-commands` |
