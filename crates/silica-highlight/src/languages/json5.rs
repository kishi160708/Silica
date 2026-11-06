//! JSON5.
//!
//! Ported from Prism 1.29.0's `prism-json.js` and `prism-json5.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR};

pub static JSON5: Grammar = Grammar::new(&[
    e(
        "property",
        &[
            pf(
                &[r#"("|')(?:\\(?:\r\n?|\n|.)|(?!\1)[^\\\r\n])*\1(?=\s*:)"#],
                GR,
            ),
            pa(
                &[r"(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*:)"],
                0,
                &["unquoted"],
            ),
        ],
    ),
    e(
        "string",
        &[pf(&[r#"("|')(?:\\(?:\r\n?|\n|.)|(?!\1)[^\\\r\n])*\1"#], GR)],
    ),
    e("comment", &[pf(&[r"//.*|/\*[\s\S]*?(?:\*/|$)"], GR)]),
    e(
        "number",
        &[p(&[
            r"[+-]?\b(?:NaN|Infinity|0x[a-fA-F\d]+)\b|[+-]?(?:\b\d+(?:\.\d*)?|",
            r"\B\.\d+)(?:[eE][+-]?\d+\b)?",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\],]"])]),
    e("operator", &[p(&[r":"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("null", &[pa(&[r"\bnull\b"], 0, &["keyword"])]),
]);
