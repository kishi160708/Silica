# Obsidian probe harness

Drives a real Obsidian over the Chrome DevTools Protocol to observe behavior
the specification needs. CDP input events are injected by the browser, so they
carry focus and take the same path a person's keyboard and mouse do — unlike
`el.dispatchEvent`, which does not and which is why the earlier
throwaway-plugin probes could not settle interaction.

Not part of `cargo test` and **not run in CI**: these scripts assert
*Obsidian's* behavior, not 's. A failure means Obsidian changed or the
app flaked, which is not a reason to block a merge. Run them by hand when the
Obsidian version moves, read the diff, and update the observation tables and
`docs//Specification.md` together. The CI-side counterpart is
`crates/-suggest/tests/obsidian_rows.rs`:  asserted against the
tables these runs produce.

## Running a block

```
tools/obsidian-probe/run-block.sh tools/obsidian-probe/search.mjs /tmp/search-results.json
```

`run-block.sh` creates a fresh scratch vault, registers it in
`~/.config/obsidian/obsidian.json` under a fresh id, calls the script in
phases — `fixtures` (Obsidian not running: write the notes), `run` (Obsidian
up), and `after-restart` if `run` ended with `c.quit()` — and restores
`obsidian.json` byte-identical on exit. It refuses to start while an Obsidian
is already running. Never point it at a real vault: not the gate vault
(`~/bstore2/raga/plmcrzy/PLMCRZY`) while a byte-level gate run is open, and
not `docs/`. Scripts get `VAULT` and `RESULTS` in the environment.

Scripts, each one checklist block of `docs//Interaction probes.md`:

| Script | Block | Phases |
| --- | --- | --- |
| `search.mjs` | the search pane: sort orders, persistence, menus, toggles, keyboard | fixtures, run, after-restart |
| `search-copy-modal.mjs` | the "Copy search results" modal and its 24 format variants; supersedes `search.mjs` step 09 | fixtures, run |
| `search-shots.mjs` | the search pane's look: `Page.captureScreenshot` PNGs of the pane with results, a hovered match, a keyboard-focused row, a collapsed row, the settings section, Explain search terms, Show more context, Collapse results, Match case, the count menu and the light theme (`SHOTS` names the folder), plus the computed styles and markup of every part | fixtures, run |
| `properties.mjs` | properties in the reading view: the editing path, Add property, remove, type change, fold, propertiesInDocument, All properties | fixtures, run, after-restart |
| `canvas.mjs` | canvas write-back: what an edited save, an unedited close and `canvas:new-file` write; key order per node and edge type, the formatter's one-line rule, unknown keys, group ordering | fixtures, run |
| `splits-bookmarks.mjs` | tab groups: what "Split right" and "Split down" put in `workspace.json`, the `dimension` a dragged divider writes, the four focus commands, closing a group and its last tab, "Open link under cursor to the right", restart fidelity; and the Bookmarks plugin: every item type's bytes, each command's modal, the block id `bookmark-current-section` writes, the pane's header and rows | fixtures, run, after-restart |
| `frame-typography.mjs` | the shell's chrome in the hidden frame: the title row, its window buttons and sidebar toggles, the ribbon, every header's height, the Lucide icon each control names, the computed font sizes, weights and colours of the shell and of a note, and what dragging a sidebar's handle and toggling a sidebar write to `workspace.json` | fixtures, run |
| `frame-settings.mjs` | the ribbon's own classes and icons, the dark and light palettes as computed custom properties | fixtures, run |
| `frame-setting-key.mjs` | the "Window frame style" setting: 1.14 opens Settings in a popout window, so its rows are reached through `app.setting.activeTab.containerEl`; records the setting's tab, label, options and default, and that no option writes anything under `.obsidian` | fixtures, run |
| `explorer-header.mjs` | the Files pane's header: its five buttons and their icons, the sort menu and what each of the six orders does to the rows and to `workspace.json`, the auto-reveal toggle with a note opened in a collapsed folder either way, the Expand all / Collapse all swap, and where the header's New note and New folder put their file | fixtures, run |
| `tab-hover.mjs` | the root tab strip's hover: which box the wash paints (the inner box, not the tab), its colour, radius and inset, what hover does to the title, the close button and the dividers of the hovered tab and its neighbours, the active tab and a dock tab hovered, in both themes, with a screenshot of the strip per state (`SHOTS` names the folder) | fixtures, run |
| `tab-list.mjs` | the strip's right end: where the new-tab, tab-list and sidebar-toggle buttons sit against the tab, the divider after the last inactive tab, and the tab-list menu: its rows, icons, checks and separators, what picking a tab, Close all, Stack tabs and Bookmark N tabs do | fixtures, run |
| `new-tab.mjs` | the new-tab button and `workspace:new-tab` with the active tab in the middle of the strip: where the tab goes, whether it is active, what its empty view offers | fixtures, run |
| `explorer-keyboard.mjs` | the Files pane's keyboard: what a click on a file, on the open file and on a folder does to focus, and Up, Down, Left, Right, Enter, Space, Home, End, Escape, F2 and Delete on file and folder rows, entering the tree through `reveal-active-file` | fixtures, run |
| `outgoing-links.mjs` | the Outgoing links pane: its markup and styles, one row per target with a glyph per kind, the subtext under a row, what clicking a heading row and an unresolved row do, the row menu, the unlinked mentions card with its "Link this file" chip and the bytes the chip writes, the two fold keys in `workspace.json`; and the Backlinks pane's rows, hover-only Link button and empty state for comparison | fixtures, run, after-restart |
| `dock-panes.mjs` | the right dock's Tags, All properties and Outline panes: each pane's markup, computed sizes and colours, header buttons and tooltips, the sort menu and the row order under every entry, the in-pane search filter (and how each pane treats Escape), hover, what a row click opens, the Tags pane's nested-tags toggle, counts and folds, the Outline's current-heading mark in source and reading mode and its auto-scroll toggle, the empty state, and every state key's round trip through `workspace.json` | fixtures, run, after-restart |
| `autopair.mjs` | auto-pairing (predates the runner: start Obsidian yourself with `--remote-debugging-port=9222` on a scratch vault and run it directly) | — |

`results/` keeps the raw JSON of the runs the specification cites, named by
date. A run is evidence; the interpretation of it lives in
`docs//Specification.md`, and the two should be read together.

## cdp.mjs

No dependencies — Node 24 has a built-in `WebSocket`. `connect()` waits for a
page target whose `app.workspace.layoutReady` is true (the endpoint answers
before the vault window boots, and the first page target can be the launcher)
and returns:

| | |
| --- | --- |
| `evaluate(body)` | run a function body in the renderer, `app` in scope, returns its value |
| `typeText`, `typeChar`, `pressKey(name, {ctrl,shift,alt,meta})` | real keystrokes |
| `rect`, `clickElement`, `rightClickElement`, `dblClickElement`, `hover`, `clickAt`, `drag`, `mouseMove` | real mouse |
| `openMenuItems()` | every open `.menu` as labels in order, `---` for separators |
| `selectInfo`, `selectSet` | a native `<select>`'s options, and setting it with a `change` event |
| `readClipboard`, `writeClipboard` | the system clipboard via Electron |
| `vaultPath()` | which vault this window has open — check it, never assume |
| `quit()` | close the window cleanly and wait for the process to go |

## Traps

- A vault id in `obsidian.json` must be **16 hex characters**; a malformed one
  is ignored and no vault window opens. `run-block.sh` generates one.
- **Reusing an id across folders inherits that id's IndexedDB cache**: a
  search on the new vault returned files from the old one until the re-index
  caught up. Fresh id per scratch vault.
- The version question: the window title and update log say **1.14.4**, the
  user agent says `obsidian/1.13.7`. App 1.14.4 on installer build 1.13.7.
- A previous instance lingers for seconds after its window closes. The kill
  pattern must admit the `--remote-debugging-port` flag between the binary
  and the asar, or it never matches a CDP launch; the runner waits for both
  the process and the endpoint to be gone, then checks `vaultPath()` after
  connecting and relaunches once on a mismatch.
- After a click, also ask whether a `.modal-container` appeared. "Copy search
  results" opens a modal; three scripts watched the clipboard for a write
  that never comes until the modal's own button is pressed.
- Obsidian's CSP blocks the renderer's own `fetch` to localhost; read a probe
  server's log from the host.
- `cp` is aliased to `cp -i` on this machine; the runner uses `command cp -f`.

See [[Development#Driving Obsidian over CDP, 2026-10-18]].
