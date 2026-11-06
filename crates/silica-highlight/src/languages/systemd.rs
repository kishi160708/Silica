//! Systemd configuration file.
//!
//! Ported from Prism 1.29.0's `prism-systemd.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static SYSTEMD: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(?m)^[;#].*"], GR)]),
    e(
        "section",
        &[pi(
            &[r"(?m)^\[[^\n\r\[\]]*\](?=[ \t]*$)"],
            GR,
            &SYSTEMD_SECTION,
        )],
    ),
    e(
        "key",
        &[pa(&[r"(?m)^[^\s=]+(?=[ \t]*=)"], GR, &["attr-name"])],
    ),
    e(
        "value",
        &[pai(
            &[
                r#"(=[ \t]*(?!\s))(?:"(?:[^\r\n"\\]|\\(?:[^\r]|\r\n?))*"(?!\S)|(?=[^"\r\n]))(?:[^\s\\]|"#,
                r#"[ 	]+(?:(?![ 	"])|"(?:[^\r\n"\\]|\\(?:[^\r]|\r\n?))*"(?!\S))|"#,
                r"\\[\r\n]+(?:[#;].*[\r\n]+)*(?![#;]))*",
            ],
            LB | GR,
            &["attr-value"],
            &SYSTEMD_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"="])]),
]);

pub(super) static SYSTEMD_SECTION: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\[|\]$"])]),
    e("section-name", &[pa(&[r"[\s\S]+"], 0, &["selector"])]),
]);

pub(super) static SYSTEMD_VALUE: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(?m)^[;#].*"], GR)]),
    e(
        "quoted",
        &[pf(
            &[r#"(^|\s)"(?:[^\r\n"\\]|\\(?:[^\r]|\r\n?))*"(?!\S)"#],
            LB | GR,
        )],
    ),
    e("punctuation", &[p(&[r"(?m)\\$"])]),
    e("boolean", &[pf(&[r"^(?:false|no|off|on|true|yes)$"], GR)]),
]);
