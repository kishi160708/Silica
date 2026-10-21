---
title: Backlinks and the side panes
description: The right dock — Backlinks with linked and unlinked mentions, Outgoing links with unresolved ones, the Outline of headings, and the vault-wide Tags pane.
---

# Backlinks and the side panes

The right dock holds four tabs. Three are about the note you are reading; Tags is about the
whole vault. `app:toggle-right-sidebar` hides and shows the dock, and each tab has a command
of its own so you can bind a key straight to it.

## Backlinks

`backlink:open`, `backlink:open-backlinks`

Every note that points at the one you are reading, in two groups.

**Linked mentions** are real links. Each row shows the linking note and the line the link
sits on, so you can see the sentence it was used in. A note that links here three times
appears three times — backlinks are per occurrence, not per note.

**Unlinked mentions** are places where this note's name — or any of its aliases — appears as
plain text that could have been a link but is not. The matching is deliberately narrow, so
the pane stays useful rather than noisy:

- Whole words only, compared case-insensitively.
- Names *and* aliases from the `aliases` property.
- When two notes' names overlap, the longest name wins.
- Never inside an existing `[[wikilink]]` or `[Markdown](link)`.
- Never inside inline code or a fenced code block.

## Outgoing links

`outgoing-links:open`, `outgoing-links:open-for-current`

Where this note points. Resolved links — the ones that find a file — and unresolved links,
which are the `[[Names]]` of notes that do not exist yet. Clicking an unresolved one creates
the note, as clicking the link in the note itself would.

This pane also lists unlinked mentions *of other notes* found in this note, which is the
other direction of the same idea: notes you have named in passing and could link to.

> **Note** Links in your frontmatter count as links, in both panes and in the graph. A
> `source: "[[Paper]]"` property is a link to *Paper*.

## Outline

`outline:open`, `outline:open-for-current`

The note's headings, indented by level. Click one to scroll the note to it. Works in reading
view and while editing.

## Tags

`tag-pane:open`

Every tag in the vault with the number of notes carrying it. Counts merge the two ways a
note can be tagged, case-insensitively:

- Inline `#tags` in the body.
- The `tags` property in frontmatter — whether written as a YAML list or as a comma-separated
  string, with or without the leading `#`.

So `#Project`, `#project` and `tags: [project]` are one tag with a count of three.

Nested tags like `#project/` are listed as themselves; a `tag:#project` search also
matches them, because a search for a parent tag matches its children. See [search
syntax](/docs/reference/search-syntax/).

## Keeping up with the vault

All four panes follow the vault. A note saved in another window, a file added by git, a
folder dropped in — the panes refresh within a moment, and so do the resolved and unresolved
link counts, because a note appearing can make a link elsewhere resolve.

## Not yet

- Sorting and grouping options in the panes.
- Collapsing a backlink group.
- A local-graph side pane (the [graph](/docs/app/graph/) has a local mode of its own).

## Next

- [Search](/docs/app/search/)
- [Internal links](/docs/format/links/) — how link resolution actually decides.
