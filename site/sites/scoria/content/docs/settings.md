---
title: Settings
description: The Settings window in  — which tabs it shows, where every value comes from in your .obsidian folder, which settings actually take effect, and why it is read-only for now.
---

# Settings

`Ctrl + ,` (`app:open-settings`) opens Settings. Every value in it is read from your vault's
`.obsidian` folder, so what you see is your real configuration rather than a set of 
defaults.

> **Warning** **Settings is read-only.** A banner at the top of the window says so. Nothing
> you see can be changed from inside ; to change a setting, change it in Obsidian or
> edit the JSON in `.obsidian/` directly. This is deliberate: Obsidian rewrites an entire
> settings file on every change, and doing that safely — without dropping keys  does
> not know about — belongs with the round-trip gate rather than ahead of it.

## The tabs

| Tab | Read from |
|---|---|
| General | `app.json` |
| Appearance | `appearance.json` |
| Interface | `app.json`, `appearance.json` |
| Editor | `app.json` |
| Files and links | `app.json` |
| [Hotkeys](/docs/hotkeys/) | `hotkeys.json`, over the command registry |
| Core plugins | `core-plugins.json` |
| Community plugins | `community-plugins.json` and each plugin's `manifest.json` |
| Quick switcher | `switcher.json` |
| Command palette | `command-palette.json` |

The last two are plugin tabs, and appear the way Obsidian shows a core plugin's own options
tab.

Each row is drawn as Obsidian draws it: a name, a description, and a control showing the
current state. The layout and the wording follow Obsidian 1.14.4.

## Which settings actually do something

Showing a setting and honoring it are different things, and the distinction matters. These
are the ones that change how  behaves:

| Setting | Key | Effect |
|---|---|---|
| Default view for new tabs | `defaultViewMode` | Whether a note opens in reading view or editing |
| Live Preview | `livePreview` | Which editing mode a note opens in |
| Strict line breaks | `strictLineBreaks` | Whether a single newline is a line break |
| Readable line length | `readableLineLength` | Soft wrap width while editing |
| Indent using tabs / Tab size | `useTab`, `tabSize` | What `Tab` inserts in a list |
| Smart list indentation | `smartIndentList` | Whether Enter continues a list |
| New link format | `newLinkFormat` | The path shape of a rewritten link |
| Use `[[Wikilinks]]` | `useMarkdownLinks` | The form of links  creates |
| Automatically update internal links | `alwaysUpdateLinks` | Whether a rename rewrites links without asking |
| Default location for new notes | `newFileLocation`, `newFileFolderPath` | Where a new note lands |
| Deleted files | `trashOption` | System trash, `.trash/`, or outright |
| Confirm file deletion | `promptDelete` | Whether deleting asks |
| Detect all file extensions | `showUnsupportedFiles` | Whether unknown kinds appear in Files |
| Base theme, accent color | `theme`, `accentColor` | The window's colors |
| Fonts and font size | `interfaceFontFamily`, `textFontFamily`, `monospaceFontFamily`, `baseFontSize` | Typography |

Settings that are **shown but not yet acted on** include spellcheck, auto-pairing of
brackets and Markdown, Vim mode, inline title, fold heading and fold indent, line numbers,
indent guides, right-to-left, and `cssTheme`. They are read from your configuration and
displayed truthfully; the behavior behind them is not implemented.

## Core and community plugins

The **Core plugins** tab shows which of Obsidian's core plugins your vault has enabled,
reading both the modern map shape and the legacy array shape of `core-plugins.json`. 
implements the core plugins as part of the application rather than as separate modules, so
the tab is informational: switching one off in Obsidian does not currently remove the
corresponding pane from .

The **Community plugins** tab lists your installed plugins, read from
`community-plugins.json` and each plugin's manifest — name, version, author, description.

> **Note** Community plugins are **listed, not run**. They are JavaScript written against an
> Electron application;  is a native binary and cannot execute them. A native plugin
> interface of 's own is the last planned phase. See [Obsidian
> compatibility](/docs/reference/compatibility/).

## Not shown

Some settings are absent rather than disabled, because the specification does not yet record
the `app.json` key behind them — and inventing a key would be worse than omitting the row.
These include: the default file to open, deleting attachments with their note, Mermaid
support, quick font-size adjustment, zoom level, window frame style, and URI callbacks.
Obsidian's Keychain tab, and the options tabs of core plugins other than the quick switcher
and the command palette, are also not shown.

## Unknown keys are safe

Every config reader carries unrecognised keys through untouched. A setting written by a
newer Obsidian than the specification covers is read, kept, and still there afterwards —
which is only meaningful because  does not rewrite these files at all today, and is
the property that has to hold before it does.

## Next

- [Hotkeys](/docs/hotkeys/)
- [Vault files and formats](/docs/reference/formats/)
