//! Prolog.
//!
//! Ported from Prism 1.29.0's `prism-prolog.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, Grammar, GR};

pub static PROLOG: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"/\*[\s\S]*?\*/|%.*"], GR)]),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\1\1|\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1(?!\1)"#],
            GR,
        )],
    ),
    e("builtin", &[p(&[r"\b(?:fx|fy|xf[xy]?|yfx?)\b"])]),
    e("function", &[p(&[r"\b[a-z]\w*(?:(?=\()|/\d+)"])]),
    e("number", &[p(&[r"\b\d+(?:\.\d*)?"])]),
    e(
        "operator",
        &[p(&[r"[:\\=><\-?*@/;+^|!$.]+|\b(?:is|mod|not|xor)\b"])],
    ),
    e("punctuation", &[p(&[r"[(){}\[\],]"])]),
]);
