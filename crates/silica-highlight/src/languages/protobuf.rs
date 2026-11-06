//! Protocol Buffers.
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-protobuf.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pai, pf, Grammar, GR, LB};

pub static PROTOBUF: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
    ),
    e(
        "class-name",
        &[
            pf(
                &[r"(\b(?:enum|extend|message|service)\s+)[A-Za-z_]\w*(?=\s*\{)"],
                LB,
            ),
            pf(
                &[
                    r"(\b(?:rpc\s+\w+|returns)\s*\(\s*(?:stream\s+)?)\.?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*(?=\s*\))",
                ],
                LB,
            ),
        ],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:enum|extend|extensions|import|message|oneof|option|optional|package|public|repeated|",
            r"required|reserved|returns|rpc(?=\s+\w)|service|stream|syntax|to)\b(?!\s*=\s*\d)",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x[\da-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "map",
        &[pai(
            &[r"(?i)\bmap<\s*[\w.]+\s*,\s*[\w.]+\s*>(?=\s+[a-z_]\w*\s*[=;])"],
            0,
            &["class-name"],
            &PROTOBUF_MAP,
        )],
    ),
    e(
        "builtin",
        &[p(&[
            r"\b(?:bool|bytes|double|s?fixed(?:32|64)|float|[su]?int(?:32|64)|string)\b",
        ])],
    ),
    e(
        "positional-class-name",
        &[pai(
            &[r"(?i)(?:\b|\B\.)[a-z_]\w*(?:\.[a-z_]\w*)*(?=\s+[a-z_]\w*\s*[=;])"],
            0,
            &["class-name"],
            &PROTOBUF_POSITIONAL_CLASS_NAME,
        )],
    ),
    e("annotation", &[pf(&[r"(?i)(\[\s*)[a-z_]\w*(?=\s*=)"], LB)]),
    e(
        "operator",
        &[p(&[r"[<>]=?|[!=]=?=?|--?|\+\+?|&&?|\|\|?|[?*/\~^%]"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static PROTOBUF_MAP: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"[<>.,]"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:bool|bytes|double|s?fixed(?:32|64)|float|[su]?int(?:32|64)|string)\b",
        ])],
    ),
]);

pub(super) static PROTOBUF_POSITIONAL_CLASS_NAME: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
