//! nginx.
//!
//! Ported from Prism 1.29.0's `prism-nginx.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static NGINX: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(^|[\s{};])#.*"], LB | GR)]),
    e(
        "directive",
        &[pi(
            &[
                r#"(^|\s)\w(?:[^;{}"'\\\s]|\\.|"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|\s+(?:#.*(?!.)|"#,
                r"(?![#\s])))*?(?=\s*[;{])",
            ],
            LB | GR,
            &NGINX_DIRECTIVE,
        )],
    ),
    e("punctuation", &[p(&[r"[{};]"])]),
]);

pub(super) static NGINX_DIRECTIVE: Grammar = Grammar::new(&[
    e(
        "string",
        &[pi(
            &[r#"((?:^|[^\\])(?:\\\\)*)(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')"#],
            LB | GR,
            &NGINX_DIRECTIVE_STRING,
        )],
    ),
    e("comment", &[pf(&[r"(\s)#.*"], LB | GR)]),
    e("keyword", &[pf(&[r"^\S+"], GR)]),
    e("boolean", &[pf(&[r"(\s)(?:off|on)(?!\S)"], LB)]),
    e("number", &[pf(&[r"(?i)(\s)\d+[a-z]*(?!\S)"], LB)]),
    e(
        "variable",
        &[p(&[
            r#"(?i)\$(?:\w[a-z\d]*(?:_[^\x00-\x1F\s"'\\()$]*)?|\{[^}\s"'\\]+\})"#,
        ])],
    ),
]);

pub(super) static NGINX_DIRECTIVE_STRING: Grammar = Grammar::new(&[
    e("escape", &[pa(&[r#"\\["'\\nrt]"#], 0, &["entity"])]),
    e(
        "variable",
        &[p(&[
            r#"(?i)\$(?:\w[a-z\d]*(?:_[^\x00-\x1F\s"'\\()$]*)?|\{[^}\s"'\\]+\})"#,
        ])],
    ),
]);
