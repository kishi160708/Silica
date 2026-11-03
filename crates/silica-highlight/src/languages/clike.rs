//! The C-like family: `clike` itself and the languages Prism derives from
//! it with `extend`, `insertBefore` and `delete`.
//!
//! Prism builds these by cloning `clike`'s ordered keys, replacing some,
//! appending the new ones and then moving or deleting a few. A Rust static
//! cannot clone-and-edit at compile time, so each language is written out
//! in the order those operations leave — the order is load-bearing, and
//! writing it out is what makes it reviewable against the original.

use crate::grammar::{e, p, pf, pi, Grammar, GR, LB};

/// The `inside` of `clike`'s `class-name`, which both it and its
/// descendants share.
static CLASS_NAME_INSIDE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

/// `prism-clike.js`, the base every language below extends.
pub static CLIKE: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
    ),
    e(
        "class-name",
        &[pi(
            &[
                r"(?i)(\b(?:class|extends|implements|instanceof|interface|new|trait)\s+|\bcatch\s+\()[\w.\\]+",
            ],
            LB,
            &CLASS_NAME_INSIDE,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:break|catch|continue|do|else|finally|for|function|if|in|instanceof|new|null|return|throw|try|while)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x[\da-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[r"[<>]=?|[!=]=?=?|--?|\+\+?|&&?|\|\|?|[?*/~^%]"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

// ---- Go -------------------------------------------------------------------
// `extend('clike')` with string, keyword, boolean, number, operator and a
// new `builtin`; `insertBefore('go', 'string', { char })`; then
// `delete go['class-name']`.

/// `prism-go.js`.
pub static GO: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("char", &[pf(&[r"'(?:\\.|[^'\\\r\n]){0,10}'"], GR)]),
    e(
        "string",
        &[pf(&[r#"(^|[^\\])"(?:\\.|[^"\\\r\n])*"|`[^`]*`"#], LB | GR)],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:break|case|chan|const|continue|default|defer|else|fallthrough|for|func|go(?:to)?|if|import|",
            r"interface|map|package|range|return|select|struct|switch|type|var)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:_|false|iota|nil|true)\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[
            p(&[r"(?i)\b0(?:b[01_]+|o[0-7_]+)i?\b"]),
            p(&[
                r"(?i)\b0x(?:[a-f\d_]+(?:\.[a-f\d_]*)?|\.[a-f\d_]+)(?:p[+-]?\d+(?:_\d+)*)?i?(?!\w)",
            ]),
            p(&[r"(?i)(?:\b\d[\d_]*(?:\.[\d_]*)?|\B\.\d[\d_]*)(?:e[+-]?[\d_]+)?i?(?!\w)"]),
        ],
    ),
    e(
        "operator",
        &[p(&[
            r"[*/%^!=]=?|\+[=+]?|-[=-]?|\|[=|]?|&(?:=|&|\^=?)?|>(?:>=?|=)?|<(?:<=?|=|-)?|:=|\.\.\.",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:append|bool|byte|cap|close|complex|complex(?:64|128)|copy|delete|error|float(?:32|64)|",
            r"u?int(?:8|16|32|64)?|imag|len|make|new|panic|print(?:ln)?|real|recover|rune|string|uintptr)\b",
        ])],
    ),
]);
