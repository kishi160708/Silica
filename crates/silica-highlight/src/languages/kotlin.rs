//! Kotlin (`kt`, `kts`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-kotlin.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static KOTLIN: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string-literal",
        &[
            pai(
                &[r#""""(?:[^$]|\$(?:(?!\{)|\{[^{}]*\}))*?""""#],
                0,
                &["multiline"],
                &KOTLIN_STRING_LITERAL_1,
            ),
            pai(
                &[r#""(?:[^"\\\r\n$]|\\.|\$(?:(?!\{)|\{[^{}]*\}))*""#],
                0,
                &["singleline"],
                &KOTLIN_STRING_LITERAL_2,
            ),
        ],
    ),
    e(
        "char",
        &[pf(&[r"'(?:[^'\\\r\n]|\\(?:.|u[a-fA-F0-9]{0,4}))'"], GR)],
    ),
    e(
        "annotation",
        &[pa(
            &[r"\B@(?:\w+:)?(?:[A-Z]\w*|\[[^\]]+\])"],
            0,
            &["builtin"],
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(^|[^.])\b(?:abstract|actual|annotation|as|break|by|catch|class|companion|const|",
                r"constructor|continue|crossinline|data|do|dynamic|else|enum|expect|external|final|finally|",
                r"for|fun|get|if|import|in|infix|init|inline|inner|interface|internal|is|lateinit|noinline|",
                r"null|object|open|operator|out|override|package|private|protected|public|reified|return|",
                r"sealed|set|super|suspend|tailrec|this|throw|to|try|typealias|val|var|vararg|when|where|",
                r"while)\b",
            ],
            LB,
        )],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("label", &[pa(&[r"\b\w+@|@\w+\b"], 0, &["symbol"])]),
    e(
        "function",
        &[
            pf(&[r"(?:`[^\r\n`]+`|\b\w+)(?=\s*\()"], GR),
            pf(&[r"(\.)(?:`[^\r\n`]+`|\w+)(?=\s*\{)"], LB | GR),
        ],
    ),
    e(
        "number",
        &[p(&[
            r"\b(?:0[xX][\da-fA-F]+(?:_[\da-fA-F]+)*|0[bB][01]+(?:_[01]+)*|",
            r"\d+(?:_\d+)*(?:\.\d+(?:_\d+)*)?(?:[eE][+-]?\d+(?:_\d+)*)?[fFL]?)\b",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"\+[+=]?|-[-=>]?|==?=?|!(?:!|==?)?|[/*%<>]=?|[?:]:?|\.\.|&&|\|\||\b(?:and|inv|or|shl|shr|",
            r"ushr|xor)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static KOTLIN_STRING_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"(?i)\$(?:[a-z_]\w*|\{[^{}]*\})"],
            0,
            &KOTLIN_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static KOTLIN_STRING_LITERAL_1_INTERPOLATION: Grammar = Grammar::new(&[
    e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{?|\}$"], 0, &["punctuation"])],
    ),
    e("expression", &[pi(&[r"[\s\S]+"], 0, &KOTLIN)]),
]);

pub(super) static KOTLIN_STRING_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"(?i)((?:^|[^\\])(?:\\{2})*)\$(?:[a-z_]\w*|\{[^{}]*\})"],
            LB,
            &KOTLIN_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);
