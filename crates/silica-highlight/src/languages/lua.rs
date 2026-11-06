//! Lua.
//!
//! Ported from Prism 1.29.0's `prism-lua.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, Grammar, GR, LB};

pub static LUA: Grammar = Grammar::new(&[
    e(
        "comment",
        &[p(&[r"(?m)^#!.+|--(?:\[(=*)\[[\s\S]*?\]\1\]|.*)"])],
    ),
    e(
        "string",
        &[pf(
            &[
                r#"(["'])(?:(?!\1)[^\\\r\n]|\\z(?:\r\n|\s)|\\(?:\r\n|[^z]))*\1|\[(=*)\[[\s\S]*?\]\2\]"#,
            ],
            GR,
        )],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x[a-f\d]+(?:\.[a-f\d]*)?(?:p[+-]?\d+)?\b|\b\d+(?:\.\B|(?:\.\d*)?(?:e[+-]?\d+)?\b)|",
            r"\B\.\d+(?:e[+-]?\d+)?\b",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:and|break|do|else|elseif|end|false|for|function|goto|if|in|local|nil|not|or|repeat|",
            r"return|then|true|until|while)\b",
        ])],
    ),
    e("function", &[p(&[r"(?!\d)\w+(?=\s*(?:[({]))"])]),
    e(
        "operator",
        &[
            p(&[r"[-+*%^\&|#]|//?|<[<=]?|>[>=]?|[=\~]=?"]),
            pf(&[r"(^|[^.])\.\.(?!\.)"], LB),
        ],
    ),
    e("punctuation", &[p(&[r"[\[\](){},;]|\.+|:+"])]),
]);
