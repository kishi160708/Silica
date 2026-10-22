---
tags:
  - /dev
---
# UI parity backlog

Where 's window still differs from Obsidian 1.14, as of 2026-10-20. Every entry below was
produced by driving the *running* app over the control socket
([[Development#Control socket and MCP server, 2026-10-19]]) and reading `state` back after every
step, and each claim then went to an adversarial reviewer whose main job was the Obsidian half —
the pass before this one had refuted four leads and every one of them had failed there.

It happened twice. The first pass filed 82 findings over seven subsystems; its run hit a session
limit, and when it resumed the probe agents re-ran rather than replaying their cache, so what came
back was an independent second audit — 92 findings, 13 of them Live Preview. The fix campaign only
ever saw the first pass plus the second pass's Live Preview file, 95 findings in all, and fixed 54
of them over seven waves with a fmt/check/clippy/test gate between each. The second pass's other 79
findings were triaged against the *finished* binary afterwards, and one more wave then fixed a
crash and a corruption bug that neither pass had looked for. The story is in
[[Development#The parity campaign, 2026-10-20]].

So an entry here is worth more than it was on 2026-10-19, and where the two passes cover the same
ground the triage is the better source: it was driven against the current code, so its line numbers
and its evidence are the ones to act on, and several of its entries narrow a finding the first pass
stated too widely. What an entry does *not* promise is that the fix is settled: several say plainly
that the Obsidian behaviour is documented rather than probed, and the probes those wait on are
collected under [[#Declined until Obsidian is probed]].

The vocabulary is unchanged. *bug* is behaviour  gets wrong on its own terms; *parity-gap* is
behaviour that works but does not match Obsidian; *missing-feature* is something Obsidian has that
 has not built. Severity is the reviewer's, not the author's.

**What the campaign worked from** — the first pass's 82 (`F` numbers) plus the second pass's Live
Preview file (`L` numbers):

| Subsystem | Confirmed | Fixed | Still open |
| --- | --- | --- | --- |
| Files view and file operations | 13 | 4 | 9 |
| Live Preview | 13 | 9 | 4 |
| Reading view | 12 | 5 | 7 |
| Right dock panes and graph | 12 | 6 | 6 |
| Search, switcher, palette and autocompletion | 12 | 8 | 4 |
| Settings | 9 | 4 | 5 |
| Source editor | 12 | 12 | 0 |
| Workspace shell | 12 | 6 | 6 |
| **Total** | **95** | **54** | **41** |

**What the second pass's other seven subsystems said when triaged against the finished binary.**
These 79 findings overlap the table above heavily and are worded differently, so the two sets are
not additive — this one is a check on the campaign's work rather than a new backlog:

| Subsystem | Findings | Already fixed | Fixed by wave 8 | Still reproduce |
| --- | --- | --- | --- | --- |
| Files view and file operations | 12 | 4 | – | 8 |
| Reading view | 11 | 2 | – | 9 |
| Right dock panes and graph | 11 | 2 | – | 9 |
| Search, switcher, palette and autocompletion | 9 | 4 | – | 5 |
| Settings | 11 | 5 | – | 6 |
| Source editor | 13 | 7 | 1 | 5 |
| Workspace shell | 12 | 3 | 1 | 8 |
| **Total** | **79** | **27** | **2** | **50** |

Plus one the triage found that neither audit had: a panic that killed the app, fixed in wave 8.

Most of the 50 are the same large gaps the first pass had already filed — no tabs, no ribbon, the
Hotkeys tab, the graph controls panel — and they are not duplicated below. What the triage added is
twenty entries nobody had filed before and fresh evidence on a dozen more; both are folded into the
sections that follow, so the sections held **61 open entries** in total; the five large builds of 2026-10-20 and the
four small entries they carried closed nine of them, so **52 remain** — plus one filed after the frame work of the same evening, below; the Files header of 2026-10-21 closed one more and halved two, and the explorer keyboard the same day closed the halved header entry.

## Files view and file operations

### Autosaving the open note re-centers the Files list and re-expands every ancestor folder

*medium, bug, small*

**.** Confirmed twice, most recently against the finished binary with a sentinel status so the
new toast notices could not mask the result: append to an unrelated file from a shell, so
`after_changes` sets `status = "zzz.md changed"`; type one character into the open note; at t+3 s
the status is "saved" and at t+10 s, once the notice has expired, it is
"Untitled.md · reading view built in 0.0 ms". That string is only ever set by `show_file`, so the
app's own autosave write came back through the watcher and re-entered it (workspace.rs:1769), and
`show_file` unconditionally calls `explorer.reveal(path)` (workspace.rs:2495), which strips every
ancestor out of `collapsed` and scrolls the row to centre. `flush_save` records nothing about the
bytes it just wrote, so `after_changes` cannot tell a self-write from an external edit.

**Obsidian.** Obsidian never scrolls the explorer on its own unless the file-explorer leaf's
`autoReveal` is on, and both Obsidian-written `workspace.json` fixtures in the repo have it off.
Saving a note moves nothing. (That last sentence is inference from the fixture, not an observation
— see [[#Declined until Obsidian is probed]].)

**Fix.** Two independent halves. The explorer half landed on 2026-10-21 with the Files header:
`set_tree` no longer reveals anything and `show_file` calls `FileExplorer::follow`, which only
highlights the row unless the leaf's `autoReveal` is on — so the re-entry still happens but moves
nothing. What remains is the self-write itself: in `after_changes`, a Modified(current) whose
on-disk text the editor already matches should call `refresh_panes` instead of `show_file` — a
content check, not an mtime stamp, so a genuine external write is never swallowed.

### Rename accepts `/` and silently creates folders; an empty name reports the dot rule

*medium, bug, small*

**.** F2 on a note, type `Sub/Renamed`, Enter: no error, the file becomes `Sub/Renamed.md`
and a `Sub/` directory that did not exist is created. F2 then Ctrl+A then Backspace then Enter
leaves the modal open with "File name must not start with a dot.". `bad?name` is refused with a
message that *names* `/` among the forbidden characters. `PendingOp::Rename` (workspace.rs:3249)
never validates the typed value; the only check is per-segment `validate_path` (ops.rs:87-95). A
path escape is **not** possible — `../Escaped` and a leading slash are both refused and nothing was
written outside the vault — so this is folder creation inside the vault, which is why the triage
raised it from low to medium.

**Obsidian.** Rename refuses any of `* " \ / < > : | ?` and an empty name with "File name cannot be
empty."; the inline rename edits a basename, not a path, so it cannot create folders.

**Fix.** `_formats::validate_file_name(value)` on the raw trimmed value as the first statement
of the `PendingOp::Rename` arm, with `Prompt::set_error` on Err, before `ext`/`new_name`/`to` are
built. The strings are already there. Note the cause of the empty-name half precisely: the validator
is not wrong, the value never reaches it, because workspace.rs:3263 turns `""` into the path `.md`
first. The sibling `PendingOp::Move` arm has the same hole, where an empty value renames a file to
itself.

### The explorer keeps highlighting a file it no longer has

*low, bug, small*

**.** Delete the open note: `file.path` is null but `explorer.active` is still the dead path,
painted with `bg_active`. Recreate the path from a shell and the watcher re-reveals it, so the row
comes back highlighted with no file open. `delete_now` clears `self.current` and calls
`detach_current` and `forget_history`, but never touches the explorer's `active`, and `set_tree`
re-reveals the stale path on every rebuild.

**Obsidian.** `is-active` tracks the active leaf's file; with no file open no row carries it.

**Fix.** `FileExplorer::clear_active`, called from `delete_now` beside `detach_current`; and have
`set_tree` drop an `active` whose path is no longer in the tree, so a stale highlight cannot survive
a rescan.

### Creating a folder steals the active-row highlight from the open note

*low, bug, small*

**.** With `Welcome.md` open, `file-explorer:new-folder` + "Archive" + Enter leaves
`explorer.active` on `Archive` while `file.path` is still `Welcome.md`, and a forced tree rebuild
does not repair it. `PendingOp::NewFolder` calls `explorer.reveal`, and `reveal` sets `active`.

**Obsidian.** Creating a folder never moves `is-active`; the spec's explorer capture shows it on a
file row.

**Fix.** The `scroll_to(path)` from the first entry, called from the NewFolder arm. `reveal` and
`set_active` should only ever be handed the path the main area is showing.

### Rename surfaces a raw OS error with the absolute host path

*low, bug, small*

**.** F2, clear, 300 `L` characters, Enter: the modal shows
`rename /tmp/.../vault/Welcome.md: File name too long (os error 36)` — the machine's filesystem path
and an errno. Confirmed again by the second pass. `validate_file_name` has no length rule and the
rename arm never runs it on the raw value anyway.

**Obsidian.** Rename errors are short sentences about the name; no absolute paths, no errno.

**Fix.** Make `OpError::Io`'s Display print the vault-relative path (the `io("rename", &from_abs)`
call at ops.rs:253 already has `from`), and add a 255-byte-per-segment check to
`validate_file_name` so the common case is caught before the syscall.

### File-operation feedback is 's wording, and fires even when nothing was rewritten

*low, parity-gap, medium*

**.** Half of this is now fixed: the notice layer means `show_file`'s "reading view built"
line can no longer overwrite the report — a rename sampled at t=0,1,2,3 s keeps saying
"Renamed zzz.md to yyy.md". The wording half reproduces exactly. That rename rewrote zero links and
still produced a message, and the link case reads "Renamed Target.md to Sub/Renamed.md, updated 3
links in 1 file". "Deleted X" and "Copied X to Y" are reported too.

**Obsidian.** A move that rewrites links shows a Notice reading exactly `Updated 1 link in 1 file.`
(Specification.md:1972-1978); a move that leaves the basename alone produces no notice, and nothing
about a file operation reaches the status bar.

**Fix.** In `apply_rename`'s summary block, return without reporting when
`outcome.rewritten_links == 0`, and otherwise emit exactly `Updated {n} link{s} in {m} file{s}.`
with no "Renamed X to Y" prefix. Drop the "Deleted" and "Copied" reports.

### Non-markdown rows show the extension inline instead of as a tag

*low, parity-gap, small*

**.** `explorer.rows` reports `{"path":"cover.png","name":"cover.png"}` — the whole filename
is the row's single label — and the row is painted `text_muted` entirely, because `display_name`
strips only `.md`.

**Obsidian.** A non-markdown row renders the basename in `.nav-file-title-content` and the extension
in a sibling `.nav-file-tag`: `cover.png` is "cover" plus a "png" chip.

**Fix.** Give Row a `stem` and `ext` beside `name`, render the stem in `theme.text` and the extension
as a small rounded chip, keep sorting on the stem, and keep `describe`'s `name` as stem+ext so the
state dump is unchanged.

## Live Preview

### Heading and list folding does not exist

*medium, missing-feature, large*

**.** `editor:toggle-fold`, `fold-all`, `unfold-all`, `fold-less` and `fold-more` are all
registered with `"runnable": false`; Ctrl+P then "fold" offers only the properties fold and fuzzy
noise. No fold indicator is reachable anywhere. This is also the open remainder of the source
editor's command-palette finding.

**Obsidian.** [[Specification#Observed heading fold behavior, Obsidian 1.14.4]] is a 14-step CDP
probe: every foldable line carries a hover-revealed indicator (headings at `left: -15px`, list
parents at `-10px`), a heading folds through the line before the next heading of the same or higher
level with trailing blanks included, the last heading folds to the end of the file, the folded line
gets a `cm-foldPlaceholder` ellipsis, and nothing is written to the note — the state lives in
localStorage under `<appId>-note-fold-<path>`.

**Fix.** Give `Editor` a `folded: Vec<Range<usize>>` of line ranges computed from the highlight
spans, skip folded lines in prepaint the way folded widgets are skipped, append the ellipsis to the
folded line's `DisplayLine`, and add the five ids to RUNNABLE. Persist per-note ranges in 's
own app data, never in the vault.

### Image embeds render only as a whole top-level paragraph

*medium, parity-gap, large*

**.** With an 80×80 PNG in the vault, `- ![[real.png]]`, `Look: ![[real.png]] here` and
`> ![[real.png]]` each measure one 24 px text row and a click on them navigates to the file, while
the same embed alone on a line measures ~104 px. Sizing works (`|40` gives ~64 px); only placement
is the limit. `build_widgets` walks only `doc.blocks` and only single-inline paragraphs.

**Obsidian.** Obsidian renders image embeds inline wherever they occur. The repo's evidence for the
*nested* case is weak: Specification.md:1041 and :1192-1198 disclaim the captured rows as unlaid-out
widget output rather than a measured Live Preview session, and the corpus holds no image embed at
all. Probe the list-item case before designing the inline-in-a-sentence one.

**Fix.** Two steps. The nested case first: have `build_widgets` walk list items', blockquotes' and
callouts' children, with the widget carrying the leading marker and indent as a prefix the renderer
re-draws — a naive promotion loses the bullet, because `widget_lines` widens a widget's span to
whole lines and the editor paints the widget *instead of* them. Inline placement then needs a
non-text piece kind in `preview.rs`'s `DisplayLine` threaded through shaping, hit testing, selection
and the cursor maths, which is an `editor.rs` project of its own.

### HTML other than an `<img>`-only block is still raw muted source

*medium, parity-gap, medium*

**.** An HTML block that holds nothing but `<img>` tags now resolves and renders as an image
widget in both views. Everything else — comments, `<div>`, `<pre>`, `<a>`, inline `<u>`, `<sup>`,
`<kbd>` — still falls through to `RBlock::Html`, drawn as muted monospace source. The promotion is
deliberately narrow: the parser drops CommonMark's html-block discriminator, so a comment and a
`<script>` arrive as the same variant and a blanket promotion would fold their source lines away to
show the same muted text.

**Obsidian.** [[Specification#Observed raw HTML behavior, Obsidian 1.14.4 (installer build 1.13.7)]]
records the measured subset: `<a>` kept with `target`/`rel`, `<pre>`, and `<b>/<i>/<u>/<kbd>/<sup>/
<sub>/<details>` kept and themed, with only scripts and event handlers stripped.

**Fix.** The reading renderer's `RBlock::Html` arm plus the inline path. `RichText` runs are flat
`(range, style)` pairs with no nesting, so `<sup>`, `<sub>` and `<kbd>` need new `InlineStyle` flags
rather than only text.

### `propertiesInDocument: hidden` still leaves a 44 px gap in Live Preview

*low, bug, small*

**.** The reading view honours the setting exactly — with it `hidden`, a note with no
frontmatter and a note with two properties both put their first clickable content in the band
82..106, a zero-pixel difference. Live Preview does not: the same two notes give first content at
y 62..78 and y 106..128, a 44 px difference where there should be none. The second pass measured a
24 px residue before the campaign, so the gap moved rather than closed.

**Obsidian.** Under `hidden` the in-document block is absent in both views; only the inline title
remains in the header (Specification.md's properties-mode table).

**Fix.** Find what the editor still reserves for the properties widget when
`PropertiesInDocument::Hidden` — the reading view's guard is in `Renderer::properties`, which both
views share, so the leftover height is in the editor's own widget accounting
(`widget_heights`/`height_of`) rather than in the renderer.

## Reading view

### A block jump lands on the wrong block after a `%%comment%%` or a footnote definition

*medium, bug, small*

**.** `Model.starts` has one entry per **parsed** block while `Model.blocks` is a `filter_map`
that drops comments and footnote definitions, so `block_at` and `scroll_to_block` index two
different lists. Measured: the same note with and without one leading `%%a block comment%%` jumps to
`{item:1}` and `{item:2}` for the same heading; a note with two footnote definitions pushes the
index past the end, so `fn#Tail Heading` is clamped to `{item:48}` of a 64-block list and the reader
lands nowhere near the heading. Every heading or offset jump — from the Outline, from Backlinks,
from a `[[Note#Heading]]` — is affected in any note holding a block comment or a footnote.

**Obsidian.** Not a parity question: this is one internal index space disagreeing with another.

**Fix.** Build `starts` with the same predicate the builder drops on, so the two cannot drift
(model.rs:232). Sturdier: have `Builder::blocks` return `(RBlock, usize)` pairs and derive `starts`
from the built list, and push one more start for the appended Footnotes section so an offset inside
a definition resolves to that block rather than past the end.

### No keyboard at all: neither scrolling nor selection nor copy

*high, missing-feature, large*

**.** Focus is `{reading, [Workspace, ReadingView]}` and every key is inert. PageDown,
PageUp, Down, Up, Home, End, Ctrl+Home, Ctrl+End and Space all leave `reading.scroll` at
`{item:0, offset:0.0}`, while a wheel event in the same session moves it to `{item:2}` — so the list
scrolls and only the keyboard is dead. A press-drag-release selects nothing, `ctrl-a` and `ctrl-c`
answer `handled: false`, and `editor:copy` answers "not available in  yet". The cause is one
line of policy: `view.rs:366` declares `.key_context("ReadingView")` and `bind_keys` registers every
binding with `ctx = Some("Workspace")`, so nothing is ever bound in that context.

**Obsidian.** The reading view is DOM: the page keys scroll it, click-drag selects across blocks,
double-click takes a word, Ctrl+A selects the note and Ctrl+C copies plain text.

**Fix.** Two sizes, and the small one is worth landing alone. Scrolling: add ScrollPageDown/Up,
ScrollLineDown/Up and ScrollToTop/Bottom actions, bind them with `ctx = Some("ReadingView")`, and
implement them beside `scroll_to_block` over the existing `ListState` (`scroll_to`, the already
wrapped `scroll_to_end`, and a `scroll_by(px)`). Selection is the large job: an anchor/head
(block index, `RichText` id, byte offset) on `ReadingView`, mouse down/move/up on the root div, the
ranges painted through the existing highlight path, and a Copy action joining the selected runs.

### Every reload throws away the reader's folds

*medium, bug, small*

**.** Expand a collapsed callout in the reading view, then write the note's own bytes back
unchanged: 1.5 s later the callout is collapsed again. `ReadingView::set_note` computes
`let same = path.is_some() && self.path == path` and uses it to keep the scroll position, but
`self.toggled.clear()` three lines later runs unconditionally. Since the watcher re-enters
`show_file` on every autosave, a fold survives only until the next save of the note you are reading.

**Obsidian.** A fold survives a reload and a relaunch; Obsidian keeps it per file outside the vault,
under `<vault id>-note-fold-<path>` (Specification.md:1463-1470).

**Fix.** `if !same { self.toggled.clear(); }` — `same` is already computed two lines up. For real
persistence promote the field to `HashMap<String, HashSet<usize>>` keyed by vault-relative path and
store it outside the vault, the way Obsidian does.

### `editor:toggle-fold-properties` is a visible no-op in reading view

*low, bug, small*

**.** Two thirds of this is now fixed: the gate moved from `mode.is_editing()` to
`note_has_keys()`, and `markdown:clear-metadata-properties` really does edit and autosave from
reading view. What is left is the fold: the command flips the hidden editor's `properties_folded`
while the reading view folds off its own `toggled` set keyed by `PROPERTIES_FOLD_ID`, so the block
on screen does not move — the properties band measured [212..236] before and after.
`markdown:add-metadata-property` likewise writes nothing and has no reachable row.

**Obsidian.** Specification.md:1467-1469 records a real 1.14.4 running
`editor:toggle-fold-properties` from a preview leaf and toggling the fold, and
[[Specification#Observed properties-in-reading-view behavior, Obsidian 1.14.4]] shows the in-document
block there as fully live — an "Add property" button, a key input per row and a type menu.

**Fix.** Branch on the mode in the command arm: when `!self.mode.is_editing()`, route to the reading
view's own `toggle_callout(PROPERTIES_FOLD_ID)` (make it `pub`), else keep the editor call. The
bigger half is giving `ReadingView` a `PropertiesHost` whose `on_action` emits a `ReadingEvent` the
workspace turns into the same `properties::plan` write the editor uses.

### No inline title anywhere; `showInlineTitle` has no reader

*medium, parity-gap, medium*

**.** `show_inline_title` occurs exactly twice in the workspace: the Settings row and the
serde field. Live, with it true and the properties block hidden, a note whose whole body is one
embed puts that embed at y 82..106 — the pane header plus the list's first-block padding — with no
room for an h1-sized title. The view header's title is not clickable either: a single and a double
click on it leave `modal.kind` at "none", so renaming is only reachable from F2 or the context menu.

**Obsidian.** With `showInlineTitle` (default on) the reading view opens with `div.mod-header.mod-ui`
holding the `.inline-title`, the basename at h1 size, above the properties block. What a plain click
on the *view header* title does is **not** settled — Specification.md:2026-2028 leaves it open — so
the inline title is the backed half of this entry and click-to-rename is not.

**Fix.** A `show_inline_title: bool` on `ReadingView` set from `apply_app_config`, drawn in Render as
a sibling header row above the list — a sibling, never a list item, so block indices and
`scroll_to_block` keep their meaning. Do the same in the editor so the two modes agree.

### Clicking a footnote reference scrolls to the end of the note

*medium, bug, medium*

**.** Now driven directly, contrary to the first pass's note that it was unreachable: 60 hits
across two x clusters on the `[^1]` and `[^2]` glyphs all take `reading.scroll` from `{item:0}` to
`{item:48}`, the clamped list end, past the whole Footnotes section — so the footnote number is
provably discarded. `LinkTarget::Footnote(_)` maps to `scroll_to_end` and touches only the reading
view, so the same click in a Live Preview widget moves nothing at all.

**Obsidian.** Each reference is `<sup class="footnote-ref"><a data-footref="1" href="#fn-1-<hash>">`
— a per-footnote anchor — and every definition ends with a `footnote-backref` link back. The corpus
holds 18 of each and 19 `<sup>` elements.

**Fix.** Record the Footnotes block's index and the per-label row order on `Model` while building,
then resolve the label and call the same mode-aware reveal the heading jumps use — `scroll_to_block`
in reading, the definition's byte span in Live Preview and Source. The superscript and the backref
arrow are separate cosmetic follow-ups.

### Headings cannot be folded; `foldHeading` has no consumer

*medium, parity-gap, medium*

**.** `RBlock::Heading` renders as a bare styled div with no `.id()`, no hover state and no
click handler, and the model records no section extent. Click sweeps over heading rows and the left
gutter change nothing. `foldHeading` occurs only in settings.rs and config.rs.

**Obsidian.** [[Specification#Observed heading fold behavior, Obsidian 1.14.4]]: the indicator is
`span.heading-collapse-indicator.collapse-indicator.collapse-icon`, absolute at left≈32 px, opacity
0 until hover, `is-collapsed` when folded; collapsed content is absent from the preview DOM, the
fold is shared with the editor and persisted per file outside the vault, and `foldHeading` defaults
to true.

**Fix.** Record for each heading the range of top-level block indices it owns, keep
`folded: HashSet<usize>` beside `toggled`, render a hover-visible triangle as an absolutely
positioned child at -28 px, and return an empty element for indices inside a folded range. Land it
with the fold-persistence fix above, since both want the same per-path store.

### The reading model is rebuilt for every file in every mode, and the status bar says so

*low, bug, medium*

**.** Opening three notes in each of live, source and reading gives the same work and the same
wording in all three: "Specification.md · reading view built in 2.9 ms",
"Development.md · … 1.4 ms", "UI parity backlog.md · … 0.8 ms". `show_file` calls
`reading::build_with` unconditionally for every markdown open before any mode check, so an editing
session pays for a view it is not showing, and the status bar reports a reading view that is not on
screen.

**Obsidian.** A leaf renders the view it is showing; nothing about build time reaches the status
bar.

**Fix.** Keep the note text on the workspace and build lazily — build only in Reading mode,
otherwise hand the view an empty model (or mark it stale) and build on the way into reading when the
stored model is not this note's. Set the "reading view built in …" status only on the path that
actually built, and give the other modes the note's own status line; see also the status-bar entry
under Workspace shell.

### Markdown image size hints are dropped

*low, bug, small*

**.** With a real 400×300 PNG in the vault, `![alt|100](pic.png)` and `![alt](pic.png)` lay
out identically (click band 404..428) while `![[pic.png|100]]` shrinks to 180..204. The
markdown-image branch strips everything after `|` from the alt and then calls `self.embed(..)` with
width and height hardcoded `None`, while `wikilink_embed` parses the hint.

**Obsidian.** Both spellings honour the hint; `|WxH` becomes width and height
([[Specification#Observed remote image behavior, Obsidian 1.14.4 (installer build 1.13.7)]] records
it for the markdown spelling specifically).

**Fix.** Split the alt the way `wikilink_embed` does instead of discarding the tail — let a trailing
part that parses as `W` or `WxH` claim the size and keep the rest as the alt — and reuse the one
`size_hint` helper so the two spellings cannot disagree again.

### An embed inside a heading, a table cell or a callout title renders as literal `![[Name]]`

*low, bug, small*

**.** `InlineKind::Wikilink { embed: true }` pushes the three literal pieces `"![["`, the
display text and `"]]"` into the builder as plain unstyled text and never attaches a link, while the
non-embed arm styles it and attaches `LinkTarget::Internal`. That path is `rich()`, which is what
headings, table cells and callout titles are built from; only paragraph content goes through
`segments()`, the one place `Segment::Embed` is produced. So an embed written anywhere but a
paragraph shows its own markup and is not even clickable.

**Obsidian.** An embed in a table cell renders the picture; in a heading it renders the embedded
content inline. Neither shows raw brackets.

**Fix.** Cheapest correct step: make the embed arm fall through to the non-embed one, so it is at
least readable and clickable. Properly: give `RichText` an inline-embed run (or let `rich()` return
`Vec<Segment>` the way `segments()` does) and draw it in the cell and heading paths.

### Callouts have no type icon, and the fold affordance is a text triangle

*medium, parity-gap, medium*

**.** The done-task half of this finding landed — a completed task is muted *and* struck
through. The icon half did not: the callout title row is still `div().flex_1().child(rich(title))`
with no icon slot, and the fold glyph is still `▸`/`▾`. No `AssetSource` is registered anywhere in
the app, so `svg()` would silently paint nothing.

**Obsidian.** The corpus proves the markup (`callout-icon`) but every one of those `<svg>` elements
is empty, so which lucide icon belongs to which canonical callout type is the one thing it does not
show.

**Fix.** Two prerequisites, in order: a probe that reads the icon name per callout type out of a
real Obsidian, and the app's first asset pipeline (an embedded `AssetSource` wired in `main.rs`).
`explorer.rs`, `search.rs` and `settings.rs` draw the same text triangles, so the chevron should
change with them, not alone.

### Raw and inline HTML is literal source; remote images render as a link

*medium, parity-gap, large*

**.** `line one<br>line two`, `<u>underlined</u>` and `<sup>2</sup>` all appear as their tags;
`![remote](https://…)` becomes a link-styled run with an External target, so clicking it would open
the system browser rather than draw an image. (An `<img>`-only HTML block is the one case that now
renders — see the Live Preview entry above.)

**Obsidian.** [[Specification#Observed remote image behavior, Obsidian 1.14.4 (installer build 1.13.7)]]:
every `http(s):` image destination is fetched with no prompt and rendered as
`<img referrerpolicy="no-referrer">`, `|200x100` becoming width and height, a failed load staying a
blank `<img>`.

**Fix.** Handle the subset in the model rather than the view: map `<br>` to a line break, map the
inline tags onto `InlineStyle` (adding underline, sup, sub, kbd), drop unknown tags keeping their
content, and add `EmbedKind::Remote(url)` for scheme-bearing image destinations with the `|WxH`
hint parsed as `wikilink_embed` already does, rendered through gpui's `img()` with nothing visible
on failure.

## Right dock panes and graph

### A jump into Live Preview lands short of the target and never corrects itself

*medium, bug, small*

**.** Click Outline row 7 of Specification.md in Live Preview (offset 34938): the cursor goes
to line 432 but the scroll stops at line 392 with `visible_lines [392,425]`, so the heading the user
asked for is below the fold — and it stays there, byte-identical at +1 s, +2 s and +3 s. A second
click on the same row gets it right. Source mode is correct on the first click. The cause is one
predicate: after a widget's real height is measured, `previous.is_some_and(|p| p != height)` arms a
corrective reveal only when the widget had been measured *before*, so the first jump over an
unmeasured table or code fence — which the reveal computed from `height_of`'s line-count estimate —
never gets its correction. Every jump inherits it: Outline, Backlinks, Outgoing links and search
hits.

**Obsidian.** The jump lands on the target the first time.

**Fix.** Compare against the estimate rather than against `None`:
`let estimate = previous.unwrap_or(line_height * w.lines.len() as f32); if estimate != height { pending_reveal = true; }`
— exactly one corrective reveal on the frame where a widget's real height first differs from the
guess, and silence once measured. This is the remaining half of the reveal work described in
[[Development#Control socket and MCP server, 2026-10-19]].

### Graph filter chips silently reset to `graph.json` on every vault change

*medium, bug, small*

**.** With `showTags: false` in `graph.json`: the graph shows 11 nodes, clicking the Tags chip
gives 14, and writing any note into the vault gives 12 — 11 plus the new note, with tags off again.
Repeated with a second note: 15 after a click, 13 after the write. `set_state` reassigns
`self.config = state.vault().graph_config().unwrap_or_default()` unconditionally, and the workspace
calls it on every vault refresh. The camera is now guarded against exactly this; the filters are
not.

**Obsidian.** A filter toggled in the controls panel stays toggled, and is written back to
`graph.json` about 300 ms later.

**Fix.** Load `graph_config()` only on the first `set_state` — a `config_loaded: bool` beside
`adopted_scale`, the same shape as the camera's `adopts_saved_scale` guard — so a vault refresh only
swaps the state and rebuilds. This must land together with the write-back entry below, or each
write is immediately re-read and undone.

### Graph wheel zoom is inverted

*medium, bug, small*

**.** `scroll dy:-4` takes the graph scale 0.932 → 1.413 (zoom in) and `dy:+4` takes it back
(zoom out). The control in the same session: `dy:+3` in the editor scrolls *up*, so `dy > 0` is
wheel-up — and `ctrl-=` (Zoom in) raises the scale. So the wheel disagrees both with the editor's
own convention and with the keyboard command.

**Obsidian.** Wheel-up zooms in.

**Fix.** Drop the minus in `let factor = (-f32::from(delta.y) * 0.004).exp();` so a positive
(wheel-up) delta gives a factor above 1, matching `zoom_in`.

### A local graph centred on a file the filters hide renders nothing at all

*medium, bug, small*

**.** With an unlinked `lonely.png` open, `graph:open-local` gives `{local: true, nodes: 0}` —
a view named after a file, with the file not in it. Toggling the Attachments chip flips it 1/0/1/0,
which proves the filter is the cause: the centre is looked up in an index the filters have already
pruned, and the build returns an empty `Built` when it is missing.

**Obsidian.** `localFile` is a separate engine option from `showAttachments`/`showTags`; the centre
of a local graph is always drawn.

**Fix.** When the centre is missing from the index, add it rather than returning empty — the BFS
then starts from a real node and the centre survives any filter. Separately, `show_graph` should
refuse `local == true` when no file is open; today it silently falls back to the global graph, which
is better than the empty canvas it used to paint but still not Obsidian's unavailable command.

### `graph.json` is never created and never written back

*medium, parity-gap, medium*

**.** Byte-identical through ~15 chip clicks, several wheel zooms, a local graph and a quit:
md5 `41ffa915…` before and after, mtime unchanged, and with the file deleted it is never created.
`grep` settles it — `write_config("graph.json", ..)` appears only inside two `#[test]` bodies, so
there is no production writer anywhere.

**Obsidian.** [[Specification#Observed graph.json write behavior, Obsidian 1.14.4 (installer build 1.13.7)]]:
opening the global graph creates the file once as a complete twenty-key document with nobody
touching the controls; each section-header click writes exactly one key ~300 ms later; wheel zoom
writes `scale`; the Close button writes `close`; opening a *local* graph writes nothing.

**Fix.** `GraphView::save(cx)` serialising the whole `GraphConfig` through the existing
`write_config` path (whose round-trip test already shows key order and float precision survive),
debounced ~300 ms, called from the three toggles and from `zoom_at` with the wheel-target scale
rather than the animating one, and once on the first global open. Skip it while a local graph is up.
Land it with the chip-reset fix above.

### Unlinked mentions emit duplicate rows and the count is the cap, not the total

*medium, bug, small*

**.** A note named `the`: the Backlinks pane shows "Unlinked mentions (200)" over 202 entries,
of which 60 distinct `(path, offset)` pairs are duplicated — `(Development.md, 4433)` three times,
`(Development.md, 6061)` four times — every copy carrying the same line offset, so they all navigate
to exactly the same place. The 200 is the `take(MAX)` cap printed as if it were the real total. The
outgoing half of the same function already dedupes with a `seen` set; the incoming half does not.

**Obsidian.** One row per mention, and the section count is the number of mentions.

**Fix.** `found.dedup()` after the existing `found.sort()` (the tuple is `(path, line_offset, line)`,
so equal tuples are exactly the same-line duplicates), then capture `let total = found.len();`
before `.take(MAX)` and build the header from `total`, marking it capped when it exceeds MAX.

### Outgoing links rows drop the subpath and the alias, and never carry an offset

*low, bug, small*

**.** `See [[Specification#Parser notes]] and [[GA#GA]].` gives two rows labelled
"Specification" and "GA" with detail the target filename and `offset: null` on both — the subpath is
gone, so a click lands at the top of the note. `[[GA|an alias for GA]]` renders as "GA": the alias
is gone too. The Backlinks pane does carry offsets and clicking one puts the cursor exactly there,
so the asymmetry is inside `refresh_panes`, not in the navigation code.

**Obsidian.** The pane lists each link with its display text, and clicking one navigates exactly as
clicking the link in the note does, heading and block subpaths included.

**Fix.** `outgoing_link_rows` currently drops the `LinkCache` after taking `l.link.as_str()`. Carry
the whole link: label from `display_text` when present, else the written `parse_linktext` path plus
subpath; and fill `offset` by resolving the subpath against the target's `CachedMetadata` with the
existing `scroll_to_subpath` resolver, so `pane_row`'s click handler reveals it.

### Unresolved rows in Outgoing links are inert

*low, parity-gap, small*

**.** `pane_row` attaches cursor, hover and `on_click` only when `path.is_some()`, so an
unresolved row has no pointer, no hover and no handler at all.

**Obsidian.** Unresolved links render dimmed (`is-unresolved`) and clicking one creates the note,
exactly as clicking an unresolved wikilink in the editor does.

**Fix.** Carry the link body on the row (a new `PaneRow` variant, or an `unresolved: Option<_>`
field) so `pane_row` can route a click to the same create-from-link path the quick switcher's
Create event uses, keeping the muted colour.

### "Open graph view" while a local graph is on screen closes the graph

*low, bug, small*

**.** With a local graph up, `graph:open` leaves graph mode entirely — `file.mode` goes back
to "live" and `graph.open` to false — so the user who asked for the global graph gets the editor
back and has to press it again. `open_graph_action` toggles whenever the mode is Graph, without
asking which graph is showing; `graph:open-local` is not a toggle, so the pair is inconsistent.

**Obsidian.** "Open graph view" from a local graph shows the global graph.

**Fix.** Guard the toggle on the graph being the global one:
`if mode == Graph && !self.graph.read(cx).is_local() { leave } else { show_graph(false) }`.
`is_local()` already exists.

### Local graph: one hop, interlinks always drawn, no Depth or direction controls

*low, parity-gap, medium*

**.** The local graph of a note with two forward links, one of which also links the other,
keeps the neighbour-to-neighbour edge and every one-hop neighbour in either direction. Depth is the
literal `if depth >= 1 { continue; }`, and `grep` for `local_jumps`, `interlink`, `forelink` or
`Depth` across the graph view and the config types returns nothing — the four Obsidian options do
not exist anywhere in the tree.

**Obsidian.** The localgraph leaf's own option set is `localJumps: 1`, `localBacklinks: true`,
`localForelinks: true`, `localInterlinks: false`; with interlinks off Obsidian hides edges between
two neighbours that do not pass through the centre, and the Filters section exposes Depth plus the
three toggles.

**Fix.** Carry the four options on `GraphView`, record each kept node's BFS depth in a map rather
than a set, walk out to `local_jumps`, follow an edge only in the directions the flags allow (which
needs the directed edge list the build already has), and drop same-depth edges in `finish` when
interlinks are off.

### Backlinks and Outgoing are flat per-occurrence lists

*medium, parity-gap, large*

**.** Development.md's Backlinks tab showed "Linked mentions (41)" followed by 41 rows all
labelled "Development", each detail a `{line}: {trimmed source line}` string, with no grouping and
nothing marking the matched text. Unlinked mentions carry a path and an offset but no way to turn
the text into a link, and there is no filter, sort, collapse-all or more-context control anywhere.
(A property link now shows the property name as its context — that half landed.)

**Obsidian.** A header row (search filter, collapse, show more context, sort), two sections with
count badges, results grouped per file as collapsible tree items with a per-file count, each match
showing surrounding text with the link highlighted, and a "Link" button on every unlinked mention.

**Fix.** Replace `PaneRow` with Section > FileGroup > Match and render it with the explorer's
tree-item styling, carrying the match's byte range so the context can highlight it. The Link button
rewrites the matched range to `[[Name]]` through the text crate's edit plan.

## Search, switcher, palette and autocompletion

### Search results cannot be navigated or opened from the keyboard

*medium, parity-gap, medium*

**.** With results on screen and focus on the query field, Down, Down and Enter each answer
`handled: true` — the `TextInput` consumed them — and change nothing: no row becomes focused,
`file.path` does not move, and Escape is swallowed with the query intact. The pane's state dump
still carries only `query`, `error`, `files`, `matches` and `results`: there is no focused-row field
at all, and the subscription matches only `InputEvent::Changed`. Mouse clicks on results do work.

**Obsidian.** ArrowDown calls `dom.changeFocusedItem("forwards")` with no input guard, ArrowLeft and
Right collapse or expand the focused file row, and Enter calls `focusedItem.onResultClick(e)`; the
focused row carries `has-focus`.

**Fix.** A `focused: Option<usize>` index into `self.rows` (which already interleaves File and Match
rows), moved by Up/Down in the subscription, painted with `bg_active`, with Submit emitting the same
`SearchEvent::Open` the mouse path emits; bind Left/Right to the existing `toggle`, and have Escape
clear the query and return focus to the editor.

### Search pane has no Match case, no settings toggles, no sort select and no count menu

*medium, missing-feature, large*

**.** `SearchPane::render` emits exactly three things: the input, an optional summary line and
the list. `Search::new` is always called without a case flag, `run_search` always sorts one way, and
`Query::explain` is never called. A grep for `matchingCase`, `sortOrder`, `explainSearch` or
`extraContext` across the app and the config types returns nothing at all — none of the six keys
Obsidian serialises exists. Per-file collapse is the one adjacent thing that landed.

**Obsidian.** [[Specification#Observed search pane behavior, Obsidian 1.14.4]]: a Match case
decorator, a sliders button revealing exactly three toggles (Collapse results, Show more context,
Explain search terms), a result-count control whose left click opens a two-item menu, and a native
`<select>` with six sort orders in the `search-results-info` bar.

**Fix.** A `SearchLeafState` of those six keys on `SearchPane`, a header row above the input, Match
case wrapping the parsed tree in `Query::CaseSensitive`, and `sort_order` threaded into
`run_search`'s sort using `VaultFile` mtime and birthtime.

### The result summary disappears when the query is emptied

*low, parity-gap, small*

**.** Search `kiwi`, then Ctrl+A and Backspace: the pane reports
`{query:"", files:0, matches:0, results:[]}` and the summary line is suppressed entirely, because
the empty-query arm returns an empty string and render only draws a non-empty one. (The other two
thirds of this finding are fixed: the summary is now `{total} results` with no file count, and a
name-only hit correctly adds to that total without adding to the file's chip.)

**Obsidian.** Once a search has run in the session Obsidian shows "0 results"; only a fresh restart
shows nothing.

**Fix.** A `has_run: bool` set when a non-empty query is dispatched, and the empty-query arm
returning "0 results" when it is true.

### The palette lists ids it cannot run, and omits ids Obsidian does have

*low, parity-gap, large*

**.** Running an editor-only command in reading view now says why — "Insert table: only
available while editing" — and the three properties ids that Obsidian really does run from a preview
leaf were exempted again in wave 5. What did not change is the list itself: `open_palette` still
filters on the static `RUNNABLE` plus an in-table check, so in reading view "template" still offers
the three Templates rows. The other direction is also still true: of 244 registered commands 86 are
runnable (up from 59), but `editor:toggle-fold`, `workspace:close`,
`workspace:split-vertical`/`-horizontal` and `app:reload` are all `runnable: false` and absent from
the palette although they are real entries in a 1.14.4 palette.

**Obsidian.** An `editorCallback` command is filtered out of the palette when there is no editor to
run it in, rather than listed and refused — and the ids above are offered.

**Fix.** Replace the static `RUNNABLE` allowlist with a `can_run(&self, id)` mirroring Obsidian's
check callbacks (editor ids need an editor with the keyboard, file ids need an open file), and
update the three call sites — the palette filter, the hotkey binder and the socket's `runnable`
flag — together so they cannot disagree. Keep the spoken refusal: it also covers the hotkey path,
which a palette-only filter would leave silently inert.

### The quick switcher ignores `switcher.json`

*low, bug, small*

**.** With `{"showExistingOnly": true, "showAttachments": false}` written and the app
relaunched, the switcher still offers an unresolved target for the query `Nonexistent` and still
offers `pic.png` for `pic`. `set_state` reads only `app().show_unsupported_files`, the unresolved
loop runs unconditionally, and `state.switcher()` is read nowhere outside settings.rs.

**Obsidian.** "Show attachments" off hides images, PDFs and other attachments; "Show existing files
only" on hides the not-created-yet targets.  already models and writes both keys.

**Fix.** Read `state.switcher()` at the top of `set_state`: skip `Image | Audio | Video | Pdf` when
`show_attachments` is false, use `show_all_file_types` for `FileKind::Other`, and guard the
unresolved block behind `show_existing_only != Some(true)`. Call `set_state` again on
`SettingChange::Switcher` so a toggle takes effect without a restart.

## Settings

### On a window narrower than about 1180 px no control in the modal can be clicked

*medium, bug, small*

**.** At 900×800: the nav column still works (clicks at x=60 and x=140 switch tabs), but a
sweep of x = 700, 760, 780, 800, 810, 820, 830, 845 across six row heights left `app.json`
byte-identical and `modal.menu`/`modal.editing` null every time, and x ≥ 860 is the backdrop, which
closes the modal. The identical click at x=1190 on a 1400 px window flips the toggle. The panel is
still a hard `.w(px(1100.))` inside a frame that is only `.h_full().max_w_full()`, so the frame is
clipped to the window while the panel keeps laying out at 1100 px and the whole control column sits
past the clip. The campaign fixed how the modal handles the mouse and never checked that the
controls were under it.

**Obsidian.** The settings modal fits the window.

**Fix.** `.w_full().max_w(px(1100.))` on the panel and `.w(relative(0.9)).max_w(px(1100.))` on
`#settings-frame`. The nav is already `flex_shrink_0` and the content pane already `flex_1().min_w_0()`,
so the content column absorbs the shrink and the controls come back inside the window.

### The Settings nav column does not scroll

*low, bug, small*

**.** At 1400×320, the nav shows general through core-plugins and everything below is clipped:
y = 284, 300 and 316 land on the backdrop and close the modal, and a wheel event over the nav leaves
the same four y values mapping to the same four tabs. `render_nav` builds a plain
`div().flex().flex_col().w(px(220.)).h_full()` with no id and no `overflow_y_scroll`, inside a root
that is `.overflow_hidden()`.

**Obsidian.** The settings nav scrolls.

**Fix.** Give the nav div an `.id("settings-nav")` and `.overflow_y_scroll()`, exactly as the content
list already does.

### Every core-plugin options tab but two shows one "Options not shown yet" row

*medium, missing-feature, medium*

**.** `plugin_items` matches only `switcher` and `command-palette`; everything else falls to a
catch-all returning the single row "Options not shown yet —  does not read this plugin's
settings file yet." The configs are already there: `DailyNotesConfig` and `TemplatesConfig` exist in
`-formats`, and their byte-exact round trip is already asserted in the vault tests.

**Obsidian.** Each enabled core plugin's tab carries its own options.

**Fix.** Add arms for `daily-notes` (format, folder, template, "Open daily note on startup") and
`templates` (folder, dateFormat, timeFormat) using the existing `field()`/`Typed` shapes, backed by
those config types, plus the matching `SettingChange` variants. While there, expose the rendered row
names in `state.modal.rows` — see [[#Blocked by the harness, not by the code]].

### An unparseable `.obsidian` json is replaced wholesale on the first write

*low, bug, medium*

**.** With `app.json` holding `{"readableLineLength": false, "myPluginData": {"keep": "this"},}`
— a trailing comma — the app launches with no warning anywhere in `state`, and one toggle click
rewrites the file to exactly `{"readableLineLength": false}`. `myPluginData` is gone, and the
in-memory config had silently fallen back to all-defaults, so the toggle also ignored the value the
file actually held.

**Obsidian.** Not an Obsidian question: this is the repo's own ground rule — never write a file
 did not change, preserve unknown keys — and the code comment at
`-vault/src/lib.rs:359` admits the behaviour is deliberate.

**Fix.** Make the unparseable branch refuse instead of overwrite (a `VaultError::Json` that
`change_setting` surfaces), and carry the load error too, so Settings can show "app.json could not
be read; settings are showing defaults and will not be written" instead of pretending the defaults
are the vault's values.

### Core-plugin toggles write the file and change nothing in the app

*low, parity-gap, medium*

**.** Turning off Files, Graph view, Command palette and Daily notes wrote
`core-plugins.json` correctly — key order and every untouched key preserved — and nothing changed:
the explorer is still there, Ctrl+P still opens the palette whose first row is literally "Command
palette: Open command palette", and "daily" still lists the three Daily notes commands. The Settings
nav *does* honour the file, so the mechanism exists one function away.

**Obsidian.** A disabled core plugin is unloaded: its view disappears and its commands are
unregistered, so neither the palette nor the Hotkeys tab lists them.

**Fix.** Reuse the filter `render_nav` already has: a `plugin_enabled(state, id)` applied in
`load_commands` and in the palette's list, both rebuilt after a `SettingChange::CorePlugins`. Hiding
the Files/Graph/Backlinks panes is the larger half and can follow.

### "Default location for new attachments" is inert

*low, parity-gap, medium*

**.** Clicks down the whole row band leave `modal.menu` null and `app.json` byte-identical, and
no "Attachment folder path" or "Subfolder name" row ever appears. The row is built with
`dropdown(...).not_yet()`, and `dropdown` is a display-only control with no `Typed` behind it, unlike
`choice`. The dependent rows are pushed only when the stored value already has that shape, so there
is no path out of the default state.

**Obsidian.** Four options — Vault folder (`/`), Same folder as current file (`./`), In subfolder
under current folder (`./<name>`) and In the folder specified below (`<folder>`) — each writing
`attachmentFolderPath` at once, with the dependent field appearing for the chosen option.

**Fix.** Replace the `dropdown(...)` with a `choice(...)` over a `Typed` on `attachmentFolderPath`,
and key the dependent rows off the selected option rather than the stored string's shape. The
`ATTACHMENT_FOLDER` text field already implements the write, Obsidian's empty→"Attachments" rule
included. Keep the `.not_yet()` tag until the labels and stored values are probed —
[[Interaction probes]] records them as an unperformed probe.

### A click that dismisses a dropdown also activates whatever it lands on

*low, parity-gap, small*

**.** Open the `newFileLocation` menu, then click the "Always focus new tabs" toggle: the menu
closes **and** `focusNewTab` flips, in one click. Escape with a dropdown open closes the whole modal
rather than just the popover.

**Obsidian.** The control here is a native `<select>` (captured in the properties probe's JSON),
which swallows the first outside click.

**Fix.** Have the root's `dismiss_open_controls` stop the click when it actually dismissed a menu,
and bind Escape to close the popover first and the modal only when nothing is open. Minor, and
arguably better as it is — decide it deliberately rather than by accident.

## Source editor

### Right-clicking in the editor does nothing: no context menu, no caret placement

*low, missing-feature, large*

**.** A right click leaves `modal` at `{kind: "none", rows: []}`, the selection untouched and
the caret where it was; `editor:context-menu` answers "Show context menu under cursor: not available
in  yet" and is `runnable: false`. The mouse-down hook filters on
`event.button == MouseButton::Left`, and a grep for any context-menu widget across the app returns
nothing — the file explorer's deferred menu is the only menu in the tree.

**Obsidian.** Right-click places the caret (or keeps the selection when the point is inside it) and
opens a context menu of file and editor commands.

**Fix.** Accept `MouseButton::Right` in the mouse-down hook — the hitbox guard beside it is already
correct — place the caret when the point is outside the current selection, and emit an
`EditorEvent::ContextMenu { position }`. The bulk of the work is the menu widget itself: a positioned
overlay list dispatching command ids, which the explorer's menu could be generalised into.

## Workspace shell

### The frame style has no store, and four title-row controls are still missing

- **Severity:** low · **Kind:** parity-gap · **Filed:** 2026-10-20

**Obsidian.** "Window frame style" sits in Settings → Interface with `hidden` (the Linux default),
`custom` and `native`, needs a restart, and writes nothing under `.obsidian` — the choice is kept
in Obsidian's own config directory, outside the vault
([[Specification#Observed shell typography and chrome, Obsidian 1.14.4 (installer build 1.13.7)]]).
The hidden frame's title row also has a tab-list button (`lucide-chevron-down`) at the right end of
the tab strip, and the view header has a bookmark action and a "More options" menu beside the mode
toggle; the Files pane header has its five buttons (#24).

**.** Since [[Development#The frame, the type and the icons, 2026-10-20]] the window draws
Obsidian's hidden frame by default and `--frame native` / `SCORIA_FRAME=native` asks for the
desktop's; the Interface tab shows the row with that value but cannot change it, since  has
no per-user settings file to keep it in. The tab-list button and its menu are drawn since
2026-10-22; the bookmark action and the "More options" menu are not.

**Fix.** A per-user config file (XDG config dir on Linux) holding the frame style, read before the
window opens; then the two remaining controls, each after a probe of what its menu holds.


### History back and forward restore the file but not the mode or the scroll

*medium, parity-gap, medium*

**.** Open a note in Reading, open another in Source, go back: the first comes back in
*source*. Scroll likewise — a note scrolled to editor line 400 comes back at line 0. The reading
view also loses its place on any mode switch or revisit, which the editor does not. (The graph half
of this finding is now its own entry above, because it turned out to affect every open route and not
only history.)

**Obsidian.** Each history entry stores the leaf's full view state, so Navigate back returns to the
mode and position the note was left in.

**Fix.** `history: Vec<HistoryEntry { path, mode, offset }>`; stamp the outgoing entry in
`open_file` before pushing, and in `navigate_history` call `set_mode` and `reveal_offset` after
`show_file`. For the reading view, keep a per-path scroll beside `set_note` the way the editor keeps
its line.

### Status bar shows parse timings where Obsidian shows the backlink count

*medium, parity-gap, small*

**.** At rest the left half reads "Welcome.md · reading view built in 0.0 ms" — in every mode,
including Source, where no reading view is on screen — and on an empty vault "0 files, 0 notes
indexed in 8 ms". The right half is
`Ln {}, Col {} · parse {:.1} ms, {} spans · shape {:.2} ms · {words} words, {chars} characters`, and
nothing renders a backlink count although `self.linked` is already computed for the right dock.

**Obsidian.** The default status bar is right-aligned only and carries the Backlinks item
("N backlinks") and the Word count item; there is no cursor position and no diagnostics.

**Fix.** Keep only "Ln, Col" (plus the cursor-count clause) and move the parse/spans/shape fragment
behind a debug flag read once at startup. Add an `{n} backlinks` child from `self.linked.len()` left
of the word count, and drop the "· reading view built in N ms" suffix from the status `show_file`
writes, so the left half carries notices only. The word count should also be skipped when no file is
open.

### Renaming with an empty name reports "File name must not start with a dot."

*low, bug, small*

**.** `workspace:edit-file-title`, Ctrl+A, Backspace, Enter: `modal.error` is "File name must
not start with a dot." with an empty query — byte-identical to the pre-campaign observation.

**Obsidian.** "File name cannot be empty." — which 's own New folder prompt already produces,
because that one validates the whole value.

**Fix.** Not in the validator: `validate_file_name` already tests empty before dot and already
returns the right string. The empty value never reaches it, because `prompt_submitted` builds
`format!("{value}.{ext}")` first, so `""` becomes the path `.md`, which is non-empty and
dot-prefixed. Guard the stem in the `PendingOp::Rename` arm before the extension is appended — the
same guard the `/`-accepting entry under Files needs — and add it to the `PendingOp::Move` arm too,
where an empty value silently renames the file to itself.

## What the campaign fixed

One line each, with the detail in [[Development#The parity campaign, 2026-10-20]] and the full
write-ups in the wave reports. Everything here was re-driven against the rebuilt binary by one of
the two verification fleets or by the triage of the second audit pass, except where a line says
otherwise.

**Files view and file operations.** A file deleted outside the app is no longer recreated by the
next autosave — the only `fs::write` is now behind a verified-unchanged read, and the open note
detaches with its buffer kept and marked. Typing into a deleted note's buffer no longer vanishes
silently; the editor is gone and the main area paints a placeholder. The explorer collates
naturally, so `1, 2, 10, a, B, Note 1, Note 2, Note 10` is exactly Obsidian's order. The rename
prompt preselects the name it is pre-filled with, and Ctrl+A in it selects all.

**Live Preview.** A click in the left margin or past the end of a line no longer toggles a checkbox
or follows a link, and the caret lands after the construct rather than inside it. Callouts render as
callout widgets and their chevron folds. `[[Note#Heading]]` reads `Note > Heading`. A closed ATX
heading drops its trailing hashes, and a setext underline is hidden at body size — the second of
those needed a new parser span, because the display layer provably could not tell the title row from
the underline. A padded task renders one checkbox instead of a bullet and a checkbox. The empty
callout title no longer leaks a heap string on every shape (unverified live; a leak needs RSS over
many frames). List-typed properties are pills, one editable item each, with a per-pill remove, and
an explicitly empty `key: []` row takes a first item instead of swallowing the click.

**Reading view.** `propertiesInDocument: hidden` is honoured — the block occupies zero height, where
it used to occupy 130 px. Task checkboxes toggle, byte-exactly and in both directions, writing
through the editor's buffer by line and column so a BOM or CRLF file cannot be corrupted. A note
deleted on disk no longer stays rendered, and an unsupported file type no longer leaves the previous
note live underneath the placeholder. The code block's permanent language label is gone and a hover
copy button took its place — see [[#Blocked by the harness, not by the code]]. A completed task is
struck through as well as muted.

**The five large builds, 2026-10-20.** Written up in
[[Development#Tabs, the layout file, the ribbon, the Hotkeys tab and the graph panel, 2026-10-20]];
each was driven over the socket on a scratch copy of the docs vault before it was committed. The
main area has tabs: a strip above the view header, `workspace:new-tab`, `close`, `close-others`,
`next-tab`, `previous-tab`, `goto-tab-1..8`, `goto-last-tab` and `undo-close-pane` runnable with
their registry hotkeys, Ctrl+click on an explorer row, a reading-view link or an editor link opening
a new tab, `editor:open-link-in-new-leaf` doing the same from the cursor, "Create new note" going to
a new tab unless the active one is empty, and "Default view for new tabs" honoured by every new tab,
which closes that entry on the behaviour rather than the tag. `workspace.json` is read at launch —
the leaves under `main` become tabs in their recorded modes, the `active` id picks the tab, each
sidebar's `collapsed` and `currentTab` are honoured, and with a layout read the startup flags no
longer pick a first note — and written on quit only when the session changed it, keeping every key
Obsidian gave a restored leaf; a main area with more than one tab group is left alone rather than
flattened on disk. A ribbon runs down the left edge with the switcher, graph, daily note, template
and palette, honouring `left-ribbon.hiddenItems`, and each sidebar has a toggle that
outlives its collapse. The left sidebar ends in Obsidian 1.14's vault profile (2026-10-21): the
vault's name behind a `chevrons-up-down` switcher, Help and the Settings gear, which is where the
gear lives now rather than at the foot of the ribbon; the switcher drops up a menu of every vault
this machine has opened (kept in `.json` outside the vaults, Obsidian's `obsidian.json` shape)
with "Open vault..." and "Open sandbox vault" under a hairline, switching puts the open vault away as
quitting would before the next restores its own layout, and Help opens a window of Obsidian's shape
(mark, name, version, four rows with buttons) rather than the browser. The Hotkeys tab binds a chord from a key capture, removes a chip, restores a
default and marks duplicates; `hotkeys.json` is written whole in Obsidian's `{modifiers, key}`
shape, and the registry and every key binding are rebuilt live. The graph has Obsidian's four-section
controls panel with timelapse, restore and close, writing `graph.json` a third of a second after a
change — the twenty-key document when the vault has none, the changed keys merged in when it does —
and draws arrowheads when `showArrow` is on. Along the way: the window title names the note and the
vault, and opening a note from the graph shows the note.

**Right dock panes and graph.** The layout simulation no longer dies when a rebuild lands while it
is running. Deleting the open note clears the Outline, Backlinks and Outgoing rows and re-centres
the graph. Links written in properties now reach both panes, with the property name as the row's
context. The `scale` saved in `graph.json` survives the auto-fit, and Ctrl+0 now outlives the next
vault change. The Orphans filter drops unlinked attachments too. The Outline rebuilds from the
buffer as you type, so its offsets match what is on screen.

**Search, switcher, palette and autocompletion.** Ctrl+Shift+F with the dock hidden or on another
tab no longer kills the keyboard. A workspace hotkey pressed over a modal no longer runs behind it.
A bare term matches the file name, highlighted in the result title and counted as a result. A
half-typed query — `(`, `"`, `/`, `file:`, `((()))` — no longer lists the whole vault. Collapsed
result rows survive a vault change. Ctrl+N and Ctrl+P no longer steal the link suggester's keys.
Results collate numerically and locale-aware. Every text input gained selection, word motion,
double- and triple-click, drag-select and the clipboard bindings. **And the app no longer dies**
when a search term sits behind a multibyte whitespace — see the wave-8 paragraph below.

**Settings.** Clicks, drags and the wheel no longer pass through the modal into the note behind it,
and global hotkeys no longer fire through it. Enter or Escape in a text field no longer strands
focus on an unrendered input. A field commits on blur, a tab switch, a backdrop click and the × as
well as on Enter — and a toggle clicked while a field is open no longer loses one of the two changes
to a stale snapshot. An open dropdown is dismissed by a click elsewhere and does not survive
reopening the modal. The two auto-pair rows no longer claim  ignores them.

**Source editor.** Down no longer sticks at the first wrap boundary, and Home and End no longer sit
one visual row out. Clicks in the find panel no longer fall through to the note, and the Replace
button replaces. Ctrl+H opens on the query field, Tab moves between query and replacement, and both
fields select their contents on open. `editor:insert-table` writes a block of its own. Auto-pairing
implements exactly the probed rows, wrapping a selection instead of destroying it. Twenty-seven
command ids landed — the seven formatting toggles, the checkbox toggle, heading 0..6,
bullet/numbered/blockquote, indent/unindent, delete paragraph, move line, insert link, wikilink and
embed, follow link — each driven over the 1000-note corpus and the real vault by the Writer gate; of
100 `editor:*` ids, 50 are runnable where 23 were. Shift+Tab outdents by `tabSize`. Status-bar
notices expire. **And Backspace and the arrow keys no longer split a grapheme cluster** — see below.

**Workspace shell.** The two keyboard-death traps are gone, and so is the one the socket could reach
(`focus` now refuses a view the main area is not painting, naming what is painted). History no
longer strands `current` on a missing file; a deleted note is pruned from it outright. A deleted
note is not recreated by the next keystroke. A detached buffer says so, reattaches when the file
comes back, and says out loud when its edits are discarded. Opening a canvas, an image or an
unreadable file no longer leaves the previous note's buffer under the new name. Notices are a
bottom-right toast stack that expires by id. **Both** routes into the typeable empty state are now
closed: deleting the open note was fixed in wave 1, and launching on a vault with nothing to open —
an empty vault, or one holding only a canvas — in wave 8.

### Wave 8: a crash and a corruption

Three things landed after the triage of the second audit pass, all verified live against the
rebuilt binary. They are listed apart because the first was filed by neither audit, and the other
two only by the pass the campaign never read.

- **The search panic.** A search term more than 40 bytes into a line, with a multibyte whitespace
  (U+00A0, U+2009, U+3000 — ordinary in pasted text and in CJK notes) as the first whitespace before
  it, sliced the snippet at a non-character boundary and panicked. The panic was inside the
  background `run_search` task and took the main thread with it (`Task polled after completion`), so
  the window vanished and any unsaved buffer went with it. The windowing moved out of the gpui crate
  into a pure `-search::snippet` with `floor`/`ceil` helpers that snap every window edge — and
  every hit range — to a character boundary, with the live repro written as a unit test before the
  fix. A sibling of the same class in `fuzzy.rs`, where `simple_search` mapped lowercased byte
  offsets back by counting characters, now carries a per-byte source map instead.
- **Grapheme-aware editing.** Backspace and the arrow keys stepped by `char`, so one Backspace left
  a bare `e` where `é` had been, split 🇩🇪 into a lone regional indicator, or left a dangling
  zero-width joiner at the end of a family emoji — and the autosave wrote the damage to the vault.
  `-text` now steps by grapheme cluster for Backspace, forward Delete, Left/Right, the
  shifted variants and word motion, and the auto-pair Backspace rule only treats a single-character
  cluster as half of a pair. Vertical motion and mouse clicks still land by pixel, so a click can
  still put the caret mid-cluster; that is in `editor.rs` and is the handoff.
- **The empty state on launch**, above.

The final gate: 501 tests pass, clippy is clean across the workspace, and the real-vault Writer gate
still round-trips 14/14 byte-identical.

## Declined until Obsidian is probed

These are not deferred for effort. Each one would write bytes into a user's vault, or register a
command, on a rule nobody has observed in a real Obsidian — which the clean-room rule forbids. They
are a to-do list for `tools/obsidian-probe/`: each line says what to run and what to read back.
Everything here is cheap once the harness is up, and each probe closes a fix that is otherwise
written and tested.

- **`editor:set-heading` ("Toggle heading", no level).** `set-heading-0` through `-6` are wired and
  have unambiguous semantics; the bare toggle's cycle is not recorded anywhere, and the id is in
  `names.txt`, so it is a real palette entry in 1.14.4. Probe: run it on a plain line, on `#`
  through `######`, and on a line inside a blockquote, reading the bytes back after each, until the
  cycle closes.
- **`editor:cycle-list-checklist` ("Cycle bullet/checkbox").** Specification.md:695-705 says in
  terms that what this does is unprobed and suspects it is the path to `[?]`, `[-]` and `[>]`.
  Probe: run it repeatedly on a plain line, a bullet, `- [ ]` and `- [x]`, reading the bytes after
  each press, and note whether the cycle differs inside a numbered list.
- **Mod+L on a line with no checkbox.** `editor:toggle-checklist-status` flips an existing checkbox
  by the observed rule (anything but a space becomes a space) and leaves every other line alone.
  Both triage passes confirmed that is deliberate, not a defect, but it is still an open question.
  Probe: press it on a plain paragraph line, on a bullet, and on a multi-line selection of mixed
  states, and read whether Obsidian inserts `- [ ] ` and what it does to the mixed selection.
- **The insert-block commands.** `editor:insert-callout`, `insert-codeblock`,
  `insert-horizontal-rule`, `insert-footnote`, `insert-mathblock` and `insert-tag` are all in
  `names.txt` — real Obsidian palette entries — but each authors new markup and the exact bytes
  (fence shape, callout type, how a footnote reference and definition are paired, how many blank
  lines either side) are unobserved. Probe: run each with the caret on a blank line, mid-paragraph
  and at end of file, reading the bytes and the resulting caret offset.
- **`editor:clear-formatting` and the six `editor:toggle-highlight-<colour>` ids.** Also all in
  `names.txt`, so the ids are observed and only the markup is not: the colour variants write some
  attribute beside `==`, and clear-formatting has to strip several markers at once. Probe: run one
  colour variant over a plain word, and clear-formatting over a span carrying two markers.
- **Alt+Up / Alt+Down for Move line.** The commands are implemented and runnable; the registry gives
  them no default hotkey and nothing records one in 1.14.4, so leaving them unbound is the current
  reading of correct parity. Probe: read the Hotkeys tab entry for Move line up. If a binding is
  there, one line in `default_hotkeys` finishes it.
- **The markers the formatting toggles write across a multi-line selection.**  wraps
  `editor:toggle-code` in a single backtick and `toggle-inline-math` in one `$`, which is right
  inline and probably wrong across lines; `toggle-comments` may have a block form. Also unprobed:
  the mixed-selection rule for the list and quote toggles ( removes only when every line
  already has the prefix), numbering an ordered toggle from 1, and skipping blank lines.
- **Auto-pairing's unprobed rows.** [[Specification#Observed auto-pair behavior, Obsidian 1.14.4]]
  lists its own gaps and  left every one of them inert: skip-over for the closers the probe
  did not run, a word-boundary rule for `_ ~ = $ % <`, suppression inside inline code and
  blockquotes, forward Delete between an empty pair, and whether typing `[[` auto-pairs at all.
- **Shift+Tab on ragged indentation.**  removes `min(tabSize, leading spaces)`; CodeMirror
  dedents to the previous multiple. Probe: Shift+Tab on a three-space-indented item in a tabSize-2
  vault.
- **`editor:insert-table` with the caret on a non-blank line.** The spec records only the blank-line
  case.  writes a blank line before the table and leaves exactly one after it, which is a
  reasoned choice. Probe: run it at the end of and in the middle of a paragraph.
- **The property pills' remaining gestures.** Per-pill remove and the trailing empty input are
  observed; Enter-adds-an-item and Backspace-removes-the-previous-pill are not — Specification.md
  files adding under Unprobed and the reviewer called backspace "pure assumption". Also unprobed:
  whether the remove × is revealed on hover or always drawn, and whether removing the *last* item
  leaves `k: []` or clears the key.
- **Clearing every property.** `markdown:clear-metadata-properties` now runs from reading view and
  turns `---\nalpha: one\nbeta: two\n---\n\nbody\n` into `\nbody\n`, leaving a blank line where the
  block was. Probe: clear all properties in Obsidian and read the first byte of the file.
- **The callout fold chevron in Live Preview.** 's fold is ephemeral, copied from its own
  reading view. Nobody has recorded whether Obsidian's chevron is ephemeral too or rewrites the
  source marker (`[!tip]-` ↔ `[!tip]+`). Probe: fold a callout in Live Preview and read the note's
  bytes.
- **The setext underline row in Live Preview.** The corpus proves only that the *reading* view drops
  it. Hiding it, demoting it to body size and the old full-size H1 are all guesses. Probe: open a
  setext heading in Live Preview and measure the underline row.
- **Callout type icons.** Every `callout-icon` div in the corpus holds an empty `<svg>`, so the
  lucide icon per canonical callout type is unknown. Probe: read the icon name out of a rendered
  callout of each built-in type.
- **The done-task decoration.** The corpus proves only `task-list-item is-checked`; the strikethrough
   now draws comes from this note's own earlier prescription, not from an observation. Probe:
  read the computed style of a checked task's text.
- **The empty-state tab.** What Obsidian's "No file is open" tab holds — the four action links, the
  absent mode switch, the hidden word count — comes from a paraphrase of the help site and appears
  nowhere in the spec or the probe results, which is why wave 8 closed the data loss and left the UI
  alone. Probe: open a vault with nothing in it and read the empty tab's DOM.
- **The view-header title.** Clicking it to rename is three lines of work, but Specification.md
  :2026-2028 leaves Obsidian's plain-click-on-header behaviour unsettled. Probe: single- and
  double-click the header title of an open note and read what, if anything, becomes editable.
- **The attachment-location dropdown.** [[Interaction probes]] records the exact option labels and
  the strings they store as an unperformed probe, so the row keeps its "not in  yet" tag until
  someone reads them.
- **`graph:open-local` with no file open.**  now shows the global graph, which is defensible;
  whether Obsidian offers the command at all with no active file (a `checkCallback`) is unobserved.
- **The local graph's camera.** Specification.md:2195-2201 records Obsidian opening a local graph at
  its own `scale: 1` while `graph.json` held 8;  keeps the global graph's pinned zoom. One
  probe settles whether that is a finding.
- **A half-typed query that mixes a real term with an empty one** (`foo OR`, `foo (`).  gates
  only a wholly empty query, because making an empty term fail to match inside the matcher would
  also re-filter and re-colour graphs built from Obsidian's own `graph.json`.
- **The one unexplained row in the search collation capture.** Obsidian put `notes/apple` ahead of
  `apple.md` on a basename tie; the path tie-break puts `apple.md` first, and the fixture leaves
  that pair out rather than faking it. (The sibling question — whether a name-only hit shows a `1`
  flair — is now settled against the backlog's old prediction: Specification.md:2329-2333 says
  filename hits count as results without adding to the file's flair, which is what  does.)
- **Ctrl+E inside a text input.** It is End today, the emacs binding every field already has, so
  Ctrl+E cannot toggle reading view while a field has focus. Nothing records what Obsidian does.
- **The explorer's context menu and the modal backdrop.** Both still differ from Obsidian in the
  code, neither is observable through `state`, and the Obsidian menu is still an open probe in
  [[Interaction probes]].

One group is declined for a different reason and needs no probe: `editor:undo`, `redo`, `copy`,
`cut`, `paste`, `go-start`, `go-end` and the `move-caret-*` ids are absent from `names.txt`, i.e.
Obsidian does not register them on desktop. Both triage passes checked and agreed. Adding them to
RUNNABLE would create palette rows Obsidian has no command for, shown as raw ids, and would desync
the Hotkeys tab — which is also why fifteen ids still report a name equal to their id. The same
held for `open-link-in-new-leaf`, `-split` and `-window` until tabs existed; `-leaf` is runnable now
that they do, while `-split` and `-window` still wait on splits and windows.

## Blocked by the harness, not by the code

Five things could not be settled from outside the process. They are not findings and not fixes —
they are measurements the socket cannot make, and each needs a human at the desk or a change to the
state dump.

**The clipboard.** gpui's `write_to_clipboard` is a no-op in a socket-driven session: a Wayland
client cannot set or receive a selection without keyboard focus and a real input serial, and
synthetic events carry none. The control is the editor's own pre-existing copy — Ctrl+A then Ctrl+C
then Ctrl+V left the note byte-identical, and `wl-paste` never saw anything the app wrote.
Confirmed again by the second pass's source triage, which verified Ctrl+A, Shift+arrows and Tab in
the find fields directly and had to mark Ctrl+V fixed on the code alone. So two things are
implemented, reviewed and unconfirmed: the reading view's code-block copy button (roughly 1500
hover-then-click attempts over its computed box produced no clipboard change, with the button's
geometry verified independently in the same session) and the clipboard round trip in every text
input. Worth checking with a real mouse: gpui's `Interactivity::paint` returns early on
`Visibility::Hidden` *before* it paints mouse listeners, so an `.invisible()` button has no click
listener in any frame where its group is not hovered.

**Drag-select.** `control.rs`'s `mouse` action always sends `MouseMoveEvent { pressed_button: None }`,
so down/move/move/up moves the caret and selects nothing. The drag-select added to `TextInput` is
therefore unverified, although single, double, triple and shift-click all verified.

**Rendered rows in the Settings modal.** `state.modal` reports the tab, the open dropdown by config
key, the open text field by config key and the hotkey filter — enough to settle most of the
subsystem by clicking a coordinate grid and watching for a state change or a byte change in
`.obsidian/*.json` — but `modal.rows` is empty for this modal, so no row's text can be read. Every
purely visual claim about it (toggle size, slider thumb, the explanatory banner, the "not in 
yet" chips, the uppercase nav headings, the rows Obsidian has that  lacks, the inert buttons)
is unreported in either direction, the Hotkeys rows cannot be read at all, and the plugin tabs'
"Options not shown yet" row had to be settled from the source. Exposing the row names in
`state.modal.rows` would make the whole tab testable.

**The status bar, for five seconds after anything.** The notice layer means `state.status` returns
the newest live notice in preference to the real status, so any finding whose evidence is a
status-bar string is unreadable for five seconds after any notice fires. The way around it, which is
how the autosave-reveal entry was settled: set a sentinel from a non-notice path (touch an unrelated
file, so `after_changes` writes "<path> changed"), let the notices drain, then act and re-read.

**The window title.** The socket's `window` method answers width, height, scale and active, with no
title field, and the machine has no `wmctrl`, `xdotool` or `xprop`. Adding `title` to that method is
one line and would close the last could-not-test in the shell subsystem.
