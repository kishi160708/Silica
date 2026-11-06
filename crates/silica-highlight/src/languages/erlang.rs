//! Erlang.
//!
//! Ported from Prism 1.29.0's `prism-erlang.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR, LB};

pub static ERLANG: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"%.+"])]),
    e("string", &[pf(&[r#""(?:\\.|[^\\"\r\n])*""#], GR)]),
    e(
        "quoted-function",
        &[pa(&[r"'(?:\\.|[^\\'\r\n])+'(?=\()"], 0, &["function"])],
    ),
    e(
        "quoted-atom",
        &[pa(&[r"'(?:\\.|[^\\'\r\n])+'"], 0, &["atom"])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:after|begin|case|catch|end|fun|if|of|receive|try|when)\b",
        ])],
    ),
    e(
        "number",
        &[
            p(&[r"\$\\?."]),
            p(&[r"(?i)\b\d+#[a-z0-9]+"]),
            p(&[r"(?i)(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?"]),
        ],
    ),
    e("function", &[p(&[r"\b[a-z][\w@]*(?=\()"])]),
    e("variable", &[pf(&[r"(^|[^@])(?:\b|\?)[A-Z_][\w@]*"], LB)]),
    e(
        "operator",
        &[
            p(&[
                r"[=/<>:]=|=[:/]=|\+\+?|--?|[=*/!]|\b(?:and|andalso|band|bnot|bor|bsl|bsr|bxor|div|not|or|",
                r"orelse|rem|xor)\b",
            ]),
            pf(&[r"(^|[^<])<(?!<)"], LB),
            pf(&[r"(^|[^>])>(?!>)"], LB),
        ],
    ),
    e("atom", &[p(&[r"\b[a-z][\w@]*"])]),
    e("punctuation", &[p(&[r"[()\[\]{}:;,.#|]|<<|>>"])]),
]);
