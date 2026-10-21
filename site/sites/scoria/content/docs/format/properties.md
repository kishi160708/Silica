---
title: Properties and tags
description: YAML frontmatter and tags in  — where frontmatter is allowed, how aliases work, the two ways to tag a note, and the surprisingly precise rules for what counts as a tag.
---

# Properties and tags

## Frontmatter

A note can open with a YAML block, which Obsidian calls *properties*:

```markdown
---
aliases:
  - JSON spec
  - RFC 8259
tags:
  - reference
  - standards
source: "[[Paper]]"
created: 2026-10-17
done: true
---

# The note starts here
```

**Frontmatter is only recognized on line 1.** A `---` block further down the note is a
horizontal rule and a paragraph, not properties. A byte-order mark before it is tolerated,
and so are CRLF line endings.

In reading view, frontmatter renders as a properties block at the top of the note. 
reads the whole block, and the keys it gives special meaning to are:

| Key | Meaning |
|---|---|
| `aliases` | Other names for this note. Used by the [quick switcher](/docs/app/switcher/), by [unlinked mentions](/docs/app/panes/), and resolvable by `[[alias]]` |
| `tags` | Tags for this note (see below) |

Every other key is kept and available to [property searches](/docs/reference/search-syntax/)
— `[status:done]`, `[created:>2026-01-01]`.

**Links in properties are real links.** `source: "[[Paper]]"` produces a backlink in *Paper*
and an edge in the graph. When *Paper* is renamed,  rewrites the link text in place
and leaves the rest of your YAML bytes untouched.

> **Note** Properties are read-only as text today: there is no property editor, so you edit
> frontmatter as YAML in the note. That is on the writer phase's list.

## Tags

A tag can be written two ways, and they mean the same thing:

```markdown
---
tags:
  - project/
  - reference
---

Inline, anywhere in the body: #project/ #reference
```

In the `tags` property, the `#` is optional, and the value may be a YAML list **or** a
comma-separated string (`tags: project, reference`). Both shapes are accepted.

The [Tags pane](/docs/app/panes/) merges the two sources **case-insensitively**, so
`#Project`, `#project` and `tags: [project]` are one tag with a count of three.

### What counts as a tag

The rules are precise, because `#` is also a heading marker and appears in URLs. 
matches Obsidian's observed behavior:

- A tag needs **at least one non-numeric character**. `#2026` is not a tag; `#q4-2026` is.
- **`# Heading` is not a tag** — the space settles it.
- A `#` **after an alphanumeric character is not a tag**: the `#section` in
  `page#section` is part of the word.
- A `#` right after **whitespace** starts a tag. So does a `#` right after **another inline
  construct**: `[[link]]#tag`, `` `code`#tag ``, `**bold**#tag`, and `#a#b` all produce
  tags.
- A `#` after `(` does **not** start a tag.
- Tags may contain **emoji and any non-ASCII letters**.

### Nested tags

`#project/` is a nested tag. It is listed as itself in the Tags pane, and a search for
`tag:#project` matches it, because a search for a parent tag matches its children. See
[search syntax](/docs/reference/search-syntax/).

## Next

- [Block ids and footnotes](/docs/format/blocks/)
