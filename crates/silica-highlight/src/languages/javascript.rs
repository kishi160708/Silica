//! JavaScript (`js`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-javascript.js`, `prism-javadoclike.js`,
//! `prism-jsdoc.js`, `prism-js-extras.js` and `prism-js-templates.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static JAVASCRIPT: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &JAVASCRIPT_DOC_COMMENT,
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
                &JAVASCRIPT_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JAVASCRIPT_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JAVASCRIPT_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JAVASCRIPT_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JAVASCRIPT_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JAVASCRIPT_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &JAVASCRIPT_TEMPLATE_STRING_7,
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
        &[
            pi(
                &[r"(\b(?:class|extends|implements|instanceof|interface|new)\s+)[\w.\\]+"],
                LB,
                &JAVASCRIPT_CLASS_NAME_1,
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
            &JAVASCRIPT_REGEX,
        )],
    ),
    e(
        "method-variable",
        &[pai(
            &[
                r"(\.\s*)#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*[=:]\s*(?:async\s",
                r"*)?(?:\bfunction\b|(?:\((?:[^()]|\([^()]*\))*\)|",
                r"(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*)\s*=>))",
            ],
            LB,
            &["function-variable", "method", "function", "property-access"],
            &JAVASCRIPT_METHOD_VARIABLE,
        )],
    ),
    e(
        "function-variable",
        &[pai(
            &[
                r"#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*[=:]\s*(?:async\s*)?(?:\b",
                r"function\b|(?:\((?:[^()]|\([^()]*\))*\)|(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{",
                r"FFFF}])*)\s*=>))",
            ],
            0,
            &["function"],
            &JAVASCRIPT_FUNCTION_VARIABLE,
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
                &JAVASCRIPT,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &JAVASCRIPT,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &JAVASCRIPT,
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
                &JAVASCRIPT,
            ),
        ],
    ),
    e(
        "known-class-name",
        &[
            pa(
                &[
                    r"\b(?:(?:Float(?:32|64)|(?:Int|Uint)(?:8|16|32)|Uint8Clamped)?Array|ArrayBuffer|BigInt|",
                    r"Boolean|DataView|Date|Error|Function|Intl|JSON|(?:Weak)?(?:Map|Set)|Math|Number|Object|",
                    r"Promise|Proxy|Reflect|RegExp|String|Symbol|WebAssembly)\b",
                ],
                0,
                &["class-name"],
            ),
            pa(&[r"\b(?:[A-Z]\w*)Error\b"], 0, &["class-name"]),
        ],
    ),
    e("constant", &[p(&[r"\b[A-Z](?:[A-Z_]|\dx?)*\b"])]),
    e(
        "imports",
        &[pi(
            &[
                r"(\bimport\b\s*)(?:(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?:\s*,\s*(?:\*",
                r"\s*as\s+(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*|\{[^{}]*\}))?|",
                r"\*\s*as\s+(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*|",
                r"\{[^{}]*\})(?=\s*\bfrom\b)",
            ],
            LB,
            &JAVASCRIPT,
        )],
    ),
    e(
        "exports",
        &[pi(
            &[
                r"(\bexport\b\s*)(?:\*(?:\s*as\s+(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*)",
                r"?(?=\s*\bfrom\b)|\{[^{}]*\})",
            ],
            LB,
            &JAVASCRIPT,
        )],
    ),
    e(
        "keyword",
        &[
            pa(&[r"\b(?:as|default|export|from|import)\b"], 0, &["module"]),
            pa(
                &[
                    r"\b(?:await|break|catch|continue|do|else|finally|for|if|return|switch|throw|try|while|",
                    r"yield)\b",
                ],
                0,
                &["control-flow"],
            ),
            pa(&[r"\bnull\b"], 0, &["null", "nil"]),
            pa(&[r"\bundefined\b"], 0, &["nil"]),
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
        ],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "method",
        &[pai(
            &[
                r"(\.\s*)#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*(?:\.\s*(?:apply|",
                r"bind|call)\s*)?\()",
            ],
            LB,
            &["function", "property-access"],
            &JAVASCRIPT_METHOD,
        )],
    ),
    e(
        "function",
        &[pi(
            &[
                r"#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*(?:\.\s*(?:apply|bind|",
                r"call)\s*)?\()",
            ],
            0,
            &JAVASCRIPT_FUNCTION,
        )],
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
    e("spread", &[pa(&[r"\.{3}"], 0, &["operator"])]),
    e("arrow", &[pa(&[r"=>"], 0, &["operator"])]),
    e(
        "operator",
        &[p(&[
            r"--|\+\+|\*\*=?|=>|&&=?|\|\|=?|[!=]==|<<=?|>>>?=?|[-+*/%\&|^!=<>]=?|\.{3}|\?\?=?|\?\.?|",
            r"[\~:]",
        ])],
    ),
    e(
        "property-access",
        &[pi(
            &[r"(\.\s*)#?(?!\s)[_$a-zA-Z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*"],
            LB,
            &JAVASCRIPT_PROPERTY_ACCESS,
        )],
    ),
    e(
        "maybe-class-name",
        &[pf(
            &[r"(^|[^$\w\xA0-\x{FFFF}])[A-Z][$\w\xA0-\x{FFFF}]+"],
            LB,
        )],
    ),
    e(
        "dom",
        &[pa(
            &[r"\b(?:document|(?:local|session)Storage|location|navigator|performance|window)\b"],
            0,
            &["variable"],
        )],
    ),
    e(
        "console",
        &[pa(&[r"\bconsole(?=\s*\.)"], 0, &["class-name"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static JAVASCRIPT_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pi(
            &[
                r"(@(?:arg|argument|param|property)\s+(?:\{(?:[^{}]|\{(?:[^{}]|",
                r"\{[^{}]*\})*\})+\}\s+)?)(?:(?!\s)[$\w\xA0-\x{FFFF}.])+(?=\s|$)",
            ],
            LB,
            &JAVASCRIPT_DOC_COMMENT_PARAMETER,
        )],
    ),
    e(
        "optional-parameter",
        &[pi(
            &[
                r"(@(?:arg|argument|param|property)\s+(?:\{(?:[^{}]|\{(?:[^{}]|",
                r"\{[^{}]*\})*\})+\}\s+)?)\[(?:(?!\s)[$\w\xA0-\x{FFFF}.])+(?:=[^\[\]]+)?\](?=\s|$)",
            ],
            LB,
            &JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER,
        )],
    ),
    e(
        "class-name",
        &[
            pi(
                &[
                    r"(@(?:augments|class|extends|interface|memberof!?|template|this|typedef)\s+(?:\{(?:[^{}]|",
                    r"\{(?:[^{}]|\{[^{}]*\})*\})+\}\s+)?)[A-Z]\w*(?:\.[A-Z]\w*)*",
                ],
                LB,
                &JAVASCRIPT_DOC_COMMENT_CLASS_NAME_1,
            ),
            pi(
                &[r"(@[a-z]+\s+)\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})+\}"],
                LB,
                &JAVASCRIPT_DOC_COMMENT_CLASS_NAME_2,
            ),
        ],
    ),
    e(
        "example",
        &[pi(
            &[r"(@example\s+(?!\s))(?:[^@\s]|\s+(?!\s))+?(?=\s*(?:\*\s*)?(?:@\w|\*/))"],
            LB,
            &JAVASCRIPT_DOC_COMMENT_EXAMPLE,
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

pub(super) static JAVASCRIPT_DOC_COMMENT_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pi(
            &[r"(^\[)[$\w\xA0-\x{FFFF}\.]+"],
            LB,
            &JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER,
        )],
    ),
    e(
        "code",
        &[pai(
            &[r"(=)[\s\S]*(?=\]$)"],
            LB,
            &["language-javascript"],
            &JAVASCRIPT,
        )],
    ),
    e("punctuation", &[p(&[r"[=\[\]]"])]),
]);

pub(super) static JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JAVASCRIPT_DOC_COMMENT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JAVASCRIPT_DOC_COMMENT_CLASS_NAME_2: Grammar = Grammar::new(&[
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
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
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
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
    e("operator", &[p(&[r"=>|\.\.\.|[\&|?:*]"])]),
    e("punctuation", &[p(&[r"[.,;=<>{}()\[\]]"])]),
]);

pub(super) static JAVASCRIPT_DOC_COMMENT_EXAMPLE: Grammar = Grammar::new(&[e(
    "code",
    &[pai(
        &[r"(?m)^([\t ]*(?:\*\s*)?)\S.*$"],
        LB,
        &["language-javascript"],
        &JAVASCRIPT,
    )],
)]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_1: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["css"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_2: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["html"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_3: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["svg"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_4: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["markdown"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_5: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["graphql"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_6: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["sql"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_7: Grammar = Grammar::new(&[
    e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
            LB,
            &JAVASCRIPT_TEMPLATE_STRING_7_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static JAVASCRIPT_TEMPLATE_STRING_7_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
    )],
    &JAVASCRIPT,
);

pub(super) static JAVASCRIPT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static JAVASCRIPT_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);

pub(super) static JAVASCRIPT_METHOD_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JAVASCRIPT_FUNCTION_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JAVASCRIPT_METHOD: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JAVASCRIPT_FUNCTION: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JAVASCRIPT_PROPERTY_ACCESS: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);
