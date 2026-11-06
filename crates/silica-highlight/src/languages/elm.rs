//! Elm.
//!
//! Ported from Prism 1.29.0's `prism-elm.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, pi, Grammar, GR, LB};

pub static ELM: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"--.*|\{-[\s\S]*?-\}"])]),
    e(
        "char",
        &[pf(
            &[r"'(?:[^\\'\r\n]|\\(?:[abfnrtv\\']|\d+|x[0-9a-fA-F]+|u\{[0-9a-fA-F]+\}))'"],
            GR,
        )],
    ),
    e(
        "string",
        &[
            pf(&[r#""""[\s\S]*?""""#], GR),
            pf(&[r#""(?:[^\\"\r\n]|\\.)*""#], GR),
        ],
    ),
    e(
        "import-statement",
        &[pi(
            &[
                r"(?m)(^[\t ]*)import\s+[A-Z]\w*(?:\.[A-Z]\w*)*(?:\s+as\s+(?:[A-Z]\w*)(?:\.[A-Z]\w*)*)?(?:\s",
                r"+exposing\s+)?",
            ],
            LB,
            &ELM_IMPORT_STATEMENT,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:alias|as|case|else|exposing|if|in|infixl|infixr|let|module|of|then|type)\b",
        ])],
    ),
    e(
        "builtin",
        &[p(&[
            r"\b(?:abs|acos|always|asin|atan|atan2|ceiling|clamp|compare|cos|curry|degrees|e|flip|floor|",
            r"fromPolar|identity|isInfinite|isNaN|logBase|max|min|negate|never|not|pi|radians|rem|round|",
            r"sin|sqrt|tan|toFloat|toPolar|toString|truncate|turns|uncurry|xor)\b",
        ])],
    ),
    e(
        "number",
        &[p(&[r"(?i)\b(?:\d+(?:\.\d+)?(?:e[+-]?\d+)?|0x[0-9a-f]+)\b"])],
    ),
    e(
        "operator",
        &[p(&[
            r"\s\.\s|[+\-/*=.$<>:\&|^?%#@\~!]{2,}|[+\-/*=$<>:\&|^?%#@\~!]",
        ])],
    ),
    e("hvariable", &[p(&[r"\b(?:[A-Z]\w*\.)*[a-z]\w*\b"])]),
    e("constant", &[p(&[r"\b(?:[A-Z]\w*\.)*[A-Z]\w*\b"])]),
    e("punctuation", &[p(&[r"[{}\[\]|(),.:]"])]),
]);

pub(super) static ELM_IMPORT_STATEMENT: Grammar =
    Grammar::new(&[e("keyword", &[p(&[r"\b(?:as|exposing|import)\b"])])]);
