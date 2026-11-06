//! NASM.
//!
//! Ported from Prism 1.29.0's `prism-nasm.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, LB};

pub static NASM: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"(?m);.*$"])]),
    e("string", &[p(&[r#"(["'`])(?:\\.|(?!\1)[^\\\r\n])*\1"#])]),
    e(
        "label",
        &[pa(
            &[r"(?m)(^\s*)[A-Za-z._?$][\w.?$@\~#]*:"],
            LB,
            &["function"],
        )],
    ),
    e(
        "keyword",
        &[
            p(&[r"\[?BITS (?:16|32|64)\]?"]),
            pf(&[r"(?im)(^\s*)section\s*[a-z.]+:?"], LB),
            p(&[r"(?i)(?:extern|global)[^;\r\n]*"]),
            p(&[r"(?m)(?:CPU|DEFAULT|FLOAT).*$"]),
        ],
    ),
    e(
        "register",
        &[pa(
            &[
                r"(?i)\b(?:st\d|[xyz]mm\d\d?|[cdt]r\d|r\d\d?[bwd]?|[er]?[abcd]x|[abcd][hl]|[er]?(?:bp|di|si|",
                r"sp)|[cdefgs]s)\b",
            ],
            0,
            &["variable"],
        )],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)(?:\b|(?=\$))(?:0[hx](?:\.[\da-f]+|[\da-f]+(?:\.[\da-f]+)?)(?:p[+-]?\d+)?|",
            r"\d[\da-f]+[hx]|\$\d[\da-f]*|0[oq][0-7]+|[0-7]+[oq]|0[by][01]+|[01]+[by]|0[dt]\d+|",
            r"(?:\d+(?:\.\d+)?|\.\d+)(?:\.?e[+-]?\d+)?[dt]?)\b",
        ])],
    ),
    e("operator", &[p(&[r"[\[\]*+\-/%<>=\&|$!]"])]),
]);
