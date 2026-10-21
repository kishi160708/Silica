---
title: Coming from Obsidian
description: What carries over from Obsidian to , what you will miss, how to try  on a vault you care about without risking it, and how to run both apps on the same folder.
---

# Coming from Obsidian

There is no migration.  opens the folder Obsidian already opens, reads the same
`.obsidian` configuration, and leaves everything where it is. You can open a vault in
 this afternoon and in Obsidian this evening, and neither will notice.

What follows is what to expect.

## What carries over, with no work

| Your | In  |
|---|---|
| Notes | Read as they are. Same files, same names, same folders |
| Wikilinks, embeds, aliases | Resolved by Obsidian's rules, including the ambiguous-basename order |
| Tags | Merged from inline `#tags` and the `tags` property, case-insensitively |
| Frontmatter | Parsed, shown as properties in reading view, and its links counted as links |
| Attachments | Images, audio, video and PDFs are recognized and shown in Files |
| `hotkeys.json` | Applied command by command. Your bindings are your bindings |
| `appearance.json` | Base theme, accent color, fonts and font size are read before the window opens |
| `graph.json` | Filters, color groups, display settings and forces drive the graph view |
| `core-plugins.json`, `community-plugins.json` | Read and shown in Settings (community plugins are listed, not run) |
| `app.json` settings | The editor and files settings are honored — see [settings](/docs/settings/) |
| Canvas and Bases files | Recognized, parsed and round-tripped on disk, but **not drawn yet** |

## What you will miss

Be realistic about this list before you make  your daily reader.

- **Community plugins.** They are JavaScript against an Electron app;  is a native
  binary. They cannot run, and making them run is out of scope. If your workflow rests on
  Dataview or Templater,  is not yet a place you can work.
- **Themes.** `appearance.json`'s base theme and accent color are honored, but a CSS theme
  is not. A variable-level subset of Obsidian's CSS custom properties is a later phase.
- **Rendered math.** `$x$` and `$$x$$` are parsed and show as their source text, not as
  typeset math.
- **HTML in notes.** Shown as text rather than rendered.
- **Remote images.** A `![](https://…)` does not load; local images do.
- **Canvas and Bases views.** The formats are implemented, the views are not.
- **Changing settings in the app.** Settings shows your real configuration and is read-only.
- **Tabs and split panes.** One note at a time, with back and forward history.
- **Multiple vaults in one window**, PDF export, heading folding, drag-and-drop in Files,
  inline title editing, pasting attachments.

The full list, kept honest, is [what differs from
Obsidian](/docs/reference/differences/).

## Trying it without risk

The safe order, in increasing commitment:

1. **Read-only, on your real vault.** Open it in  and browse. Reading writes nothing
   at all — not a cache, not a settings file, not `workspace.json`. There is no risk here
   beyond the program crashing, which loses nothing.

2. **Edit a scratch copy.** Copy a dozen notes into a new folder, including a couple with
   frontmatter, tables and CRLF line endings if you have them. Edit them in , then
   open the folder in Obsidian and look for anything that changed that you did not change.
   That is exactly the round-trip check 's own phase-3 gate makes.

3. **Edit for real, with a backup.** A git repository in the vault is the ideal form of
   this: `git diff` after an editing session tells you precisely what  wrote, which is
   both a safety net and the most useful bug report you can send.

> **Warning** Until the round-trip gate is met, treat  as a reader on a vault you
> cannot afford to lose, and as an editor on a vault you can.

## Running both on one folder

This works, and it is how the project itself is developed — 's own specification
vault is edited in both apps. Two things to know:

- ** watches the folder.** A save in Obsidian re-indexes in  within a moment,
  and the open note reloads keeping its scroll position. You do not need to reopen anything.
- **Do not have the same note unsaved in both.** Obsidian and  both autosave, and
  whichever writes last wins.  will not overwrite a note that changed on disk while
  you have unsaved edits in it — it leaves your buffer alone — but then you have two
  versions and have to pick. The habit that avoids it is the ordinary one: edit a note in
  one place at a time.

Running both at once is also the best way to find bugs. If the two apps disagree about how
a note renders or where a link points, that is a real defect and worth
[reporting](https://github.com/lw-xiong//issues).

## Things that will feel different

Not bugs, just differences you will notice in the first ten minutes:

- **It starts immediately.** There is no splash and no window-restore step, because the
  layout is not restored —  opens its own default three-pane shell every time.
- **One window, one vault.** Run it twice for two vaults.
- **There are no tabs.** One note is open at a time, and `Ctrl + Alt + ←` / `→` walk the
  history of what you have opened. Split panes, stacked tabs and popout windows do not
  exist here.
- **Settings is a window you read.** Every value is your real value, pulled from
  `.obsidian`; none of them are editable yet. To change a setting, change it in Obsidian or
  edit the JSON.

## Next

- [The window](/docs/app/window/) — a tour of what you are looking at.
- [Reading view](/docs/app/reading-view/) — what is drawn and what is not.
- [Obsidian compatibility](/docs/reference/compatibility/) — the tiers and the numbers.
