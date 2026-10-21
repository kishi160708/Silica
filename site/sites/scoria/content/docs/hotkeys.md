---
title: Hotkeys
description: How  applies your vault's hotkeys.json — rebinding, unbinding, how a binding becomes a label, the Hotkeys tab's filters, and what happens to bindings for commands  cannot run yet.
---

# Hotkeys

Every shortcut in  is an Obsidian command bound from its hotkey. There are no
hard-coded shortcuts in the shell — the one exception is `Ctrl + Q`, which quits.

The practical consequence: **your vault's `hotkeys.json` applies unchanged.** If you have
spent years rebinding Obsidian to your liking,  opens with your keys.

## How a binding is decided

For each command, in order:

1. If `hotkeys.json` has an **entry** for the command, that entry *replaces* the defaults.
2. An entry with an **empty list** unbinds the command entirely.
3. Otherwise the command keeps **Obsidian's default** binding, if it has one.

This is Obsidian's own rule, including the detail that an entry replaces rather than adds to
the defaults.

`hotkeys.json` is read **before the window opens**, so the first frame already has your
keys.

## The shape of the file

```json
{
  "editor:toggle-source": [{ "modifiers": ["Mod", "Shift"], "key": "S" }],
  "graph:open": []
}
```

`Mod` means Ctrl on Windows and Linux and Cmd on macOS. `Ctrl` means Ctrl everywhere,
including macOS — which is why Obsidian's own default for *Next tab* is `Ctrl + Tab` rather
than `Mod + Tab`.

An empty list, as on `graph:open` above, unbinds the command.

## Labels

A binding is shown the way Obsidian shows it: `Ctrl + Shift + F`, `Ctrl + Alt + ←`.
Modifiers are in Obsidian's order, arrow keys are arrows, and a single letter is
capitalised.

## The Hotkeys tab

Settings' Hotkeys tab lists every command id  knows — all 244 of them — with its name
and its effective binding. Four filters, as in Obsidian:

| Filter | Shows |
|---|---|
| All | Every command |
| Assigned | Commands that have a binding |
| Unassigned | Commands with no binding |
| Assigned by me | Commands your `hotkeys.json` has an entry for |

A command whose binding came from your file rather than from the defaults is marked as
custom.

> **Note** The tab lists every command, not only the ones  can run. That is on purpose:
> your bindings for features that are not implemented yet are real configuration, and seeing
> them here is how you know they survived. The [command
> palette](/docs/app/switcher/), by contrast, offers only commands that actually do
> something.

## Changing a binding

Not from inside  — [Settings is read-only](/docs/settings/) for now. Rebind in
Obsidian, or edit `.obsidian/hotkeys.json` and reopen the vault.

## Next

- [Commands and hotkeys](/docs/reference/commands/) — the ids, the names and the defaults.
- [Search syntax](/docs/reference/search-syntax/)
