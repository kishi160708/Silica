//! PHP.
//!
//! Ported from Prism 1.29.0's `prism-markup.js`, `prism-markup-templating.js`, `prism-php.js`,
//! `prism-css.js`, `prism-javascript.js`, `prism-actionscript.js`,
//! `prism-javadoclike.js`, `prism-phpdoc.js` and `prism-php-extras.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static PHP: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pa(&[r"(?i)\?>$|^<\?(?:php(?=\s)|=)?"], 0, &["important"])],
    ),
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &PHP_DOC_COMMENT,
        )],
    ),
    e("comment", &[p(&[r"/\*[\s\S]*?\*/|//.*|#(?!\[).*"])]),
    e(
        "string",
        &[
            pai(
                &[r"<<<'([^']+)'[\r\n](?:.*[\r\n])*?\1;"],
                GR,
                &["nowdoc-string"],
                &PHP_STRING_1,
            ),
            pai(
                &[
                    r#"(?i)<<<(?:"([^"]+)"[\r\n](?:.*[\r\n])*?\1;|([a-z_]\w*)[\r\n](?:.*[\r\n])*?\2;)"#,
                ],
                GR,
                &["heredoc-string"],
                &PHP_STRING_2,
            ),
            pa(
                &[r"`(?:\\[\s\S]|[^\\`])*`"],
                GR,
                &["backtick-quoted-string"],
            ),
            pa(&[r"'(?:\\[\s\S]|[^\\'])*'"], GR, &["single-quoted-string"]),
            pai(
                &[r#""(?:\\[\s\S]|[^\\"])*""#],
                GR,
                &["double-quoted-string"],
                &PHP_STRING_5,
            ),
        ],
    ),
    e(
        "attribute",
        &[pi(
            &[
                r#"(?im)#\[(?:[^"'/#]|/(?![*/])|//.*$|#(?!\[).*$|/\*(?:[^*]|\*(?!/))*\*/|"(?:\\[\s\S]|"#,
                r#"[^\\"])*"|'(?:\\[\s\S]|[^\\'])*')+\](?=\s*[a-z$#])"#,
            ],
            GR,
            &PHP_ATTRIBUTE,
        )],
    ),
    e("this", &[pa(&[r"\$this\b"], 0, &["keyword"])]),
    e(
        "global",
        &[p(&[
            r"\$(?:GLOBALS|HTTP_RAW_POST_DATA|_(?:COOKIE|ENV|FILES|GET|POST|REQUEST|SERVER|SESSION)|argc|",
            r"argv|http_response_header|php_errormsg)\b",
        ])],
    ),
    e("scope", &[pi(&[r"\b[\w\\]+::"], 0, &PHP_SCOPE)]),
    e("variable", &[p(&[r"\$+(?:\w+\b|(?=\{))"])]),
    e(
        "package",
        &[pi(
            &[r"(?i)(namespace\s+|use\s+(?:function\s+)?)(?:\\?\b[a-z_]\w*)+\b(?!\\)"],
            LB,
            &PHP_PACKAGE,
        )],
    ),
    e(
        "class-name-definition",
        &[pa(
            &[r"(?i)(\b(?:class|enum|interface|trait)\s+)\b[a-z_]\w*(?!\\)\b"],
            LB,
            &["class-name"],
        )],
    ),
    e(
        "function-definition",
        &[pa(
            &[r"(?i)(\bfunction\s+)[a-z_]\w*(?=\s*\()"],
            LB,
            &["function"],
        )],
    ),
    e(
        "keyword",
        &[
            pa(
                &[
                    r"(?i)(\(\s*)\b(?:array|bool|boolean|float|int|integer|object|string)\b(?=\s*\))",
                ],
                LB | GR,
                &["type-casting"],
            ),
            pa(
                &[
                    r"(?i)([(,?]\s*)\b(?:array(?!\s*\()|bool|callable|(?:false|null)(?=\s*\|)|float|int|iterable|",
                    r"mixed|object|self|static|string)\b(?=\s*\$)",
                ],
                LB | GR,
                &["type-hint"],
            ),
            pa(
                &[
                    r"(?i)(\)\s*:\s*(?:\?\s*)?)\b(?:array(?!\s*\()|bool|callable|(?:false|null)(?=\s*\|)|float|",
                    r"int|iterable|mixed|never|object|self|static|string|void)\b",
                ],
                LB | GR,
                &["return-type"],
            ),
            pa(
                &[r"(?i)\b(?:array(?!\s*\()|bool|float|int|iterable|mixed|object|string|void)\b"],
                GR,
                &["type-declaration"],
            ),
            pa(
                &[r"(?i)(\|\s*)(?:false|null)\b|\b(?:false|null)(?=\s*\|)"],
                LB | GR,
                &["type-declaration"],
            ),
            pa(
                &[r"(?i)\b(?:parent|self|static)(?=\s*::)"],
                GR,
                &["static-context"],
            ),
            pf(&[r"(?i)(\byield\s+)from\b"], LB),
            p(&[r"(?i)\bclass\b"]),
            pf(
                &[
                    r"(?i)((?:^|[^\s>:]|(?:^|[^-])>|(?:^|[^:]):)\s*)\b(?:abstract|and|array|as|break|callable|",
                    r"case|catch|clone|const|continue|declare|default|die|do|echo|else|elseif|empty|enddeclare|",
                    r"endfor|endforeach|endif|endswitch|endwhile|enum|eval|exit|extends|final|finally|fn|for|",
                    r"foreach|function|global|goto|if|implements|include|include_once|instanceof|insteadof|",
                    r"interface|isset|list|match|namespace|never|new|or|parent|print|private|protected|public|",
                    r"readonly|require|require_once|return|self|static|switch|throw|trait|try|unset|use|var|",
                    r"while|xor|yield|__halt_compiler)\b",
                ],
                LB,
            ),
        ],
    ),
    e(
        "argument-name",
        &[pf(&[r"(?i)([(,]\s*)\b[a-z_]\w*(?=\s*:(?!:))"], LB)],
    ),
    e(
        "class-name",
        &[
            pf(
                &[
                    r"(?i)(\b(?:extends|implements|instanceof|new(?!\s+self|\s+static))\s+|",
                    r"\bcatch\s*\()\b[a-z_]\w*(?!\\)\b",
                ],
                LB | GR,
            ),
            pf(&[r"(?i)(\|\s*)\b[a-z_]\w*(?!\\)\b"], LB | GR),
            pf(&[r"(?i)\b[a-z_]\w*(?!\\)\b(?=\s*\|)"], GR),
            pai(
                &[r"(?i)(\|\s*)(?:\\?\b[a-z_]\w*)+\b"],
                LB | GR,
                &["class-name-fully-qualified"],
                &PHP_CLASS_NAME_4,
            ),
            pai(
                &[r"(?i)(?:\\?\b[a-z_]\w*)+\b(?=\s*\|)"],
                GR,
                &["class-name-fully-qualified"],
                &PHP_CLASS_NAME_5,
            ),
            pai(
                &[
                    r"(?i)(\b(?:extends|implements|instanceof|new(?!\s+self\b|\s+static\b))\s+|",
                    r"\bcatch\s*\()(?:\\?\b[a-z_]\w*)+\b(?!\\)",
                ],
                LB | GR,
                &["class-name-fully-qualified"],
                &PHP_CLASS_NAME_6,
            ),
            pa(&[r"(?i)\b[a-z_]\w*(?=\s*\$)"], GR, &["type-declaration"]),
            pai(
                &[r"(?i)(?:\\?\b[a-z_]\w*)+(?=\s*\$)"],
                GR,
                &["class-name-fully-qualified", "type-declaration"],
                &PHP_CLASS_NAME_8,
            ),
            pa(&[r"(?i)\b[a-z_]\w*(?=\s*::)"], GR, &["static-context"]),
            pai(
                &[r"(?i)(?:\\?\b[a-z_]\w*)+(?=\s*::)"],
                GR,
                &["class-name-fully-qualified", "static-context"],
                &PHP_CLASS_NAME_10,
            ),
            pa(
                &[r"(?i)([(,?]\s*)[a-z_]\w*(?=\s*\$)"],
                LB | GR,
                &["type-hint"],
            ),
            pai(
                &[r"(?i)([(,?]\s*)(?:\\?\b[a-z_]\w*)+(?=\s*\$)"],
                LB | GR,
                &["class-name-fully-qualified", "type-hint"],
                &PHP_CLASS_NAME_12,
            ),
            pa(
                &[r"(?i)(\)\s*:\s*(?:\?\s*)?)\b[a-z_]\w*(?!\\)\b"],
                LB | GR,
                &["return-type"],
            ),
            pai(
                &[r"(?i)(\)\s*:\s*(?:\?\s*)?)(?:\\?\b[a-z_]\w*)+\b(?!\\)"],
                LB | GR,
                &["class-name-fully-qualified", "return-type"],
                &PHP_CLASS_NAME_14,
            ),
        ],
    ),
    e(
        "constant",
        &[
            pa(&[r"(?i)\b(?:false|true)\b"], 0, &["boolean"]),
            pf(&[r"(?i)(::\s*)\b[a-z_]\w*\b(?!\s*\()"], LB | GR),
            pf(
                &[r"(?i)(\b(?:case|const)\s+)\b[a-z_]\w*(?=\s*[;=])"],
                LB | GR,
            ),
            p(&[r"(?i)\b(?:null)\b"]),
            p(&[r"\b[A-Z_][A-Z0-9_]*\b(?!\s*\()"]),
        ],
    ),
    e(
        "function",
        &[pi(
            &[r"(?i)(^|[^\\\w])\\?[a-z_](?:[\w\\]*\w)?(?=\s*\()"],
            LB,
            &PHP_FUNCTION,
        )],
    ),
    e("property", &[pf(&[r"(->\s*)\w+"], LB)]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0b[01]+(?:_[01]+)*\b|\b0o[0-7]+(?:_[0-7]+)*\b|\b0x[\da-f]+(?:_[\da-f]+)*\b|",
            r"(?:\b\d+(?:_\d+)*\.?(?:\d+(?:_\d+)*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"<?=>|\?\?=?|\.{3}|\??->|[!=]=?=?|::|\*\*=?|--|\+\+|&&|\|\||<<|>>|[?\~]|[/^|%*\&<>.+-]=?",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\](),:;]"])]),
]);

pub(super) static PHP_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pf(
            &[
                r"(@(?:global|param|property(?:-read|-write)?|var)\s+(?:(?:\b[a-zA-Z]\w*|[|",
                r"\\\[\]])+\s+)?)\$\w+",
            ],
            LB,
        )],
    ),
    e(
        "class-name",
        &[pi(
            &[
                r"(@(?:global|package|param|property(?:-read|-write)?|return|subpackage|throws|",
                r"var)\s+)(?:\b[a-zA-Z]\w*|[|\\\[\]])+",
            ],
            LB,
            &PHP_DOC_COMMENT_CLASS_NAME,
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

pub(super) static PHP_DOC_COMMENT_CLASS_NAME: Grammar = Grammar::new(&[
    e(
        "keyword",
        &[p(&[
            r"\b(?:array|bool|boolean|callback|double|false|float|int|integer|mixed|null|object|resource|",
            r"self|string|true|void)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[|\\\[\]()]"])]),
]);

pub(super) static PHP_STRING_1: Grammar = Grammar::new(&[e(
    "delimiter",
    &[pai(
        &[r"(?i)^<<<'[^']+'|[a-z_]\w*;$"],
        0,
        &["symbol"],
        &PHP_STRING_1_DELIMITER,
    )],
)]);

pub(super) static PHP_STRING_1_DELIMITER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"^<<<'?|[';]$"])])]);

pub(super) static PHP_STRING_2: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pai(
            &[r#"(?i)^<<<(?:"[^"]+"|[a-z_]\w*)|[a-z_]\w*;$"#],
            0,
            &["symbol"],
            &PHP_STRING_2_DELIMITER,
        )],
    ),
    e(
        "interpolation",
        &[pi(
            &[
                r"\{\$(?:\{(?:\{[^{}]+\}|[^{}]+)\}|[^{}])+\}|(^|[^\\{])\$+(?:\w+(?:\[[^\r\n\[\]]+\]|->\w+)?)",
            ],
            LB,
            &PHP,
        )],
    ),
]);

pub(super) static PHP_STRING_2_DELIMITER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r#"^<<<"?|[";]$"#])])]);

pub(super) static PHP_STRING_5: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(
        &[
            r"\{\$(?:\{(?:\{[^{}]+\}|[^{}]+)\}|[^{}])+\}|(^|[^\\{])\$+(?:\w+(?:\[[^\r\n\[\]]+\]|->\w+)?)",
        ],
        LB,
        &PHP,
    )],
)]);

pub(super) static PHP_ATTRIBUTE: Grammar = Grammar::new(&[
    e(
        "attribute-content",
        &[pi(
            &[r"^(#\[)[\s\S]+(?=\]$)"],
            LB,
            &PHP_ATTRIBUTE_ATTRIBUTE_CONTENT,
        )],
    ),
    e("delimiter", &[pa(&[r"^#\[|\]$"], 0, &["punctuation"])]),
]);

pub(super) static PHP_ATTRIBUTE_ATTRIBUTE_CONTENT: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"/\*[\s\S]*?\*/|//.*|#(?!\[).*"])]),
    e(
        "string",
        &[
            pai(
                &[r"<<<'([^']+)'[\r\n](?:.*[\r\n])*?\1;"],
                GR,
                &["nowdoc-string"],
                &PHP_STRING_1,
            ),
            pai(
                &[
                    r#"(?i)<<<(?:"([^"]+)"[\r\n](?:.*[\r\n])*?\1;|([a-z_]\w*)[\r\n](?:.*[\r\n])*?\2;)"#,
                ],
                GR,
                &["heredoc-string"],
                &PHP_STRING_2,
            ),
            pa(
                &[r"`(?:\\[\s\S]|[^\\`])*`"],
                GR,
                &["backtick-quoted-string"],
            ),
            pa(&[r"'(?:\\[\s\S]|[^\\'])*'"], GR, &["single-quoted-string"]),
            pai(
                &[r#""(?:\\[\s\S]|[^\\"])*""#],
                GR,
                &["double-quoted-string"],
                &PHP_STRING_5,
            ),
        ],
    ),
    e(
        "attribute-class-name",
        &[
            pa(
                &[r"(?i)([^:]|^)\b[a-z_]\w*(?!\\)\b"],
                LB | GR,
                &["class-name"],
            ),
            pai(
                &[r"(?i)([^:]|^)(?:\\?\b[a-z_]\w*)+"],
                LB | GR,
                &["class-name", "class-name-fully-qualified"],
                &PHP_ATTRIBUTE_ATTRIBUTE_CONTENT_ATTRIBUTE_CLASS_NAME_2,
            ),
        ],
    ),
    e(
        "constant",
        &[
            pa(&[r"(?i)\b(?:false|true)\b"], 0, &["boolean"]),
            pf(&[r"(?i)(::\s*)\b[a-z_]\w*\b(?!\s*\()"], LB | GR),
            pf(
                &[r"(?i)(\b(?:case|const)\s+)\b[a-z_]\w*(?=\s*[;=])"],
                LB | GR,
            ),
            p(&[r"(?i)\b(?:null)\b"]),
            p(&[r"\b[A-Z_][A-Z0-9_]*\b(?!\s*\()"]),
        ],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b0b[01]+(?:_[01]+)*\b|\b0o[0-7]+(?:_[0-7]+)*\b|\b0x[\da-f]+(?:_[\da-f]+)*\b|",
            r"(?:\b\d+(?:_\d+)*\.?(?:\d+(?:_\d+)*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"<?=>|\?\?=?|\.{3}|\??->|[!=]=?=?|::|\*\*=?|--|\+\+|&&|\|\||<<|>>|[?\~]|[/^|%*\&<>.+-]=?",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\](),:;]"])]),
]);

pub(super) static PHP_ATTRIBUTE_ATTRIBUTE_CONTENT_ATTRIBUTE_CLASS_NAME_2: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_SCOPE: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"\b(?:parent|self|static)\b"])]),
    e("punctuation", &[p(&[r"::|\\"])]),
]);

pub(super) static PHP_PACKAGE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_4: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_5: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_6: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_8: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_10: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_12: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_CLASS_NAME_14: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);

pub(super) static PHP_FUNCTION: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\\"])])]);
