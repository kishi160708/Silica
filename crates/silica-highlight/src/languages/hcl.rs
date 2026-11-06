//! HCL.
//!
//! Ported from Prism 1.29.0's `prism-hcl.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static HCL: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"(?://|#).*|/\*[\s\S]*?(?:\*/|$)"])]),
    e(
        "heredoc",
        &[pa(&[r"(?m)<<-?(\w+\b)[\s\S]*?^[ \t]*\1"], GR, &["string"])],
    ),
    e(
        "keyword",
        &[
            pi(
                &[r#"(?i)(?:data|resource)\s+(?:"(?:\\[\s\S]|[^\\"])*")(?=\s+"[\w-]+"\s+\{)"#],
                0,
                &HCL_KEYWORD_1,
            ),
            pi(
                &[
                    r#"(?i)(?:backend|module|output|provider|provisioner|variable)\s+(?:[\w-]+|"(?:\\[\s\S]|"#,
                    r#"[^\\"])*")\s+(?=\{)"#,
                ],
                0,
                &HCL_KEYWORD_2,
            ),
            p(&[r"[\w-]+(?=\s+\{)"]),
        ],
    ),
    e(
        "property",
        &[
            p(&[r"[-\w\.]+(?=\s*=(?!=))"]),
            p(&[r#""(?:\\[\s\S]|[^\\"])+"(?=\s*[:=])"#]),
        ],
    ),
    e(
        "string",
        &[pi(
            &[
                r#""(?:[^\\$"]|\\[\s\S]|\$(?:(?=")|\$+(?!\$)|[^"${])|\$\{(?:[^{}"]|"(?:[^\\"]|"#,
                r#"\\[\s\S])*")*\})*""#,
            ],
            GR,
            &HCL_STRING,
        )],
    ),
    e(
        "number",
        &[p(&[r"(?i)\b0x[\da-f]+\b|\b\d+(?:\.\d*)?(?:e[+-]?\d+)?"])],
    ),
    e("boolean", &[p(&[r"(?i)\b(?:false|true)\b"])]),
    e("punctuation", &[p(&[r"[=\[\]{}]"])]),
]);

pub(super) static HCL_KEYWORD_1: Grammar = Grammar::new(&[e(
    "type",
    &[pa(
        &[r#"(?i)(resource|data|\s+)(?:"(?:\\[\s\S]|[^\\"])*")"#],
        LB,
        &["variable"],
    )],
)]);

pub(super) static HCL_KEYWORD_2: Grammar = Grammar::new(&[e(
    "type",
    &[pa(
        &[
            r#"(?i)(backend|module|output|provider|provisioner|variable)\s+(?:[\w-]+|"(?:\\[\s\S]|"#,
            r#"[^\\"])*")\s+"#,
        ],
        LB,
        &["variable"],
    )],
)]);

pub(super) static HCL_STRING: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(
        &[r#"(^|[^$])\$\{(?:[^{}"]|"(?:[^\\"]|\\[\s\S])*")*\}"#],
        LB,
        &HCL_STRING_INTERPOLATION,
    )],
)]);

pub(super) static HCL_STRING_INTERPOLATION: Grammar = Grammar::new(&[
    e(
        "type",
        &[pa(
            &[r"(?i)(\b(?:count|data|local|module|path|self|terraform|var)\b\.)[\w\*]+"],
            LB,
            &["variable"],
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"(?i)\b(?:count|data|local|module|path|self|terraform|var)\b",
        ])],
    ),
    e("function", &[p(&[r"\w+(?=\()"])]),
    e("string", &[pf(&[r#""(?:\\[\s\S]|[^\\"])*""#], GR)]),
    e(
        "number",
        &[p(&[r"(?i)\b0x[\da-f]+\b|\b\d+(?:\.\d*)?(?:e[+-]?\d+)?"])],
    ),
    e(
        "punctuation",
        &[p(&[r"[!\$#%\&'()*+,./;<=>@\[\\\]^`{|}\~?:]"])],
    ),
]);
