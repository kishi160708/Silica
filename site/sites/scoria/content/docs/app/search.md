---
title: Search
description: Vault-wide search in  — the query grammar, how results are grouped and read, explaining a query, and what is not implemented yet.
---

# Search

`Ctrl + Shift + F` (`global-search:open`) opens the Search tab in the left dock. It searches
every note in the vault, with the same query language Obsidian uses.

## A query

The simplest query is a word, and several words mean all of them:

```
focus stacking          both words, anywhere in the note
"focus stacking"        that exact phrase
focus OR stacking       either one
focus -stacking         "focus" but not "stacking"
tag:#project            notes tagged #project, or any child of it
path:archive/           notes under archive/
/colou?r/i              a regular expression
```

The full grammar — every operator, how precedence works, the `[property]` forms — is the
[search syntax reference](/docs/reference/search-syntax/). It is worth a read: the scoped
operators (`line:`, `block:`, `section:`, `task:`) are where the grammar earns its keep,
because they let you ask for two words *in the same line* rather than merely in the same
note.

## Reading the results

Results are grouped per file. Each file shows the matching lines with their context, and the
matched text is highlighted inside them. Click a result and the note opens at that line.

Matching is case-insensitive unless you ask otherwise with `match-case:`. Searching runs on
a background thread across the indexed note text, so a large vault stays responsive while
you type.

## Explaining a query

A query that returns the wrong thing is usually a precedence surprise — `OR` binds looser
than the implicit `AND`, so `a b OR c` is `(a AND b) OR c`.  can print the parse tree
with the same labels Obsidian's *Explain search terms* shows, so you can see how your query
was actually read.

## Searching for a tag by clicking

Clicking a tag in a note, or in the [Tags pane](/docs/app/panes/), searches the vault for
it.

## Not yet

- **Sort options** for results (by name, by modified time, by relevance).
- **Copy search results.**
- **Collapsing** a file's results.
- **Search and replace across the vault.** Replace works inside one note — see [find and
  replace](/docs/app/find/).
- **Saved searches.**

## Next

- [Search syntax](/docs/reference/search-syntax/) — the whole grammar.
- [Switcher and palette](/docs/app/switcher/) — for finding a note by name instead.
