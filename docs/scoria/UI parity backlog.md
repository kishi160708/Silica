---
tags:
  - /dev
---
# UI parity backlog

Where 's window still differs from Obsidian 1.14, as of 2026-10-19. Eight readers each
took one subsystem, read the code and [[Specification]], and wrote down what they could
evidence; a second pass tried to refute each claim from both sides. The pass did not finish
(the run hit a model session limit after 69 of 232 checks), so a finding here is a lead with
a file and a line behind it, not a verified defect. Four claims were refuted outright, and
every one of them failed on the *Obsidian* half: what  does was right, what Obsidian
does was assumed rather than observed. Read the table that way — the  column is
grounded in code, the Obsidian column needs [[Interaction probes|a probe]] before it is
worth acting on.

The control socket makes these checkable from the outside now: open the app with
`--control`, drive it, and read `state` back (see
[[Development#Control socket and MCP server, 2026-10-19]]). The ones marked *bug* are
behaviour  gets wrong on its own terms; *parity-gap* is behaviour that works but does
not match Obsidian; *missing-feature* is something Obsidian has that  has not built.

116 findings across eight subsystems, 20 of them high severity.

| Subsystem | Findings | High |
| --- | --- | --- |
| Files view and file operations | 13 | 0 |
| Live Preview | 15 | 4 |
| Reading view | 15 | 4 |
| Right dock panes and graph | 13 | 1 |
| Search, switcher, palette and autocompletion | 15 | 4 |
| Settings | 15 | 1 |
| Source editor | 15 | 2 |
| Workspace shell | 15 | 4 |

## Files view and file operations

### Explorer scroll re-centers on every vault change and every save of the open note

*medium, bug, small*

**.** `FileExplorer::reveal` always calls `scroll_to_item(ix, ScrollStrategy::Center)` (explorer.rs:283-289). It is invoked from `set_tree` whenever the tree is rebuilt (explorer.rs:272-280, called from `refresh_vault_views` workspace.rs:1421 on any Added/Removed file in the vault) and from `show_file` (workspace.rs:1978-1980), which `after_changes` re-runs whenever the watcher reports a Modified event for the current file (workspace.rs:1527-1528). `flush_save` writes the open note with plain `std::fs::write` and nothing filters that write out of the watcher (workspace.rs:2286-2325), so the autosave two seconds after typing very likely produces Modified(current) and a re-center as well. The user scrolls the Files list away to look at another folder, and it jumps back to the active file whenever any file appears, disappears, or the open note saves.

**Obsidian.** Obsidian never moves the explorer scroll on its own unless the file-explorer leaf's `autoReveal` option is on, and that option defaults to false (`"state":{"sortOrder":"alphabetical","autoReveal":false,...}` in docs//.obsidian/workspace.json and crates/-markdown/tests/corpus/vault/.obsidian/workspace.json, both written by Obsidian). Even with it on, it only scrolls the row into view when it is off screen; saving a note or an attachment appearing does not scroll the list.

**Evidence.** crates//src/explorer.rs:272-289, crates//src/workspace.rs:1421, crates//src/workspace.rs:1519-1531, crates//src/workspace.rs:1978-1980, crates//src/workspace.rs:2286-2325, docs//.obsidian/workspace.json (file-explorer leaf state)

**Check it.** Open a vault with more folders than fit in the left dock. Open a note near the top of the tree, scroll the Files list to the bottom. In a terminal `touch <vault>/zzz-new.md` (or just type a character in the note and wait 2 s for the autosave): the Files list scrolls back so the active note is centered.

**Fix.** Split `reveal` into `set_active(path)` (highlight only) and `reveal(path)` (expand + scroll). Call the scrolling variant only from `file-explorer:reveal-active-file` and, when `autoReveal` from the file-explorer leaf state is true, from `show_file` on a genuine open — never from `set_tree` or from the Modified branch of `after_changes`. Use a scroll strategy that scrolls only when the row is outside the viewport (compute from `scroll.logical_scroll_top()`/item count or add a `ScrollStrategy` that is a no-op when visible).

### Opening a note always expands its folders (auto-reveal forced on; `autoReveal` ignored)

*medium, parity-gap, small*

**.** `show_file` unconditionally calls `explorer.reveal(path)` (workspace.rs:1978-1980), and `FileTree::reveal` removes every ancestor from `collapsed` (explorer.rs:168-179). Following a wikilink into a deeply nested note therefore pops open every folder on the way, and there is no setting to stop it. The file-explorer leaf's `autoReveal` in workspace.json is never read (no `workspace_json`/`autoReveal` use anywhere in crates//src).

**Obsidian.** Obsidian's explorer has an "Auto-reveal current file" toggle (file-explorer leaf state `autoReveal`, default false, as in both workspace.json fixtures in the repo). With it off, opening a note via a link or the switcher highlights the row only if it is already visible and expands nothing; `file-explorer:reveal-active-file` (Reveal current file in navigation) does the expand-and-scroll on demand.

**Evidence.** crates//src/workspace.rs:1978-1980, crates//src/explorer.rs:168-179, crates//src/workspace.rs:1743-1750, docs//.obsidian/workspace.json (file-explorer leaf: autoReveal false)

**Check it.** Collapse all folders, open a root note that links to `A/B/C/Deep.md`, click the link: folders A, B and C all expand and the list scrolls, although the vault's workspace.json has `autoReveal: false`.

**Fix.** Read the file-explorer leaf's `state.autoReveal` (and `sortOrder`) from `vault.workspace_json()` into the explorer; in `show_file` only set the active path unless autoReveal is on; keep the full reveal for `file-explorer:reveal-active-file`. Offer the toggle in the sort menu once a header exists.

### Rename, new note and new folder go through a modal prompt instead of editing the row inline

*medium, parity-gap, large*

**.** `start_rename` opens the `Prompt` modal titled "Rename file"/"Rename folder" with the path as detail (workspace.rs:2391-2416); `start_new_folder` opens a "New folder" modal pre-filled with `Untitled` (workspace.rs:2454-2483); `new_note` creates `Untitled.md`, opens it, then immediately opens the Rename modal (workspace.rs:2350-2365). The prompt is a 480 px card dropped 90 px from the top with no backdrop (prompt.rs:243-259, workspace.rs:3486-3497).

**Obsidian.** Spec "Observed file explorer drag-and-drop and inline rename behavior" (Specification.md:1983-2003): inline rename edits the row in place — the `.nav-file-title-content` div becomes `contenteditable` with the whole basename selected, the row gains `is-being-renamed`, blur commits, Escape abandons; a non-markdown file's extension stays outside the editable. New folder likewise creates an `Untitled` row in edit mode. Ctrl+N (Create new note) creates `Untitled` and puts the caret in the inline title with it selected rather than opening any dialog (well-known behaviour; the spec notes `workspace:edit-file-title` from the editor makes `.inline-title` or `.view-header-title` editable).

**Evidence.** crates//src/workspace.rs:2350-2365, crates//src/workspace.rs:2391-2416, crates//src/workspace.rs:2454-2483, crates//src/prompt.rs:243-259, docs//Specification.md:1983-2003

**Check it.** Right-click a note in Files → Rename...: a floating card appears at the top of the window rather than the row turning into a text field. Press Ctrl+N: a note opens and a "Rename file" dialog pops up over it.

**Fix.** Add an `editing: Option<(row path, TextInput)>` state to `FileExplorer`; render that row's name slot as the `TextInput` (basename only, extension drawn as a tag outside it), select all on open, commit on Enter/blur, discard on Escape, emit `ExplorerEvent::RenameTo{from,to}`; the workspace keeps `rename_with_links` and routes `displayError` back into the row. For Ctrl+N, focus the inline title (editor subsystem) instead of opening the prompt; keep the modal only for Move (a suggest modal in Obsidian too).

### Files pane header buttons (New note, New folder, Change sort order, Collapse all) are missing

*medium, missing-feature, medium*

**.** The left dock renders a tab strip (FILES / SEARCH text tabs plus the vault name, workspace.rs:3360-3407), then the explorer body, then a footer with a "Settings" link (workspace.rs:3408-3435). `FileExplorer::render` is just the uniform list (explorer.rs:476-490); `collapse_all`/`expand_all` exist only under `#[cfg(test)]` (explorer.rs:150-165). New note and new folder are reachable only via right-click or the palette.

**Obsidian.** Obsidian's Files pane has a `.nav-header` with four icon buttons: New note (lucide-edit), New folder (lucide-folder-plus), Change sort order (lucide-arrow-up-narrow-wide, opens a menu), and Collapse all / Expand all (lucide-chevrons-down-up, toggling). The spec section refers to `.nav-header` and `.nav-buttons-container` as existing explorer elements (Specification.md:1948). The left sidebar's tab strip shows icons (Files, Search, Bookmarks) and the vault name/settings live in the ribbon, not in the pane.

**Evidence.** crates//src/explorer.rs:150-165, crates//src/explorer.rs:476-490, crates//src/workspace.rs:3360-3435, docs//Specification.md:1948

**Check it.** Look at the top of the Files pane: there is no button row above the tree; there is no way to collapse all expanded folders without clicking each one.

**Fix.** Add a 36 px `nav-header` row in `FileExplorer::render` with four icon buttons: emit `NewNote{folder: ""}`/`NewFolder{parent: ""}`, open a sort menu (reusing the deferred menu code) that sets a `SortOrder` enum, and a collapse/expand-all toggle that calls the existing `collapse_all`/`expand_all` (drop the `cfg(test)`). Move the vault name and Settings out of the pane once a ribbon exists.

### Only one sort order, without natural numeric ordering; `sortOrder` from workspace.json is ignored

*medium, parity-gap, medium*

**.** `sort_key` is `(name.to_lowercase(), name)` (explorer.rs:45-47) applied to folder names and file display names (explorer.rs:189-207), so `Note 10` sorts before `Note 2` and non-ASCII letters sort by code point. There is no modified/created ordering (the `VaultFile.mtime` field is not consulted) and the file-explorer leaf's `sortOrder` is never read (no match for `sortOrder`/`workspace_json` in crates//src).

**Obsidian.** Obsidian offers six orders in the sort menu — File name (A to Z), File name (Z to A), Modified time (new to old / old to new), Created time (new to old / old to new) — persisted as `sortOrder` (`alphabetical`, `alphabeticalReverse`, `byModifiedTime`, `byModifiedTimeReverse`, `byCreatedTime`, `byCreatedTimeReverse`) in the leaf state (see the fixtures' `"sortOrder": "alphabetical"`). Folders always come first. The alphabetical compare is locale-aware with numeric collation, so `Note 2` precedes `Note 10` (moderately sure; widely reported Obsidian behaviour).

**Evidence.** crates//src/explorer.rs:45-47, crates//src/explorer.rs:189-207, docs//.obsidian/workspace.json (file-explorer leaf: sortOrder)

**Check it.** Create `Note 1.md`, `Note 2.md`, `Note 10.md` in a folder:  lists them 1, 10, 2. Set the vault's explorer to "Modified time (new to old)" in Obsidian and open the vault in : still alphabetical.

**Fix.** Add a `SortOrder` enum parsed from the leaf state string; implement a natural compare (split digit runs, compare numerically, tie-break case-insensitively) for the alphabetical modes and use `VaultFile.mtime`/ctime for the time modes; keep folders first and always alphabetical for folders (as Obsidian does). Write the chosen order back to the leaf state only when  persists workspace.json.

### Delete confirmation has no "Don't ask again" checkbox and its button is not styled as destructive

*medium, missing-feature, small*

**.** `start_delete` opens the generic prompt with title `Delete "name"?`, a one-line detail about the destination, and Cancel/Delete buttons (workspace.rs:2485-2509). `PromptSpec` has no checkbox field (prompt.rs:36-48); the confirm button is always the accent colour (prompt.rs:235). `promptDelete` is only read (workspace.rs:466-468), never written, so the user cannot turn the prompt off from the dialog.

**Obsidian.** Obsidian's delete dialog (title "Delete file" / "Delete folder", text "Are you sure you want to delete \"X\"?" plus where it goes) carries a "Don't ask again" checkbox that sets `promptDelete: false` in app.json (Settings → Files and links → "Confirm file deletion"), and its Delete button is the red `mod-warning` style. Exact title/body wording is from memory and may differ slightly.

**Evidence.** crates//src/workspace.rs:2485-2509, crates//src/prompt.rs:36-48, crates//src/prompt.rs:228-242, crates//src/workspace.rs:466-468

**Check it.** Right-click a note → Delete: the dialog shows two neutral/accent buttons and no checkbox; there is no way to stop being asked without editing app.json.

**Fix.** Add `checkbox: Option<(label, bool)>` to `PromptSpec` and a `Submit{value, checked}` payload; when checked on Delete, write `promptDelete: false` through the existing config writer (`vault.write_config` style, preserving unknown keys). Add a `danger: bool` to the spec that paints the confirm button red.

### No drag and drop in the Files view

*medium, missing-feature, large*

**.** Rows register only `on_click` and a right-button `on_mouse_down` (explorer.rs:447-453); there is no `on_drag`, drop target, drag ghost, spring-loading or root drop zone. Moving is only possible through the "Move file to..." modal (workspace.rs:2418-2452). Development.md:1157 lists drag and drop in Files as not done.

**Obsidian.** Specification.md:1921-1981: every row is draggable; a drop on a folder row moves the file into it, on a file row into that file's parent, on the list background to the vault root; the ghost shows the title plus `Move into “Dest”`; the hovered container gets `is-being-dragged-over`; a collapsed folder spring-loads open after ~700-800 ms; a name collision auto-numbers (`Alpha 1.md`) with no prompt; dropping a folder onto its own descendant does nothing.

**Evidence.** crates//src/explorer.rs:433-470, crates//src/workspace.rs:2418-2452, docs//Development.md:1157, docs//Specification.md:1921-1981

**Check it.** Press on a note row in Files and drag it over a folder row: nothing follows the pointer and releasing changes nothing.

**Fix.** Use gpui's `on_drag` with a `DraggedRow{path, is_folder}` payload and a ghost element; make folder rows, file rows and the list container `drag_over`/`on_drop` targets that emit `ExplorerEvent::MoveTo{from, folder}`; the workspace calls `rename_with_links(from, available_path(folder, stem, ext))` so collisions number like Obsidian. Add a 750 ms hover timer that expands a collapsed folder.

### Non-markdown files show the extension inline instead of as a tag badge

*low, parity-gap, small*

**.** `display_name` strips only `.md` (explorer.rs:223-229); any other file is one string `cover.png` drawn entirely in `text_muted` (explorer.rs:431, 444). Canvas and base files thus read `Board.canvas`.

**Obsidian.** Specification.md:1996-1998: a non-markdown row renders the basename in `.nav-file-title-content` and the extension in a sibling `.nav-file-tag` div (`cover.png` → `cover` + `png`); the tag is a small uppercase pill on the right, and the name itself is in the normal text colour.

**Evidence.** crates//src/explorer.rs:223-229, crates//src/explorer.rs:431-444, docs//Specification.md:1996-1998

**Check it.** Open a vault with an image and a canvas: the rows read `cover.png` and `Board.canvas` in grey, with no badge.

**Fix.** Split `Row.name` into `stem` and `Option<ext>`; render the ext as a trailing `div` with small uppercase text, a rounded background and `flex_shrink_0`, and keep the stem in `theme.text`. Sort by stem as now.

### Rename prompt accepts `/` and silently moves the file into a new folder; empty name gives the wrong error

*low, bug, small*

**.** `prompt_submitted` joins the typed value into `folder/value.ext` (workspace.rs:2569-2592) and `Vault::rename` validates per `/`-segment (ops.rs:87-95, 229-233) then `create_dir_all`s the parent (ops.rs:247-249), so typing `Sub/New` in Rename creates `Sub/` and moves the note there. 's own `FORBIDDEN_NAME_CHARS` lists `/` and the doc comment says `validate_file_name` is the explorer-rename rule (-formats/src/lib.rs:71-85), but the prompt never calls it on the whole value. An empty value becomes `.md`, which is rejected as "File name must not start with a dot." rather than "File name cannot be empty." (workspace.rs:2585, lib.rs:76-80).

**Obsidian.** Obsidian's rename refuses the name with "File name cannot contain any of the following characters: * " \ / < > : | ?" and an empty name with "File name cannot be empty." (standard Obsidian messages; the forbidden set is the one -formats already encodes). The spec marks "inline rename onto a name already taken" as unprobed but not the forbidden characters.

**Evidence.** crates//src/workspace.rs:2569-2592, crates/-vault/src/ops.rs:87-95, crates/-vault/src/ops.rs:229-249, crates/-formats/src/lib.rs:71-85

**Check it.** Right-click `Note.md` → Rename..., type `Sub/Note` and press Enter: a `Sub` folder appears and the note moves into it; no error. Clear the field and press Enter: the error mentions a dot, not an empty name.

**Fix.** In `PendingOp::Rename`, run `validate_file_name(value)` on the raw value first and surface its message via `prompt.set_error`; only then build the path.

### Context menu entries and order differ from Obsidian's file and folder menus

*low, parity-gap, small*

**.** File rows get: New note, New folder, Rename..., Move file to..., Make a copy, Delete; folder rows: New note, New folder, Rename..., Move folder to..., Delete (explorer.rs:328-365). There are no separators, no Open in new tab/to the right, no Bookmark..., no New canvas/New base, no Show in system explorer / Open in default app, no Copy path.

**Obsidian.** Less sure of the exact list, from memory of 1.x desktop: a file's menu is roughly Open in new tab, Open to the right, Open in new window, [sep], Make a copy, Move file to..., Bookmark..., [sep], Copy Obsidian URL / Open in default app / Show in system explorer, [sep], Rename..., Delete — New note / New folder are not offered on a file. A folder's menu starts New note, New folder, New canvas, New base, Set as attachment folder, then Rename..., Move folder to..., Delete and the system items, with separators. Interaction probes.md:218 asks to record the folder menu verbatim, so this is still an open probe.

**Evidence.** crates//src/explorer.rs:328-365, docs//Interaction probes.md:218

**Check it.** Right-click any note in Files and compare the list with Obsidian's on the same vault.

**Fix.** Record the real menus with the probe checklist step, then reorder `items` and add separators (a 1 px `theme.border` div) plus the entries  can honour now (Open in default app / Show in system explorer via `xdg-open`, Copy path); leave tabs/bookmarks until those subsystems exist.

### Explorer has no keyboard focus, no arrow-key navigation, and no menu on empty space

*low, missing-feature, medium*

**.** `FileExplorer` holds no `FocusHandle`, registers no key context or actions (explorer.rs:253-258, 476-490), so clicking a row does not focus the pane and Up/Down/Left/Right/Enter do nothing there; F2 only reaches `workspace:edit-file-title` for the open note. The right-click handler is per row (explorer.rs:448-453), so right-clicking the blank area under the last row opens nothing.

**Obsidian.** Clicking a row focuses the explorer (the spec shows rows gaining `has-focus`, Specification.md:1983-1988); Up/Down move the selection, Left/Right collapse/expand or move to the parent, and Enter opens (moderately sure). Right-clicking the empty part of the file list opens the vault-root folder menu (New note, New folder, ...) — moderately sure.

**Evidence.** crates//src/explorer.rs:253-258, crates//src/explorer.rs:448-453, crates//src/explorer.rs:476-490, docs//Specification.md:1983-1988

**Check it.** Click a note in Files, press Down: the editor, not the explorer, receives the key. Right-click below the last row: no menu.

**Fix.** Give `FileExplorer` a focus handle and an `Explorer` key context with Up/Down/Left/Right/Enter/F2 actions over a `selected` index; put the `on_mouse_down(Right)` on the outer container as well, defaulting to the root folder menu when no row is hit.

### Rename/move feedback goes to the status bar in 's wording instead of a Notice with Obsidian's text

*low, parity-gap, small*

**.** `apply_rename` reports "Renamed X to Y, updated N links in M files" (or "Moved N files") in the status bar for every rename, including ones that touched no link (workspace.rs:2745-2783); `delete_now` reports "Deleted path" (workspace.rs:2539-2541); `duplicate` reports "Copied ... to ..." (workspace.rs:2553-2556).

**Obsidian.** Specification.md:1962-1967: a move that rewrites links shows a Notice reading exactly `Updated 1 link in 1 file.`; moves that leave the basename alone produce no notice at all. Obsidian shows nothing in the status bar for file operations.

**Evidence.** crates//src/workspace.rs:2745-2783, crates//src/workspace.rs:2539-2556, docs//Specification.md:1962-1967

**Check it.** Move a note whose name is unique into another folder: the status bar says "Renamed A.md to B/A.md" although Obsidian would say nothing; rename a linked note: the text differs from `Updated 1 link in 1 file.`

**Fix.** Only report when `rewritten_links > 0`, using the `Updated {n} link{s} in {m} file{s}.` format, and route it through a transient toast once one exists; keep the stale-file warning as a separate message.

### Prompt modals have no dimmed backdrop and sit at a fixed 90 px from the top

*low, parity-gap, small*

**.** The Rename/Move/Delete/New folder prompt (and switcher/palette) is placed in an `absolute inset_0` wrapper with `items_start().pt(px(90.))` and no background (workspace.rs:3486-3497), while only the Settings modal gets a `rgba(0x00000080)` backdrop (workspace.rs:3500-3506). The prompt itself is 480 px wide with `shadow_lg` (prompt.rs:249-258).

**Obsidian.** Every Obsidian modal is drawn over a `.modal-bg` dim layer; confirmation dialogs (Delete, Update links) are centered vertically, and suggest-style modals (Move file to..., switcher) sit near the top. Clicking the dim layer closes the modal. Fairly sure of the backdrop; the exact vertical offsets are from memory.

**Evidence.** crates//src/workspace.rs:3486-3506, crates//src/prompt.rs:249-258

**Check it.** Right-click a note → Delete: the card floats near the top with the page fully bright behind it; compare with Settings, which dims the page.

**Fix.** Reuse the Settings backdrop div for the Prompt/Switcher/Palette branch (closing on click), and give `PromptSpec` a `centered: bool` that switches `items_start().pt(90)` to `items_center()` for confirmation prompts.

## Live Preview

### Heading and list folding does not exist in Live Preview (no indicator, no fold commands)

*high, missing-feature, large*

**.** The editor has no fold state or fold logic at all: grepping editor.rs for fold yields only the properties fold (editor.rs:2635, :3699) and the Box-leak comment; the workspace's RUNNABLE command list (workspace.rs:86-140) contains editor:toggle-fold-properties but none of editor:toggle-fold, editor:fold-all, editor:unfold-all, editor:fold-more, editor:fold-less, so the palette never offers them even though commands.txt lists them (-commands/data/commands.txt:60-62,121,134). No fold chevron is painted in the left margin, and nothing appends a placeholder to a heading line.

**Obsidian.** Spec 'Observed heading fold behavior' (Specification.md:709-855): in Live Preview every foldable line carries a hover-revealed collapse indicator at left:-15px (headings) / -10px (list parents); editor:toggle-fold folds the heading through the line before the next heading of same-or-higher level (trailing blanks included, last heading to EOF), the folded heading line stays rendered with a trailing '…' (cm-foldPlaceholder), fold-all/unfold-all work on headings and list parents, state is kept in localStorage under '<appId>-note-fold-<path>' and nothing is written to the vault; foldHeading/foldIndent default to true.

**Evidence.** crates//src/workspace.rs:86-140, crates//src/editor.rs:2635, -commands/data/commands.txt:60-62, -commands/data/commands.txt:121, -commands/data/commands.txt:134, docs//Specification.md:709-760

**Check it.** Open a note with '## A' / body / '## B' in Live Preview. Hover the left margin of '## A': no triangle appears. Open the command palette (Ctrl+P) and type 'fold': only 'Toggle fold properties' is listed; 'Toggle fold on the current line' and 'Fold all headings and lists' are absent.

**Fix.** Add a fold set (line ranges) to Editor; compute a heading's extent from the parse (heading level -> next same/higher heading, trailing blanks included; list parent -> its children) and skip folded lines in prepaint the way folded widgets are skipped (height 0, line = range.end), appending '…' to the display text of the folded line. Paint a chevron quad in the gutter on hover of a foldable line; add the five editor:*fold* ids to RUNNABLE and dispatch them to the editor. Persist per-note fold ranges in 's own app-data (not the vault), mirroring the localStorage record.

### Callouts render as a bare quote with a renamed title, not as a callout widget

*high, parity-gap, medium*

**.** build_widgets only promotes Yaml, Table, Math and single-embed paragraphs (reading/model.rs:254-262); a Callout block is left to the per-line transform. preview.rs:325-355 hides the '[!type]' token on the title line (showing the capitalised type when the title is empty) and editor.rs:3825-3830 paints a 2px muted quote bar per level. The result is a title line in accent text with a thin grey bar, the body as plain quoted text, no icon, no type-coloured background/border, no fold chevron for '[!note]-' or '+', and the body never collapses.

**Obsidian.** Live Preview renders a callout the cursor is not in as an embed widget (the spec's code-copy probe saw 'one cm-embed-block cm-callout' inside the CodeMirror DOM, Specification.md:948-951, 961-965): a block with the type's colour as left border and tinted background, a Lucide icon and bold coloured title, a collapse chevron when the type has +/-, and the body rendered. Clicking into it reveals the source lines. 's own reading renderer already draws this shape (reading/view.rs:525-560).

**Evidence.** crates//src/reading/model.rs:254-262, crates//src/preview.rs:325-355, crates//src/editor.rs:3825-3830, crates//src/reading/view.rs:525-560, docs//Specification.md:948-951

**Check it.** In Live Preview type '> [!warning]- Careful' then '> body'. Move the cursor to another paragraph: the block shows only a grey left bar, the word 'Careful' in accent colour and the body in normal text; no orange background, icon or chevron; nothing collapses when the title is clicked.

**Fix.** Add BlockKind::Callout to build_widgets (reading/model.rs) so it becomes a folded widget rendered by Renderer::block's RBlock::Callout arm; keep the existing unfold-on-touch rule so a selection inside the callout's lines reveals its source, and route the fold chevron through the ToggleHandler the editor already wires for the properties block (editor.rs:3689-3698).

### Code blocks show no language flair / copy affordance and no syntax colouring in Live Preview

*high, parity-gap, medium*

**.** A fenced or indented block's lines get a single CodeBlock span (highlight.rs:203) which run_for turns into monospace on a bg_alt band (editor.rs:3436-3442); prepaint paints the band (editor.rs:3816-3823). Nothing paints a language label or a copy button, and nothing tokenises the code. The Renderer's RBlock::Code arm, which does draw a language label (reading/view.rs:482-508), is never used for Live Preview because code blocks are not widgets (reading/model.rs:254-262).

**Obsidian.** Spec 'Observed code-block copy behavior' (Specification.md:948-960): in the editor a fence line gets span.code-block-flair at top-right (absolute, right:6px, top:6px, font-ui-smaller, text-muted, hover background) with aria-label 'Copy' and the language's display name ('JavaScript' for ```js); clicking it copies the body. Obsidian also syntax-highlights fenced code in the editor (well-known; help.obsidian.md 'Basic formatting syntax > Code blocks' lists supported languages).

**Evidence.** crates/-markdown/src/highlight.rs:203, crates//src/editor.rs:3436-3442, crates//src/editor.rs:3816-3823, crates//src/reading/model.rs:254-262, docs//Specification.md:948-960

**Check it.** In Live Preview type a ```js fence with 'const x = 1;' and move the cursor out. The block is a grey band with the fence lines visible; no 'JavaScript' label at the top right, nothing to click to copy, and 'const' is not coloured.

**Fix.** In prepaint, when a line is the first line of a CodeBlock span and live_preview is on, lay out a small label element (language display name, text-muted, padding 2/4px, rounded) at the band's top-right and record its bounds in Layout so mouse_down can copy the fence body (body minus exactly one trailing newline, per the spec) to the clipboard. Syntax colouring is a separate, larger piece (a tokenizer per language feeding extra runs).

### Images and note embeds only render when they are a whole top-level paragraph; anywhere else they show as link text

*high, parity-gap, large*

**.** build_widgets accepts a Paragraph only when its content is exactly one Wikilink{embed:true} or inline Image (reading/model.rs:257-266) and only walks doc.blocks, i.e. top level. An image with any other text on the line, or inside a list item, blockquote or callout, falls through to preview.rs:238-256 / 258-283 which show '![[pic.png|300]]' as the text 'pic.png' in link colour and '![alt](a.png)' as 'alt' (see the test at preview.rs:445-450).

**Obsidian.** Obsidian renders image embeds inline wherever they occur in Live Preview (help.obsidian.md 'Embed files' shows images in lists and paragraphs; the spec's raw-HTML probe records the editor-side form of an image embed as div.internal-embed with image-wrapper, image-resize-corner and embed-actions, Specification.md:1046-1047), honouring |width and |WxH.

**Evidence.** crates//src/reading/model.rs:254-266, crates//src/preview.rs:238-256, crates//src/preview.rs:445-450, docs//Specification.md:1046-1047

**Check it.** In Live Preview write '- ![[pic.png|200]]' as a list item, and on another line 'Look: ![[pic.png]] here'. Move the cursor away: both show the file name as underlined link text; no picture appears. The same '![[pic.png|200]]' alone on its own line does render.

**Fix.** Extend the Embed/Image hide in preview.rs to substitute an inline image placeholder piece that prepaint replaces with an img element of the requested width (measure height into widget_heights like block widgets), and let build_widgets also walk list items/quotes/callouts for embed-only paragraphs. Minimum step: treat an embed-only paragraph nested in a list item as a widget too.

### Headings, list markers, checkboxes and callout titles are painted in the accent colour

*medium, parity-gap, small*

**.** run_for colours Heading runs with p.accent (editor.rs:3421-3424) and Callout/ListMarker/Task runs with p.accent (editor.rs:3458-3460); the accent is the purple 0x8a7cf5/0x7b6cd9 (theme.rs:44,62). 's own reading view draws headings in the normal text colour (reading/view.rs:458-470), so the two surfaces disagree.

**Obsidian.** Obsidian's default theme sets --h1-color … --h6-color to var(--text-normal) (headings are bold normal text, not accent) and --list-marker-color to var(--text-faint), so bullets and numbers are dimmed; checkbox borders use --checkbox-border-color (muted) with accent only as the checked fill. Well-known default-theme CSS variables (help.obsidian.md 'CSS variables' > Headings / Lists); I am confident about headings and list markers.

**Evidence.** crates//src/editor.rs:3421-3424, crates//src/editor.rs:3458-3460, crates//src/theme.rs:44, crates//src/reading/view.rs:458-470

**Check it.** Open any note with '# Title' and '- item' in Live Preview: the title text and the bullet are purple. Press Ctrl+E to the reading view: the same title is the normal text colour.

**Fix.** In run_for use base.color for Heading (keep bold and scale), p.muted/faint for ListMarker, and keep accent only for a checked Task glyph; leave Callout title colour to the callout widget (finding 2).

### [[Note#Heading]] and [[Note#^id]] display as 'Note#Heading' instead of 'Note > Heading'

*medium, parity-gap, small*

**.** The Wikilink hide only strips '[[' / ']]' or everything up to '|' (preview.rs:238-256); the test at preview.rs:453-457 pins '[[Note#H]]' rendering as 'Note#H'. The same '#' survives for block refs.

**Obsidian.** Spec 'Observed Obsidian behavior': '`[[Note#H]]` displays as `Note > H`' (Specification.md:141), and the reading model titles a block embed 'Note > ^id' (reading/model.rs:95). Obsidian's Live Preview shows the same 'Note > H' label for an un-aliased heading link.

**Evidence.** crates//src/preview.rs:238-256, crates//src/preview.rs:453-457, docs//Specification.md:141

**Check it.** Type '[[Some Note#Intro]]' in Live Preview and move the cursor off the line: the link reads 'Some Note#Intro'.

**Fix.** When the body has no '|', substitute the whole '[[body]]' with a rendered label built like wikilink_label plus ' > ' joins for each subpath segment ('#' and '#^'); keep the piece a Sub so clicks still resolve the full body.

### No hover affordances: no pointer over links, no Ctrl+hover page preview

*medium, missing-feature, large*

**.** The only MouseMoveEvent handler acts when the left button is pressed (editor.rs:4058-4064); nothing tracks the hovered link, changes the cursor style or opens a popover. link_at exists (editor.rs:2459) but is used only on click. No hover/popover code exists anywhere in workspace.rs or reading/view.rs beyond row background hovers.

**Obsidian.** Hovering a rendered link or tag in Live Preview shows a pointer cursor; with the core Page preview plugin (enabled by default) Ctrl/Cmd+hover over an internal link opens a preview popover of the target note (help.obsidian.md 'Page preview'). Spec Interaction probes refer to the Ctrl+hover page-preview popup as expected on internal links (Interaction probes.md:362).

**Evidence.** crates//src/editor.rs:4058-4064, crates//src/editor.rs:2459-2510, docs//Interaction probes.md:362

**Check it.** In Live Preview hover a rendered [[link]] with and without Ctrl held: the cursor stays an I-beam and no preview card appears.

**Fix.** Handle MouseMoveEvent without a pressed button: hit-test with display_hit + link_at, set window cursor style to pointer over links/tags/checkboxes, and when Ctrl is held for ~300 ms open a floating popover (reuse Renderer over the target note's first screen) anchored under the link; close on move-away.

### Checked tasks are a Unicode glyph and their text is not struck through or dimmed

*medium, parity-gap, medium*

**.** The Task hide substitutes the literal glyphs '☐'/'☑' (preview.rs:319-324) painted in accent (editor.rs:3458-3460); the rest of the line keeps its normal colour and weight regardless of status. Clicking writes [x]/[ ] (editor.rs:2525-2540), which matches Obsidian.

**Obsidian.** Obsidian draws a real square checkbox (input.task-list-item-checkbox) and, in the default theme, a done task's text gets --checklist-done-decoration: line-through and --checklist-done-color: var(--text-muted) in both Live Preview and reading view (default-theme CSS variables; any non-space status counts as checked per Specification.md:669-684).

**Evidence.** crates//src/preview.rs:319-324, crates//src/editor.rs:3458-3460, docs//Specification.md:669-684

**Check it.** Type '- [x] done' and '- [ ] todo' in Live Preview and move the cursor away: 'done' is drawn in the same colour and without a strike; the box is a font glyph rather than a drawn square.

**Fix.** Paint the checkbox as a quad (border muted, accent fill + check when the status byte is not a space) at the Sub piece's position instead of a glyph, and in runs_for_display apply strikethrough + muted colour to the Copy pieces that follow a checked Task on the same line.

### Tags are coloured text, not pills

*medium, parity-gap, small*

**.** run_for gives a Tag run only color = p.tag (editor.rs:3434), with no background or padding; display_line leaves the '#' visible (preview.rs:378-381 treat Tag as no-op).

**Obsidian.** In Live Preview the default theme renders #tags as pills: .cm-hashtag gets --tag-background, --tag-padding, --tag-radius and --tag-color (accent), the '#' included (help.obsidian.md 'CSS variables > Tags'; default-theme appearance). Clicking opens a search for the tag, which  already does.

**Evidence.** crates//src/editor.rs:3434, crates//src/preview.rs:378-381

**Check it.** Type '#project' in Live Preview: it is purple text on the plain background with no rounded pill behind it.

**Fix.** Give Tag runs a background_color (accent at ~10-15% alpha) in run_for; for the rounded pill, paint a rounded quad behind each Tag span in prepaint using row_and_x of its start/end (same technique as the find-match quads at editor.rs:3836-3862).

### A task with more than one space between the bullet and '[ ]' shows both a bullet and a checkbox

*medium, bug, small*

**.** The ListMarker branch hides the marker only when a Task span starts exactly at r.end + 1 (preview.rs:305-316, the find at :309); otherwise the '-' is replaced by '•'. Since the 2026-10-18 fix the Task span comes from the parser's task_span (highlight.rs:221-224), which accepts 1-4 spaces or a tab after the marker (Development.md:355-380), so '- [x] a' (two spaces) renders as '• ☑ a' and a tab-padded task as '•\t☑ a'.

**Obsidian.** Obsidian renders '- [x] a' as a single checkbox followed by 'a' (the item is a task; spec Specification.md:669-684 shows every accepted padding yields a task-list-item with one input).

**Evidence.** crates//src/preview.rs:305-316, crates/-markdown/src/highlight.rs:221-224, docs//Development.md:355-380

**Check it.** In Live Preview type '- [x] padded' (two spaces after the dash) and move the cursor to another line: the line shows a bullet dot, a gap, then the checked box.

**Fix.** In the ListMarker branch look for the Task span whose start is >= r.end and whose bytes between are only spaces/tabs (or just use the first Task span after r.end on this line), and hide r.start..task.range.start as now.

### HTML blocks are shown as raw muted source instead of being rendered

*medium, parity-gap, large*

**.** HtmlBlock gets one Html span (highlight.rs:204) which run_for paints muted (editor.rs:3451-3455); display_line does nothing for Html (preview.rs:378-381) and build_widgets never promotes HtmlBlock (reading/model.rs:254-262). The reading Renderer's RBlock::Html arm is also raw text (reading/view.rs:509-515).

**Obsidian.** Spec 'Observed raw HTML behavior': the editor renders raw HTML through a span.cm-html-embed widget whose <img>s carry resolved app:// srcs (Specification.md:1049-1050); reading view renders <img>, <a>, <pre>, <b> etc. with a sanitizer (Specification.md:1017-1078).

**Evidence.** crates/-markdown/src/highlight.rs:204, crates//src/editor.rs:3451-3455, crates//src/reading/model.rs:254-262, docs//Specification.md:1049-1050

**Check it.** Put '<img src="pic.png" width="100">' alone on a line in Live Preview and move the cursor away: the tag text stays visible in grey; no image.

**Fix.** Start with the subset the spec measured: make HtmlBlock a widget and render a tiny HTML subset in Renderer (img with vault-resolved src and width/height, a, b/i/u/kbd/sup/sub, pre) — the rest can stay raw.

### Callout with an empty title leaks a heap string every time the line is shaped

*low, bug, small*

**.** preview.rs:336-343 builds the capitalised type name and calls Box::leak(name.into_boxed_str()) to fit the &'static str slot of hides. display_line runs for every visible line on every frame, and the editor shapes a line several times per frame (height_of, shape, prepaint, the cursor-line reshape at editor.rs:3909-3925), so a note with '> [!tip]' on screen leaks a small allocation per shape indefinitely.

**Obsidian.** n/a (resource leak); a callout with no title shows the type name as the title, which  does correctly.

**Evidence.** crates//src/preview.rs:336-343, crates//src/editor.rs:3909-3925

**Check it.** Leave a note containing '> [!tip]' (no title) open in Live Preview and scroll or type for a while; process RSS grows slowly and never comes back. (Read-only confirmation: the Box::leak is unconditional on that path.)

**Fix.** Change the hides tuple's replacement to Cow<'static, str> (or String) so the type name is owned by the DisplayLine, and drop Box::leak.

### Ctrl+click on a link opens it in the current pane rather than a new tab

*low, parity-gap, medium*

**.** click_rendered emits EditorEvent::OpenLink(target) for both plain and Ctrl clicks (editor.rs:2544-2550); the event carries no modifier and workspace::follow_link always opens in the single current pane (workspace.rs:2207-2239). The workspace has no editor tabs today (the only Tab enums are the sidebar tabs, workspace.rs:744-750).

**Obsidian.** Plain click follows a rendered link in Live Preview (matches); Ctrl/Cmd+click opens the link in a new tab, Ctrl+Alt in a split (help.obsidian.md 'Internal links' / 'Tabs'). Confident.

**Evidence.** crates//src/editor.rs:2544-2550, crates//src/workspace.rs:2207-2239, crates//src/workspace.rs:744-750

**Check it.** Ctrl+click a rendered [[link]] in Live Preview: the current note is replaced in place; no new tab header appears.

**Fix.** Add a `new_tab: bool` (and later `split`) to EditorEvent::OpenLink from event.modifiers; once the workspace has tab headers, open_file into a new tab when set. Until then this is blocked on the tab subsystem.

### List-typed properties are edited as one comma-joined text field, not as individual pills

*low, parity-gap, medium*

**.** Row.edit_text is value.display() which joins list items with ', ' (properties.rs:85) and commit() re-parses the whole field as the type (properties.rs:363 via Value::parse_as), so the field for tags/aliases/multitext is a single line of 'a, b'; an alias containing a comma ('Smith, John') cannot be entered or round-tripped, and there is no per-pill remove or Enter-to-add.

**Obsidian.** In Obsidian's properties widget a List/Tags/Aliases value is a set of pills with an inline input: Enter or comma adds an item, Backspace on the empty input removes the last pill, each pill has its own remove 'x' (help.obsidian.md 'Properties > Property types: List'). The spec confirms the widget is the same editable form in both views (Specification.md:243-244).

**Evidence.** crates//src/properties.rs:85, crates//src/properties.rs:363, docs//Specification.md:243-244

**Check it.** Open a note whose frontmatter has 'aliases:\n - One\n - Two' in Live Preview and click the value: a single text field reading 'One, Two' opens instead of two pills with an input after them.

**Fix.** For list types open the Field on a new empty item (Target::Existing + an item index), commit by appending to Value::List; Backspace in an empty item removes the previous pill; render an 'x' per pill routed through PropertyAction.

### Block ids are hidden entirely in Live Preview

*low, parity-gap, small*

**.** The BlockId branch hides '^id' together with the space before it when the cursor is off the construct (preview.rs:294-304; the test at preview.rs:482-484 expects 'text end' for '... end ^id').

**Obsidian.** Less sure: my recollection of Obsidian's Live Preview is that a block id stays visible at the end of its line as faint text (span.cm-blockid) rather than being hidden, so a user can see that a block is referenced. The spec's block-reference section covers writing ids, not their display (Specification.md:431-527). Treat as needing a probe before acting.

**Evidence.** crates//src/preview.rs:294-304, crates//src/preview.rs:482-484

**Check it.** Write 'A paragraph ^abc123' in Live Preview and move the cursor to another line: the ' ^abc123' disappears completely.

**Fix.** If a probe confirms, keep the BlockId text as a Copy piece styled muted (run_for already colours BlockId muted) instead of hiding it; only drop the hide at preview.rs:294-304.

## Reading view

### `propertiesInDocument: hidden` is ignored by the reading view; the properties table is always drawn

*high, bug, small*

**.** `Renderer::properties` only returns an empty element when a `PropertiesHost` exists with `mode == Hidden` (view.rs:884-889). The reading view passes `None` for the host (view.rs:222-225), and `Workspace::apply_app_config` pushes the `propertiesInDocument` setting into the editor only (`e.set_properties_in_document(properties)`, workspace.rs:1130-1131); `ReadingView` has no field for it (view.rs:102-114). So with `hidden` in app.json the editor hides the block but the reading view still shows the full table.

**Obsidian.** Spec, 'Observed properties-in-reading-view behavior' (Specification.md ~1222-1236): with `hidden` "the block is gone from the rendered text" in the reading view too; `source` shows the same table in reading view (only the editor swaps to YAML). The preview root loses `show-properties` and the container computes `display: none`.

**Evidence.** crates//src/reading/view.rs:884-889, crates//src/reading/view.rs:222-225, crates//src/workspace.rs:1108-1132, docs//Specification.md:1222-1236

**Check it.** Open a vault whose .obsidian/app.json has "propertiesInDocument": "hidden" (or set it from Settings > Editor), open a note with frontmatter, press Ctrl+E until the mode label reads Reading. The Properties heading and rows are still visible at the top of the note; switch to Live Preview and they are hidden.

**Fix.** Add a `properties_in_document: PropertiesInDocument` field to `ReadingView`, set it from `apply_app_config` alongside `set_readable_line_length`, pass it into `Renderer` (either as a separate parameter or by building a read-only `PropertiesHost` with `on_action` a no-op) and make `properties()` check that mode regardless of host. `source` must still draw the table in the reading view.

### No text selection or copy in the reading view

*high, missing-feature, large*

**.** Inline content is `InteractiveText` with only an `on_click` over link ranges (view.rs:1316-1320); `ReadingView` holds no selection state (view.rs:102-114) and no mouse-drag or key handlers; the `ReadingView` key context is declared (view.rs:255) but `bind_keys` binds every command only in the `Workspace` context (workspace.rs:151-164) and no copy action exists for the reading view. Dragging over text selects nothing and Ctrl+C copies nothing.

**Obsidian.** Obsidian's reading view is rendered DOM: click-drag selects text across blocks, double-click selects a word, Ctrl+A selects the whole note and Ctrl+C copies the selection as plain text (well-known, help.obsidian.md 'Read and edit' / standard Electron behaviour).

**Evidence.** crates//src/reading/view.rs:1295-1321, crates//src/reading/view.rs:102-114, crates//src/workspace.rs:151-164

**Check it.** Open any note in Reading mode, press the left mouse button on a word and drag across a paragraph: no selection highlight appears. Press Ctrl+C and paste into another app: the clipboard is unchanged.

**Fix.** Add a selection model to `ReadingView` (anchor/head as (block index, RichText id, byte offset)), track mouse down/drag/up on the view, paint selection backgrounds through `StyledText` highlights (background_color on the selected ranges), and bind a `Copy` action in the `ReadingView` context that joins the selected runs with newlines and writes `ClipboardItem::new_string`. Ctrl+A selects all blocks.

### Task checkboxes are not clickable in the reading view

*high, parity-gap, medium*

**.** `Renderer::item` draws the checkbox as a plain `div` with a border and a check glyph and attaches no id, cursor or click handler (view.rs:737-755); the reading view's `Renderer` has no handler type for it at all (only `on_link` and `on_toggle`, view.rs:304-305). Clicking a task does nothing. Development.md:1229-1230 lists checkbox toggling as not yet done for the reading view.

**Obsidian.** Spec 'Observed task checkbox behavior' (Specification.md:641-660): clicking `input.task-list-item-checkbox` in the reading view writes exactly that line, `[ ]` -> `[x]` and any non-space status -> `[ ]`, and leaves the rest of the file byte-identical.

**Evidence.** crates//src/reading/view.rs:737-755, crates//src/reading/view.rs:304-305, docs//Specification.md:641-660

**Check it.** Open a note containing `- [ ] todo` in Reading mode and click the box. Nothing changes on screen and the file on disk is untouched; in Live Preview the same click toggles it.

**Fix.** Carry the task marker's byte span from the parser into `RItem` (the editor already gets it from the parser per the 'Carry the task checkbox span' commit), add an `on_task: Rc<dyn Fn(Range<usize>, &mut Window, &mut App)>` to `Renderer`, make the box an `id`'d element with `cursor_pointer().on_click`, and have the workspace apply the three-byte edit through the editor's existing toggle path then rebuild the model. Inside an embed the span belongs to another file; either pass the embed's path with the span or leave embedded checkboxes inert.

### No inline title at the top of the note (showInlineTitle defaults to true)

*high, parity-gap, small*

**.** `ReadingView::render` draws only the block list (view.rs:250-289); the first block gets a 24 px top padding (view.rs:237) and no title element exists. The file name appears only in the 36 px pane header as 12 px text (workspace.rs:3217-3222, 3238-3246). Development.md:929-933 lists inline title among settings written for Obsidian but not implemented.

**Obsidian.** With `showInlineTitle` (default on) the reading view starts with `div.mod-header.mod-ui` holding the `.inline-title` (the note's basename, h1-sized, editable to rename) above the properties block (Specification.md:1259-1262, 2026-2028; help.obsidian.md 'Appearance' > 'Show inline title').

**Evidence.** crates//src/reading/view.rs:250-289, crates//src/reading/view.rs:231-245, crates//src/workspace.rs:3217-3222, docs//Development.md:929-933, docs//Specification.md:1259-1262

**Check it.** Open any note in Reading mode: the body starts directly with the properties table or first paragraph. In Obsidian the same note shows its name in large type above everything.

**Fix.** Add a `ShowInlineTitle` flag to `ReadingView` set from `apply_app_config`; in `render_block` for `ix == 0` (or as an extra first list item) render the basename (`display_name`) at the h1 size/weight with Obsidian's `--inline-title` margins, before the properties block. Later: click-to-rename like Obsidian, sharing the explorer's rename path.

### Following `[[Note#Heading]]` / `[[Note#^id]]` scrolls only the reading view and never flashes the target

*medium, bug, medium*

> [!done] Half fixed 2026-10-19
> The scroll half is fixed: the editor is revealed too. Obsidian's flash highlight on
> the target is still missing.

**.** `scroll_to_subpath` resolves the offset from the cache and then calls only `self.reading.update(.. scroll_to_offset ..)` (workspace.rs:2175-2205); `open_file` calls it unconditionally (workspace.rs:1972-1975) and so do search, outline and backlink rows (workspace.rs:928-931, 2870-2872, 2920-2923). When the workspace is in Live Preview or Source the editor is what is shown and it is not scrolled, so the link lands at the top of the note. `scroll_to_block` only sets a `ListOffset` (view.rs:161-169); no highlight of the target block is drawn.

**Obsidian.** Obsidian opens the target in whichever mode the leaf uses and scrolls to the heading or block, then briefly flashes it (the `is-flashing` class on the target element) so the reader sees where they landed (well-known; help.obsidian.md 'Internal links' > 'Link to a heading/block').

**Evidence.** crates//src/workspace.rs:2175-2205, crates//src/workspace.rs:1972-1975, crates//src/workspace.rs:928-931, crates//src/reading/view.rs:161-169

**Check it.** In Live Preview, click a wikilink with a heading subpath that points at a heading near the bottom of a long note (e.g. `[[Spec#Observed search pane behavior]]`). The note opens scrolled to its top. In Reading mode it scrolls to the heading but nothing is highlighted.

**Fix.** In `scroll_to_subpath`, branch on `self.mode`: for editing modes convert the offset to a line and call the editor's `scroll_to_line` (editor.rs:1064) / reveal; for the reading view keep `scroll_to_offset`. Add a `flash: Option<(usize, Instant)>` to `ReadingView` (and the editor) that paints the target block with `theme.highlight_bg` fading over ~1 s via `window.on_next_frame`.

### Headings have no fold indicator and cannot be collapsed in the reading view

*medium, parity-gap, medium*

**.** `RBlock::Heading` renders as a styled `div` with no id, hover state or click handler (view.rs:458-475), and the model keeps no notion of a heading's section extent. `foldHeading` is listed as not implemented (Development.md:932-933).

**Obsidian.** Spec 'Observed heading fold behavior' (Specification.md:709-800): in reading view each heading carries a `heading-collapse-indicator` (right-triangle, visible on hover, `is-collapsed` when folded); clicking it hides everything to the next heading of the same or higher level, the fold is shared with the editor and persisted per file, and `foldHeading` defaults to true.

**Evidence.** crates//src/reading/view.rs:458-475, docs//Specification.md:709-800, docs//Development.md:932-933

**Check it.** Open a note with several `##` headings in Reading mode and hover a heading: no triangle appears to the left; clicking the heading text does nothing. In Obsidian a triangle appears at the left margin and collapses the section.

**Fix.** In `build_with`, record for each top-level heading the range of top-level block indices it owns (to the next heading of level <= its own). Keep a `folded: HashSet<usize>` on `ReadingView`, render a hover-visible triangle in the left gutter of the heading (an absolutely positioned child at -28 px), and have `render_block` return an empty element for indices inside a folded range (or build a filtered index list so the `list` item count shrinks). Honour `foldHeading`.

### Code blocks lack the hover copy button and show a language label Obsidian does not

*medium, parity-gap, small*

**.** `RBlock::Code` draws the text in a scrolling container and, when an info string exists, an absolutely positioned language label in the top-right corner (view.rs:482-508). There is no copy affordance anywhere in the reading view (Development.md:1229-1230).

**Obsidian.** Spec 'Observed code-block copy behavior' (Specification.md:857-900): every reading-view `pre` has a `button.copy-code-button` (lucide-copy) at top/inset-end, shown only while the block is hovered; clicking copies the code's plain text with exactly one trailing newline removed and swaps the icon to a check for ~0.5-0.9 s. 'Reading view shows no language name anywhere on the block.' (The language flair belongs to the editor only.)

**Evidence.** crates//src/reading/view.rs:482-508, docs//Specification.md:857-900

**Check it.** Open a note with a ```js fence in Reading mode: the word `js` sits in the block's top-right corner at all times and hovering the block reveals no button. In Obsidian the corner is empty until hovered, then shows a copy icon.

**Fix.** Drop the language label in the reading view (keep it for the editor's fence line). Wrap the code block in an `id`'d `group` element and add an absolutely positioned child with `.visible_on_hover` (or a hovered flag) that on click writes `text` to the clipboard (`cx.write_to_clipboard`), swaps its glyph to a check and reverts after ~600 ms via a timer.

### Footnotes: reference is not superscript, click jumps to the end of the note rather than the footnote, and there is no back-reference

*medium, parity-gap, medium*

**.** The reference is pushed as literal `[n]` text at baseline with the `footnote` style, which only colours it (model.rs:624-628, view.rs:1354-1356). Clicking it calls `scroll_to_end` regardless of which footnote (workspace.rs:2231-2233). The footnote section lists `n.` and the body with no return link (view.rs:701-730).

**Obsidian.** Obsidian renders `<sup class="footnote-ref"><a class="footnote-link">[1]</a></sup>` inline and a `<section class="footnotes"><ol>` whose items end with a `footnote-backref` link (↩︎) back to the reference; clicking the ref scrolls to that specific footnote. 's own HTML renderer reproduces this markup for the corpus (html.rs:684, 739) and the corpus holds 18 `footnote-backref` links.

**Evidence.** crates//src/reading/model.rs:624-628, crates//src/reading/view.rs:701-730, crates//src/workspace.rs:2231-2233, crates/-markdown/src/html.rs:684, crates/-markdown/src/html.rs:739

**Check it.** Open a long note with `text[^1]` early and `[^1]: note` at the bottom plus several more footnotes. The `[1]` sits on the baseline in link colour; clicking it scrolls to the very end of the note (below the last footnote) instead of to footnote 1, and no ↩ link is shown to return.

**Fix.** Render the ref run smaller and raised (a separate `InteractiveText` in a `div` with smaller text size and negative top offset, since gpui has no vertical-align), store the footnote block index in `Model` so `LinkTarget::Footnote(n)` can scroll to the Footnotes block and flash row n, and append a clickable `↩` link per footnote that scrolls back to the block containing the first reference (record each ref's top-level block index while building).

### Unresolved internal links look identical to resolved ones; external links have no outgoing-link icon

*medium, parity-gap, small*

**.** Every wikilink becomes `LinkTarget::Internal` with `s.link = true` at build time without consulting the resolver (model.rs:552-567, 512-534); the `highlight` function applies one link style (accent colour plus underline) to internal, unresolved and external links alike (view.rs:1357-1364). Only the Outgoing links pane knows a link is unresolved (workspace.rs:2097).

**Obsidian.** Obsidian adds `is-unresolved` to an internal link whose target does not exist and the default theme draws it dimmed (`--link-unresolved-opacity`, with a dashed/lighter decoration); external links (`a.external-link`) get a small arrow icon after the text (well-known default-theme behaviour; help.obsidian.md 'Internal links' describes unresolved links as shown differently). Moderately sure of the exact decoration, sure of the distinction.

**Evidence.** crates//src/reading/model.rs:552-567, crates//src/reading/view.rs:1357-1364, crates//src/workspace.rs:2097-2100

**Check it.** In Reading mode view a note containing `[[Existing Note]]`, `[[Does Not Exist]]` and `[https://example.com](https://example.com)`: all three render in the same colour and underline, and the external one has no icon.

**Fix.** `EmbedSource` already offers `resolve`; in `Builder::inline` for wikilinks and vault markdown links call it and set a new `InlineStyle::unresolved` flag when it returns `None` (keep `LinkTarget::Internal` so clicking still creates the note). In `highlight`, give unresolved links reduced opacity (`t.link` with lower alpha) and external links an appended small `↗`/lucide-style glyph run.

### Tags render as coloured text instead of pills

*medium, parity-gap, medium*

**.** `InlineStyle::tag` only sets `h.color = t.tag` (view.rs:1351-1353); the tag has no background, padding or rounded corners and sits inline as plain text with a `#`.

**Obsidian.** Obsidian's reading view renders `a.tag` and the default theme draws it as a pill: `--tag-background` fill, `--tag-radius` (2em) corners, `--tag-padding-x/y`, smaller font and accent text (well-known; help.obsidian.md 'Tags'). The corpus shows `<a href="#tag" class="tag">` for every tag.

**Evidence.** crates//src/reading/view.rs:1351-1353, crates//src/reading/model.rs:569-579

**Check it.** Open a note with `#project` in its body in Reading mode: the word is simply blue-ish text. In Obsidian it sits in a rounded tinted capsule.

**Fix.** gpui `HighlightStyle` has `background_color` but no padding/radius; either render tags as separate inline-flex children (split the RichText around tag ranges into text / pill / text siblings in a `flex_wrap` row) or accept `background_color` with a tinted `t.tag` background as a first approximation.

### Callouts have no type icon; fold glyph and done-task styling differ from the default theme

*medium, parity-gap, medium*

**.** The callout title row is the title text plus, when foldable, a text `▸`/`▾` on the right (view.rs:550-569); there is no icon slot. Completed tasks are only dimmed (`text_color(t.text_muted)`, view.rs:762, 782) with no line-through; the fold chevron is text rather than a chevron icon rotated 90°.

**Obsidian.** Every Obsidian callout title starts with `div.callout-icon` holding a lucide SVG per type (info, lightbulb, check, flame, bug, list, quote, ...) and a `div.callout-fold` chevron at the end of the title when foldable — the corpus's expected HTML has `callout-icon` on every callout and 94/97 `callout-fold` / `callout-fold is-collapsed`. The default theme strikes through and dims checked tasks (`--checklist-done-decoration: line-through`), well-known.

**Evidence.** crates//src/reading/view.rs:525-581, crates//src/reading/view.rs:762-783, crates/-markdown/tests/corpus/expected/edge/Blocks.html:33-36

**Check it.** Open a note with `> [!tip]- Title` and `- [x] done` in Reading mode: the callout title has no lightbulb icon and a small text triangle on the right; the done task is grey but not struck through.

**Fix.** Add a clean-room icon set (Lucide is ISC-licensed; embed the handful of paths as `svg()` elements) keyed by canonical callout type and draw it before the title at 16 px in the callout colour; replace the text triangles with the same chevron glyph used for the properties fold. Add `strikethrough` to the done-task text style.

### Raw and inline HTML is shown as literal source text

*medium, parity-gap, large*

**.** `RBlock::Html` is drawn as monospace muted text (view.rs:509-515) and `InlineKind::Html` is pushed verbatim with the `html` style (mono, muted; model.rs:507, view.rs:1348-1350). So `a<br>b`, `<u>x</u>`, `<sup>2</sup>`, `<kbd>`, `<img src="pic.png">` all appear as their tags. Development.md:1171-1172 records 'HTML tags (shown as text)' as a known gap.

**Obsidian.** Spec 'Observed raw HTML behavior' (Specification.md:1014-1080): Obsidian renders sanitised HTML — `<img>` with vault `src` resolved by link resolution, `<a>` kept with target/rel, `<pre>` literal, `<b>/<u>/<sup>/<sub>/<kbd>/<details>` etc. kept and themed; only scripts and event handlers are stripped.

**Evidence.** crates//src/reading/view.rs:509-515, crates//src/reading/model.rs:507, crates//src/reading/view.rs:1348-1350, docs//Specification.md:1014-1080

**Check it.** Open a note containing `line one<br>line two` and `<img src="pic.png" width="100">` on its own line in Reading mode: the `<br>` and the `<img ...>` tag text are visible verbatim and no image is drawn.

**Fix.** Handle the common subset in the model: map `<br>` to a line break, `<b>/<strong>/<i>/<em>/<u>/<s>/<mark>/<sup>/<sub>/<kbd>/<code>` to styles (add `underline`, `sup`, `sub`, `kbd` to `InlineStyle`), and `<img>` (block or inline) to an image `Embed` resolved via `EmbedSource::resolve`. Leave unknown tags unwrapped (content shown, tag dropped) as Obsidian does for unknown elements.

### Remote images render as a link instead of being fetched

*medium, parity-gap, medium*

**.** `Builder::inline` turns `![alt](http://...)` and reference-style images into a link-styled run (`alt` or the URL) with an `External` target (model.rs:535-551) and the comment says 'shown as a link until remote images load'.

**Obsidian.** Spec 'Observed remote image behavior' (Specification.md:1081-1140): every `http(s):` image destination is requested with no prompt or gate and rendered as `<img>` with `referrerpolicy=no-referrer`; `|200x100` sizes apply as width/height attributes; a failed load stays a blank `<img>` with no placeholder.

**Evidence.** crates//src/reading/model.rs:535-551, docs//Specification.md:1081-1140

**Check it.** Open a note with `![logo|200](https://obsidian.md/images/obsidian-logo-gradient.svg)` in Reading mode: a blue underlined `logo` link is shown, no picture.

**Fix.** Add `EmbedKind::Remote(url)` and emit it from `inline` for scheme-bearing image destinations (with the `|WxH` size hint parsed from alt as `wikilink_embed` does); in the view use gpui's `img()` with a URL source (gpui supports remote image sources with its own cache) and the width/height hints; render nothing visible on failure, sending no Referer.

### Properties block is read-only and lacks the in-document 'Add property' row

*medium, parity-gap, large*

**.** The reading view passes `None` as the `PropertiesHost` (view.rs:222-225) so keys, values, the type glyph and `×` get no handlers (view.rs:1038-1053, 1066-1087, 1113-1131, 1202-1204), and the `+ Add property` row is only built when a host exists (`add = host.map(..)`, view.rs:898, 954-980). view.rs:876-880 and Development.md:1086-1087 acknowledge this divergence.

**Obsidian.** Spec 'Observed properties-in-reading-view behavior' (Specification.md:1296-1300, 1368-1440): the in-document block in the reading view ends with the `metadata-add-button` 'Add property'; clicking a value opens an editable field in place, Enter/Tab/blur commit and write exactly one line to the file, the key input is live (renames in place), pills can be removed, the icon opens the type menu.

**Evidence.** crates//src/reading/view.rs:222-225, crates//src/reading/view.rs:898, crates//src/reading/view.rs:954-980, crates//src/reading/view.rs:876-880, docs//Specification.md:1296-1300, docs//Specification.md:1368-1440

**Check it.** Open a note with frontmatter in Reading mode: there is no 'Add property' row under the last property, and clicking a value or key does nothing. Switch to Live Preview and both work.

**Fix.** Give `ReadingView` its own `PropertiesHost` whose `on_action` forwards to the workspace, which applies the same plan the editor uses (`_formats::properties::plan` + write) and rebuilds the model; reuse the editor's `Field` routing for the open field, with key events bound in the `ReadingView` context. Show the 'Add property' row whenever the block is drawn.

### Fold state (properties, callouts) resets on every reload or note switch; note-embed header is always visible and uses collision-prone element ids

*low, parity-gap, small*

**.** `set_note` calls `self.toggled.clear()` unconditionally (view.rs:146-158), so a folded Properties block or a reader-toggled callout springs back after a save-triggered reload or when returning to the note. Note embeds draw a permanent `↗ title` line above the content (view.rs:829-839) with element id `("embed-title", path.len() + title.len())` and attachments `("embed-file", link.len())` (view.rs:830, 854), which collide for two embeds whose strings have the same lengths, sharing hover/click state.

**Obsidian.** Spec (Specification.md:1456-1470): the properties fold is per file and outlives the view and a relaunch (stored under `<vault id>-note-fold-<path>`); heading folds likewise (Specification.md:770-775). An embedded note shows its content in `.markdown-embed` with an `.embed-title` that the default theme does not display and a hover-only `.markdown-embed-link` icon in the top-right corner that opens the note (corpus markup: `embed-title markdown-embed-title`, `markdown-embed-content`; well-known default-theme styling).

**Evidence.** crates//src/reading/view.rs:146-158, crates//src/reading/view.rs:829-839, crates//src/reading/view.rs:854, docs//Specification.md:1456-1470

**Check it.** Fold the Properties heading in Reading mode, open another note, come back: it is unfolded. Then open a note containing `![[A]]` and `![[B]]` (two notes with one-letter names): both embeds show a muted `↗ A` / `↗ B` caption permanently; hovering one may highlight the caption of the other.

**Fix.** Key `toggled` by path (a `HashMap<String, HashSet<usize>>`, or persist to a -side store since Obsidian keeps it in localStorage, never the vault) and only clear when the model's ids change meaning. Give embeds ids from the model's unique `next_id` counter (store an `id` in `Embed`), and replace the caption with a corner link icon shown on hover of the embed box.

## Right dock panes and graph

### Outline, Backlinks, Outgoing and search-result clicks only scroll the hidden reading view; in Live Preview/Source the editor does not move

*high, bug, medium*

> [!done] Fixed 2026-10-19
> Navigation now reveals the offset in the editor as well as the reading view
> (`Workspace::reveal_offset`, `Editor::reveal_offset`). Confirmed over the control
> socket: in Live Preview a `Long#Target heading` jump moves the editor from line 0 to
> scroll line 89 with the cursor on line 123, inside the visible range.

**.** Every pane click navigates by `self.reading.update(|r| r.scroll_to_offset(offset))`: outline rows at crates//src/workspace.rs:2869-2872, backlink/outgoing rows at 2915-2923, search results at 926-931, subpath links at 2201-2204. But when the mode is LivePreview or Source the main area renders `self.editor`, not `self.reading` (3339-3341), and `Editor` exposes no scroll/goto API (editor.rs public fns at 600-878, 1527, 1938-2075, 2617-2636; `scroll_to_line` at 1064 is private and only used by the scrollbar). Notes open in Live Preview by default, so an outline click does nothing visible, and a backlink click opens the source note at the top rather than at the link.

**Obsidian.** Clicking an Outline heading scrolls the active editor to that heading and flashes it; clicking a backlink/outgoing/search result opens the note scrolled to and highlighting the matched text, in whichever mode the note is in (help.obsidian.md Outline, Backlinks, Search).

**Evidence.** crates//src/workspace.rs:2869-2872, crates//src/workspace.rs:2915-2923, crates//src/workspace.rs:926-931, crates//src/workspace.rs:3339-3341, crates//src/editor.rs:1064

**Check it.** Open a long note with several headings (it opens in Live Preview). Open the Outline tab in the right dock and click the last heading: the editor does not scroll. Toggle to Reading view (Ctrl+E) and click again: it scrolls. Same with a Backlinks row: the source note opens at line 1.

**Fix.** Add `Editor::scroll_to_offset(offset, window, cx)` (convert byte offset to line via the buffer, call the existing `scroll_to_line`, place the cursor, and flash the line). In Workspace, add a `reveal_offset(offset)` helper that dispatches to the editor or reading view by `self.mode`, and use it from all four call sites.

### Links made from properties (frontmatter links) never appear in the Backlinks or Outgoing links panes

*medium, bug, small*

**.** The cache indexes `frontmatter_links` into the backlinks set (crates/-cache/src/vault.rs:110-121), but `MetadataCache::backlinks()` (vault.rs:215-233) only iterates `meta.links` and `meta.embeds` when materialising rows, so a note linked solely from another note's property yields zero `Backlink`s. The Outgoing pane likewise iterates only `meta.links.iter().chain(meta.embeds.iter())` (crates//src/workspace.rs:2099).

**Obsidian.** Since 1.4 links in properties are real links: they show in Backlinks (the context shows the property key) and in Outgoing links (help.obsidian.md Properties: 'Links in properties are counted as links'). Confident for Backlinks; fairly confident for Outgoing links.

**Evidence.** crates/-cache/src/vault.rs:110-121, crates/-cache/src/vault.rs:215-233, crates//src/workspace.rs:2099

**Check it.** Create B.md with frontmatter `related: "[[A]]"` and no body link. Open A.md, Backlinks tab: 'Linked mentions (0)'. Open B.md, Outgoing tab: 'Links (0)'.

**Fix.** In `backlinks()`, also walk `meta.frontmatter_links` and emit a `Backlink` variant carrying the property key instead of a `LinkCache` (or synthesise a position-less link); in `refresh_panes`, chain `meta.frontmatter_links` into the outgoing loop with the key as detail.

### Panes keep showing the deleted note's outline, backlinks and outgoing links

*medium, bug, small*

**.** `refresh_panes` returns immediately when `self.current` is `None` (crates//src/workspace.rs:2056-2059) and nothing else clears `outline`, `backlink_rows`, `outgoing_rows`, `linked` or `outgoing_links`. Deleting the open note sets `current = None` (2534) and calls `after_changes` -> `refresh_panes`, so the right dock still lists the dead note's headings and mentions while the header says 'No file is open'.

**Obsidian.** When no file is active the Backlinks/Outgoing/Outline panes are empty (Outline shows nothing; Backlinks shows no sections).

**Evidence.** crates//src/workspace.rs:2056-2059, crates//src/workspace.rs:2520-2536

**Check it.** Open a note with headings, switch to the Outline tab, run 'Delete current file' and confirm. Header reads 'No file is open' but the outline rows remain and are clickable.

**Fix.** In `refresh_panes`, on `current == None` clear all five row vectors (and bump `mention_generation`) before returning, then `cx.notify()`.

### Local graph draws links between neighbours (interlinks) and has no depth/backlinks/forelinks controls

*medium, parity-gap, medium*

**.** The local build keeps the centre and every node one hop away (crates//src/graph/view.rs:702-726; depth is hard-coded by `if depth >= 1 { continue }` at 711) and then `finish` keeps every edge whose two ends survived (745-775), so edges between two neighbours that bypass the centre are drawn. There is no Depth slider and no Backlinks/Forelinks/Interlinks toggles.

**Obsidian.** Spec 'Observed graph.json write behavior': the local graph's leaf options are `localJumps: 1`, `localBacklinks: true`, `localForelinks: true`, `localInterlinks: false` by default, so neighbour-to-neighbour edges are hidden unless Interlinks is ticked, and the Filters section offers Depth (1-5) plus those three toggles.

**Evidence.** crates//src/graph/view.rs:702-726, crates//src/graph/view.rs:745-775, docs//Specification.md:2176-2186

**Check it.** Make A->B, A->C, B->C. Open A, run 'Open local graph':  draws the B-C edge; Obsidian does not by default.

**Fix.** Carry `local_jumps`, `local_backlinks`, `local_forelinks`, `local_interlinks` on GraphView (defaults 1/true/true/false); BFS to `local_jumps` honouring direction flags; in `finish`, when local and !interlinks, keep only edges that touch a node on the BFS path from the centre (i.e. drop edges between two nodes at the same depth).

### graph.json is never created or written back (toggles, zoom scale)

*medium, parity-gap, medium*

**.** `toggle_orphans/attachments/tags` only set `self.config.*` in memory (crates//src/graph/view.rs:488-505); `zoom_at` only changes `self.scale` (368-379). `Vault::graph_config` is read-only use (crates/-vault/src/lib.rs:400-402) and no caller of `write_config("graph.json", ..)` exists outside tests. Opening the graph in a vault without graph.json does not create it. Development.md:1246-1248 lists this as 'not yet'.

**Obsidian.** Spec 'Observed graph.json write behavior': opening the global graph creates the full twenty-key file; each section-header click and the Close button write one key; wheel zoom writes `scale` (saturating at 8); the local graph writes nothing.

**Evidence.** crates//src/graph/view.rs:488-505, crates//src/graph/view.rs:368-379, crates/-vault/src/lib.rs:400-402, docs//Specification.md:2075-2140, docs//Development.md:1246-1248

**Check it.** Open a vault whose .obsidian has no graph.json, press Ctrl+G, click the Orphans chip, zoom, quit: no graph.json is created. With one present, toggle Tags and reopen : the toggle is forgotten.

**Fix.** Add `GraphView::save(cx)` that writes the full `GraphConfig` through `write_config("graph.json")` (the existing round-trip test shows it preserves key order/precision), debounced ~300 ms; call it from the toggles and from `zoom_at` with the wheel-target scale, and once on first global open when the file is absent. Skip saves while `local.is_some()`.

### Saved zoom (`scale`) is overridden and the view keeps re-fitting while the simulation runs

*medium, bug, small*

**.** `set_state` applies `graph.json`'s `scale` (crates//src/graph/view.rs:133-135) but `apply` resets `user_adjusted = false` (232) and the simulation loop calls `fit()` on every 16 ms tick until the layout settles (276-281), rewriting both `scale` and `offset` (238-254). The saved zoom is therefore never shown and the whole graph visibly zooms/recentres frame by frame for the first seconds of every rebuild, including every local-graph recenter on note change.

**Obsidian.** Spec: `scale` persists in graph.json and the renderer's `targetScale` is what the user set; Obsidian does not continuously auto-fit while nodes settle (less sure on the last point: it centres once on open, then nodes move under a fixed camera).

**Evidence.** crates//src/graph/view.rs:133-135, crates//src/graph/view.rs:232, crates//src/graph/view.rs:238-254, crates//src/graph/view.rs:276-281

**Check it.** Set `"scale": 4` in graph.json, open the graph: it appears fitted, not at 4x. Watch the first two seconds after Ctrl+G: the view scale changes every frame.

**Fix.** Fit once when the layout first has bounds (or after N steps), and only if no `scale` came from graph.json; otherwise centre with the configured scale and leave the camera alone while bodies move.

### No graph controls panel: no search filter box, 'Existing files only', arrows, text-fade/node-size/line sliders, force sliders, or colour-group editor

*medium, missing-feature, large*

**.** The only UI is four chips plus a status label (crates//src/graph/view.rs:1030-1073). `hide_unresolved`, `search`, `color_groups`, the three display multipliers and the forces are read from graph.json and applied (520-525, 544-575, 650-677, 318-332, 361-364) but cannot be changed in the app. `show_arrow` is parsed (crates/-formats/src/config.rs:358-359) and never referenced in view.rs, so arrows are never drawn.

**Obsidian.** Spec 'Controls panel': `div.graph-controls` with four collapsible sections Filters (search box, Tags, Attachments, Existing files only, Orphans), Groups, Display (Arrows, Text fade threshold, Node size, Link thickness), Forces (Center/Repel/Link force, Link distance) and Close / Open graph settings / timelapse / Restore default buttons; `showArrow: true` draws arrowheads on edges.

**Evidence.** crates//src/graph/view.rs:1030-1073, crates/-formats/src/config.rs:358-359, docs//Specification.md:2160-2172

**Check it.** Ctrl+G: top-left shows Global/Orphans/Attachments/Tags chips only. Set `showArrow: true` in graph.json and reopen: edges have no arrowheads.

**Fix.** Render a collapsible controls panel (top-left overlay) bound to `GraphConfig` with the four sections and their `collapse-*` keys; draw arrowheads in the edge path when `show_arrow`; wire each control to the save from the graph.json write-back fix.

### Backlinks pane is a flat list without per-file groups, highlighted context, toolbar, or 'Link' buttons on unlinked mentions

*medium, parity-gap, large*

**.** Linked mentions are one row per link occurrence, label = source file name repeated per hit, detail = `"{line}: {trimmed line}"` (crates//src/workspace.rs:2076-2094); headers are 10 px uppercase text in a 40 px row (2882-2893). Unlinked mentions are plain rows (3150-3160, 3178-3190) with no action. There is no search filter, sort, collapse-all or 'show more context' control, and nothing marks the matched text.

**Obsidian.** Backlinks pane: a header row with search-filter, collapse, show-more-context and sort buttons; 'Linked mentions' and 'Unlinked mentions' sections with count badges; results grouped per file as collapsible tree items (file name + per-file count), each match showing the surrounding text with the link highlighted; each unlinked mention has a 'Link' button that converts the text into a wikilink (help.obsidian.md Backlinks). The Unlinked mentions section starts collapsed (less sure).

**Evidence.** crates//src/workspace.rs:2076-2094, crates//src/workspace.rs:2882-2893, crates//src/workspace.rs:3150-3160, crates//src/workspace.rs:3178-3190

**Check it.** Open a note linked three times from the same file: Backlinks shows three rows each titled with the same file name and a leading line number, no group header, no highlight, no Link button under Unlinked mentions.

**Fix.** Replace `PaneRow` with a tree model: Section { title, count, collapsed } > FileGroup { path, count, collapsed } > Match { context, highlight range, offset }. Render with the explorer's tree-item style; add a `Link` button on unlinked rows that rewrites `range` to `[[Name]]` via the text crate's edit plan; add a filter input and sort toggle in a pane header.

### Unresolved rows in Outgoing links are inert; clicking should create the note

*low, parity-gap, small*

**.** Unresolved links get `path: None`, detail 'Not created yet' (crates//src/workspace.rs:2110-2118) and `pane_row` only attaches cursor/hover/click when `path.is_some()` (2904-2919), so the row is dead; there is also no link text shown (label is the parsed path only).

**Obsidian.** Outgoing links pane lists each link with the display text as written; unresolved links are rendered dimmed (`is-unresolved`) and clicking one creates the note like clicking an unresolved wikilink in the editor (fairly confident).

**Evidence.** crates//src/workspace.rs:2110-2118, crates//src/workspace.rs:2904-2919

**Check it.** Type `[[Does not exist]]` in a note, open the Outgoing tab, click the row: nothing happens.

**Fix.** Keep the link body on the row; on click call the existing `create_from_link(name)` path used by the quick switcher's Create event.

### Outline indents by absolute heading level, has no collapse chevrons and no tree nesting

*low, parity-gap, medium*

**.** Each row is padded `8 + (level-1)*12` px (crates//src/workspace.rs:2860) in a flat `uniform_list`; there are no collapse affordances and no active-heading state.

**Obsidian.** Outline is a tree: a heading nests under the nearest preceding shallower heading, so `# A` followed by `### B` shows B one level in, not three; every node with children has a collapse chevron and there is a collapse-all button (help.obsidian.md Outline). Fairly confident on depth-vs-level.

**Evidence.** crates//src/workspace.rs:2855-2876

**Check it.** Note with `# A`, `### B`, `## C`:  indents B by 24 px and C by 12 px; Obsidian shows both one level under A.

**Fix.** Build a tree from `headings` with a stack (push while level deeper), store depth and children on `OutlineRow`, keep a `collapsed: HashSet<usize>` and render chevrons like the file explorer.

### Tags pane is flat: no nested tag tree, no sort menu, no collapse

*low, parity-gap, medium*

**.** Rows come straight from the cache's BTreeMap in lowercase-name order (crates//src/workspace.rs:1422-1429, 1502-1509) and are rendered as `#name` + count with no hierarchy (2983-3025).

**Obsidian.** Tags view nests `#a/b` under `#a` with chevrons (Show nested tags, on by default), rolls child counts into the parent, and has a sort menu (name/count asc/desc) and collapse-all (help.obsidian.md Tags view). Clicking runs a `tag:#name` search, which  matches.

**Evidence.** crates//src/workspace.rs:1422-1429, crates//src/workspace.rs:2983-3025

**Check it.** Vault with `#project/a` and `#project/b`: Tags tab shows two flat rows `#project/a`, `#project/b` and no `#project` parent.

**Fix.** Split names on `/` into a trie, sum counts up the tree, render with depth indent and chevrons; add a sort button cycling the four orders.

### Right dock chrome: uppercase text tabs instead of icon tabs, fixed 280 px width, no pane title bar

*low, parity-gap, medium*

**.** Tabs are 11 px uppercase text buttons (crates//src/workspace.rs:2820-2840, 3085-3105); the dock is `w(px(280.))` with no resize handle (3087); panes have no header naming the view.

**Obsidian.** Default theme sidebar tab headers are icon-only buttons with tooltips (links-coming-in, links-going-out, list, tag), the dock is resizable by dragging its edge, and each pane has a title bar ('Backlinks for <note>', 'Outgoing links for <note>', 'Outline of <note>', 'Tags').

**Evidence.** crates//src/workspace.rs:2820-2840, crates//src/workspace.rs:3085-3105

**Check it.** Look at the right dock top strip: 'BACKLINKS OUTGOING OUTLINE TAGS' text; try dragging the dock's left edge: it does not resize.

**Fix.** Reuse the ribbon's icon drawing for the four tab icons with hover tooltips; add a drag handle on the dock border updating a `right_width` field; add a pane title row.

### Orphan filter leaves unlinked attachments visible; unresolved nodes drawn as hollow rings

*low, parity-gap, small*

**.** `finish` drops only `NodeKind::Note` orphans (crates//src/graph/view.rs:750), so with Attachments on and Orphans off every unlinked image still appears. Unresolved nodes are painted as transparent circles with a 1 px border (957-961).

**Obsidian.** With Orphans off, any node without links is hidden regardless of kind (fairly confident). Unresolved nodes are filled circles in the dimmer `--graph-node-unresolved` colour, same shape as notes (less sure; default theme).

**Evidence.** crates//src/graph/view.rs:750, crates//src/graph/view.rs:957-961

**Check it.** Vault with an unreferenced PNG: Ctrl+G, enable Attachments, disable Orphans: the PNG node stays. Add `[[Missing]]` to a note: its node is a ring, not a disc.

**Fix.** Drop the `node.kind == Note` clause in the orphan test; paint unresolved nodes filled with reduced alpha.

## Search, switcher, palette and autocompletion

### Search pane has none of Obsidian's header controls: match case, settings toggles, sort select, result-count menu

*high, missing-feature, large*

**.** The pane renders exactly three things: the input, an optional summary line, and the list (crates//src/search.rs:287-315). There is no Match case decorator, no Search settings button, no Collapse results / Show more context / Explain search terms toggles, no sort dropdown and no count menu with Copy search results / Bookmark. `Search::new` is always called without a matching-case flag (search.rs:122) and results are always sorted one way (search.rs:392-397). `Query::explain` exists in -search (query.rs:114) but nothing in the app surfaces it.

**Obsidian.** Spec 'Observed search pane behavior' (Specification.md:2236-2330): the header holds Match case (`uppercase-lowercase-a`, `is-active` when on), Search settings (`lucide-sliders-horizontal`) which reveals exactly three `mod-toggle` rows — Collapse results, Show more context, Explain search terms — and a result-count control whose left click opens a two-item menu, Copy search results and Bookmark...; a native `<select class="dropdown">` with six orders (File name A to Z / Z to A, Modified time new to old / old to new, Created time new to old / old to new) sits in the `search-results-info` bar beside the count. Each match line carries hover chevrons that expand context above and below.

**Evidence.** crates//src/search.rs:287-315, crates//src/search.rs:122, crates//src/search.rs:392-397, docs//Specification.md:2262-2300, docs//Specification.md:2340-2360

**Check it.** Open a vault, press Ctrl+Shift+F, type `needle`. Look right of the input for an Aa / sliders icon (none), look below the input for a sort dropdown or a '16 results' control that opens a menu on click (none), hover a match line for chevrons (none).

**Fix.** Add a header row to SearchPane: a Match case toggle that passes `matching_case` into the matcher (wrap the tree in `Query::CaseSensitive` or add a flag to `Search::new`), a settings button toggling a three-row section, a sort enum with the six stored strings (`alphabetical`, `alphabeticalReverse`, `byModifiedTime`, `byModifiedTimeReverse`, `byCreatedTime`, `byCreatedTimeReverse`) using VaultFile mtime/birthtime, and a count control opening a menu whose Copy item opens the copy modal (textarea + Show path toggle + Link style + List prefix selects, output per the spec table). Explain renders `Query::explain()` above the settings section.

### Clicking a search match in Live Preview or Source mode opens the note but does not jump to the match

*high, bug, medium*

**.** `SearchEvent::Open(path, offset)` is handled by calling `open_file` and then `self.reading.update(|r| r.scroll_to_offset(offset))` unconditionally (crates//src/workspace.rs:925-933). Only the reading view has a `scroll_to_offset` (crates//src/reading/view.rs:171); the editor has no equivalent and is never told the offset, so in the default Live Preview mode the note opens at its previous position. `scroll_to_subpath` has the same shape (workspace.rs:2200-2204).

**Obsidian.** Clicking a search result opens the file and reveals the match in the active editor (the match is selected/highlighted in the editor). Spec: file and match rows both call `focusedItem.onResultClick`, whose Ctrl/Shift variants open in a new tab/pane (Specification.md:2630-2640). The editor-reveal part is well-known Obsidian behaviour, not probed in the spec.

**Evidence.** crates//src/workspace.rs:925-933, crates//src/workspace.rs:2200-2204, crates//src/reading/view.rs:171

**Check it.** Open a long note in Live Preview, scroll to the top. Ctrl+Shift+F, search a word that only occurs near the end of that note, click the match row. The note opens (or stays) at the top; the cursor is not moved to the match. Switch to reading view and click again: it scrolls.

**Fix.** Give Editor a `reveal_offset(offset, len)` that sets the cursor/selection at the byte range and scrolls it into view; in the SearchEvent handler, branch on `self.mode.is_editing()` and call it (and do the same in `scroll_to_subpath`). Also carry the match's byte range, not just the line offset, in `MatchLine` so the selection can cover the hit.

### TextInput cannot paste, select, or select-all; Ctrl+A moves to line start

*high, bug, medium*

**.** The field binds only left/right/home/end, ctrl-a→Home, ctrl-e→End, backspace/delete/ctrl-backspace, ctrl-u, enter, escape, up, down (crates//src/text_input.rs:36-54). There is no Paste/Copy/Cut action, no shift+arrow selection, no double-click word selection, and the model holds a single `cursor` with no selection range (text_input.rs:72-80). So a query cannot be pasted into Search, the quick switcher, the palette or the template chooser, and Ctrl+A jumps to the start instead of selecting.

**Obsidian.** All four prompts use a native `<input>`: Ctrl+V pastes, Ctrl+A selects all, Shift+arrows/Home/End extend a selection, Ctrl+C/X copy and cut, double-click selects a word (standard browser input behaviour).

**Evidence.** crates//src/text_input.rs:18-54, crates//src/text_input.rs:72-80

**Check it.** Copy any word to the clipboard, press Ctrl+O, press Ctrl+V: nothing appears. Type `abc`, press Ctrl+A then type `x`: result is `xabc` (cursor went to start), not `x`.

**Fix.** Add a `selection: Range<usize>` (anchor + head) to TextInput; bind ctrl-v/ctrl-c/ctrl-x (read/write via `cx.read_from_clipboard`/`write_to_clipboard`), ctrl-a→SelectAll, shift-left/right/home/end, and paint the selection rect; typed text and Backspace replace the selection. Drop the emacs ctrl-a/ctrl-e bindings.

### Search results cannot be navigated or opened from the keyboard

*high, parity-gap, medium*

**.** SearchPane's subscription handles only `InputEvent::Changed` (crates//src/search.rs:73-77); the Up/Down/Submit events TextInput emits for the arrow keys and Enter (text_input.rs:197-211) are dropped. There is no focused-row state in the pane at all (search.rs:51-60), and ArrowLeft/Right have no collapse behaviour.

**Obsidian.** Spec keyboard table (Specification.md:2622-2640): ArrowDown calls `dom.changeFocusedItem("forwards")` with no input guard, ArrowLeft/Right collapse or expand the focused file row (or all rows when the list root is focused), and Enter calls `focusedItem.onResultClick(e)` on a file or match row. Rows carry `has-focus`.

**Evidence.** crates//src/search.rs:73-77, crates//src/search.rs:51-60, crates//src/text_input.rs:197-211, docs//Specification.md:2622-2640

**Check it.** Ctrl+Shift+F, type a term with results, press Down several times then Enter: no row is highlighted and nothing opens; the caret stays in the input.

**Fix.** Add `focused: Option<usize>` (row index) to SearchPane; handle InputEvent::Down/Up to move it (wrapping into the list from the input), draw the focused row with `bg_active`, handle Submit to emit Open for the focused row, and bind left/right in the pane's own key context to collapse/expand (all rows when nothing is focused).

### Quick switcher lacks Shift+Enter create, Ctrl+Enter open-in-new-tab, the create row and the instructions footer

*medium, parity-gap, medium*

**.** The switcher reacts only to Submit, Cancel, Up, Down (crates//src/switcher.rs:77-83); TextInput binds only plain `enter` (text_input.rs:49) so Shift+Enter and Ctrl+Enter do nothing. When nothing matches it shows the static text 'No matching files. Enter to create.' instead of a selectable row (switcher.rs:375-383), and when something matches there is no way to create a note with the typed name. The modal has no instructions footer (switcher.rs:349-384), unlike the palette (palette.rs:333-348).

**Obsidian.** Obsidian's quick switcher footer reads '↑↓ to navigate · ↵ to open · ctrl ↵ to open in new tab · shift ↵ to create · esc to dismiss' (well-known default UI; the exact set and modifiers for the new-tab/split variants are from memory of 1.x, not from the spec). With a query that matches nothing the typed text itself is shown as a row that Enter creates; Shift+Enter creates the note named by the query even when rows match.

**Evidence.** crates//src/switcher.rs:77-83, crates//src/switcher.rs:243-256, crates//src/switcher.rs:349-384, crates//src/text_input.rs:49, crates//src/palette.rs:333-348

**Check it.** Ctrl+O, type the name of an existing note plus a suffix so one row still matches by fuzzy, press Shift+Enter: nothing happens (Obsidian creates). Look at the bottom of the modal: no footer. Type gibberish: a grey sentence appears instead of a highlighted row.

**Fix.** Bind `shift-enter` and `ctrl-enter` in the TextInput context to new actions (SubmitSecondary/SubmitTertiary → InputEvent variants); in QuickSwitcher map Shift+Enter to `SwitcherEvent::Create(query)` regardless of selection, Ctrl+Enter to Open with an `in_new_tab` flag once tabs exist; render a synthetic 'create' row when no candidate matches; add the same footer component the palette uses with the switcher's instruction set.

### Search results sort by byte order of the lowercased name, not Obsidian's natural, locale-aware collation

*medium, bug, small*

**.** `run_search` sorts with `a.name.to_lowercase().cmp(&b.name.to_lowercase())` then path (crates//src/search.rs:392-397). Byte order puts `apple10` before `apple2` ('1' < '2'), `9 needle` before `_needle` ('9' 0x39 < '_' 0x5F), and `apple pie` after `apple2`? no: space 0x20 < '2' so that one happens to match.

**Obsidian.** Spec sort table and analysis (Specification.md:2340-2375): under `alphabetical` the order is `_needle`, `9 needle`, `notes/apple`, `apple`, `Apple Pie`, `apple2`, `apple10`, ...; 'Byte order is ruled out three times over — it puts apple10 before apple2, 9 needle before _needle, and Apple Pie first among the apples — and the data is consistent with a locale-aware numeric collation.'

**Evidence.** crates//src/search.rs:392-397, docs//Specification.md:2340-2375

**Check it.** Create notes `apple2.md`, `apple10.md`, `_needle.md`, `9 needle.md` all containing `needle`; Ctrl+Shift+F `needle`.  lists `9 needle` before `_needle` and `apple10` before `apple2`; Obsidian lists `_needle`, `9 needle`, ..., `apple2`, `apple10`.

**Fix.** Add a `natural_cmp` in -search (or -vault, shared with the file explorer's `alphabetical` sort): case-fold, compare digit runs numerically, and rank punctuation before digits before letters (ICU-like). Use it here and keep path as the tie-break; it also fixes the same order in the Files view if that uses byte order.

### Search leaf state (query, matchingCase, explainSearch, collapseAll, extraContext, sortOrder) is never persisted or restored

*medium, parity-gap, medium*

**.** SearchPane holds no sort/toggle state (crates//src/search.rs:51-60) and nothing in the app reads or writes `workspace.json`: the only reference is a reader `Vault::workspace_json` with a test (crates/-vault/src/lib.rs:449-450) that no app code calls. Recent searches (`<appId>-recent-searches`) are not kept either. Restarting  always opens an empty search.

**Obsidian.** Spec (Specification.md:2244-2262, 2385-2400): the search leaf's `state` in `.obsidian/workspace.json` holds exactly six keys, emitted even at their defaults, and after a restart 'the dropdown showed Modified time (new to old) and the two toggles came back enabled'; every query run ends with `requestSaveLayout()` and `requestSaveSearch()`.

**Evidence.** crates//src/search.rs:51-60, crates/-vault/src/lib.rs:449-450, docs//Specification.md:2244-2262, docs//Specification.md:2385-2400

**Check it.** Open the docs/ vault in Obsidian, search `gpui`, switch sort to Modified time, enable Show more context, quit. Open the same vault in : the search tab is empty and has no sort/toggle to show the restored values.

**Fix.** Model the six keys on SearchPane (`SearchLeafState` in -formats with `#[serde(flatten)] extra`), read them from the search leaf in workspace.json at startup once a layout reader exists, and write them on every run via the same debounced layout save. This is coupled to the workspace layout persistence work; at minimum keep the state on the pane so the header controls in finding 1 have something to bind to.

### Result count reads 'N results in M files' and ignores file-name hits; a file:-only match shows as a '0' chip

*medium, parity-gap, small*

**.** The summary is `format!("{total} results in {} files")` where total sums the content match lines (crates//src/search.rs:279-285). `Search::matches` returns an empty range list when only the name, path, tags or properties matched (crates/-search/src/matcher.rs:104-107), so a `file:needle` hit produces a file row with the chip '0' (search.rs:187) and adds nothing to the total; the name is not highlighted.

**Obsidian.** Spec (Specification.md:2318-2330): the count element text is `<span>7 results</span>` — matches, not files, with no file count; it read `16 results` over rows whose flairs summed to 14 because two files whose names contain the term carry flair 1 each and render the term inside `search-result-file-matched-text` in their titles. After emptying the query it reads `0 results`.

**Evidence.** crates//src/search.rs:279-285, crates//src/search.rs:187, crates/-search/src/matcher.rs:104-107

**Check it.** Create `needle.md` with no body text and `kiwi.md` containing `needle`; search `needle`.  shows '1 results in 2 files' and a '0' chip on `needle`; Obsidian shows '2 results', a '1' flair on `needle` and highlights 'needle' in its title.

**Fix.** Return name/path match ranges separately from `Search::matches` (e.g. a `MatchSet { content, name }`), count a name hit as one result and as flair 1, highlight it in the title with `search-result-file-matched-text` styling, and change the label to `{n} results` (`0 results` once a query has been emptied in-session).

### Prompt modals have no dimmed backdrop, and the switcher and template chooser have no instructions footer

*medium, parity-gap, small*

**.** For Switcher, Palette, Prompt and Chooser the overlay is a transparent absolute container with `pt(90)` and the child (crates//src/workspace.rs:3477-3496); only Settings gets the `rgba(0x00000080)` backdrop (workspace.rs:3497-3511). The modals are 620px wide (switcher.rs:356, palette.rs:305, chooser.rs:230). Only the palette draws a footer (palette.rs:333-348); the switcher (switcher.rs:349-384) and chooser (chooser.rs:223-258) end at the list.

**Obsidian.** Every Obsidian modal, the prompts included, sits over a `.modal-bg` that dims the app; the quick switcher and the template suggester (both `FuzzySuggestModal`s) show a `prompt-instructions` footer ('↑↓ to navigate', '↵ to use/open', 'esc to dismiss'); the default `--prompt-width` is 700px with a max of 80vw (default theme CSS, from memory — less sure of the exact pixel value).

**Evidence.** crates//src/workspace.rs:3477-3511, crates//src/switcher.rs:349-384, crates//src/chooser.rs:223-258, crates//src/palette.rs:333-348

**Check it.** Press Ctrl+O: the note behind stays at full brightness and the modal has no footer; press Ctrl+, : the Settings modal does dim the background. Run 'Templates: Insert template' from the palette: same, no footer.

**Fix.** Reuse the Settings backdrop div for all modals (with `on_click` → close and `stop_propagation` on the frame), extract `instructions_footer(&[(keys, text)])` from palette.rs into a shared helper and add it to QuickSwitcher ('↑↓ to navigate', '↵ to open', 'shift ↵ to create', 'esc to dismiss') and Chooser ('↑↓ to navigate', '↵ to use', 'esc to dismiss'); consider 700px width.

### Quick switcher ignores switcher.json (Show attachments, Show existing files only)

*medium, bug, small*

**.** `QuickSwitcher::set_state` filters only on `state.app().show_unsupported_files` (crates//src/switcher.rs:99-104) and always appends unresolved link targets (switcher.rs:127-145). `state.switcher()` is read nowhere in the app outside settings.rs and a test (workspace.rs:3606), even though the Settings tab exposes and writes `showAttachments` and `showExistingOnly` (crates//src/settings.rs:219-223).

**Obsidian.** Obsidian's Quick switcher settings (help.obsidian.md/Plugins/Quick+switcher): 'Show attachments' off hides images, PDFs and other attachments from the list; 'Show existing files only' on hides the 'not created yet' unresolved targets; 'Show all file types' corresponds to the unsupported-files flag.

**Evidence.** crates//src/switcher.rs:99-104, crates//src/switcher.rs:127-145, crates//src/settings.rs:219-223

**Check it.** Settings → Quick switcher → turn 'Show existing files only' on and 'Show attachments' off. Ctrl+O, type the name of an unresolved link target or of an image in the vault: both still appear.

**Fix.** In `set_state`, read `state.switcher()`: skip `FileKind::Image/Audio/Video/Pdf` (and canvases? no — those are notes-like) when `show_attachments == Some(false)`, skip the unresolved loop when `show_existing_only == Some(true)`, and use `show_all_file_types` for Other. Rebuild candidates when `SettingChange::Switcher` is applied.

### Command palette offers only the ~60 ids in RUNNABLE; most of Obsidian's 204 commands are absent

*medium, parity-gap, large*

**.** `open_palette` filters the registry to `RUNNABLE` (crates//src/workspace.rs:85-150, 1603-1606), a hand-kept list of about 60 ids; a search for e.g. 'bold', 'fold', 'split', 'reload', 'close' finds nothing. Development.md:1188-1193 records this as intended ('the palette shows only runnable commands'), but the set has not grown with the editor (e.g. undo/redo, find-next, the editor's selection commands are implemented as actions without a palette entry).

**Obsidian.** Spec: '204 commands are registered' (Specification.md:2474); the palette lists every registered command whose check callback passes, which for an open note is nearly all of them.

**Evidence.** crates//src/workspace.rs:85-150, crates//src/workspace.rs:1603-1606, docs//Development.md:1188-1193, docs//Specification.md:2474

**Check it.** Ctrl+P, type `bold`, `fold`, `split`, `close`, `reload`: each shows 'No commands found.'

**Fix.** Replace the static allowlist with a `can_run(id, &self)` check that mirrors Obsidian's check callbacks (editor commands need an editing note, etc.), and wire the editor actions that already exist (undo, redo, select all, find next/previous, toggle fold, toggle bold/italic once implemented) to their ids so they appear.

### Selection clamps at the ends instead of wrapping in the popup, switcher, palette and chooser

*low, parity-gap, small*

**.** `SuggestState::step` clamps to `0..=last` and its doc comment claims that is 'the way Obsidian's list moves' (crates//src/suggest.rs:34-43). The three modals do the same: Up is `saturating_sub(1)` and Down is `min(len-1)` (switcher.rs:81-82, 233-241; palette.rs:80-81, 168-176; chooser.rs:68-69, 147-155).

**Obsidian.** Obsidian's suggestion containers (editor suggest and the fuzzy modals) wrap: Down on the last row selects the first and Up on the first selects the last. This is from memory of Obsidian's `setSelectedItem` using a modulo wrap, not from the spec — mark as to-be-probed; the spec's autocompletion section lists the bound keys but not the end behaviour.

**Evidence.** crates//src/suggest.rs:34-43, crates//src/switcher.rs:233-241, crates//src/palette.rs:168-176, crates//src/chooser.rs:147-155

**Check it.** Ctrl+P with an empty query, press Up once: the first row stays selected (Obsidian jumps to the last). In a note type `[[a`, press Down past the last row: it sticks.

**Fix.** If a probe confirms wrapping, change `step` to `(selected + delta).rem_euclid(len)` and the three `select` calls to wrap; the comment in suggest.rs should cite the probe either way.

### Link-completion rows show the folder twice (full path as label plus folder as detail)

*low, bug, small*

**.** For a file row the label is `display_path(&file.path)` — the full vault path without `.md` (crates/-suggest/src/candidates.rs:96-104, 148-153) — and the detail is `folder_detail(&file.path)`, the folder again (candidates.rs:106-108, 136-139). `suggest.rs` draws label left and detail right (crates//src/suggest.rs:142-157), so `Sub/Alpha.md` renders as 'Sub/Alpha Sub'.

**Obsidian.** The link suggester row shows the path once (`Sub/Alpha` as the suggestion content); alias rows show the alias with a forward-arrow flair and the file name. (Standard UI; the spec records what is offered and written, not the row layout.)

**Evidence.** crates/-suggest/src/candidates.rs:96-108, crates/-suggest/src/candidates.rs:136-153, crates//src/suggest.rs:142-157

**Check it.** In a vault with `notes/apple.md`, type `[[app` in a note: the row reads 'notes/apple' with 'notes' repeated at the right edge.

**Fix.** Drop `folder_detail` for `Item::File` rows (keep it only where the label is a bare name, e.g. the quick switcher), or make the label the basename and keep the folder as detail — pick one and apply to both the suggester and the switcher for consistency.

### Search and switcher placeholders use a Unicode ellipsis where Obsidian uses three dots

*low, parity-gap, small*

**.** Search placeholder is "Search…" (crates//src/search.rs:72) and the switcher's is "Find or create a note…" (crates//src/switcher.rs:74), both with U+2026; the palette and chooser use ASCII "..." (palette.rs:73, workspace.rs:1329).

**Obsidian.** Obsidian's strings end in three ASCII periods: 'Search...', 'Find or create a note...', consistent with the observed 'Type name of a template...' and 'Select a command...' in the spec and the 'Rename...'/'Move file to...' menu items (Specification.md:2520-2535). The first two are from the default UI, not probed.

**Evidence.** crates//src/search.rs:72, crates//src/switcher.rs:74, crates//src/palette.rs:73

**Check it.** Open the Search tab and Ctrl+O with empty fields; compare the dots in the placeholders with Obsidian's (tighter, three separate periods).

**Fix.** Change the two literals to `...`.

### A query that does not parse shows an error line; Obsidian shows nothing

*low, parity-gap, small*

**.** `Search::new` errors (bad regex, etc.) are stored in `self.error` and drawn in the accent colour where the count normally goes (crates//src/search.rs:122-131, 280-304).

**Obsidian.** Spec: `startSearch` 'compiles the input with the matchingCase flag and returns early when that yields no matcher' (Specification.md:2280-2287) — an unparseable query simply yields an empty list with no message.

**Evidence.** crates//src/search.rs:122-131, crates//src/search.rs:280-304, docs//Specification.md:2280-2287

**Check it.** Ctrl+Shift+F, type `/(/`:  prints a regex error in the accent colour under the input; Obsidian shows an empty result area.

**Fix.** Treat a compile error as 'no matcher': clear results and show nothing (or keep the message only behind Explain search terms once that exists).

## Settings

### Hotkeys tab cannot change anything: no key capture, no ⊕, no ×, no restore-default, hotkeys.json never written

*high, missing-feature, large*

**.** render_hotkey_rows draws only the command name and read-only chips (settings.rs:1949-2010); the only interactive elements on the tab are the search box and the four filter chips (2012-2065). No code path builds a SettingChange for hotkeys (SettingChange enum, 523-540, has App/Appearance/Switcher/CorePlugins/CommunityPlugins only) and VaultState::apply_setting (workspace.rs:316-344) writes no hotkeys.json. -commands only reads overrides (lib.rs:161-178). Development.md:497 states it: " never writes that one". Conflicting bindings are not highlighted either.

**Obsidian.** Obsidian's Hotkeys tab (help.obsidian.md/Hotkeys, and the Interaction probes step 4 which lists per-row keys) lets you click the ⊕ beside a command, press a chord to assign it, click × on a chip to remove it, click the restore icon to go back to the default, and marks duplicate bindings in red; every change writes `.obsidian/hotkeys.json` as a map of command id to a list of {modifiers, key} (spec line 64: an entry replaces the defaults, an empty list unbinds).

**Evidence.** crates//src/settings.rs:1949-2010, crates//src/settings.rs:2012-2065, crates//src/settings.rs:523-540, crates/-commands/src/lib.rs:161-178, crates//src/workspace.rs:316-344, docs//Development.md:497

**Check it.** Ctrl+, → Hotkeys. Hover a row: no ⊕ or × appears; clicking the chip or the row does nothing; there is no way to press a key to rebind. Compare with Obsidian's row for e.g. 'Toggle bold'.

**Fix.** Add a `capturing: Option<String>` (command id) to SettingsView; render a ⊕ per row that enters capture mode and shows a 'Press hotkey...' chip; in capture mode intercept the next keystroke (gpui on_key_down on the focused settings div), convert the Keystroke to a _formats::config::Hotkey (modifiers Mod/Ctrl/Shift/Alt/Meta, key names per KeyboardEvent.key), and emit a new SettingChange::Hotkeys(Hotkeys) that `apply_setting` writes to hotkeys.json (IndexMap keeps order) and re-applies to the Commands global and the workspace's bound RunCommand actions. Render × on each chip (remove from the list; an empty list unbinds) and a restore icon when `custom` (remove the id from the map). Highlight chips whose label collides with another command's.

### Text fields write only on Enter; clicking away or switching tabs silently discards the typed value

*medium, bug, small*

**.** edit_field opens the shared TextInput (settings.rs:1719-1733); commit_field runs only on InputEvent::Submit (881-884, 1737-1752). There is no blur/focus-out handling, so clicking another nav tab, a toggle or the backdrop while the field is open never emits a change; `editing` stays Some (so the stale input reappears when you return to the tab) and the vault file is untouched. Escape (Cancel, 885-887) also discards.

**Obsidian.** Obsidian text settings (e.g. 'Folder to create new notes in', 'Attachment folder path') write on every change (`onChange`), with no Enter needed; the spec's write probe shows a typed string lands verbatim and a cleared field writes "" or the placeholder (Specification.md:528-566, 'A text field writes its string verbatim... an emptied field writes ""').

**Evidence.** crates//src/settings.rs:881-887, crates//src/settings.rs:1719-1752

**Check it.** Ctrl+, → Files and links, set 'Default location for new notes' to 'In the folder specified below', click 'Folder to create new notes in', type `Inbox`, then click the Appearance tab. `cat .obsidian/app.json`: newFileFolderPath is unchanged. Return to Files and links: the input is still open with `Inbox` in it.

**Fix.** Subscribe to InputEvent::Changed for `field` and emit the write on each change (debounced if the file writes are expensive), or at least commit in a focus-out handler (track `field.focus_handle(cx).is_focused(window)` in render and commit when it stops being focused) and when select_tab/close runs. Clear `editing` on close.

### After Enter or Escape in a text field, focus is left on an unrendered input and Escape no longer closes Settings

*medium, bug, small*

**.** edit_field moves window focus to `self.field` (settings.rs:1731). commit_field and the Cancel handler clear `editing` (1739, 885-887) so the TextInput is no longer rendered, but neither hands focus back to `self.focus_handle` (compare focus_tab, 932-938, which is only called from open/select_tab). The Close action is bound in the "Settings" key context on the div that tracks `self.focus_handle` (41-43, 2084-2087), so with focus on a handle that is not in the tree the Escape keystroke reaches no Settings handler. Also, while the field is open, Escape is consumed by TextInput's own `escape → Cancel` binding (text_input.rs:50, 201-203, no propagate), so Escape inside a field never closes the modal.

**Obsidian.** In Obsidian, Escape closes the Settings modal from anywhere inside it, including with the caret in a text setting (the modal's scope handles Escape; inputs do not swallow it). I am confident of the first part; the exact in-input behaviour is well-known but not in the spec.

**Evidence.** crates//src/settings.rs:1719-1752, crates//src/settings.rs:885-887, crates//src/settings.rs:932-938, crates//src/settings.rs:2084-2087, crates//src/text_input.rs:50, crates//src/text_input.rs:201-203

**Check it.** Ctrl+, → Files and links (with newFileLocation = folder), click 'Folder to create new notes in', press Enter. Now press Escape: the modal should close but stays open. Also: with the field open, Escape only closes the field, not the modal.

**Fix.** In commit_field and the Cancel branch call `window.focus(&self.focus_handle, cx)` (they need a &mut Window — thread it through or use cx.defer_in). For Escape-closes-everything, have the field's Cancel handler emit SettingsEvent::Close after discarding, as the hotkey search already does (880).

### Default location for new attachments cannot be changed; the folder/subfolder fields only appear when the file already holds that shape

*medium, parity-gap, medium*

**.** files_items derives (location, path, subfolder) from the current attachmentFolderPath string (settings.rs:1258-1264), shows the 'Default location for new attachments' row as a plain read-only `dropdown(...).not_yet()` with no `Typed` (1283-1289), pushes the 'Attachment folder path' field only when the path is non-empty (1290-1300) and 'Subfolder name' only as read-only `text` when the value starts with ./ (1301-1306). So a vault at the default `/` has no way to move attachments to a folder from Settings, and a vault at `./att` can never edit the subfolder.

**Obsidian.** Obsidian's dropdown has 'Vault folder' (/), 'Same folder as current file' (./), 'In subfolder under current folder' (./<name>, with a Subfolder name field) and 'In the folder specified below' (<folder>, with Attachment folder path) — the values are the ones the attachment paste probe exercised (Specification.md:1844-1853: "" or /, Attachments, ./att, Sub). Picking an option writes attachmentFolderPath at once and the dependent field appears.

**Evidence.** crates//src/settings.rs:1258-1306, docs//Specification.md:1844-1853

**Check it.** Open a vault whose app.json has no attachmentFolderPath (or "/"). Ctrl+, → Files and links: the 'Default location for new attachments' row shows 'Vault folder' with a ▾ that does nothing, and there is no field to type a folder into.

**Fix.** Make the row a `Typed` over attachmentFolderPath whose four options write "/", "./", "./Attachments"-style ("./" + current subfolder or a default) and the current/placeholder folder name; show the folder field for the 'folder' option and a writing Subfolder-name TextField (writes "./" + name) for the subfolder option, both regardless of the current value. Keep the .not_yet() tag until paste honours it.

### Base color scheme, fonts and font size take effect only on restart

*medium, parity-gap, medium*

**.** The Appearance rows are tagged `.on_restart()` (settings.rs:1017-1023, 1054-1075); change_setting writes the file and re-applies only app.json-driven editor options (workspace.rs:1139-1152, apply_app_config 1107-1134) and never rebuilds the `Theme` global, which is built from appearance.json once at startup (main.rs:240-241, 290).

**Obsidian.** Obsidian applies Base color scheme, Accent color, Interface/Text/Monospace font and Font size immediately as the control changes (standard, visible behaviour; the write side is in Specification.md:528-566).

**Evidence.** crates//src/settings.rs:1017-1075, crates//src/workspace.rs:1139-1152, crates//src/main.rs:240-241, crates//src/main.rs:290

**Check it.** Ctrl+, → Appearance, pick 'Light'/'Dark' in Base color scheme or click a Font size step: appearance.json changes, the window does not.

**Fix.** In Workspace::change_setting, when the change is SettingChange::Appearance, recompute `Theme::from_appearance(&cfg, window.appearance() is dark)` and `cx.set_global`, then notify every view (the editor caches font metrics so give it a `set_theme`/rebuild hook). Drop the on_restart tags once done.

### Daily notes, Templates, Backlinks and the other plugin tabs show 'Options not shown yet' although the configs are loaded

*medium, missing-feature, medium*

**.** plugin_items handles only "switcher" and "command-palette"; every other plugin tab renders a single placeholder row (settings.rs:1478-1483). Yet VaultState already holds TemplatesConfig and DailyNotesConfig (workspace.rs:183-185, 238-239, 301-306) and the commands use them (1191-1206); the types carry Obsidian's keys (config.rs:451-495: folder, dateFormat, timeFormat; format, folder, template).

**Obsidian.** Obsidian's Daily notes tab has 'Date format', 'New file location', 'Template file location' and 'Open daily note on startup'; Templates has 'Template folder location', 'Date format', 'Time format'; Backlinks has 'Backlink in document' (help.obsidian.md plugin pages). Each writes its plugin's json (daily-notes.json, templates.json, backlink.json).

**Evidence.** crates//src/settings.rs:1478-1483, crates//src/workspace.rs:183-185, crates//src/workspace.rs:238-239, crates/-formats/src/config.rs:451-495

**Check it.** Ctrl+, → under Core plugins click 'Daily notes' or 'Templates': one row 'Options not shown yet'.

**Fix.** Add TextField tables for templates.json (folder, dateFormat, timeFormat) and daily-notes.json (format, folder, template) with SettingChange::Templates/DailyNotes variants written through apply_setting; 'Open daily note on startup' as a flag (autorun). Reuse the existing Typed/TextField shape and tests.

### Most toggles are written for Obsidian but ignored by  (inline title, folding, line numbers, spellcheck, auto-pair, ribbon, tab title bar, core plugins…)

*medium, parity-gap, large*

**.** The tags are honest: only seven toggles are untagged (test only_the_toggles__reads_are_untagged, settings.rs:2414-2437) — readable line length, strict line breaks, smart lists, indent using tabs, automatically update links, show all file types, confirm before deleting. Everything else is `.not_yet()`: SHOW_INLINE_TITLE, FOLD_HEADING/INDENT, SHOW_LINE_NUMBER, SHOW_INDENT_GUIDE, RIGHT_TO_LEFT, SPELLCHECK, AUTO_PAIR_*, AUTO_CONVERT_HTML, VIM_MODE, FOCUS_NEW_TAB, USE_WIKILINKS (1109-1245, 1319-1325), SHOW_VIEW_HEADER/SHOW_RIBBON/NATIVE_MENUS (1081-1104), 'Show editing mode in status bar' (1131-1136) and every core plugin toggle (1364-1380). No code outside settings.rs reads auto_pair_*, show_inline_title, fold_heading, show_line_number, show_ribbon or show_view_header (grep over crates/, -text).

**Obsidian.** Each of these changes the running app at once: the inline title appears/disappears, line numbers show in the gutter, the ribbon hides, disabling the Files core plugin removes the Files view, etc. (Interaction probes step 8 records Fold heading off removing the fold affordance immediately).

**Evidence.** crates//src/settings.rs:2414-2437, crates//src/settings.rs:1081-1136, crates//src/settings.rs:1364-1380, docs//Interaction probes.md:296

**Check it.** Ctrl+, → Editor, toggle 'Inline title' or 'Line numbers': app.json changes, the editor does not. Core plugins → turn off 'Files': the Files sidebar stays.

**Fix.** Prioritise the cheap ones in apply_app_config: show_inline_title (editor/reading already render a title), show_ribbon and show_view_header (workspace layout flags), useMarkdownLinks already flows to link_options so USE_WIKILINKS can lose its tag; core-plugin toggles should hide the matching ribbon/sidebar views (file-explorer, global-search, graph, backlink, outgoing-link, outline, tag-pane, word-count).

### An open dropdown or text field survives closing the modal and is still open when Settings is reopened

*low, bug, small*

**.** `menu` is only cleared in pick (1770) and edit_field (1725); `editing` only in commit/cancel. open() sets the tab and focus (settings.rs:922-929) and close_action just emits Close (944-946); neither resets `menu` or `editing`, so Escape with a dropdown open leaves `menu` set and the next Ctrl+, shows the options list already expanded (render 1626-1662 draws it whenever `self.menu == Some(key)`).

**Obsidian.** A native <select> closes with the modal; reopening Settings shows every control collapsed.

**Evidence.** crates//src/settings.rs:922-929, crates//src/settings.rs:944-946, crates//src/settings.rs:1620-1662, crates//src/settings.rs:1770

**Check it.** Ctrl+, → Editor, click 'Default view for new tabs' so the options show, press Escape, press Ctrl+, again: the options popover is still open.

**Fix.** Reset `self.menu = None; self.editing = None;` in open() and in close_action/select_tab.

### Hotkeys tab lists commands of disabled core plugins

*low, parity-gap, small*

**.** load_commands keeps every registry command that is named or custom (settings.rs:903-913) and never consults core-plugins.json, so e.g. 'Daily notes: Open today's daily note' or 'Templates: Insert template' are listed with their default keys even when the plugin is off; the count label ('N commands', 2056-2059) includes them.

**Obsidian.** Obsidian's Hotkeys tab (and palette) only lists commands currently registered; a disabled core plugin's commands are unregistered and absent (standard plugin lifecycle; Development.md:1181-1191 notes the palette already hides unrunnable commands).

**Evidence.** crates//src/settings.rs:903-913, crates//src/settings.rs:2056-2059

**Check it.** Ctrl+, → Core plugins, turn off 'Daily notes'; go to Hotkeys and type 'daily': the Daily notes rows are still there.

**Fix.** Commands ids are prefixed by their plugin id (`daily-notes:`, `templates:`, `switcher:`…); filter out those whose prefix is a core plugin that is_enabled_or_default == false, and rebuild the list in set_state and after a CorePlugins change.

### Rows Obsidian has are missing: Zoom level, Quick font size adjustment, Window frame style, Translucent window, Keychain tab, Restricted mode

*low, missing-feature, medium*

**.** The Interface tab has only three toggles (settings.rs:1078-1105) and Appearance ends at Font size (1065-1075). AppearanceConfig already parses `translucency` (config.rs:262) but no row shows it. Development.md:1200-1202 lists what was knowingly left out: default file to open, delete attachments, Mermaid, quick font size, zoom, frame style, URI callbacks, the Keychain tab. The Community plugins tab has no Restricted mode row (1382-1420). The Keychain tab was observed in 1.14.4 (Specification.md:1209 lists General, Appearance, Hotkeys, Keychain, Core plugins, Community plugins).

**Obsidian.** Obsidian desktop shows 'Zoom level' (slider), 'Quick font size adjustment' (Ctrl+wheel toggle), 'Window frame style' (dropdown) and 'Translucent window' (toggle) among the appearance/interface rows, a 'Keychain' tab, and 'Restricted mode' at the top of Community plugins. Which of these keys live in appearance.json (translucency is observed in the type) versus local storage is not in the spec; mark as less sure for zoom and frame style.

**Evidence.** crates//src/settings.rs:1078-1105, crates//src/settings.rs:1382-1420, crates/-formats/src/config.rs:262, docs//Development.md:1200-1202, docs//Specification.md:1209

**Check it.** Ctrl+, → Interface: no Zoom level, frame style or translucent rows; nav has no Keychain entry; Community plugins has no Restricted mode row.

**Fix.** Add a 'Translucent window' AppearanceFlag over `translucency` (.not_yet()); probe appearance.json for the keys the other controls write before adding them (the memory note describes the throwaway-plugin dump); add a Keychain nav entry with Obsidian's rows read-only.

### Scroll position is shared between tabs: switching tabs keeps the previous tab's scroll offset

*low, bug, small*

**.** Every non-Hotkeys tab renders its items in the same scroll container `div().id("settings-items").overflow_y_scroll()` (settings.rs:2075-2080). gpui keeps scroll offsets per element id, so after scrolling the long Editor tab and clicking Appearance the new content starts scrolled down (clamped to its height). Not run; inferred from the shared id.

**Obsidian.** Each tab's content opens at the top (the tab content element is replaced on switch).

**Evidence.** crates//src/settings.rs:2075-2080

**Check it.** Ctrl+, → Editor, scroll to 'Vim key bindings' at the bottom, click 'Files and links': check whether the list starts at 'Always focus new tabs' or part-way down.

**Fix.** Give the container an id that includes the tab name (`format!("settings-items-{}", tab.name())`) or hold a ScrollHandle per tab and reset it in select_tab.

### Dropdown options popover is not dismissed by clicking elsewhere

*low, bug, small*

**.** The popover is toggled only by a click on the same control (settings.rs:1620-1625) or by picking an option (1770); no handler on the items list, the modal div or the backdrop clears `menu`, so clicking a toggle, a nav tab or empty space leaves the options floating (the deferred popover at 1636-1662 keeps rendering). Switching tabs hides it only because the row is gone, but `menu` stays set for when you return.

**Obsidian.** A native <select> closes on any click outside it.

**Evidence.** crates//src/settings.rs:1620-1662, crates//src/settings.rs:1770

**Check it.** Ctrl+, → Editor, open 'Properties in document', then click the 'Readable line length' toggle: the toggle flips and the options list stays open.

**Fix.** Add `on_mouse_down_out` (gpui) on the popover, or an on_click on the root settings div that sets `menu = None` with the control's click stopping propagation; also clear it in select_tab.

### Modal is a fixed 1100px wide and overflows narrower windows

*low, bug, small*

**.** SettingsView renders `w(px(1100.)).h_full()` (settings.rs:2088-2094); the frame around it is `h_full().max_w_full()` (workspace.rs:3513-3518) but that only caps the wrapper, not the fixed-width child, so on a window narrower than 1100 + 80px of padding the content pane and its controls run off the right edge of the backdrop. Not run; inferred from the fixed width.

**Obsidian.** Obsidian's settings modal is `--modal-width: 90vw` capped at `--modal-max-width: 1100px` and `--modal-height: 85vh` capped at 1000px (documented CSS variables for the modal component), so it shrinks with the window; moderately sure of the exact numbers.

**Evidence.** crates//src/settings.rs:2088-2094, crates//src/workspace.rs:3513-3518

**Check it.** Resize the window to ~900px wide and press Ctrl+,: the right edge of the settings pane (close ×, controls) should be clipped or off-screen.

**Fix.** Use `.w_full().max_w(px(1100.))` on the SettingsView root and `.w(relative(0.9))` on the frame; cap height with `max_h(px(1000.))`.

### -only chrome in the modal: explanatory banner, 'on restart'/'not in  yet' tags, tab-name title; inert 'Check for updates'/'Open'/'Browse' buttons

*low, parity-gap, small*

**.** Each tab shows an 18px tab-name header with × (settings.rs:2106-2131), then a grey banner paragraph about which controls write (2132-2141), and tagged rows carry a bordered chip next to the name (1845-1856). Control::Button renders a muted box with no handler and no pointer cursor (1877-1888): 'Check for updates' (984), 'Open' for Help (992), 'Browse' and 'Check for updates' on Community plugins (1385-1397) do nothing.

**Obsidian.** Obsidian's content pane has no banner and no per-row status chips; the × sits in the modal corner. Help → Open opens the help vault/site; Browse opens the community plugin browser. The tags are a deliberate  decision (Development.md:933-947) but they are still a visible difference.

**Evidence.** crates//src/settings.rs:2106-2141, crates//src/settings.rs:1845-1856, crates//src/settings.rs:1877-1888, crates//src/settings.rs:984-992, crates//src/settings.rs:1385-1397, docs//Development.md:933-947

**Check it.** Ctrl+, → General: the banner sits above 'Version…'; click 'Open' beside Help — nothing happens and the cursor does not change.

**Fix.** Wire Help/Open to cx.open_url("https://help.obsidian.md"); give buttons an id, cursor_pointer and hover; shrink the banner to a one-line footer or a hover tooltip on the tag, and let the tags disappear as settings get honoured.

### Toggle and slider do not look or behave like Obsidian's: 36×20 toggle, cell-per-step slider with no thumb or drag, uppercase nav group labels

*low, parity-gap, medium*

**.** Toggle: 36×20 track with a 14px thumb at left 2/17 (settings.rs:1567-1584). Slider: a row of `width = 120/steps` cells 4px tall that only respond to clicks, with the numeric value permanently shown beside it (1817-1875; Development.md:862-866 says dragging 'can come later'). Nav group titles are rendered `to_uppercase()` at 11px (1494-1502).

**Obsidian.** Obsidian's default toggle is 40px wide with an 18px thumb and 2px border (22px tall) per the documented toggle CSS variables; sliders are a draggable range input with a thumb, keyboard arrows, and the value shown in a tooltip while hovering/dragging rather than always; the nav group titles 'Options' / 'Core plugins' / 'Community plugins' are not uppercased. Moderately sure on the exact pixel values.

**Evidence.** crates//src/settings.rs:1567-1584, crates//src/settings.rs:1817-1875, crates//src/settings.rs:1494-1502, docs//Development.md:862-866

**Check it.** Ctrl+, → Appearance: try dragging the Font size control (nothing moves), note the permanent '16' label and the 'OPTIONS' nav heading.

**Fix.** Draw the slider as a track + thumb positioned from (value-min)/(max-min), handle on_drag/mouse move to set the nearest step, show the value in a small tooltip on hover; bump toggle to 40×22 with an 18px thumb; render group titles in sentence case.

## Source editor

### No auto-pairing of brackets, quotes or Markdown markers (and no selection wrapping)

*high, missing-feature, medium*

**.** Typed characters go straight through `replace_text_in_range` -> `insert_all` (editor.rs:3119-3146, buffer.rs:672-680) with no pairing logic anywhere; grep for autopair/auto_pair in editor.rs finds nothing, and Development.md:1348-1349 lists auto-pair as 'Not yet'. Typing `(` inserts `(`; typing `)` before an auto-inserted closer inserts a second `)`; Backspace between `()` deletes one byte; typing `*` with a word selected replaces the selection instead of wrapping it.

**Obsidian.** Spec 'Observed auto-pair behavior, Obsidian 1.14.4' (Specification.md:1637-1769): with `autoPairBrackets` on, `(` `[` `{` `"` `'` close unless a word character follows; with `autoPairMarkdown` on, `` ` `` `*` `_` close when preceded by a space/line start at end of line; typing a closer over an auto-inserted one skips; Backspace between an empty pair deletes both; any pair character typed over a selection wraps it (per range, including `~ = $ %`, except `<`); `[[` gives `[[]]`; one undo removes opener and closer together. Both toggles read true by default.

**Evidence.** crates//src/editor.rs:3119-3146, crates/-text/src/buffer.rs:672-680, docs//Development.md:1348-1349, docs//Specification.md:1637-1769

**Check it.** Open any note in Source mode, put the caret at the end of a line after a space and type `(`:  shows `(`, Obsidian shows `()` with the caret inside. Select a word and type `*`:  replaces the word with `*`, Obsidian wraps it as `*word*` and keeps the selection.

**Fix.** Add an `autopair` module in -text (pure functions over line text + cursor + the two settings) implementing the spec table: closer insertion with the word-char/space rules, skip-over when the next char is the matching closer, pair-backspace, per-selection wrapping. Call it from `replace_text_in_range` for single-character, non-IME inserts and from `backspace`; record opener+closer as one `EditKind::Typing` step. Read `autoPairBrackets`/`autoPairMarkdown` from app.json via a new `set_autopair_options`.

### Ctrl+B, Ctrl+I, Ctrl+/ and the other formatting toggles do nothing and are missing from the palette

*high, missing-feature, medium*

**.** `RUNNABLE` (workspace.rs:87-140) omits `editor:toggle-bold`, `editor:toggle-italics`, `editor:toggle-highlight`, `editor:toggle-code`, `editor:toggle-strikethrough`, `editor:toggle-comments`, `editor:clear-formatting`; `bind_keys` only binds hotkeys of RUNNABLE ids (workspace.rs:151-163), and the editor's own bindings (editor.rs:107-155) have no Ctrl+B/I//. Pressing Ctrl+B in the editor therefore does nothing, and the palette does not list these commands.

**Obsidian.** 's own observed registry binds `editor:toggle-bold` to Mod+B, `editor:toggle-italics` to Mod+I and `editor:toggle-comments` to Mod+/ (-commands/src/lib.rs:50-52), and commands.txt:116-127 lists the toggles. In Obsidian a toggle wraps the selection in `**`/`*`/`==`/`` ` ``/`~~`/`%%` (or inserts the pair with the caret inside when nothing is selected) and removes the markers when already applied (help.obsidian.md 'Basic formatting syntax' / default hotkeys).

**Evidence.** crates//src/workspace.rs:87-140, crates//src/workspace.rs:151-163, crates//src/editor.rs:107-155, crates/-commands/src/lib.rs:50-52, crates/-commands/data/commands.txt:116-127

**Check it.** Open a note in Source mode, select a word, press Ctrl+B: nothing changes (Obsidian gives `**word**`). Press Ctrl+P and type 'bold': no 'Toggle bold' row.

**Fix.** Add `TextBuffer::toggle_wrap(marker: &str)` in -text: per selection, if the range is already bracketed by the marker (inside or just outside) remove it, else insert the pair around it (empty selection: insert pair, caret inside), one undo step. Add the seven ids to RUNNABLE and dispatch in `run_command`; the registry hotkeys then bind automatically.

### Ctrl+K link insertion, Alt+Enter follow link and Ctrl+Enter open-in-new-tab are dead keys

*medium, missing-feature, small*

**.** `editor:insert-link`, `editor:insert-wikilink`, `editor:insert-embed`, `editor:follow-link`, `editor:open-link-in-new-leaf/split/window` are not in RUNNABLE (workspace.rs:87-140), so their observed hotkeys (lib.rs:49, 56-70: Mod+K, Alt+Enter, Mod+Enter, Mod+Alt+Enter) are never bound (workspace.rs:153-163) and the commands are absent from the palette. Links can only be followed by Ctrl+click (editor.rs:2539-2545).

**Obsidian.** Default hotkeys observed in Obsidian 1.14.4 (-commands/src/lib.rs:49-70): Mod+K inserts `[text](url)` around the selection (selection becomes the link text, caret in the URL), Alt+Enter follows the link under the caret, Mod+Enter opens it in a new tab. (The task brief's 'Ctrl+Enter toggles checkbox' is not Obsidian's default; Ctrl+Enter is open-link-in-new-leaf.)

**Evidence.** crates//src/workspace.rs:87-140, crates//src/workspace.rs:151-163, crates/-commands/src/lib.rs:49-70, crates//src/editor.rs:2539-2545

**Check it.** Source mode, select a word, press Ctrl+K: nothing happens (Obsidian: `[word]()` with the caret between the parentheses). Put the caret inside `[[Welcome]]` and press Alt+Enter: nothing opens.

**Fix.** Add `insert_link` (markdown and wikilink variants) to TextBuffer; add `follow_link`/`open_link_in_new_leaf` to Editor using the existing `link_at` on the caret's line and emitting `EditorEvent::OpenLink`. Add the ids to RUNNABLE.

### Ctrl+L (toggle checkbox) and 'Cycle bullet/checkbox' are not implemented

*medium, missing-feature, small*

**.** A task can only be toggled by clicking the rendered checkbox in Live Preview (editor.rs:2524-2537). `editor:toggle-checklist-status` and `editor:cycle-list-checklist` are not in RUNNABLE (workspace.rs:87-140), so Mod+L (lib.rs:54) is unbound and Source mode has no way to toggle a task from the keyboard.

**Obsidian.** Spec 'Observed task checkbox behavior' (Specification.md:699-705) names both commands with their observed labels ('Toggle checkbox status', 'Cycle bullet/checkbox'); the registry binds `editor:toggle-checklist-status` to Mod+l (lib.rs:53-54). Toggling writes the literal `[x]`/`[ ]` on the caret's line (and on every line of a multi-line selection); cycle turns a plain line into `- `, then `- [ ] `, then back.

**Evidence.** crates//src/editor.rs:2524-2537, crates//src/workspace.rs:87-140, crates/-commands/src/lib.rs:53-54, docs//Specification.md:699-705

**Check it.** Source mode, caret on `- [ ] task`, press Ctrl+L: line unchanged (Obsidian: `- [x] task`). Ctrl+P 'checkbox': no rows.

**Fix.** Reuse `lists::parse_prefix` (task range at lists.rs:145-157) in a `TextBuffer::toggle_checklist()` over `selected_lines()`: lines with a task get `[x]`<->`[ ]`, list lines without one get `[ ] ` inserted; one undo step. `cycle_list_checklist` rotates plain -> bullet -> task -> plain. Add both ids to RUNNABLE.

### Ctrl+D (delete paragraph) and Move line up/down are missing

*medium, missing-feature, small*

**.** `editor:delete-paragraph`, `editor:swap-line-up` and `editor:swap-line-down` are absent from RUNNABLE (workspace.rs:87-140) and from the editor's bindings (editor.rs:107-155); Ctrl+D and Alt+Up/Down do nothing in the editor.

**Obsidian.** Registry observed in Obsidian 1.14.4 binds `editor:delete-paragraph` to Mod+D (lib.rs:55) — it deletes the paragraph (blank-line-delimited block) around the caret. commands.txt:98-99 lists `editor:swap-line-up/down` ('Move line up/down'); they swap the caret's line (or every selected line) with its neighbour. Less sure: Alt+Up/Down reach them by default through CodeMirror's default keymap in Obsidian; they show no hotkey in Settings.

**Evidence.** crates//src/workspace.rs:87-140, crates//src/editor.rs:107-155, crates/-commands/src/lib.rs:55, crates/-commands/data/commands.txt:98-99

**Check it.** Source mode, caret in a paragraph, press Ctrl+D: nothing (Obsidian deletes the paragraph). Ctrl+P 'move line': no rows.

**Fix.** `TextBuffer::delete_paragraph()` removes from the previous blank line to the next one; `swap_lines(up: bool)` moves `selected_lines()` one line, keeping the selection on the moved text (edit_many with shifted cursor). Add the ids to RUNNABLE; bind Alt+Up/Down in the Editor context if confirmed.

### Heading and list folding does not exist in the editor

*medium, missing-feature, large*

**.** The only folding is of Live Preview widgets and the properties block (`folded_widget`, editor.rs:919-952; `toggle_properties_fold`, editor.rs:2637-2640). There is no fold state for headings or list parents, no fold indicator in the gutter, and `editor:toggle-fold`, `fold-all`, `unfold-all`, `fold-more`, `fold-less` are not in RUNNABLE (workspace.rs:87-140).

**Obsidian.** Spec 'Observed heading fold behavior' (Specification.md:709-855): with `foldHeading`/`foldIndent` (both default true) every heading and list parent carries a hover-revealed collapse indicator left of the line; `editor:toggle-fold` folds the heading through the line before the next heading of the same or higher level (trailing blanks included, last heading to end of file), list folds cover the children only; the folded line shows `…`; state is kept in localStorage per note (nothing written to the vault) and shared with reading view; fold-all/unfold-all exist.

**Evidence.** crates//src/editor.rs:919-952, crates//src/editor.rs:2637-2640, crates//src/workspace.rs:87-140, docs//Specification.md:709-855

**Check it.** Open a note with `##` headings in Live Preview, hover a heading: no triangle appears in the left margin; Ctrl+P 'fold': only 'Toggle fold properties'.

**Fix.** Add a `folds: Vec<Range<usize>>` (line ranges) to Editor, computed per the spec's extent rule from the highlight spans; make `height_of`/prepaint skip folded lines (same mechanism as folded widgets) and append `…` to the folded line's display; paint a hover-only triangle at content_x - 15px; implement the five commands and persist per note in 's own local store, not the vault.

### Find/Replace fields cannot paste, and Ctrl+A moves to the start instead of selecting all

*medium, bug, small*

**.** `TextInput` binds `ctrl-a` to Home and `ctrl-e` to End (text_input.rs:43-44) and has no selection, no Paste/Copy/Cut actions (text_input.rs:18-54). The editor's Ctrl+V binding lives in the `Editor` key context (editor.rs:149, 3218), but the find panel is rendered as a sibling of that div (editor.rs:3269-3274) under `FindPanel`/`TextInput`, so Ctrl+V with the query focused does nothing; no other file binds a paste action (grep `Paste` outside editor.rs is empty). Shift+arrows do not select either.

**Obsidian.** Obsidian's search and replace fields are native `<input>` elements: Ctrl+A selects the whole query so typing replaces it, Ctrl+V pastes, Shift+arrows and double-click select. (Standard browser input behaviour; high confidence.)

**Evidence.** crates//src/text_input.rs:36-54, crates//src/text_input.rs:43-44, crates//src/editor.rs:149, crates//src/editor.rs:3269-3274

**Check it.** Copy a word from the note, press Ctrl+F, press Ctrl+V: the query stays empty. Type `abc`, press Ctrl+A then type `x`: the field reads `xabc`, not `x`.

**Fix.** Give TextInput an anchor/selection (Shift+Left/Right/Home/End, Ctrl+A = select all, double-click word), and bind ctrl-c/ctrl-x/ctrl-v in the `TextInput` context using `cx.read_from_clipboard()`; keep Ctrl+U clear. The same input is used by the quick switcher and search, so they benefit too.

### Shift+Tab and Enter-on-empty-nested-item outdent by up to 4 spaces regardless of tabSize

*medium, bug, small*

**.** `lists::outdent_width` (lists.rs:236-242) removes one tab or 'up to four spaces' with no knowledge of `tabSize`; `TextBuffer::outdent` (buffer.rs:945-957) and `continuation`'s empty-item outdent (lists.rs:199-208) both use it. With `useTab: false, tabSize: 2`, Tab indents by two spaces (lists.rs:245-251) but Shift+Tab removes four, so a twice-indented ` - b` becomes `- b` in one press and an item nested one level cannot be outdented symmetrically.

**Obsidian.** Obsidian's Unindent (Shift+Tab) and the empty-item Enter remove one indentation level, where the level is a tab or `tabSize` spaces as set under Settings > Editor (help.obsidian.md 'Editing and formatting settings'; CodeMirror's `indentLess` uses the configured indent unit). Fairly sure.

**Evidence.** crates/-text/src/lists.rs:236-242, crates/-text/src/lists.rs:199-208, crates/-text/src/lists.rs:245-251, crates/-text/src/buffer.rs:945-957

**Check it.** Set `useTab: false, tabSize: 2` in app.json. In Source mode type `- a`, Enter, Tab, Tab (gives ` - `), type `b`, then Shift+Tab:  yields `- b` (two levels removed); Obsidian yields ` - b`.

**Fix.** Thread `use_tab`/`tab_size` into `outdent_width`/`outdent_range`/`continuation` (take `min(tab_size, leading spaces)` when the first char is a space), and pass them from `TextBuffer::outdent` and `newline`.

### Home on a wrapped row places the caret at the end of the previous row

*medium, bug, small*

**.** `row_boundary` (editor.rs:1497-1523) computes Home as `index_at(0, row)`, which for a non-first wrapped row is the wrap-boundary byte (gpui `_index_for_position` returns `wrapped_line_start_index`, line_layout.rs:308-347). But `row_and_x` (editor.rs:340-346) maps that same byte through `position_for_index`, which assigns an index equal to a row's end to the *earlier* row (line_layout.rs:362-389: `index > line_end_ix` is the only skip). So the caret is painted at the end of the previous visual row, and a click at the very left of a wrapped row shows the same jump; a following Up/Down starts from the wrong row.

**Obsidian.** CodeMirror 6 (Obsidian) binds Home to `cursorLineBoundaryBackward`, which is wrap-aware and leaves the caret at the start of the current visual row (it tracks the side with `assoc`), so Home on the second row of a wrapped paragraph never moves the caret up a row. High confidence.

**Evidence.** crates//src/editor.rs:1497-1523, crates//src/editor.rs:340-346, crates//src/editor.rs:3897-3905

**Check it.** Open a note with a long paragraph that wraps (readable line length on), click in the middle of its second visual row, press Home: the caret jumps to the end of the first row instead of the start of the second. Clicking at the far left of the second row shows the same.

**Fix.** Carry a row-side bit (`assoc`) with the primary selection, or special-case in `row_and_x`: when `col` equals a wrap boundary and the caret arrived via Home/click/Down on that row, return `(row, 0)`; compute the boundary set from `line.wrap_boundaries()` once per shaped line.

### The editor cannot scroll past the end of the note

*medium, parity-gap, small*

**.** `clamp_scroll_end` (editor.rs:1033-1050) is applied on every downward wheel/drag/scrollbar move (editor.rs:1052-1069), so the last line is pinned to the bottom of the view and the final lines of a note can only ever be edited at the bottom edge of the pane.

**Obsidian.** Obsidian's editor lets you scroll until the last line sits near the top of the pane (CodeMirror's scroll-past-end padding), so the end of a note can be edited mid-screen. Fairly sure, from use; not recorded in the spec.

**Evidence.** crates//src/editor.rs:1033-1050, crates//src/editor.rs:1052-1069

**Check it.** Open a note longer than the window, scroll the wheel down until it stops: the last line sits flush with the bottom edge and will not go higher; in Obsidian it keeps scrolling until roughly a viewport of empty space shows below it.

**Fix.** In `clamp_scroll_end` allow `filled` to be as small as one row height (or the view height minus one line height, matching CodeMirror), and in `reveal_cursor` keep the cursor row at the bottom as today.

### Heading, list, blockquote, insert and undo/redo commands are missing from the palette

*medium, missing-feature, medium*

**.** RUNNABLE (workspace.rs:87-140) has none of `editor:set-heading`, `set-heading-0..6`, `toggle-bullet-list`, `toggle-numbered-list`, `toggle-blockquote`, `indent-list`, `unindent-list`, `insert-callout`, `insert-codeblock`, `insert-horizontal-rule`, `insert-footnote`, `insert-mathblock`, `insert-tag`, `undo`, `redo`, `cut`, `copy`, `paste`, `go-start`, `go-end`, so they neither appear in Ctrl+P nor honour a user's hotkeys.json binding for them.

**Obsidian.** All of these ids are in Obsidian's registry (-commands/data/commands.txt:50-127) and show in the command palette with the labels in names.txt; a user can bind any of them in hotkeys.json. Semantics: set-heading-N rewrites the line's leading `#`s, toggle-bullet-list adds/removes `- `, toggle-blockquote adds/removes `> `, insert-codeblock inserts a fenced block with the caret inside, etc.

**Evidence.** crates//src/workspace.rs:87-140, crates/-commands/data/commands.txt:50-127

**Check it.** Ctrl+P, type 'heading': no 'Set as heading' rows. Type 'undo': no row.

**Fix.** Implement line-prefix rewrites in -text (`set_heading(n)`, `toggle_line_prefix("- ")`, `toggle_line_prefix("> ")`, `insert_block(text, caret_offset)`) over `selected_lines()`, wire the ids in `run_command`, and map undo/redo/cut/copy/paste/go-start/go-end to the existing actions.

### The caret never blinks

*low, parity-gap, small*

**.** The cursor is painted as a solid 2 px quad every frame (editor.rs:3897-3905, 4038-4040) with no blink timer; it only disappears when the editor loses focus (editor.rs:3954-3956).

**Obsidian.** Obsidian's editor caret blinks (CodeMirror's `drawSelection` default, roughly a 1.2 s cycle, restarting on each keystroke). Fairly sure.

**Evidence.** crates//src/editor.rs:3897-3905, crates//src/editor.rs:3954-3956, crates//src/editor.rs:4038-4040

**Check it.** Click into a note and wait: the caret stays solid.

**Fix.** Keep a `blink_on: bool` and `last_input: Instant` on Editor; after each edit/move reset to visible and spawn a 600 ms timer that flips `blink_on` and calls `cx.notify()`; skip painting the caret quads when off. Cancel the timer when unfocused.

### No right-click context menu in the editor

*low, missing-feature, medium*

**.** Only `MouseButton::Left` is handled (editor.rs:4049-4056, 4066-4075); a right click selects nothing and opens nothing, and `editor:context-menu` is not in RUNNABLE (workspace.rs:87-140).

**Obsidian.** Right-clicking in the editor opens the editor context menu: Cut, Copy, Paste, Paste as plain text, Select all, plus Format/Paragraph/Insert submenus, and link items when over a link (commands.txt:49 `editor:context-menu`; help.obsidian.md 'Editor'). High confidence.

**Evidence.** crates//src/editor.rs:4049-4056, crates//src/editor.rs:4066-4075, crates/-commands/data/commands.txt:49

**Check it.** Right-click anywhere in an open note: nothing appears.

**Fix.** Handle `MouseButton::Right` in paint's mouse-down hook, place the caret if outside the selection, and emit an `EditorEvent::ContextMenu { position }` that the workspace renders with the existing menu widget, dispatching the command ids.

### Find panel is a floating box at the top right rather than Obsidian's full-width search bar

*low, parity-gap, small*

**.** `render_find_panel` makes an `absolute()` box at `top(8) right(24)` with its own border and background, overlaying the text (editor.rs:3384-3399); the query field is a fixed 220 px (editor.rs:3323).

**Obsidian.** Obsidian's document search (`.document-search-container`) is a bar across the full width of the editor pane at its top, below the tab header, with the input stretching to fill and the buttons on the right; the note content is pushed down rather than covered. Fairly sure.

**Evidence.** crates//src/editor.rs:3384-3399, crates//src/editor.rs:3323

**Check it.** Press Ctrl+F: compare the small box at the top right with Obsidian's bar spanning the pane.

**Fix.** Render the panel as the first child of a flex column above the EditorElement (not absolute), full width, with the query field `flex_1()`, and subtract its height from the editor bounds.

### A line copied with no selection pastes mid-line instead of as a whole line above

*low, parity-gap, small*

**.** Ctrl+C with an empty selection copies the whole line including its newline (buffer.rs:961-979); Ctrl+V always inserts the clipboard at the caret (buffer.rs:991-1009), so pasting mid-line splits that line and inserts the copied line between the halves.

**Obsidian.** CodeMirror 6 remembers that the last copy was line-wise and, when the selection is empty, pastes the text as a complete line above the caret's line (`doPaste` linewise branch), leaving the current line intact. Fairly sure.

**Evidence.** crates/-text/src/buffer.rs:961-979, crates/-text/src/buffer.rs:991-1009, crates//src/editor.rs:1906-1931

**Check it.** Source mode, caret on line `alpha` with nothing selected, Ctrl+C; move the caret into the middle of `beta` on the next line, Ctrl+V:  gives `be` / `alpha` / `ta`; Obsidian gives `alpha` inserted above `beta` intact.

**Fix.** Record `last_linewise_copy: Option<String>` on Editor in `copy`/`cut`; in `paste`, if every cursor is empty and the clipboard equals it, insert at each cursor's line start instead of at the caret.

## Workspace shell

### Outline, backlink, search-result and [[Note#Heading]] navigation only scrolls the reading view, so in the default Live Preview mode clicking them does nothing visible

*high, bug, medium*

> [!done] Fixed 2026-10-19
> Navigation now reveals the offset in the editor as well as the reading view
> (`Workspace::reveal_offset`, `Editor::reveal_offset`). Confirmed over the control
> socket: in Live Preview a `Long#Target heading` jump moves the editor from line 0 to
> scroll line 89 with the cursor on line 123, inside the visible range.

**.** Every "go to position" path targets `self.reading` only: outline rows (workspace.rs:2868-2871), backlink/outgoing rows (workspace.rs:2904-2912), search results (workspace.rs:967-973), and `scroll_to_subpath` for `#Heading`/`#^block` links (workspace.rs:2201-2205). The editor has no scroll-to-offset entry point at all (`grep 'pub fn scroll_to' crates//src/editor.rs` finds nothing). `default_mode` (workspace.rs:1902-1915) returns Live Preview unless `defaultViewMode` is `preview`, so the note is normally shown in the editor while the hidden reading view is the one being scrolled.

**Obsidian.** Clicking an Outline heading, a Backlinks match, a Search result or following `[[Note#Heading]]` scrolls the active leaf to that position in whichever mode it is in (in editing mode it also places the cursor there). Documented behaviour of the Outline, Backlinks and Search core plugins (help.obsidian.md).

**Evidence.** crates//src/workspace.rs:2868-2871, crates//src/workspace.rs:2904-2912, crates//src/workspace.rs:967-973, crates//src/workspace.rs:2201-2205, crates//src/workspace.rs:1902-1915

**Check it.** Open a vault with a long note in Live Preview (the default). Open the Outline tab on the right and click a heading far down the note: the editor does not move. Switch to Reading (Ctrl+E) and click the same heading: it scrolls. Same with a Search hit or a backlink row.

**Fix.** Add `Editor::scroll_to_offset(offset, cx)` (set the primary cursor to `offset` and reveal it), then route the four call sites through a `Workspace::reveal_offset` that dispatches on `self.mode.is_editing()`.

### No tab bar: Ctrl+T, Ctrl+W, Ctrl+Tab/Ctrl+Shift+Tab and Ctrl+1..9 do nothing and only one note can be open

*high, missing-feature, large*

**.** The workspace holds a single `current: Option<String>` (workspace.rs:843) and one editor/reading view. `RUNNABLE` (workspace.rs:86-149) contains no `workspace:new-tab`, `workspace:close`, `workspace:next-tab`, `workspace:previous-tab` or `workspace:goto-tab-N`, so `bind_keys` (workspace.rs:151-166) never binds their default hotkeys (registry defaults at -commands/src/lib.rs:72-83); pressing them is silently swallowed or, via the palette, reported as "not available in  yet" (workspace.rs:1792-1803). The view header (workspace.rs:3248-3351) has no tab strip.

**Obsidian.** Each pane has a tab header row above the view header; Ctrl+T opens a new empty tab, Ctrl+W closes the active tab (closing the last one leaves the "No file is open" tab), Ctrl+Tab / Ctrl+Shift+Tab cycle tabs, Ctrl+1..8 jump to a tab and Ctrl+9 to the last. Documented at help.obsidian.md (Tabs) and in the registry's own default hotkeys.

**Evidence.** crates//src/workspace.rs:843, crates//src/workspace.rs:86-149, crates//src/workspace.rs:151-166, crates/-commands/src/lib.rs:72-83, crates//src/workspace.rs:3248-3351

**Check it.** Open any note, press Ctrl+T: nothing. Press Ctrl+W: nothing (note stays open). Open the command palette and run "Close current tab": the status bar says it is not available.

**Fix.** Introduce a `Tab { path, mode, history, scroll }` list on `Workspace` with an active index; render a tab strip (title, close x, + button) above the view header; move `current`/`history`/`mode` into the tab; implement `workspace:new-tab`, `close`, `next-tab`, `previous-tab`, `goto-tab-N`, `goto-last-tab`, `undo-close-pane` and add them to RUNNABLE.

### Startup opens the first note alphabetically instead of restoring the last workspace, and nothing is ever restored from workspace.json

*high, parity-gap, medium*

**.** main.rs:357-363 calls `ws.open_first(cx)` when no `--note` is given; `open_first` (workspace.rs:1945-1954) takes `markdown_files().next()`. `Vault::workspace_json` exists (-vault/src/lib.rs:449-451) but no caller in `crates//src` uses it; the writer gate note at -text/tests/writer_roundtrip.rs:967 says  neither models nor writes it. Sidebar visibility (`left_visible`/`right_visible`, workspace.rs:1010-1011) and the active dock tabs always start at Files / Backlinks.

**Obsidian.** On launch Obsidian restores `.obsidian/workspace.json`: the open tabs and their active file, mode, sidebar collapse state, sidebar widths and which sidebar tab was active. With nothing recorded it shows the "No file is open" empty state rather than picking a note. (Documented: help.obsidian.md "Workspace" and the probe notes in Interaction probes.md that quit with Ctrl+Q so workspace.json is flushed.)

**Evidence.** crates//src/main.rs:357-363, crates//src/workspace.rs:1945-1954, crates/-vault/src/lib.rs:449-451, crates/-text/tests/writer_roundtrip.rs:967, crates//src/workspace.rs:1010-1011

**Check it.** Open a vault whose workspace.json last had `Zulu.md` open and the left sidebar collapsed;  opens `Alpha.md` with both sidebars shown. Open a brand-new vault with no notes open:  still opens the first note rather than the empty state.

**Fix.** Read `workspace.json` at `set_state`: walk `main.children[].children[]` for `type == "markdown"` leaves, open the one under `active`, honour `left.collapsed`/`right.collapsed` and the active sidebar leaf; when there is no file to restore, leave `current` as None. Writing it back is a separate decision under the vault-is-truth rule.

### No ribbon, and once a sidebar is collapsed there is no on-screen control to bring it back

*high, missing-feature, medium*

**.** `render` composes only `left`, `main` and the right dock (workspace.rs:3526-3545); there is no ribbon column (`grep -i ribbon crates//src` matches only the Settings toggle, settings.rs:214). The sidebars are removed with `.when(left_visible, ...)` (workspace.rs:3539-3543) and the only ways to toggle them are `app:toggle-left-sidebar`/`app:toggle-right-sidebar` (workspace.rs:1726-1733), which have no default hotkey in the registry (-commands/src/lib.rs:55-93), so a user who runs the command from the palette must reopen the palette to undo it. Settings is reached from a text button pinned to the bottom of the left dock (workspace.rs:3390-3410). Development.md:932-934 lists the ribbon as not implemented.

**Obsidian.** Default theme: a 44 px ribbon on the far left with icon buttons (Open quick switcher, Open graph view, Create new canvas, Open today's daily note, Insert template, Open command palette) and, at the bottom, Open another vault / Help / Settings. Each sidebar also has a collapse toggle in its header, and the toggles remain visible when the sidebar is collapsed. (help.obsidian.md "Ribbon" and "Sidebar".)

**Evidence.** crates//src/workspace.rs:3526-3545, crates//src/workspace.rs:3539-3543, crates//src/workspace.rs:1726-1733, crates/-commands/src/lib.rs:55-93, crates//src/workspace.rs:3390-3410, docs//Development.md:932-934

**Check it.** Ctrl+P, run "Toggle left sidebar": the Files pane disappears and no icon, edge handle or button remains to restore it. Look at the far left of the window: no ribbon.

**Fix.** Add a `render_ribbon` column (icon buttons bound to `switcher:open`, `graph:open`, `daily-notes`, `insert-template`, `command-palette:open`; `app:open-settings` at the bottom) honouring `appearance.showRibbon`; add a collapse chevron to each dock header and a matching expand chevron that stays visible in the ribbon / title row when a dock is hidden.

### Notices are written into the status bar and never expire; there is no toast

*medium, parity-gap, small*

**.** `report` (workspace.rs:2343-2347) assigns `self.status`, which the status bar renders verbatim on its left (workspace.rs:3435-3448). Everything user-facing goes there: "not available in  yet" (1792-1803), "The cursor is not in a table" (1718-1721), "X saved" / "Could not save" (2310-2318), "Deleted X" (2540), rename summaries (2784), "changed on disk; unsaved edits kept" (2008). Nothing clears the message; it stays until the next status write (often the next reading-view build line at 2022-2023).

**Obsidian.** `Notice` toasts appear stacked at the top-right of the window, fade after about 5 seconds, and dismiss on click; the status bar is not used for messages. (help.obsidian.md and the probe scripts in Interaction probes.md refer to Notices as toasts.)

**Evidence.** crates//src/workspace.rs:2343-2347, crates//src/workspace.rs:3435-3448, crates//src/workspace.rs:1792-1803, crates//src/workspace.rs:2310-2318

**Check it.** Ctrl+P, run "Reload app without saving": the message "Reload app without saving: not available in  yet" sits in the bottom-left status bar indefinitely. Delete a note: "Deleted X" stays until something else overwrites it.

**Fix.** Add a `notices: Vec<(SharedString, Instant)>` to Workspace, render them as an absolutely positioned top-right stack in `render`, spawn a 5 s timer per notice to drop it, and make `report` push a notice instead of writing `status`; keep `status` for persistent state only.

### Status bar shows cursor position and internal timings, and omits the backlink count

*medium, parity-gap, small*

**.** In editing mode the right side of the status bar is `Ln {}, Col {} · parse {:.1} ms, {} spans · shape {:.2} ms · N words, N characters` (workspace.rs:3416-3434, 3449-3451); the left side shows whatever `status` last held, e.g. "path · reading view built in 1.2 ms" (workspace.rs:2022-2023) or "N files, N notes indexed in N ms" (1155-1161). The number of backlinks computed for the Backlinks pane (`self.linked.len()`, workspace.rs:2089-2091) is never shown in the status bar.

**Obsidian.** Default status bar is right-aligned only and shows, for the active note, the Backlinks plugin item ("N backlinks") and the Word count item ("N words, N characters"); there is no cursor position and no diagnostics. (help.obsidian.md: Backlinks, Word count core plugins.) Moderately confident on the exact backlink item wording.

**Evidence.** crates//src/workspace.rs:3416-3434, crates//src/workspace.rs:3449-3451, crates//src/workspace.rs:2022-2023, crates//src/workspace.rs:2089-2091

**Check it.** Open any note in Live Preview and read the bottom bar: "Ln 1, Col 1 · parse 3.4 ms, 212 spans · shape 0.31 ms · 480 words, 2910 characters". No backlink count anywhere in the bar.

**Fix.** Gate the timing/cursor string behind a `--debug` flag or env var; render `{n} backlinks` (from `self.linked.len() - 1`, excluding the header row) left of the word count; hide the word-count item when `current` is None.

### Inline title is not rendered and the header title cannot be clicked to rename

*medium, missing-feature, medium*

**.** `showInlineTitle` is parsed (-formats/src/config.rs:57-60) and toggled in Settings (settings.rs:188, 1146-1149) but no view reads it (`grep -rn inline_title crates//src` hits settings.rs only); Development.md:932-934 and 1157 list the inline title as not built. The note name appears only in the view header breadcrumb div (workspace.rs:3291-3308), which has no click handler; renaming is reachable only through F2 / the Files context menu (workspace.rs:1769-1773).

**Obsidian.** With `showInlineTitle` on (the default) the note's title is shown as a large editable heading above the body in both editing and reading view; editing it renames the file. The view-header title is also click-to-rename. (help.obsidian.md: Appearance, "Show inline title".)

**Evidence.** crates/-formats/src/config.rs:57-60, crates//src/settings.rs:188, crates//src/workspace.rs:3291-3308, docs//Development.md:932-934, docs//Development.md:1157

**Check it.** Open any note: the body starts directly with the first line; no large title above it. Click the note name in the view header: nothing happens.

**Fix.** Render an inline-title block as the first item of the editor's and reading view's list when `show_inline_title != Some(false)`, backed by a `TextInput` that commits through `rename_current`; make the header title div open the same rename on click.

### The open note being deleted externally keeps a stale buffer that a later edit writes back, recreating the file

*medium, bug, small*

**.** `after_changes` (workspace.rs:1521-1526) handles `FileChange::Removed(current)` by only setting the status "X was deleted" and refreshing the panes: `current`, the editor buffer and the history entry stay. On the next keystroke `schedule_save` fires and `flush_save` (workspace.rs:2296-2320) does `if let Ok(on_disk) = read_to_string(..)` so the stale-disk check is skipped for a missing file and `std::fs::write` recreates it. Going back in history to a removed path (`navigate_history`, 2263-2274) calls `show_file`, which sets `current` to the missing path and reports "Could not read" (2024-2026). Only 's own `delete_now` clears the state (2517-2535).

**Obsidian.** When the file behind an open tab is deleted outside the app, Obsidian drops the view (the tab turns into the empty state) and does not resurrect the file from the editor buffer. Confident it never recreates the file; less sure whether the tab closes or empties.

**Evidence.** crates//src/workspace.rs:1521-1526, crates//src/workspace.rs:2296-2320, crates//src/workspace.rs:2263-2274, crates//src/workspace.rs:2517-2535

**Check it.** Open `Note.md` in Live Preview, `rm Note.md` from a terminal; the status says it was deleted but the text stays. Type one character and wait 2 s: `Note.md` is back on disk.

**Fix.** In `after_changes`, when `current` is removed, reuse the clearing code from `delete_now` (reset editor/reading, `current = None`, cancel `save_task`) and prune the path from `history` (adjusting `history_ix`); in `flush_save`, treat a missing file as "changed on disk" rather than writing.

### Changing the base color scheme, accent or fonts in Settings takes effect only after restart

*medium, parity-gap, small*

**.** `Theme` is set as a global exactly twice, both in main.rs before any UI interaction (main.rs:238-241, 276-294). `change_setting` (workspace.rs:1139-1151) writes `appearance.json` and calls `apply_app_config` + `refresh_vault_views`, neither of which rebuilds the Theme; Development.md:936-937 documents "on restart" for theme and fonts. `theme:toggle-light-dark` and `theme:switch` are not in RUNNABLE (workspace.rs:86-149).

**Obsidian.** Settings → Appearance → Base color scheme switches the whole window immediately; so do accent colour and the font settings. `theme:toggle-light-dark` is a palette command (names in -commands/data/commands.txt:177-178).

**Evidence.** crates//src/main.rs:238-241, crates//src/main.rs:276-294, crates//src/workspace.rs:1139-1151, docs//Development.md:936-937, crates/-commands/data/commands.txt:177-178

**Check it.** Ctrl+, → Appearance → set Base color scheme to Light on a dark vault: the file is written but the window stays dark until relaunch.

**Fix.** In `change_setting`, on `SettingChange::Appearance(cfg)` call `cx.set_global(Theme::from_appearance(&cfg, window_is_dark))` and `cx.notify()`; add `theme:toggle-light-dark` to RUNNABLE, flipping `theme` between `obsidian` and `moonstone` through the same path.

### Empty state after closing/deleting the open note is a blank reading view with live Graph/mode buttons and "0 words"

*medium, parity-gap, small*

**.** When `current` is None the header title becomes "No file is open" (workspace.rs:3217-3222) but the main body still renders `self.reading` with an empty model (3353-3365; `delete_now` feeds it `build_with("", "")`, 2521-2528), the Graph and mode-toggle buttons stay (3310-3351), the status bar shows "0 words, 0 characters" (3449-3451), and the mode label toggles into an empty editor.

**Obsidian.** The empty tab shows a centred "No file is open" title with action links: Create new note (Ctrl+N), Go to file (Ctrl+O), See recent files, Close (Ctrl+W); the view header has no mode switch and the word count item is hidden. (Documented at help.obsidian.md "Tabs"; the wording is standard default-theme UI.)

**Evidence.** crates//src/workspace.rs:3217-3222, crates//src/workspace.rs:3353-3365, crates//src/workspace.rs:2521-2528, crates//src/workspace.rs:3310-3351, crates//src/workspace.rs:3449-3451

**Check it.** Open a note, delete it from the Files context menu and confirm: header says "No file is open", body is blank white/dark, bottom-right shows "0 words, 0 characters", and clicking the mode label still toggles Reading/Live Preview.

**Fix.** In `render`, when `self.current.is_none() && mode != Graph`, replace `main_body` with an `empty_state` element (title + three/four action rows wired to `file-explorer:new-file`, `switcher:open`, `workspace:close`), omit the header's mode/graph buttons and the word-count item.

### Left sidebar lacks the Bookmarks tab and the right sidebar lacks the Properties pane

*medium, missing-feature, large*

**.** `LeftTab` has only Files and Search (workspace.rs:744-747); `RightTab` has Backlinks, Outgoing, Outline, Tags (750-755). `bookmarks:open` and the `file-properties` view are absent from RUNNABLE (86-149), so the palette hides "Bookmarks: Show bookmarks".

**Obsidian.** A new vault's left sidebar has Files, Search and Bookmarks tabs; the right sidebar has Backlinks, Outgoing links, Tags, Outline and (since 1.4, with the Properties core plugin on) a Properties tab for the current file. Confident on Bookmarks (core plugin on by default since 1.2); the Properties tab being in the default right dock I am less sure of, but the pane exists and is in the audit's target layout.

**Evidence.** crates//src/workspace.rs:744-747, crates//src/workspace.rs:750-755, crates//src/workspace.rs:86-149

**Check it.** Look at the top of the left sidebar: only FILES and SEARCH. Right sidebar: no Properties tab; Ctrl+P "Bookmarks" shows nothing runnable.

**Fix.** Add `LeftTab::Bookmarks` reading `.obsidian/bookmarks.json` into a tree (groups, files, headings) and `RightTab::Properties` reusing the editor's properties table widget on the current file; add `bookmarks:open` and the file-properties command ids to RUNNABLE.

### History back/forward restores the file but not the view mode or scroll position

*low, parity-gap, small*

**.** `history` is a `Vec<String>` of paths (workspace.rs:844-845); `navigate_history` (2263-2274) calls `show_file`, which rebuilds the reading view and loads the editor at the top (`set_text`, 2016-2018) and leaves `self.mode` as it was. Opening a note in Reading, switching to Live Preview, following a link and pressing Ctrl+Alt+Left returns to the note in Live Preview at line 1.

**Obsidian.** Each history entry stores the leaf's view state (file, mode and editor/scroll state), so Navigate back returns to the mode and scroll position the note was left in. (help.obsidian.md "Navigate back"; moderately confident about scroll restoration, confident about mode.)

**Evidence.** crates//src/workspace.rs:844-845, crates//src/workspace.rs:2263-2274, crates//src/workspace.rs:2016-2018

**Check it.** Open note A in Reading, scroll to the bottom, click a link to B, press Ctrl+Alt+Left: A reopens at the top (and if you had toggled modes in B, in B's mode).

**Fix.** Make `history` a `Vec<HistoryEntry { path, mode, scroll/cursor }>`; record the current entry's mode and position in `open_file` before pushing, and have `navigate_history` call `set_mode` and the reveal helper after `show_file`.

### Dock tabs are uppercase text labels with the vault name in the tab row, not icon tabs; "Outgoing" is mislabelled

*low, parity-gap, small*

**.** `dock_tab` (workspace.rs:2820-2840) renders `label.to_uppercase()` at 11 px with a rounded background for the active tab; the left tab row also holds the vault name right-aligned (3365-3375); the right tab labels are "Backlinks", "Outgoing", "Outline", "Tags" (3092-3095).

**Obsidian.** Sidebar tab headers are icon-only buttons (folder, search, bookmark; links, link-out, tag, list for the right side) with a tooltip giving the view name ("Outgoing links", not "Outgoing"), a collapse chevron at the end of the row, and the vault name lives in the ribbon's vault switcher at the bottom left, not in the sidebar header. (Default theme, help.obsidian.md "Sidebar".)

**Evidence.** crates//src/workspace.rs:2820-2840, crates//src/workspace.rs:3365-3375, crates//src/workspace.rs:3092-3095

**Check it.** Look at the top of either sidebar: text like FILES SEARCH <vault name> and BACKLINKS OUTGOING OUTLINE TAGS instead of a row of icons.

**Fix.** Give `dock_tab` an icon glyph (own SVG/unicode, clean room) plus a hover tooltip with the full view name; move the vault name to the ribbon; rename the label to "Outgoing links".

### View header has a text mode label and a "Graph" button instead of Obsidian's icon mode switch and More-options menu

*low, parity-gap, medium*

**.** The header's right side is a "Graph" text button (workspace.rs:3310-3330) and a text label of the current mode ("Reading", "Live Preview", "Source": 3238-3243, 3334-3350) that toggles reading/editing on click. There is no more-options ("...") menu and no way from the header to open in Source vs Live Preview, rename, delete, copy path, or open in default app.

**Obsidian.** The view header's actions are an icon button whose tooltip reads "Current view: editing / Click to read" (book-open vs pencil, no mode text) and a "More options" (three dots) menu with Open in default app, Reveal in Files, Rename, Delete, Copy path, toggle Source/Live Preview, etc. Graph view is opened from the ribbon or Ctrl+G, never from a note header. (Default theme; help.obsidian.md "Edit and preview Markdown".)

**Evidence.** crates//src/workspace.rs:3310-3330, crates//src/workspace.rs:3238-3243, crates//src/workspace.rs:3334-3350

**Check it.** Open a note: top-right of the editor pane shows the words "Graph" and "Live Preview". No "..." button.

**Fix.** Replace the mode label with an icon button + tooltip, move "Graph" to the ribbon, and add a "More options" popover listing the file commands already implemented (`workspace:edit-file-title`, `app:delete-file`, `file-explorer:reveal-active-file`, `file-explorer:move-file`, `editor:toggle-source`).

### Window title is a static "" and dark-theme muted text is darker than Obsidian's

*low, parity-gap, small*

**.** `TitlebarOptions { title: Some("") }` (main.rs:270-275) is never updated on `open_file`. theme.rs:41-42 uses `text_muted` #999999 and `text_faint` #666666 for dark, and `text_faint` #9a9a9a for light (theme.rs:62).

**Obsidian.** Obsidian's window title is "<Note name> - <Vault name> - Obsidian v1.14.4" and changes with the active file. Default dark palette (less sure, from memory of app.css): --text-muted is base-70 #bababa and --text-faint base-50 #666; light --text-faint is base-50 #ababab.

**Evidence.** crates//src/main.rs:270-275, crates//src/theme.rs:41-42, crates//src/theme.rs:62

**Check it.** Open two different notes: the window/taskbar title stays "". Compare the grey of secondary text (status bar, breadcrumb) with Obsidian dark side by side.

**Fix.** Call `window.set_window_title(&format!("{title} - {vault} - "))` in `show_file`/`delete_now`; adjust the two faint/muted constants after sampling Obsidian's rendered colours.

