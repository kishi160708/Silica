---
title: Commands and hotkeys
description: The commands  registers — the 244 Obsidian command ids, the 41 with a default hotkey, and the 34  can actually run today, with every id and default key listed.
---

# Commands and hotkeys

 registers **244 command ids** — the exact strings Obsidian registers, so your
`hotkeys.json` matches them — of which **227 have an observed desktop name**. **41** carry a
default hotkey. **34** do something in  today.

Those four numbers are different things and the difference is the point:

- An **id** is what your `hotkeys.json` keys on. Registering all of them is why your
  bindings survive opening the vault in .
- A **name** is what the command palette and the Hotkeys tab display.
- A **default hotkey** is Obsidian's binding, applied unless your file overrides it.
- **Runnable** means  implements the behavior. The [command
  palette](/docs/app/switcher/) offers only these; the [Hotkeys tab](/docs/hotkeys/) lists
  all 244, so you can see your bindings for everything else.

Ids come from Obsidian 1.12.7; names and default hotkeys were captured from Obsidian 1.14.4
with a throwaway plugin that dumped `app.commands` and the settings tabs.

## Commands with a default hotkey

`Ctrl` is Cmd on macOS, except where Obsidian itself registers a literal `Ctrl` — the tab
commands below, which are `Ctrl` everywhere.

| Command | Id | Default key | In  |
|---|---|---|---|
| Add file property | `markdown:add-metadata-property` | `Ctrl + ;` | — |
| Close current tab | `workspace:close` | `Ctrl + W` | — |
| Close window | `workspace:close-window` | `Ctrl + Shift + W` | — |
| Command palette: Open command palette | `command-palette:open` | `Ctrl + P` | Yes |
| Create new note | `file-explorer:new-file` | `Ctrl + N` | Yes |
| Create note to the right | `file-explorer:new-file-in-new-pane` | `Ctrl + Shift + N` | Yes |
| Delete paragraph | `editor:delete-paragraph` | `Ctrl + D` | — |
| Follow link under cursor | `editor:follow-link` | `Alt + Enter` | — |
| Go to last tab | `workspace:goto-last-tab` | `Ctrl + 9` | — |
| Go to next tab | `workspace:next-tab` | `Ctrl + Tab`, `Ctrl + Page Down` | — |
| Go to previous tab | `workspace:previous-tab` | `Ctrl + Shift + Tab`, `Ctrl + Page Up` | — |
| Go to tab #1 … #8 | `workspace:goto-tab-1` … `-8` | `Ctrl + 1` … `Ctrl + 8` | — |
| Graph view: Open graph view | `graph:open` | `Ctrl + G` | Yes |
| Insert Markdown link | `editor:insert-link` | `Ctrl + K` | — |
| Navigate back | `app:go-back` | `Ctrl + Alt + ←` | Yes |
| Navigate forward | `app:go-forward` | `Ctrl + Alt + →` | Yes |
| New tab | `workspace:new-tab` | `Ctrl + T` | — |
| Open help | `app:open-help` | `F1` | — |
| Open link under cursor in new tab | `editor:open-link-in-new-leaf` | `Ctrl + Enter` | — |
| Open link under cursor in new window | `editor:open-link-in-new-window` | `Ctrl + Alt + Shift + Enter` | — |
| Open link under cursor to the right | `editor:open-link-in-new-split` | `Ctrl + Alt + Enter` | — |
| Open settings | `app:open-settings` | `Ctrl + ,` | Yes |
| Quick switcher: Open quick switcher | `switcher:open` | `Ctrl + O` | Yes |
| Rename file | `workspace:edit-file-title` | `F2` | Yes |
| Save current file | `editor:save-file` | `Ctrl + S` | Yes |
| Search & replace in current file | `editor:open-search-replace` | `Ctrl + H` | Yes |
| Search current file | `editor:open-search` | `Ctrl + F` | Yes |
| Search: Search in all files | `global-search:open` | `Ctrl + Shift + F` | Yes |
| Toggle bold | `editor:toggle-bold` | `Ctrl + B` | — |
| Toggle checkbox status | `editor:toggle-checklist-status` | `Ctrl + L` | — |
| Toggle comment | `editor:toggle-comments` | `Ctrl + /` | — |
| Toggle italic | `editor:toggle-italics` | `Ctrl + I` | — |
| Toggle reading view | `markdown:toggle-preview` | `Ctrl + E` | Yes |
| Undo close tab | `workspace:undo-close-pane` | `Ctrl + Shift + T` | — |

> **Note** `Ctrl + B` and `Ctrl + I` are in this table with a dash: the ids are registered and
> your bindings for them are kept, but  does not toggle bold or italic yet. Type the
> asterisks.

## Runnable commands with no default key

These do something, and Obsidian ships no binding for them — so if you want a key, bind one
in `hotkeys.json`. Several are worth binding on day one: `editor:toggle-source` and
`graph:open-local` especially.

| Command | Id |
|---|---|
| Add cursor above | `editor:add-cursor-above` |
| Add cursor below | `editor:add-cursor-below` |
| Backlinks: Open backlinks for the current note | `backlink:open-backlinks` |
| Backlinks: Show backlinks | `backlink:open` |
| Create new note in current tab | `file-explorer:new-file-in-current-tab` |
| Delete current file | `app:delete-file` |
| Files: Create new folder | `file-explorer:new-folder` |
| Files: Reveal current file in navigation | `file-explorer:reveal-active-file` |
| Files: Show file explorer | `file-explorer:open` |
| Graph view: Open local graph | `graph:open-local` |
| Make a copy of the current file | `file-explorer:duplicate-file` |
| Move current file to another folder | `file-explorer:move-file` |
| Outgoing links: Open outgoing links for the current file | `outgoing-links:open-for-current` |
| Outgoing links: Show outgoing links | `outgoing-links:open` |
| Outline: Open outline of the current file | `outline:open-for-current` |
| Outline: Show outline | `outline:open` |
| Tags view: Show tags | `tag-pane:open` |
| Toggle left sidebar | `app:toggle-left-sidebar` |
| Toggle right sidebar | `app:toggle-right-sidebar` |
| Toggle Live Preview/Source mode | `editor:toggle-source` |

## Keys that are not commands

A few keys belong to a widget rather than to the command registry, and are not rebindable:

| Key | Where | Does |
|---|---|---|
| `Ctrl + Q` | Anywhere | Quits |
| `Escape` | A modal or the find panel | Closes it |
| `Enter`, `↑`, `↓` | The switcher or palette | Choose and move |
| `Enter`, `↑`, `↓` | The find panel | Next and previous match |
| `Alt + Enter` | The find panel | *Find all* — a cursor at every match |
| `Alt + click` | The editor | Add or remove a cursor |
| `Ctrl + click` | Source mode | Follow a link |
| `Ctrl + 0` | The graph | Refit the view |

## The full id list

All 244 ids live in `crates/-commands/data/commands.txt` in the repository, and the
names in `data/names.txt` beside it. Settings' [Hotkeys tab](/docs/hotkeys/) shows the same
list with your own bindings applied, which is usually the faster way to look one up.

## Next

- [Hotkeys](/docs/hotkeys/) — how `hotkeys.json` is applied.
- [Obsidian compatibility](/docs/reference/compatibility/)
