//! React JSX.
//!
//! Ported from Prism 1.29.0's `prism-markup.js`, `prism-clike.js`, `prism-javascript.js`,
//! `prism-jsx.js`, `prism-css.js`, `prism-actionscript.js`,
//! `prism-javadoclike.js`, `prism-jsdoc.js`, `prism-js-extras.js` and
//! `prism-js-templates.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static JSX: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("prolog", &[pf(&[r"<\?[\s\S]+?\?>"], GR)]),
    e(
        "doctype",
        &[pi(
            &[
                r#"(?i)<!DOCTYPE(?:[^>"'\[\]]|"[^"]*"|'[^']*')+(?:\[(?:[^<"'\]]|"[^"]*"|'[^']*'|<(?!!--)|"#,
                r"<!--(?:[^-]|-(?!->))*-->)*\]\s*)?>",
            ],
            GR,
            &JSX_DOCTYPE,
        )],
    ),
    e(
        "script",
        &[pi(
            &[
                r"(?i)(<script[^>]*>)(?:<!\[CDATA\[(?:[^\]]|\](?!\]>))*\]\]>|",
                r"(?!<!\[CDATA\[)[\s\S])*?(?=</script>)",
            ],
            LB | GR,
            &JSX_SCRIPT,
        )],
    ),
    e(
        "style",
        &[pi(
            &[
                r"(?i)(<style[^>]*>)(?:<!\[CDATA\[(?:[^\]]|\](?!\]>))*\]\]>|",
                r"(?!<!\[CDATA\[)[\s\S])*?(?=</style>)",
            ],
            LB | GR,
            &JSX_STYLE,
        )],
    ),
    e("cdata", &[pf(&[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"], GR)]),
    e(
        "tag",
        &[pi(
            &[
                r#"</?(?:[\w.:-]+(?:(?:\s|//.*(?!.)|/\*(?:[^*]|\*(?!/))\*/)+(?:[\w.:$-]+(?:=(?:"(?:\\[\s\S]|"#,
                r#"[^\\"])*"|'(?:\\[\s\S]|[^\\'])*'|[^\s{'"/>=]+|(?:\{(?:\{(?:\{[^{}]*\}|[^{}])*\}|"#,
                r"[^{}])*\})))?|(?:\{(?:\s|//.*(?!.)|/\*(?:[^*]|\*(?!/))\*/)*\.{3}(?:[^{}]|",
                r"(?:\{(?:\{(?:\{[^{}]*\}|[^{}])*\}|[^{}])*\}))*\})))*(?:\s|//.*(?!.)|/\*(?:[^*]|",
                r"\*(?!/))\*/)*/?)?>",
            ],
            GR,
            &JSX_TAG,
        )],
    ),
    e(
        "entity",
        &[
            pa(&[r"(?i)&[\da-z]{1,8};"], 0, &["named-entity"]),
            p(&[r"(?i)&#x?[\da-f]{1,8};"]),
        ],
    ),
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &JSX_DOC_COMMENT,
        )],
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_7,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_CLASS_NAME_1,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_REGEX,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD_VARIABLE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION_VARIABLE,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_PROPERTY_ACCESS,
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

pub(super) static JSX_DOCTYPE: Grammar = Grammar::new(&[
    e(
        "internal-subset",
        &[pi(&[r"(^[^\[]*\[)[\s\S]+(?=\]>$)"], LB | GR, &JSX)],
    ),
    e("string", &[pf(&[r#""[^"]*"|'[^']*'"#], GR)]),
    e("punctuation", &[p(&[r"^<!|>$|[\[\]]"])]),
    e("doctype-tag", &[p(&[r"(?i)^DOCTYPE"])]),
    e("name", &[p(&[r#"[^\s<>'"]+"#])]),
]);

pub(super) static JSX_SCRIPT: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &JSX_SCRIPT_INCLUDED_CDATA,
        )],
    ),
    e(
        "language-javascript",
        &[pi(
            &[r"[\s\S]+"],
            0,
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-javascript",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT,
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
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_7,
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
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_CLASS_NAME_1,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_REGEX,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_METHOD_VARIABLE,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_FUNCTION_VARIABLE,
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
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
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
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_METHOD,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_FUNCTION,
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
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_PROPERTY_ACCESS,
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

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT: Grammar = Grammar::new(
    &[
        e(
            "parameter",
            &[pi(
                &[
                    r"(@(?:arg|argument|param|property)\s+(?:\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^{}]*\})*\})+\}\s+)?)(?:(?!\s)[$\w\xA0-\x{FFFF}.])+(?=\s|$)",
                ],
                LB,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_PARAMETER,
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
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER,
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
                    &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_CLASS_NAME_1,
                ),
                pi(
                    &[r"(@[a-z]+\s+)\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})+\}"],
                    LB,
                    &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_CLASS_NAME_2,
                ),
            ],
        ),
        e(
            "example",
            &[pi(
                &[r"(@example\s+(?!\s))(?:[^@\s]|\s+(?!\s))+?(?=\s*(?:\*\s*)?(?:@\w|\*/))"],
                LB,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_EXAMPLE,
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
    ],
);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER:
    Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pi(
            &[r"(^\[)[$\w\xA0-\x{FFFF}\.]+"],
            LB,
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER,
        )],
    ),
    e(
        "code",
        &[pai(
            &[r"(=)[\s\S]*(?=\]$)"],
            LB,
            &["language-javascript"],
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
    e("punctuation", &[p(&[r"[=\[\]]"])]),
]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_CLASS_NAME_2: Grammar =
    Grammar::new(&[
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

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_DOC_COMMENT_EXAMPLE: Grammar =
    Grammar::new(&[e(
        "code",
        &[pai(
            &[r"(?m)^([\t ]*(?:\*\s*)?)\S.*$"],
            LB,
            &["language-javascript"],
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    )]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_1: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["css"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_2: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["html"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_3: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["svg"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_4: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["markdown"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_5: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["graphql"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_6: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["sql"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_7: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e(
            "interpolation",
            &[pi(
                &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
                LB,
                &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_7_INTERPOLATION,
            )],
        ),
        e("string", &[p(&[r"[\s\S]+"])]),
    ]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_7_INTERPOLATION:
    Grammar = Grammar::with_rest(
    &[e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
    )],
    &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_METHOD_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_FUNCTION_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_METHOD: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_FUNCTION: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_PROPERTY_ACCESS: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_STYLE: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &JSX_STYLE_INCLUDED_CDATA,
        )],
    ),
    e(
        "language-css",
        &[pi(&[r"[\s\S]+"], 0, &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS)],
    ),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-css",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"/\*[\s\S]*?\*/"])]),
    e(
        "atrule",
        &[pi(
            &[
                r#"@[\w-](?:[^;{\s"']|\s+(?!\s)|(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|"#,
                r"[^'\\\r\n])*'))*?(?:;|(?=\s*\{))",
            ],
            0,
            &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE,
        )],
    ),
    e(
        "url",
        &[pi(
            &[
                r#"(?i)\burl\((?:(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')|"#,
                r#"(?:[^\\\r\n()"']|\\[\s\S])*)\)"#,
            ],
            GR,
            &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_URL,
        )],
    ),
    e(
        "selector",
        &[pi(
            &[
                r#"(^|[{}\s])[^{}\s](?:[^{};"'\s]|\s+(?![\s{])|(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|"#,
                r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*'))*(?=\s*\{)",
            ],
            LB,
            &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT,
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#"(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')"#],
            GR,
        )],
    ),
    e(
        "variable",
        &[pf(
            &[
                r"(?i)(^|[^-\w\xA0-\x{FFFF}])--(?!\s)[-_a-z\xA0-\x{FFFF}](?:(?!\s)[-\w\xA0-\x{FFFF}])*",
            ],
            LB,
        )],
    ),
    e(
        "property",
        &[pf(
            &[
                r"(?i)(^|[^-\w\xA0-\x{FFFF}])(?!\s)[-_a-z\xA0-\x{FFFF}](?:(?!\s)[-\w\xA0-\x{FFFF}])*(?=\s*:)",
            ],
            LB,
        )],
    ),
    e("important", &[p(&[r"(?i)!important\b"])]),
    e("operator", &[pf(&[r"(\s)[+\-*/](?=\s)"], LB)]),
    e("hexcode", &[pa(&[r"(?i)\B#[\da-f]{3,8}\b"], 0, &["color"])]),
    e(
        "color",
        &[
            pf(
                &[
                    r"(?i)(^|[^\w-])(?:AliceBlue|AntiqueWhite|Aqua|Aquamarine|Azure|Beige|Bisque|Black|",
                    r"BlanchedAlmond|Blue|BlueViolet|Brown|BurlyWood|CadetBlue|Chartreuse|Chocolate|Coral|",
                    r"CornflowerBlue|Cornsilk|Crimson|Cyan|DarkBlue|DarkCyan|DarkGoldenRod|DarkGr[ae]y|DarkGreen|",
                    r"DarkKhaki|DarkMagenta|DarkOliveGreen|DarkOrange|DarkOrchid|DarkRed|DarkSalmon|DarkSeaGreen|",
                    r"DarkSlateBlue|DarkSlateGr[ae]y|DarkTurquoise|DarkViolet|DeepPink|DeepSkyBlue|DimGr[ae]y|",
                    r"DodgerBlue|FireBrick|FloralWhite|ForestGreen|Fuchsia|Gainsboro|GhostWhite|Gold|GoldenRod|",
                    r"Gr[ae]y|Green|GreenYellow|HoneyDew|HotPink|IndianRed|Indigo|Ivory|Khaki|Lavender|",
                    r"LavenderBlush|LawnGreen|LemonChiffon|LightBlue|LightCoral|LightCyan|LightGoldenRodYellow|",
                    r"LightGr[ae]y|LightGreen|LightPink|LightSalmon|LightSeaGreen|LightSkyBlue|LightSlateGr[ae]y|",
                    r"LightSteelBlue|LightYellow|Lime|LimeGreen|Linen|Magenta|Maroon|MediumAquaMarine|MediumBlue|",
                    r"MediumOrchid|MediumPurple|MediumSeaGreen|MediumSlateBlue|MediumSpringGreen|MediumTurquoise|",
                    r"MediumVioletRed|MidnightBlue|MintCream|MistyRose|Moccasin|NavajoWhite|Navy|OldLace|Olive|",
                    r"OliveDrab|Orange|OrangeRed|Orchid|PaleGoldenRod|PaleGreen|PaleTurquoise|PaleVioletRed|",
                    r"PapayaWhip|PeachPuff|Peru|Pink|Plum|PowderBlue|Purple|RebeccaPurple|Red|RosyBrown|",
                    r"RoyalBlue|SaddleBrown|Salmon|SandyBrown|SeaGreen|SeaShell|Sienna|Silver|SkyBlue|SlateBlue|",
                    r"SlateGr[ae]y|Snow|SpringGreen|SteelBlue|Tan|Teal|Thistle|Tomato|Transparent|Turquoise|",
                    r"Violet|Wheat|White|WhiteSmoke|Yellow|YellowGreen)(?![\w-])",
                ],
                LB,
            ),
            pi(
                &[
                    r"(?i)\b(?:hsl|rgb)\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*\)\B|\b(?:hsl|",
                    r"rgb)a\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*,\s*(?:0|0?\.\d+|1)\s*\)\B",
                ],
                0,
                &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_COLOR_2,
            ),
        ],
    ),
    e("entity", &[p(&[r"(?i)\\[\da-f]{1,8}"])]),
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e(
        "function",
        &[pf(&[r"(?i)(^|[^-a-z0-9])[-a-z0-9]+(?=\()"], LB)],
    ),
    e("punctuation", &[p(&[r"[(){};:,]"])]),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE: Grammar = Grammar::with_rest(
    &[
        e("rule", &[p(&[r"^@[\w-]+"])]),
        e(
            "selector-function-argument",
            &[pai(
                &[
                    r"(\bselector\s*\(\s*(?![\s)]))(?:[^()\s]|\s+(?![\s)])|\((?:[^()]|\([^()]*\))*\))+(?=\s*\))",
                ],
                LB,
                &["selector"],
                &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT,
            )],
        ),
        e(
            "keyword",
            &[pf(&[r"(^|[^\w-])(?:and|not|only|or)(?![\w-])"], LB)],
        ),
    ],
    &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT: Grammar =
    Grammar::new(&[
        e(
            "pseudo-element",
            &[p(&[
                r":(?:after|before|first-letter|first-line|selection)|::[-\w]+",
            ])],
        ),
        e("pseudo-class", &[p(&[r":[-\w]+"])]),
        e("class", &[p(&[r"\.[-\w]+"])]),
        e("id", &[p(&[r"#[-\w]+"])]),
        e(
            "attribute",
            &[pi(
                &[r#"\[(?:[^\[\]"']|("|')(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1)*\]"#],
                GR,
                &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_ATTRIBUTE,
            )],
        ),
        e(
            "n-th",
            &[
                pi(
                    &[r"(\(\s*)[+-]?\d*[\dn](?:\s*[+-]\s*\d+)?(?=\s*\))"],
                    LB,
                    &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_N_TH_1,
                ),
                pf(&[r"(?i)(\(\s*)(?:even|odd)(?=\s*\))"], LB),
            ],
        ),
        e("combinator", &[p(&[r">|\+|~|\|\|"])]),
        e("punctuation", &[p(&[r"[(),]"])]),
    ]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_ATTRIBUTE: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\[|\]$"])]),
    e("case-sensitivity", &[pa(&[r"(?i)(\s)[si]$"], LB, &["keyword"])]),
    e("namespace", &[pi(&[r"^(\s*)(?:(?!\s)[-*\w\xA0-\x{FFFF}])*\|(?!=)"], LB, &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_ATTRIBUTE_NAMESPACE)]),
    e("attr-name", &[pf(&[r"^(\s*)(?:(?!\s)[-\w\xA0-\x{FFFF}])+"], LB)]),
    e("attr-value", &[p(&[r#"("|')(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#]), pf(&[r"(=\s*)(?:(?!\s)[-\w\xA0-\x{FFFF}])+(?=\s*$)"], LB)]),
    e("operator", &[p(&[r"[|\~*^$]?="])]),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_ATTRIBUTE_NAMESPACE: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"\|$"])]),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE_SELECTOR_FUNCTION_ARGUMENT_N_TH_1:
    Grammar = Grammar::new(&[
    e("number", &[p(&[r"[\dn]+"])]),
    e("operator", &[p(&[r"[+-]"])]),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_URL: Grammar = Grammar::new(&[
    e("function", &[p(&[r"(?i)^url"])]),
    e("punctuation", &[p(&[r"^\(|\)$"])]),
    e(
        "string",
        &[pa(
            &[r#"^(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')$"#],
            0,
            &["url"],
        )],
    ),
]);

pub(super) static JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_COLOR_2: Grammar = Grammar::new(&[
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e("function", &[p(&[r"[\w-]+(?=\()"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);

pub(super) static JSX_TAG: Grammar = Grammar::new(&[
    e("tag", &[pi(&[r"^</?[^\s>/]*"], 0, &JSX_TAG_TAG)]),
    e(
        "script",
        &[pai(
            &[r"=(?:\{(?:\{(?:\{[^{}]*\}|[^{}])*\}|[^{}])*\})"],
            0,
            &["language-javascript"],
            &JSX_TAG_SCRIPT,
        )],
    ),
    e(
        "special-attr",
        &[
            pi(
                &[
                    r#"(?i)(^|["'\s])(?:on(?:abort|blur|change|click|composition(?:end|start|update)|dblclick|"#,
                    r"error|focus(?:in|out)?|key(?:down|up)|load|mouse(?:down|enter|leave|move|out|over|up)|",
                    r#"reset|resize|scroll|select|slotchange|submit|unload|wheel))\s*=\s*(?:"[^"]*"|'[^']*'|"#,
                    r#"[^\s'">=]+(?=[\s>]))"#,
                ],
                LB,
                &JSX_TAG_SPECIAL_ATTR_1,
            ),
            pi(
                &[r#"(?i)(^|["'\s])(?:style)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))"#],
                LB,
                &JSX_TAG_SPECIAL_ATTR_2,
            ),
        ],
    ),
    e(
        "attr-value",
        &[pi(
            &[r#"=(?!\{)(?:"(?:\\[\s\S]|[^\\"])*"|'(?:\\[\s\S]|[^\\'])*'|[^\s'">]+)"#],
            0,
            &JSX_TAG_ATTR_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"/?>"])]),
    e(
        "spread",
        &[pi(
            &[
                r"(?:\{(?:\s|//.*(?!.)|/\*(?:[^*]|\*(?!/))\*/)*\.{3}(?:[^{}]|(?:\{(?:\{(?:\{[^{}]*\}|",
                r"[^{}])*\}|[^{}])*\}))*\})",
            ],
            0,
            &JSX,
        )],
    ),
    e("attr-name", &[pi(&[r"[^\s>/]+"], 0, &JSX_TAG_ATTR_NAME)]),
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
]);

pub(super) static JSX_TAG_TAG: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^</?"])]),
    e("namespace", &[p(&[r"^[^\s>/:]+:"])]),
    e("class-name", &[p(&[r"^[A-Z]\w*(?:\.[A-Z]\w*)*$"])]),
]);

pub(super) static JSX_TAG_SCRIPT: Grammar = Grammar::with_rest(
    &[e(
        "script-punctuation",
        &[pa(&[r"^=(?=\{)"], 0, &["punctuation"])],
    )],
    &JSX,
);

pub(super) static JSX_TAG_SPECIAL_ATTR_1: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(&[r"=[\s\S]+"], 0, &JSX_TAG_SPECIAL_ATTR_1_ATTR_VALUE)],
    ),
]);

pub(super) static JSX_TAG_SPECIAL_ATTR_1_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["javascript", "language-javascript"],
            &JSX_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static JSX_TAG_SPECIAL_ATTR_2: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(&[r"=[\s\S]+"], 0, &JSX_TAG_SPECIAL_ATTR_2_ATTR_VALUE)],
    ),
]);

pub(super) static JSX_TAG_SPECIAL_ATTR_2_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["css", "language-css"],
            &JSX_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static JSX_TAG_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "punctuation",
        &[
            pa(&[r"^="], 0, &["attr-equals"]),
            pf(&[r#"^(\s*)["']|["']$"#], LB),
        ],
    ),
    e(
        "entity",
        &[
            pa(&[r"(?i)&[\da-z]{1,8};"], 0, &["named-entity"]),
            p(&[r"(?i)&#x?[\da-f]{1,8};"]),
        ],
    ),
]);

pub(super) static JSX_TAG_ATTR_NAME: Grammar =
    Grammar::new(&[e("namespace", &[p(&[r"^[^\s>/:]+:"])])]);

pub(super) static JSX_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pi(
            &[
                r"(@(?:arg|argument|param|property)\s+(?:\{(?:[^{}]|\{(?:[^{}]|",
                r"\{[^{}]*\})*\})+\}\s+)?)(?:(?!\s)[$\w\xA0-\x{FFFF}.])+(?=\s|$)",
            ],
            LB,
            &JSX_DOC_COMMENT_PARAMETER,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER,
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
                &JSX_DOC_COMMENT_CLASS_NAME_1,
            ),
            pi(
                &[r"(@[a-z]+\s+)\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})+\}"],
                LB,
                &JSX_DOC_COMMENT_CLASS_NAME_2,
            ),
        ],
    ),
    e(
        "example",
        &[pi(
            &[r"(@example\s+(?!\s))(?:[^@\s]|\s+(?!\s))+?(?=\s*(?:\*\s*)?(?:@\w|\*/))"],
            LB,
            &JSX_DOC_COMMENT_EXAMPLE,
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

pub(super) static JSX_DOC_COMMENT_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pi(
            &[r"(^\[)[$\w\xA0-\x{FFFF}\.]+"],
            LB,
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER,
        )],
    ),
    e(
        "code",
        &[pai(
            &[r"(=)[\s\S]*(?=\]$)"],
            LB,
            &["language-javascript"],
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
        )],
    ),
    e("punctuation", &[p(&[r"[=\[\]]"])]),
]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &JSX_DOC_COMMENT,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_1,
            ),
            pi(
                &[
                    r"((?:\bhtml|\.\s*(?:inner|outer)HTML\s*\+?=)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_2,
            ),
            pi(
                &[
                    r"((?:\bsvg)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_3,
            ),
            pi(
                &[
                    r"((?:\b(?:markdown|md))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|",
                    r"(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_4,
            ),
            pi(
                &[
                    r"((?:\b(?:gql|graphql(?:\s*\.\s*experimental)?))\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_5,
            ),
            pi(
                &[
                    r"((?:\bsql)\s*)`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`",
                ],
                LB | GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_6,
            ),
            pi(
                &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
                GR,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_7,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_CLASS_NAME_1,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_REGEX,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD_VARIABLE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION_VARIABLE,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION,
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
            &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_PROPERTY_ACCESS,
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

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_1: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["css"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_2: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["html"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_3: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["svg"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_4: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["markdown"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_5: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["graphql"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_6: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e("embedded-code", &[pa(&[r"[\s\S]+"], 0, &["sql"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_7: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e(
            "interpolation",
            &[pi(
                &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
                LB,
                &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_7_INTERPOLATION,
            )],
        ),
        e("string", &[p(&[r"[\s\S]+"])]),
    ]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_TEMPLATE_STRING_7_INTERPOLATION: Grammar =
    Grammar::with_rest(
        &[e(
            "interpolation-punctuation",
            &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
        )],
        &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
    );

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_REGEX: Grammar = Grammar::new(&[
    e(
        "regex-source",
        &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
    ),
    e("regex-delimiter", &[p(&[r"^/|/$"])]),
    e("regex-flags", &[p(&[r"^[a-z]+$"])]),
]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION_VARIABLE: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_METHOD: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_FUNCTION: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE_PROPERTY_ACCESS: Grammar =
    Grammar::new(&[e("maybe-class-name", &[p(&[r"^[A-Z][\s\S]*"])])]);

pub(super) static JSX_DOC_COMMENT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JSX_DOC_COMMENT_CLASS_NAME_2: Grammar = Grammar::new(&[
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

pub(super) static JSX_DOC_COMMENT_EXAMPLE: Grammar = Grammar::new(&[e(
    "code",
    &[pai(
        &[r"(?m)^([\t ]*(?:\*\s*)?)\S.*$"],
        LB,
        &["language-javascript"],
        &JSX_DOC_COMMENT_OPTIONAL_PARAMETER_CODE,
    )],
)]);
