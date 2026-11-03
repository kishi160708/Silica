//! Prism's tokenizer, ported.
//!
//! Obsidian highlights a rendered code block with Prism.js, so a theme
//! addresses `.token.keyword` and the token *names* are the compatibility
//! surface (see `docs/silica/Specification.md`). Matching the names without
//! matching the algorithm would not do: Prism's output depends on the order
//! its grammar's keys are declared in, on a greedy pattern's right to reach
//! across tokens already matched, and on the rematch that follows when one
//! does. This is a port of `matchGrammar` from Prism's
//! `components/prism-core.js`, so a grammar ported from the same source
//! produces the same tokens.
//!
//! Two structural differences, neither of which changes the output.
//!
//! Prism splices a doubly linked list; this is a `Vec` with the same splice
//! points, which is an index where Prism has a pointer and is what the rest
//! of the crate wants to walk afterwards anyway.
//!
//! And every node is a **byte range of the original text** rather than a
//! string. Prism only ever stores slices of the text it was given, so no
//! generality is lost, nothing is allocated per token, and the painter gets
//! the offsets it needs without measuring anything back.

use crate::compiled::{compile, Compiled};
use crate::grammar::Grammar;

/// A matched token, or a run of text no pattern claimed.
///
/// `Text` is Prism's bare string node; `Token` is its `Token`, whose
/// content is the tokens an `inside` grammar found, or a single `Text` when
/// the pattern had none. Every offset is into the text passed to
/// [`tokenize`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Text { start: usize, end: usize },
    Token(Token),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The grammar key that matched: `keyword`, `string`, `comment`, …
    pub kind: &'static str,
    /// The pattern's `alias`, which Prism puts in the class list after the
    /// type and which a theme may address instead (`boolean important`).
    pub alias: &'static [&'static str],
    pub start: usize,
    pub end: usize,
    pub content: Vec<Node>,
}

impl Node {
    pub fn start(&self) -> usize {
        match self {
            Node::Text { start, .. } => *start,
            Node::Token(t) => t.start,
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Node::Text { end, .. } => *end,
            Node::Token(t) => t.end,
        }
    }

    /// What Prism's `Token.length` counts: the source this node covers.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn shift(&mut self, by: usize) {
        match self {
            Node::Text { start, end } => {
                *start += by;
                *end += by;
            }
            Node::Token(t) => {
                t.start += by;
                t.end += by;
                for child in &mut t.content {
                    child.shift(by);
                }
            }
        }
    }
}

/// Prism's `_.tokenize(text, grammar)`.
pub fn tokenize(text: &str, grammar: &'static Grammar) -> Vec<Node> {
    tokenize_compiled(text, compile(grammar), 0)
}

fn tokenize_compiled(text: &str, grammar: &'static Compiled, depth: usize) -> Vec<Node> {
    let mut list = vec![Node::Text {
        start: 0,
        end: text.len(),
    }];
    let mut rematch = None;
    match_grammar(text, &mut list, grammar, 0, 0, &mut rematch, depth);
    // An empty source leaves the one empty text node Prism also leaves;
    // dropping it keeps a painter from drawing a zero-width run.
    list.retain(|n| !n.is_empty());
    list
}

/// Prism's `RematchOptions`: the pattern whose match began a nested run,
/// and how far into the text that run has already reached.
struct Rematch {
    cause: (usize, usize),
    reach: usize,
}

/// How deep `inside` grammars may nest before the engine gives up. Prism
/// has no such limit and relies on grammars being well founded; a ported
/// grammar with a cycle would otherwise take the stack with it.
const MAX_DEPTH: usize = 24;

/// Prism's `matchGrammar`.
///
/// `start_ix` is the **first node to look at**, where Prism passes the one
/// before it and immediately takes `startNode.next` — Prism's list has a
/// head sentinel and a `Vec` has not. `start_pos` is that node's offset.
fn match_grammar(
    text: &str,
    list: &mut Vec<Node>,
    grammar: &'static Compiled,
    start_ix: usize,
    start_pos: usize,
    rematch: &mut Option<Rematch>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    for (key_ix, (kind, patterns)) in grammar.entries.iter().enumerate() {
        let kind = *kind;
        for (j, pattern) in patterns.iter().enumerate() {
            // The pattern whose own match began this nested run: stop, or
            // it would match the same thing again for ever.
            if rematch.as_ref().is_some_and(|r| r.cause == (key_ix, j)) {
                return;
            }

            let mut current = start_ix;
            let mut pos = start_pos;
            while current < list.len() {
                if rematch.as_ref().is_some_and(|r| pos >= r.reach) {
                    break;
                }
                // Prism's "something went terribly wrong, ABORT": a grammar
                // that splits without consuming cannot make more nodes than
                // the text has bytes.
                if list.len() > text.len() + 1 {
                    return;
                }
                if matches!(list[current], Node::Token(_)) {
                    pos += list[current].len();
                    current += 1;
                    continue;
                }

                let mut remove_count = 1usize;
                // The slice the match is taken from — this node, or the
                // wider run a greedy match reached across — and where the
                // match sits inside it.
                let slice_start;
                let slice_end;
                let found;

                if pattern.greedy() {
                    // A greedy pattern matches against the whole text from
                    // here, so it may run past this node and over tokens
                    // already made.
                    let Some((from, to)) = pattern.find(text, pos) else {
                        break;
                    };
                    if from >= text.len() {
                        break;
                    }
                    // Walk to the node the match starts in.
                    let mut p = pos + list[current].len();
                    while from >= p {
                        current += 1;
                        if current >= list.len() {
                            return;
                        }
                        p += list[current].len();
                    }
                    p -= list[current].len();
                    pos = p;
                    // The match starts inside a token already made, which
                    // is not a place a new one may begin.
                    if matches!(list[current], Node::Token(_)) {
                        pos += list[current].len();
                        current += 1;
                        continue;
                    }
                    // Walk to the last node the match covers. Prism keeps
                    // taking string nodes even past the match's end, which
                    // is what merges the run back together.
                    let mut k = current;
                    while k < list.len() && (p < to || matches!(list[k], Node::Text { .. })) {
                        remove_count += 1;
                        p += list[k].len();
                        k += 1;
                    }
                    remove_count -= 1;
                    slice_start = pos;
                    slice_end = p;
                    found = Some((from, to));
                } else {
                    // A plain pattern only ever sees this one node.
                    slice_start = list[current].start();
                    slice_end = list[current].end();
                    found = pattern
                        .find(&text[slice_start..slice_end], 0)
                        .map(|(a, b)| (a + slice_start, b + slice_start));
                }

                let Some((from, to)) = found else {
                    pos += list[current].len();
                    current += 1;
                    continue;
                };

                let reach = pos + (slice_end - slice_start);
                if let Some(r) = rematch.as_mut() {
                    if reach > r.reach {
                        r.reach = reach;
                    }
                }

                // Splice `before`, the token and `after` in place of the
                // nodes the match covered.
                let mut fresh: Vec<Node> = Vec::with_capacity(3);
                if from > slice_start {
                    fresh.push(Node::Text {
                        start: slice_start,
                        end: from,
                    });
                    pos += from - slice_start;
                }
                let content = match pattern.inside {
                    Some(inside) => {
                        let mut inner =
                            tokenize_compiled(&text[from..to], compile(inside), depth + 1);
                        for node in &mut inner {
                            node.shift(from);
                        }
                        inner
                    }
                    None => vec![Node::Text {
                        start: from,
                        end: to,
                    }],
                };
                let token_in_fresh = fresh.len();
                fresh.push(Node::Token(Token {
                    kind,
                    alias: pattern.alias,
                    start: from,
                    end: to,
                    content,
                }));
                if to < slice_end {
                    fresh.push(Node::Text {
                        start: to,
                        end: slice_end,
                    });
                }
                list.splice(current..current + remove_count, fresh);
                current += token_in_fresh;

                if remove_count > 1 {
                    // A greedy match swallowed at least one token, so every
                    // pattern before this one has to be tried again over
                    // what the splice left.
                    let mut nested = Some(Rematch {
                        cause: (key_ix, j),
                        reach,
                    });
                    // Prism restarts at `currentNode.prev`, so the first
                    // node looked at again is the token just made.
                    match_grammar(text, list, grammar, current, pos, &mut nested, depth + 1);
                    if let (Some(r), Some(n)) = (rematch.as_mut(), nested.as_ref()) {
                        if n.reach > r.reach {
                            r.reach = n.reach;
                        }
                    }
                    // The nested run may have replaced the token with
                    // several nodes; find it again by where it starts.
                    if !matches!(list.get(current), Some(Node::Token(t)) if t.start == from) {
                        match list.iter().position(|n| n.start() == from) {
                            Some(ix) => current = ix,
                            None => break,
                        }
                    }
                }
                pos += list[current].len();
                current += 1;
            }
        }
    }
}

/// Prism's `Token.stringify`: the HTML a rendered block carries, which is
/// what the corpus test compares against Obsidian's own output.
pub fn to_html(text: &str, nodes: &[Node]) -> String {
    let mut out = String::new();
    write_html(text, nodes, &mut out);
    out
}

fn write_html(text: &str, nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text { start, end } => escape_html(&text[*start..*end], out),
            Node::Token(t) => {
                out.push_str("<span class=\"token ");
                out.push_str(t.kind);
                for alias in t.alias {
                    out.push(' ');
                    out.push_str(alias);
                }
                out.push_str("\">");
                write_html(text, &t.content, out);
                out.push_str("</span>");
            }
        }
    }
}

/// What `innerHTML` gives back for a text node: the HTML serializer
/// escapes `&`, `<`, `>` and the no-break space, and leaves the quotes
/// alone (they are only special inside an attribute).
fn escape_html(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            _ => out.push(ch),
        }
    }
}

/// A token as a painter sees it: where it is, and what it is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
    pub alias: &'static [&'static str],
}

/// Every token in the tree, flattened. A token's own run comes before the
/// ones inside it, so a painter applying them in order ends on the
/// innermost — which is how the cascade resolves a nested span.
pub fn spans(nodes: &[Node]) -> Vec<Span> {
    let mut out = Vec::new();
    flatten(nodes, &mut out);
    out
}

fn flatten(nodes: &[Node], out: &mut Vec<Span>) {
    for node in nodes {
        if let Node::Token(t) = node {
            out.push(Span {
                start: t.start,
                end: t.end,
                kind: t.kind,
                alias: t.alias,
            });
            flatten(&t.content, out);
        }
    }
}
