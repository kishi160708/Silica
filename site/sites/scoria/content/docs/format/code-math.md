---
title: Tables, code and math
description: Tables and the pipe-escaping rule, fenced and inline code, math blocks, and %%comments%% — what  parses, what it renders, and what shows as source.
---

# Tables, code and math

## Tables

```markdown
| Option | Default | What it does      |
|--------|:-------:|------------------:|
| `a`    |   on    | something         |
| `b`    |   off   | something else    |
```

Alignment comes from the delimiter row: `:---` left, `:---:` centre, `---:` right.

**A `|` inside a code span does not split a cell.** This is the GitHub rule and Obsidian
follows it, so you can write `` `a|b` `` in a cell. To write a literal pipe outside code,
escape it: `\|`. That is also why a wikilink alias inside a table is written
`[[Note\|alias]]`.

Three structural rules decide whether you have a table at all:

- The header row must be a **one-line paragraph** followed immediately by the delimiter row.
- The header row **or** the delimiter row must contain a pipe.
- A table **cannot interrupt a longer paragraph**, and a row line with no pipe ends the
  table.

### How columns are sized

In reading view, columns follow HTML's automatic table layout, measured with the real font:
each column wants its widest cell, needs at least its longest unbreakable word, and when the
total does not fit, the slack is shared out in proportion. See [reading
view](/docs/app/reading-view/).

> **Note** Table **cells are not editable** yet. In Live Preview a top-level table renders as
> a widget; put the cursor in it and you edit the Markdown as text. A table editor is on the
> writer phase's list.

## Code

Inline code is backticks: `` `like this` ``. A backtick run with no closer retries one
backtick shorter, so a stray triple backtick does not swallow the rest of the line.

Fenced blocks take a language label:

````markdown
```rust
fn main() {
    println!("hello");
}
```
````

The label is shown on the block in reading view. **Code never wraps** — a long line scrolls
sideways inside its own block, which is almost always what you want for code and is what
Obsidian does.

Nothing inside a code span or a fenced block is parsed as anything else: no wikilinks, no
tags, no math, and a rename never rewrites a link that appears in code.

## Math

```markdown
Inline: $e^{i\pi} = -1$

Block:

$$
\int_0^1 x^2 \, dx = \tfrac{1}{3}
$$
```

> **Warning** Math is **parsed but not typeset**. In reading view you see the formula's
> source text, not rendered mathematics. In Live Preview the surrounding dollar signs are
> hidden. MathJax is a later phase.

The delimiter rules are Obsidian's, and they are stricter than you might expect:

- `$...$` needs **no space** just inside the dollars, and the closing `$` must not be
  followed by a digit. That is what stops `$5 and $7` from becoming math.
- An unterminated `$$` block runs to the **end of the file**.
- `$$x$$` opened and closed on one line stays **inline** inside its paragraph.

## Comments

```markdown
Visible text %%hidden%% more visible text.

%%
A whole block that does not render.
%%
```

`%%…%%` hides text in both reading view and Live Preview.

- An **inline** `%%comment%%` does **not** cross a line break.
- An unterminated `%%` block runs to the end of the file, like `$$`.
- `%%x%%` closed on one line stays inline.

## Next

- [Settings](/docs/settings/)
