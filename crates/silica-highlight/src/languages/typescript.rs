//! TypeScript (`ts`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-javascript.js`, `prism-typescript.js`,
//! `prism-javadoclike.js`, `prism-jsdoc.js`, `prism-js-extras.js` and
//! `prism-js-templates.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static TYPESCRIPT: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &TYPESCRIPT_DOC_COMMENT,
        )],
    ),
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("hashbang", &[pa(&[r"^#!.*"], GR, &["comment"])]),
    e(
        "template-string",
        &[
            pi(
                &[
                    r"((?:\b(?:styled(?:\([^)]*\))?(?:\s*\.\s*\w+(?:\([^)]*\))*)*|css(?:\s*\.\s*(?:global|",
                    r"resolve))?|createGlobalStyle|keyframes))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &TYPESCRIPT_TEMPLATE_STRING_7,
            ),
        ],
    ),
    e(
        "string-property",
        &[pa(
            &[r#"(?m)((?:^|[,{])[ \t]*)(["'])(?:\\(?:\r\n|[\s\S])|(?!\2)[^\\\r\n])*\2(?=\s*:)"#],
            LB | GR,
            &["property"],
        )],
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
        &[pi(
            &[
                r"(\b(?:class|extends|implements|instanceof|interface|new|",
                r"type)\s+)(?!keyof\b)(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?:\s*<(?:[^",
                r"<>]|<(?:[^<>]|<[^<>]*>)*>)*>)?",
            ],
            LB | GR,
            &TYPESCRIPT_CLASS_NAME,
        )],
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
            &TYPESCRIPT_REGEX,
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
    e("constant", &[p(&[r"\b[A-Z](?:[A-Z_]|\dx?)*\b"])]),
    e(
        "keyword",
        &[
            pf(&[r"((?:^|\})\s*)catch\b"], LB),
            pf(
                &[
                    r"(^|[^.]|\.\.\.\s*)\b(?:as|assert(?=\s*\{)|async(?=\s*(?:function\b|\(|[$\w\xA0-\x{FFFF}]|",
                    r"$))|await|break|case|class|const|continue|debugger|default|delete|do|else|enum|export|",
                    r#"extends|finally(?=\s*(?:\{|$))|for|from(?=\s*(?:['"]|$))|function|(?:get|"#,
                    r"set)(?=\s*(?:[#\[$\w\xA0-\x{FFFF}]|$))|if|implements|import|in|instanceof|interface|let|",
                    r"new|null|of|package|private|protected|public|return|static|super|switch|this|throw|try|",
                    r"typeof|undefined|var|void|while|with|yield)\b",
                ],
                LB,
            ),
            p(&[r"\b(?:abstract|declare|is|keyof|readonly|require)\b"]),
            p(&[
                r"\b(?:asserts|infer|interface|module|namespace|type)\b(?=\s*(?:[{_$a-zA-Z\xA0-\x{FFFF}]|$))",
            ]),
            p(&[r"\btype\b(?=\s*(?:[\{*]|$))"]),
        ],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "decorator",
        &[pi(&[r"@[$\w\xA0-\x{FFFF}]+"], 0, &TYPESCRIPT_DECORATOR)],
    ),
    e(
        "generic-function",
        &[pi(
            &[
                r"#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*\s*<(?:[^<>]|<(?:[^<>]|",
                r"<[^<>]*>)*>)*>(?=\s*\()",
            ],
            GR,
            &TYPESCRIPT_GENERIC_FUNCTION,
        )],
    ),
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
        "operator",
        &[p(&[
            r"--|\+\+|\*\*=?|=>|&&=?|\|\|=?|[!=]==|<<=?|>>>?=?|[-+*/%\&|^!=<>]=?|\.{3}|\?\?=?|\?\.?|",
            r"[\~:]",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:Array|Function|Promise|any|boolean|console|never|number|string|symbol|unknown)\b",
        ])],
    ),
]);

pub(super) static TYPESCRIPT_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*@(?:arg|arguments|param)\s+)\w+"],
            LB,
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*|\{)@[a-z][a-zA-Z-]+\b"],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[{}]"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_1: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["css"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_2: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["html"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_3: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["svg"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_4: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["markdown"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_5: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["graphql"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_6: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["sql"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_7: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
            LB,
            &TYPESCRIPT_TEMPLATE_STRING_7_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static TYPESCRIPT_TEMPLATE_STRING_7_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
    )],
    &TYPESCRIPT,
);

pub(super) static TYPESCRIPT_CLASS_NAME: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &TYPESCRIPT_CLASS_NAME_DOC_COMMENT,
        )],
    ),
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("hashbang", &[pa(&[r"^#!.*"], GR, &["comment"])]),
    e(
        "template-string",
        &[
            pi(
                &[
                    r"((?:\b(?:styled(?:\([^)]*\))?(?:\s*\.\s*\w+(?:\([^)]*\))*)*|css(?:\s*\.\s*(?:global|",
                    r"resolve))?|createGlobalStyle|keyframes))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_7,
            ),
        ],
    ),
    e(
        "string-property",
        &[pa(
            &[r#"(?m)((?:^|[,{])[ \t]*)(["'])(?:\\(?:\r\n|[\s\S])|(?!\2)[^\\\r\n])*\2(?=\s*:)"#],
            LB | GR,
            &["property"],
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
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
            &TYPESCRIPT_CLASS_NAME_REGEX,
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
    e("constant", &[p(&[r"\b[A-Z](?:[A-Z_]|\dx?)*\b"])]),
    e(
        "keyword",
        &[
            pf(&[r"((?:^|\})\s*)catch\b"], LB),
            pf(
                &[
                    r"(^|[^.]|\.\.\.\s*)\b(?:as|assert(?=\s*\{)|async(?=\s*(?:function\b|\(|[$\w\xA0-\x{FFFF}]|",
                    r"$))|await|break|case|class|const|continue|debugger|default|delete|do|else|enum|export|",
                    r#"extends|finally(?=\s*(?:\{|$))|for|from(?=\s*(?:['"]|$))|function|(?:get|"#,
                    r"set)(?=\s*(?:[#\[$\w\xA0-\x{FFFF}]|$))|if|implements|import|in|instanceof|interface|let|",
                    r"new|null|of|package|private|protected|public|return|static|super|switch|this|throw|try|",
                    r"typeof|undefined|var|void|while|with|yield)\b",
                ],
                LB,
            ),
            p(&[r"\b(?:abstract|declare|is|keyof|readonly|require)\b"]),
            p(&[
                r"\b(?:asserts|infer|interface|module|namespace|type)\b(?=\s*(?:[{_$a-zA-Z\xA0-\x{FFFF}]|$))",
            ]),
            p(&[r"\btype\b(?=\s*(?:[\{*]|$))"]),
        ],
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
        "operator",
        &[p(&[
            r"--|\+\+|\*\*=?|=>|&&=?|\|\|=?|[!=]==|<<=?|>>>?=?|[-+*/%\&|^!=<>]=?|\.{3}|\?\?=?|\?\.?|",
            r"[\~:]",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:Array|Function|Promise|any|boolean|console|never|number|string|symbol|unknown)\b",
        ])],
    ),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*@(?:arg|arguments|param)\s+)\w+"],
            LB,
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*|\{)@[a-z][a-zA-Z-]+\b"],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[{}]"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_1: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["css"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_2: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["html"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_3: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["svg"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_4: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["markdown"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_5: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["graphql"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_6: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["sql"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_7: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
            LB,
            &TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_7_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static TYPESCRIPT_CLASS_NAME_TEMPLATE_STRING_7_INTERPOLATION: Grammar =
    Grammar::with_rest(
        &[e(
            "interpolation-punctuation",
            &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
        )],
        &TYPESCRIPT_CLASS_NAME,
    );

pub(super) static TYPESCRIPT_CLASS_NAME_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);

pub(super) static TYPESCRIPT_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);

pub(super) static TYPESCRIPT_DECORATOR: Grammar = Grammar::new(&[
    e("at", &[pa(&[r"^@"], 0, &["operator"])]),
    e("function", &[p(&[r"^[\s\S]+"])]),
]);

pub(super) static TYPESCRIPT_GENERIC_FUNCTION: Grammar = Grammar::new(&[
    e(
        "function",
        &[p(&[
            r"^#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*",
        ])],
    ),
    e(
        "generic",
        &[pai(
            &[r"<[\s\S]+"],
            0,
            &["class-name"],
            &TYPESCRIPT_CLASS_NAME,
        )],
    ),
]);
