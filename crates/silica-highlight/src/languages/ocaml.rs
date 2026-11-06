//! OCaml.
//!
//! Ported from Prism 1.29.0's `prism-ocaml.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR};

pub static OCAML: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"\(\*[\s\S]*?\*\)"], GR)]),
    e(
        "char",
        &[pf(
            &[r"(?i)'(?:[^\\\r\n']|\\(?:.|[ox]?[0-9a-f]{1,3}))'"],
            GR,
        )],
    ),
    e(
        "string",
        &[
            pf(&[r#""(?:\\(?:[\s\S]|\r\n)|[^\\\r\n"])*""#], GR),
            pf(&[r"\{([a-z_]*)\|[\s\S]*?\|\1\}"], GR),
        ],
    ),
    e(
        "number",
        &[
            p(&[r"(?i)\b(?:0b[01][01_]*|0o[0-7][0-7_]*)\b"]),
            p(&[r"(?i)\b0x[a-f0-9][a-f0-9_]*(?:\.[a-f0-9_]*)?(?:p[+-]?\d[\d_]*)?(?!\w)"]),
            p(&[r"(?i)\b\d[\d_]*(?:\.[\d_]*)?(?:e[+-]?\d[\d_]*)?(?!\w)"]),
        ],
    ),
    e("directive", &[pa(&[r"\B#\w+"], 0, &["property"])]),
    e("label", &[pa(&[r"\B~\w+"], 0, &["property"])]),
    e("type-variable", &[pa(&[r"\B'\w+"], 0, &["function"])]),
    e("variant", &[pa(&[r"`\w+"], 0, &["symbol"])]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:as|assert|begin|class|constraint|do|done|downto|else|end|exception|external|for|fun|",
            r"function|functor|if|in|include|inherit|initializer|lazy|let|match|method|module|mutable|",
            r"new|nonrec|object|of|open|private|rec|sig|struct|then|to|try|type|val|value|virtual|when|",
            r"where|while|with)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "operator-like-punctuation",
        &[pa(&[r"\[[<>|]|[>|]\]|\{<|>\}"], 0, &["punctuation"])],
    ),
    e(
        "operator",
        &[p(&[
            r"\.[.\~]|:[=>]|[=<>@^|\&+\-*/$%!?\~][!$%\&*+\-./:<=>?@^|\~]*|\b(?:and|asr|land|lor|lsl|lsr|",
            r"lxor|mod|or)\b",
        ])],
    ),
    e("punctuation", &[p(&[r";;|::|[(){}\[\].,:;#]|\b_\b"])]),
]);
