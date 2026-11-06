//! CoffeeScript (`coffee`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-javascript.js`, `prism-coffeescript.js`,
//! `prism-javadoclike.js`, `prism-jsdoc.js`, `prism-js-extras.js` and
//! `prism-js-templates.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static COFFEESCRIPT: Grammar = Grammar::new(&[
    e(
        "multiline-comment",
        &[pa(&[r"###[\s\S]+?###"], 0, &["comment"])],
    ),
    e(
        "block-regex",
        &[pai(
            &[r"/{3}[\s\S]*?/{3}"],
            0,
            &["regex"],
            &COFFEESCRIPT_BLOCK_REGEX,
        )],
    ),
    e("comment", &[p(&[r"#(?!\{).+"])]),
    e("hashbang", &[pa(&[r"^#!.*"], GR, &["comment"])]),
    e(
        "string-property",
        &[pa(
            &[r#"(?m)((?:^|[,{])[ \t]*)(["'])(?:\\(?:\r\n|[\s\S])|(?!\2)[^\\\r\n])*\2(?=\s*:)"#],
            LB | GR,
            &["property"],
        )],
    ),
    e(
        "inline-javascript",
        &[pi(
            &[r"`(?:\\[\s\S]|[^\\`])*`"],
            0,
            &COFFEESCRIPT_INLINE_JAVASCRIPT,
        )],
    ),
    e(
        "multiline-string",
        &[
            pa(&[r"'''[\s\S]*?'''"], GR, &["string"]),
            pai(
                &[r#""""[\s\S]*?""""#],
                GR,
                &["string"],
                &COFFEESCRIPT_MULTILINE_STRING_2,
            ),
        ],
    ),
    e(
        "string",
        &[
            pf(&[r"'(?:\\[\s\S]|[^\\'])*'"], GR),
            pi(&[r#""(?:\\[\s\S]|[^\\"])*""#], GR, &COFFEESCRIPT_STRING_2),
        ],
    ),
    e(
        "class-name",
        &[
            pi(
                &[r"(\b(?:class|extends|implements|instanceof|interface|new)\s+)[\w.\\]+"],
                LB,
                &COFFEESCRIPT_CLASS_NAME_1,
            ),
            pf(
                &[
                    r"(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$A-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\.(?:cons",
                    r"tructor|prototype))",
                ],
                LB,
            ),
        ],
    ),
    e(
        "regex",
        &[pi(
            &[
                r#"((?:^|[^$\w\xA0-\x{FFFF}."'\])\s]|\b(?:return|yield))\s*)/(?:(?:\[(?:[^\]\\\r\n]|\\.)*\]|"#,
                r"\\.|[^/\\\[\r\n])+/[dgimyus]{0,7}|(?:\[(?:[^\[\]\\\r\n]|\\.|\[(?:[^\[\]\\\r\n]|\\.|",
                r"\[(?:[^\[\]\\\r\n]|\\.)*\])*\])*\]|\\.|[^/\\\[\r\n])+/[dgimyus]{0,7}v[dgimyus]{0,7})(?=(?:",
                r"\s|/\*(?:[^*]|\*(?!/))*\*/)*(?:$|[\r\n,.;:})\]]|//))",
            ],
            LB | GR,
            &COFFEESCRIPT_REGEX,
        )],
    ),
    e(
        "function-variable",
        &[pa(
            &[
                r"#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*[=:]\s*(?:async\s*)?(?:\b",
                r"function\b|(?:\((?:[^()]|\([^()]*\))*\)|(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{",
                r"FFFF}])*)\s*=>))",
            ],
            0,
            &["function"],
        )],
    ),
    e(
        "parameter",
        &[
            pi(
                &[
                    r"(function(?:\s+(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*)?\s*\(\s*)(?!\s)",
                    r"(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\))",
                ],
                LB,
                &COFFEESCRIPT,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &COFFEESCRIPT,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &COFFEESCRIPT,
            ),
            pi(
                &[
                    r"((?:\b|\s|^)(?!(?:as|async|await|break|case|catch|class|const|continue|debugger|default|",
                    r"delete|do|else|enum|export|extends|finally|for|from|function|get|if|implements|import|in|",
                    r"instanceof|interface|let|new|null|of|package|private|protected|public|return|set|static|",
                    r"super|switch|this|throw|try|typeof|undefined|var|void|while|with|",
                    r"yield)(?![$\w\xA0-\x{FFFF}]))(?:(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*",
                    r"\s*)\(\s*|\]\s*\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*\{)",
                ],
                LB,
                &COFFEESCRIPT,
            ),
        ],
    ),
    e("constant", &[p(&[r"\b[A-Z](?:[A-Z_]|\dx?)*\b"])]),
    e("property", &[p(&[r"(?!\d)\w+(?=\s*:(?!:))"])]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:and|break|by|catch|class|continue|debugger|delete|do|each|else|extend|extends|false|",
            r"finally|for|if|in|instanceof|is|isnt|let|loop|namespace|new|no|not|null|of|off|on|or|own|",
            r"return|super|switch|then|this|throw|true|try|typeof|undefined|unless|until|when|while|",
            r"window|with|yes|yield)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "function",
        &[p(&[
            r"#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*(?:\.\s*(?:apply|bind|",
            r"call)\s*)?\()",
        ])],
    ),
    e(
        "number",
        &[pf(
            &[
                r"(^|[^\w$])(?:NaN|Infinity|0[bB][01]+(?:_[01]+)*n?|0[oO][0-7]+(?:_[0-7]+)*n?|",
                r"0[xX][\dA-Fa-f]+(?:_[\dA-Fa-f]+)*n?|\d+(?:_\d+)*n|(?:\d+(?:_\d+)*(?:\.(?:\d+(?:_\d+)*)?)?|",
                r"\.\d+(?:_\d+)*)(?:[Ee][+-]?\d+(?:_\d+)*)?)(?![\w$])",
            ],
            LB,
        )],
    ),
    e(
        "literal-property",
        &[pa(
            &[
                r"(?m)((?:^|[,{])[ \t]*)(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*:)",
            ],
            LB,
            &["property"],
        )],
    ),
    e(
        "operator",
        &[p(&[
            r"--|\+\+|\*\*=?|=>|&&=?|\|\|=?|[!=]==|<<=?|>>>?=?|[-+*/%\&|^!=<>]=?|\.{3}|\?\?=?|\?\.?|",
            r"[\~:]",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e("class-member", &[pa(&[r"@(?!\d)\w+"], 0, &["variable"])]),
]);

pub(super) static COFFEESCRIPT_BLOCK_REGEX: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"#(?!\{).+"])]),
    e("interpolation", &[pa(&[r"#\{[^}]+\}"], 0, &["variable"])]),
]);

pub(super) static COFFEESCRIPT_INLINE_JAVASCRIPT: Grammar = Grammar::new(&[
    e("delimiter", &[pa(&[r"^`|`$"], 0, &["punctuation"])]),
    e(
        "script",
        &[pai(
            &[r"[\s\S]+"],
            0,
            &["language-javascript"],
            &super::javascript::JAVASCRIPT,
        )],
    ),
]);

pub(super) static COFFEESCRIPT_MULTILINE_STRING_2: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pa(&[r"#\{[^}]+\}"], 0, &["variable"])],
)]);

pub(super) static COFFEESCRIPT_STRING_2: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pa(&[r"#\{[^}]+\}"], 0, &["variable"])],
)]);

pub(super) static COFFEESCRIPT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static COFFEESCRIPT_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);
