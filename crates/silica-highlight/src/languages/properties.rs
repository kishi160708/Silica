//! .properties.
//!
//! Ported from Prism 1.29.0's `prism-properties.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, Grammar, LB};

pub static PROPERTIES: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"(?m)^[ \t]*[#!].*$"])]),
    e(
        "value",
        &[pa(
            &[
                r"(?m)(^[ \t]*(?:\\(?:\r\n|[\s\S])|[^\\\s:=])+(?: *[=:] *(?! )| ))(?:\\(?:\r\n|[\s\S])|",
                r"[^\\\r\n])+",
            ],
            LB,
            &["attr-value"],
        )],
    ),
    e(
        "key",
        &[pa(
            &[r"(?m)^[ \t]*(?:\\(?:\r\n|[\s\S])|[^\\\s:=])+(?= *[=:]| )"],
            0,
            &["attr-name"],
        )],
    ),
    e("punctuation", &[p(&[r"[=:]"])]),
]);
