---
title: The Files view
description: 's file tree — the sort order, which files are shown and hidden, revealing the open note, and the right-click menu for creating, renaming, moving, copying and deleting.
---

# The Files view

The Files tab in the left dock is the vault's folder tree. It is the tab  opens on,
and `file-explorer:open` returns to it.

## What you see

- **Folders before files**, each group sorted by name, compared case-insensitively. This is
  Obsidian's order, and it does not depend on your locale.
- **Notes without their extension.** `Ideas.md` shows as *Ideas*, because the `.md` on every
  note is noise.
- **Attachments with theirs.** `diagram.png` shows as *diagram.png*.
- **Empty folders.** They exist in the vault, so they are in the tree.

Click a folder to collapse or expand it; the collapse state is kept while the window is
open. Click a file to open it.

## What is hidden

| Hidden | Why |
|---|---|
| Dot-folders (`.obsidian`, `.git`, `.trash`) | Configuration and machinery, as in Obsidian |
| Unknown file kinds | Unless `showUnsupportedFiles` is on in `app.json` |

A "known kind" is a note, an image, an audio file, a video, a PDF, a Canvas or a Base. A
`.zip` or a `.xlsx` in your vault is an unknown kind: with `showUnsupportedFiles` off it is
not listed, with it on it is listed but clicking it does nothing —  has no handler to
open it with.

## Reveal the open note

`file-explorer:reveal-active-file` expands the tree down to the note you are reading and
highlights it. The tree also reveals automatically when you open a note from somewhere else
— a link, the switcher, the graph — so the left dock always shows where you are.

## The right-click menu

Right-click a file or a folder for:

| On a file | On a folder |
|---|---|
| New note | New note (in that folder) |
| New folder | New folder |
| Rename… | Rename… |
| Make a copy | — |
| Delete | Delete |

Moving is not in this menu: run `file-explorer:move-file`, which opens a fuzzy folder
picker. Each of the others is a command too, so it can have a key of its own —
`file-explorer:new-file`, `file-explorer:new-folder`, `workspace:edit-file-title` (`F2`),
`file-explorer:duplicate-file`, `app:delete-file`.

Renaming and moving rewrite the links in the rest of the vault. What exactly that means —
which links change, which are left byte for byte, what happens to aliases — is the subject
of [managing notes](/docs/app/manage/), and worth reading once before you rename anything
in a vault you care about.

## Files that change on disk

The tree follows the folder. A file added, removed, renamed or moved by anything else — git,
a sync client, Obsidian in another window — appears or disappears here within a moment,
without a refresh. A folder moved into the vault is walked and added whole; a folder removed
takes everything under it out of the index.

## Not yet

- **Drag and drop.** Run `file-explorer:move-file` instead, which offers a fuzzy folder
  picker.
- **Inline title editing.** Rename through the menu or `F2`.
- **Pasting or dropping attachments into the vault.** Put the file in the folder yourself;
  the tree will notice.
- **Sorting options.** Folders-then-name, case-insensitive, is the only order.
- **File starring or bookmarks.**

## Next

- [Managing notes](/docs/app/manage/) — what renaming does to your links.
- [Reading view](/docs/app/reading-view/)
