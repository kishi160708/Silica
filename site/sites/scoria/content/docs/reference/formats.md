---
title: Vault files and formats
description: Every file kind  recognizes, what lives in .obsidian and which parts  reads, the JSON Canvas and Bases formats, forbidden file names, and what  writes.
---

# Vault files and formats

## File kinds

 classifies every file in the vault by extension:

| Kind | Extensions |
|---|---|
| Note | `.md` |
| Image | `.bmp`, `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg`, `.webp`, `.avif` |
| Audio | `.mp3`, `.wav`, `.m4a`, `.3gp`, `.flac`, `.ogg`, `.oga`, `.opus` |
| Video | `.mp4`, `.webm`, `.ogv`, `.mov`, `.mkv` |
| PDF | `.pdf` |
| Canvas | `.canvas` |
| Base | `.base` |
| Unknown | everything else |

Unknown kinds are hidden in the [Files view](/docs/app/files/) unless
`showUnsupportedFiles` is on, and there is nothing to open them with either way.

Only notes are parsed, indexed and searched. The rest are tracked as files — so they appear
in Files, can be embedded, and are counted as link targets.

## File names

Obsidian forbids a set of characters in a file name, and  validates the same set:

```
*  "  \  /  <  >  :  |  ?
```

A name may also not begin with a dot. Vault-relative paths use `/` and keep their original
case; comparisons between them are case-insensitive.

## The `.obsidian` folder

Every file is optional, every key within it is optional, and **keys  does not
recognize are carried through untouched**.

| File |  |
|---|---|
| `app.json` | Reads — the editor and files settings |
| `appearance.json` | Reads — base theme, accent color, fonts, font size |
| `hotkeys.json` | Reads — your key bindings |
| `core-plugins.json` | Reads — both the modern map and the legacy array shape |
| `community-plugins.json` | Reads the list, and each plugin's `manifest.json` |
| `graph.json` | Reads — filters, color groups, display settings, forces |
| `switcher.json` | Reads |
| `command-palette.json` | Reads — pinned commands |
| `workspace.json` | Reads, does not apply, never writes |
| `themes/`, `snippets/` | Not applied (tier 3) |
| `plugins/` | Manifests read; plugin code not executed |

 writes **none of these** today. See [settings](/docs/settings/) for why.

Which `app.json` keys are read, and which of them actually change behavior, is the table in
[settings](/docs/settings/).

## `graph.json`

Read in full enough detail to reproduce your graph:

| Group | Keys |
|---|---|
| Filters | `search`, `showTags`, `showAttachments`, `hideUnresolved`, `showOrphans` |
| Color groups | `colorGroups`, each `{query, color: {a, rgb}}` |
| Display and force | Node size, link thickness, text fade, centre, repel, link force, link distance |
| View | `scale`, `close` |

`collapse-*` keys round-trip.

## JSON Canvas

`.canvas` files are [JSON Canvas 1.0](https://jsoncanvas.org).  implements the format:
its fields, z-order by array order, and tab-indented output when writing. A canvas
round-trips through  unchanged.

> **Note** There is no canvas **view** yet. The file is recognized in Files and its format is
> implemented; opening it does not draw a canvas. That is phase 4.

## Bases

`.base` files are Obsidian's Bases format, YAML.  implements the top-level keys and the
recursive filter structure, and round-trips a base file unchanged. As with Canvas, there is
no base view yet.

## What  writes, completely

The whole list:

1. **A note you edited** — on autosave, `Ctrl + S`, navigation, mode change or close. Its
   byte-order mark and line endings are restored as found.
2. **Notes whose links need rewriting** after you rename or move a file, with
   `alwaysUpdateLinks` on. Only the link bytes change.
3. **Files you create, copy or delete.**
4. **A `.trashinfo` record and a moved file**, when deleting with `trashOption: system` on
   Linux, or a file moved into `.trash/` with `local`.

Nothing else. No cache, no index, no sidecar, no configuration, no lock file.

## Not a format: Sync and Publish

Obsidian Sync and Obsidian Publish are services, not file formats, and  has nothing to
do with either. A vault synced by Obsidian Sync is still a folder, so  opens it; the
syncing is Obsidian's doing and  neither helps nor hinders it.

## Next

- [How  works](/docs/reference/architecture/)
- [Glossary](/docs/reference/glossary/)
