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
| `properties.mjs` | properties in the reading view: the editing path, Add property, remove, type change, fold, propertiesInDocument, All properties | fixtures, run, after-restart |
| `canvas.mjs` | canvas write-back: what an edited save, an unedited close and `canvas:new-file` write; key order per node and edge type, the formatter's one-line rule, unknown keys, group ordering | fixtures, run |
| `splits-bookmarks.mjs` | tab groups: what "Split right" and "Split down" put in `workspace.json`, the `dimension` a dragged divider writes, the four focus commands, closing a group and its last tab, "Open link under cursor to the right", restart fidelity; and the Bookmarks plugin: every item type's bytes, each command's modal, the block id `bookmark-current-section` writes, the pane's header and rows | fixtures, run, after-restart |
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
