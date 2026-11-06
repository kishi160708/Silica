//! Dart.
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-dart.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static DART: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string-literal",
        &[pi(
            &[r#"r?(?:("""|''')[\s\S]*?\1|(["'])(?:\\.|(?!\2)[^\\\r\n])*\2(?!\2))"#],
            GR,
            &DART_STRING_LITERAL,
        )],
    ),
    e("metadata", &[pa(&[r"@\w+"], 0, &["function"])]),
    e(
        "generics",
        &[pi(
            &[r"<(?:[\w\s,.\&?]|<(?:[\w\s,.\&?]|<(?:[\w\s,.\&?]|<[\w\s,.\&?]*>)*>)*>)*>"],
            0,
            &DART_GENERICS,
        )],
    ),
    e(
        "class-name",
        &[
            pi(
                &[
                    r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z](?:[\d_A-Z]*[a-z]\w*)?\b",
                ],
                LB,
                &DART_GENERICS_CLASS_NAME,
            ),
            pi(
                &[
                    r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z]\w*(?=\s+\w+\s*[;,=()])",
                ],
                LB,
                &DART_GENERICS_CLASS_NAME,
            ),
        ],
    ),
    e(
        "keyword",
        &[
            p(&[r"\b(?:async|sync|yield)\*"]),
            p(&[
                r"\b(?:abstract|assert|async|await|break|case|catch|class|const|continue|covariant|default|",
                r"deferred|do|dynamic|else|enum|export|extends|extension|external|factory|final|finally|for|",
                r"get|hide|if|implements|import|in|interface|library|mixin|new|null|on|operator|part|rethrow|",
                r"return|set|show|static|super|switch|sync|this|throw|try|typedef|var|void|while|with|",
                r"yield)\b",
            ]),
        ],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x[\da-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"\bis!|\b(?:as|is)\b|\+\+|--|&&|\|\||<<=?|>>=?|~(?:/=?)?|[+\-*/%\&^|=!<>]=?|\?",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static DART_STRING_LITERAL: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)\$(?:\w+|\{(?:[^{}]|\{[^{}]*\})*\})"],
            LB,
            &DART_STRING_LITERAL_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static DART_STRING_LITERAL_INTERPOLATION: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\$\{?|\}$"])]),
    e("expression", &[pi(&[r"[\s\S]+"], 0, &DART)]),
]);

pub(super) static DART_GENERICS: Grammar = Grammar::new(&[
    e(
        "class-name",
        &[pi(
            &[r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z](?:[\d_A-Z]*[a-z]\w*)?\b"],
            LB,
            &DART_GENERICS_CLASS_NAME,
        )],
    ),
    e(
        "keyword",
        &[
            p(&[r"\b(?:async|sync|yield)\*"]),
            p(&[
                r"\b(?:abstract|assert|async|await|break|case|catch|class|const|continue|covariant|default|",
                r"deferred|do|dynamic|else|enum|export|extends|extension|external|factory|final|finally|for|",
                r"get|hide|if|implements|import|in|interface|library|mixin|new|null|on|operator|part|rethrow|",
                r"return|set|show|static|super|switch|sync|this|throw|try|typedef|var|void|while|with|",
                r"yield)\b",
            ]),
        ],
    ),
    e("punctuation", &[p(&[r"[<>(),.:]"])]),
    e("operator", &[p(&[r"[?\&|]"])]),
]);

pub(super) static DART_GENERICS_CLASS_NAME: Grammar = Grammar::new(&[e(
    "namespace",
    &[pi(
        &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
        0,
        &DART_GENERICS_CLASS_NAME_NAMESPACE,
    )],
)]);

pub(super) static DART_GENERICS_CLASS_NAME_NAMESPACE: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
