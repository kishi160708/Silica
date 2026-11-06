//! R.
//!
//! Ported from Prism 1.29.0's `prism-r.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR};

pub static R: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"#.*"])]),
    e(
        "string",
        &[pf(&[r#"(['"])(?:\\.|(?!\1)[^\\\r\n])*\1"#], GR)],
    ),
    e("percent-operator", &[pa(&[r"%[^%\s]*%"], 0, &["operator"])]),
    e("boolean", &[p(&[r"\b(?:FALSE|TRUE)\b"])]),
    e("ellipsis", &[p(&[r"\.\.(?:\.|\d+)"])]),
    e(
        "number",
        &[
            p(&[r"\b(?:Inf|NaN)\b"]),
            p(&[r"(?:\b0x[\dA-Fa-f]+(?:\.\d*)?|\b\d+(?:\.\d*)?|\B\.\d+)(?:[EePp][+-]?\d+)?[iL]?"]),
        ],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:NA|NA_character_|NA_complex_|NA_integer_|NA_real_|NULL|break|else|for|function|if|in|",
            r"next|repeat|while)\b",
        ])],
    ),
    e(
        "operator",
        &[p(&[r"->?>?|<(?:=|<?-)?|[>=!]=?|::?|&&?|\|\|?|[+*/^$@\~]"])],
    ),
    e("punctuation", &[p(&[r"[(){}\[\],;]"])]),
]);
