---
title: Internal links
description: Wikilinks and Markdown links in  — the full link grammar including headings, block references and aliases, and the exact rule that decides which file an ambiguous link means.
---

# Internal links

A link between notes can be written two ways, and  reads both.

```markdown
[[Note]]                      a wikilink
[[Note|what to show]]         with an alias
[[Note#Heading]]              to a heading
[[Note#Heading#Subheading]]   to a nested heading
[[Note#^block-id]]            to a block
[[folder/Note]]               with a path
[Text](Note.md)               a Markdown link
[Text](<Note with spaces.md>) angle brackets for awkward paths
```

When  **rewrites** an existing link — because you renamed or moved its target — the
link keeps the form you wrote it in, and only its path changes; `newLinkFormat` decides what
shape that path takes. `useMarkdownLinks` is read and governs the form of links 
*creates*. See [managing notes](/docs/app/manage/).

## The wikilink grammar

The body of a wikilink is `path#heading#heading|alias`, or `path#^block-id`. Parsed in that
order: everything before the first `|` is the target, everything after it is the alias.

A couple of rules are worth knowing because they bite:

- **`[[` needs its `]]` on the same line.** A wikilink does not span lines.
- **Wikilinks and embeds are not parsed inside code spans.** `` `[[Note]]` `` is text.
- **`[[Note\|alias]]`** — with a backslash before the pipe, as you must write it inside a
  table — is the alias form: the target is the text before the first `|`, with a trailing
  backslash dropped. Inside a code span, `\|` stays literal.

## How a link finds its file

Most links are unambiguous and this never comes up. When two notes share a basename, the
order matters, and it is not what you would guess — this was wrong in 's own
specification until a probe against a running Obsidian corrected it.

**Extensions.** A link may name the `.md` or leave it off.

**Paths with a prefix are not searched for:**

| Written | Resolved |
|---|---|
| `/path/Note` | From the vault root **only** |
| `./Note`, `../Note` | Relative to the linking note **only** |
| `path/Note` | Root first, then relative to the linking note |

**A bare basename**, where more than one file could match, picks in this order:

1. A file at the **vault root**.
2. A file in the **linking note's own folder**.
3. The **shortest path**.
4. Failing all that, path order.

So `[[Zed]]` written in `Long/Way/Down/C.md` resolves to `Zed.md` at the root — *not* to the
sibling `Long/Way/Down/Zed.md`. But `[[X]]` in the same note resolves to the sibling
`Long/Way/Down/X.md` over `Mid/X.md`, because neither is at the root and the sibling rule
applies next.

Comparisons are **case-insensitive**, while the stored path keeps its original case.

## Links that go nowhere

A `[[Name]]` with no file behind it is an *unresolved link*.  tracks these rather than
discarding them:

- They appear in [Outgoing links](/docs/app/panes/) as unresolved.
- They appear in the [graph](/docs/app/graph/), unless `hideUnresolved` is on.
- They appear in the [quick switcher](/docs/app/switcher/), marked as not created.
- **Clicking one creates the note.** With a folder in the path, the folders are created too.

## Markdown links

`[Text](destination)` is a vault link when the destination has **no URL scheme**; with a
scheme it is an external link. The destination is percent-decoded before resolution, so
`[x](My%20Note.md)` finds `My Note.md`.

Markdown links count as links in the metadata cache, which means they produce backlinks and
graph edges exactly as wikilinks do.

## Links in frontmatter

A link written in a YAML property is a real link:

```yaml
---
source: "[[Paper]]"
related:
  - "[[Other note]]"
---
```

These appear in Outgoing links, produce backlinks in the target, and draw edges in the graph.
When the target is renamed,  rewrites the link text in place and leaves the rest of
the YAML bytes alone — Obsidian re-serialises the whole block, which is a difference in
's favor.

## Display text

| Written | Shown in reading view |
|---|---|
| `[[Note]]` | Note |
| `[[Note\|alias]]` | alias |
| `[[Note#Heading]]` | Note > Heading |
| `[[Note#Heading\|alias]]` | alias, with `Note > Heading` as its accessible label |

## Clicking a link

In reading view, a click follows it. In Live Preview, a click on a rendered link follows it;
in Source mode, `Ctrl + click` does. A `#Heading` or `#^block-id` link opens the note
scrolled to that block.

## Not yet

- Autocompletion when you type `[[`.
- `Ctrl + hover` page previews.

## Next

- [Embedding files](/docs/format/embeds/)
