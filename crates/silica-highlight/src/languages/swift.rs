//! Swift.
//!
//! Ported from Prism 1.29.0's `prism-swift.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static SWIFT: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"(^|[^\\:])(?://.*|/\*(?:[^/*]|/(?!\*)|\*(?!/)|/\*(?:[^*]|\*(?!/))*\*/)*\*/)"],
            LB | GR,
        )],
    ),
    e(
        "string-literal",
        &[
            pi(
                &[
                    r##"(^|[^"#])(?:"(?:\\(?:\((?:[^()]|\([^()]*\))*\)|\r\n|[^(])|[^\\\r\n"])*"|"##,
                    r##""""(?:\\(?:\((?:[^()]|\([^()]*\))*\)|[^(])|[^\\"]|"(?!""))*""")(?!["#])"##,
                ],
                LB | GR,
                &SWIFT_STRING_LITERAL_1,
            ),
            pi(
                &[
                    r##"(^|[^"#])(#+)(?:"(?:\\(?:#+\((?:[^()]|\([^()]*\))*\)|\r\n|[^#])|[^\\\r\n])*?"|"##,
                    r#""""(?:\\(?:#+\((?:[^()]|\([^()]*\))*\)|[^#])|[^\\])*?""")\2"#,
                ],
                LB | GR,
                &SWIFT_STRING_LITERAL_2,
            ),
        ],
    ),
    e(
        "directive",
        &[pai(
            &[
                r"#(?:(?:elseif|if)\b(?:[ 	]*(?:![ \t]*)?(?:\b\w+\b(?:[ \t]*\((?:[^()]|\([^()]*\))*\))?|",
                r"\((?:[^()]|\([^()]*\))*\))(?:[ \t]*(?:&&|\|\|))?)+|(?:else|endif)\b)",
            ],
            0,
            &["property"],
            &SWIFT_DIRECTIVE,
        )],
    ),
    e(
        "literal",
        &[pa(
            &[
                r"#(?:colorLiteral|column|dsohandle|file(?:ID|Literal|Path)?|function|imageLiteral|line)\b",
            ],
            0,
            &["constant"],
        )],
    ),
    e("other-directive", &[pa(&[r"#\w+\b"], 0, &["property"])]),
    e("attribute", &[pa(&[r"@\w+"], 0, &["atrule"])]),
    e(
        "function-definition",
        &[pa(&[r"(\bfunc\s+)\w+"], LB, &["function"])],
    ),
    e(
        "label",
        &[pa(
            &[r"\b(break|continue)\s+\w+|\b[a-zA-Z_]\w*(?=\s*:\s*(?:for|repeat|while)\b)"],
            LB,
            &["important"],
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:Any|Protocol|Self|Type|actor|as|assignment|associatedtype|associativity|async|await|",
            r"break|case|catch|class|continue|convenience|default|defer|deinit|didSet|do|dynamic|else|",
            r"enum|extension|fallthrough|fileprivate|final|for|func|get|guard|higherThan|if|import|in|",
            r"indirect|infix|init|inout|internal|is|isolated|lazy|left|let|lowerThan|mutating|none|",
            r"nonisolated|nonmutating|open|operator|optional|override|postfix|precedencegroup|prefix|",
            r"private|protocol|public|repeat|required|rethrows|return|right|safe|self|set|some|static|",
            r"struct|subscript|super|switch|throw|throws|try|typealias|unowned|unsafe|var|weak|where|",
            r"while|willSet)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("nil", &[pa(&[r"\bnil\b"], 0, &["constant"])]),
    e("short-argument", &[p(&[r"\$\d+\b"])]),
    e("omit", &[pa(&[r"\b_\b"], 0, &["keyword"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b(?:[\d_]+(?:\.[\de_]+)?|0x[a-f0-9_]+(?:\.[a-f0-9p_]+)?|0b[01_]+|0o[0-7_]+)\b",
        ])],
    ),
    e("class-name", &[p(&[r"\b[A-Z](?:[A-Z_\d]*[a-z]\w*)?\b"])]),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e("constant", &[p(&[r"\b(?:[A-Z_]{2,}|k[A-Z][A-Za-z_]+)\b"])]),
    e(
        "operator",
        &[p(&[r"[-+*/%=!<>\&|^\~?]+|\.[.\-+*/%=!<>\&|^\~?]+"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\]();,.:\\]"])]),
]);

pub(super) static SWIFT_STRING_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(&[r"(\\\()(?:[^()]|\([^()]*\))*(?=\))"], LB, &SWIFT)],
    ),
    e(
        "interpolation-punctuation",
        &[pa(&[r"^\)|\\\($"], 0, &["punctuation"])],
    ),
    e("punctuation", &[p(&[r"\\(?=[\r\n])"])]),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static SWIFT_STRING_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(&[r"(\\#+\()(?:[^()]|\([^()]*\))*(?=\))"], LB, &SWIFT)],
    ),
    e(
        "interpolation-punctuation",
        &[pa(&[r"^\)|\\#+\($"], 0, &["punctuation"])],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static SWIFT_DIRECTIVE: Grammar = Grammar::new(&[
    e("directive-name", &[p(&[r"^#\w+"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("number", &[p(&[r"\b\d+(?:\.\d+)*\b"])]),
    e("operator", &[p(&[r"!|&&|\|\||[<>]=?"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);
