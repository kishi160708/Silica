//! Crystal.
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-ruby.js` and `prism-crystal.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static CRYSTAL: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(?m)#.*|^=begin\s[\s\S]*?^=end"], GR)]),
    e("attribute", &[pi(&[r"@\[.*?\]"], 0, &CRYSTAL_ATTRIBUTE)]),
    e(
        "expansion",
        &[pi(&[r"\{(?:\{.*?\}|%.*?%)\}"], 0, &CRYSTAL_EXPANSION)],
    ),
    e(
        "char",
        &[pf(
            &[r"'(?:[^\\\r\n]{1,2}|\\(?:.|u(?:[A-Fa-f0-9]{1,4}|\{[A-Fa-f0-9]{1,6}\})))'"],
            GR,
        )],
    ),
    e(
        "string-literal",
        &[
            pi(
                &[
                    r"%[qQiIwWs]?(?:([^a-zA-Z0-9\s{(\[<=])(?:(?!\1)[^\\]|\\[\s\S])*\1|\((?:[^()\\]|\\[\s\S]|",
                    r"\((?:[^()\\]|\\[\s\S])*\))*\)|\{(?:[^{}\\]|\\[\s\S]|\{(?:[^{}\\]|\\[\s\S])*\})*\}|",
                    r"\[(?:[^\[\]\\]|\\[\s\S]|\[(?:[^\[\]\\]|\\[\s\S])*\])*\]|<(?:[^<>\\]|\\[\s\S]|<(?:[^<>\\]|",
                    r"\\[\s\S])*>)*>)",
                ],
                GR,
                &CRYSTAL_STRING_LITERAL_1,
            ),
            pi(
                &[r#"("|')(?:#\{[^}]+\}|#(?!\{)|\\(?:\r\n|[\s\S])|(?!\1)[^\\#\r\n])*\1"#],
                GR,
                &CRYSTAL_STRING_LITERAL_2,
            ),
            pai(
                &[r"(?i)<<[-\~]?([a-z_]\w*)[\r\n](?:.*[\r\n])*?[\t ]*\1"],
                GR,
                &["heredoc-string"],
                &CRYSTAL_STRING_LITERAL_3,
            ),
            pai(
                &[r"(?i)<<[-\~]?'([a-z_]\w*)'[\r\n](?:.*[\r\n])*?[\t ]*\1"],
                GR,
                &["heredoc-string"],
                &CRYSTAL_STRING_LITERAL_4,
            ),
        ],
    ),
    e(
        "command-literal",
        &[
            pi(
                &[
                    r"%x(?:([^a-zA-Z0-9\s{(\[<=])(?:(?!\1)[^\\]|\\[\s\S])*\1|\((?:[^()\\]|\\[\s\S]|\((?:[^()\\]|",
                    r"\\[\s\S])*\))*\)|\{(?:[^{}\\]|\\[\s\S]|\{(?:[^{}\\]|\\[\s\S])*\})*\}|\[(?:[^\[\]\\]|",
                    r"\\[\s\S]|\[(?:[^\[\]\\]|\\[\s\S])*\])*\]|<(?:[^<>\\]|\\[\s\S]|<(?:[^<>\\]|\\[\s\S])*>)*>)",
                ],
                GR,
                &CRYSTAL_COMMAND_LITERAL_1,
            ),
            pi(
                &[r"`(?:#\{[^}]+\}|#(?!\{)|\\(?:\r\n|[\s\S])|[^\\`#\r\n])*`"],
                GR,
                &CRYSTAL_COMMAND_LITERAL_2,
            ),
        ],
    ),
    e(
        "class-name",
        &[pi(
            &[r"(\b(?:class|module)\s+|\bcatch\s+\()[\w.\\]+|\b[A-Z_]\w*(?=\s*\.\s*new\b)"],
            LB,
            &CRYSTAL_CLASS_NAME,
        )],
    ),
    e(
        "regex-literal",
        &[
            pi(
                &[
                    r"%r(?:([^a-zA-Z0-9\s{(\[<=])(?:(?!\1)[^\\]|\\[\s\S])*\1|\((?:[^()\\]|\\[\s\S]|\((?:[^()\\]|",
                    r"\\[\s\S])*\))*\)|\{(?:[^{}\\]|\\[\s\S]|\{(?:[^{}\\]|\\[\s\S])*\})*\}|\[(?:[^\[\]\\]|",
                    r"\\[\s\S]|\[(?:[^\[\]\\]|\\[\s\S])*\])*\]|<(?:[^<>\\]|\\[\s\S]|<(?:[^<>\\]|",
                    r"\\[\s\S])*>)*>)[egimnosux]{0,6}",
                ],
                GR,
                &CRYSTAL_REGEX_LITERAL_1,
            ),
            pi(
                &[
                    r"(^|[^/])/(?!/)(?:\[[^\r\n\]]+\]|\\.|[^\[/\\\r\n])+/[egimnosux]{0,6}(?=\s*(?:$|",
                    r"[\r\n,.;})#]))",
                ],
                LB | GR,
                &CRYSTAL_REGEX_LITERAL_2,
            ),
        ],
    ),
    e("variable", &[p(&[r"[@$]+[a-zA-Z_]\w*(?:[?!]|\b)"])]),
    e(
        "symbol",
        &[
            pf(
                &[
                    r#"(^|[^:]):(?:"(?:\\.|[^"\\\r\n])*"|(?:\b[a-zA-Z_]\w*|[^\s\x00-\x7F]+)[?!]?|\$.)"#,
                ],
                LB | GR,
            ),
            pf(
                &[
                    r#"([\r\n{(,][ \t]*)(?:"(?:\\.|[^"\\\r\n])*"|(?:\b[a-zA-Z_]\w*|[^\s\x00-\x7F]+)[?!]?|"#,
                    r"\$.)(?=:(?!:))",
                ],
                LB | GR,
            ),
        ],
    ),
    e(
        "method-definition",
        &[pi(
            &[r"(\bdef\s+)\w+(?:\s*\.\s*\w+)?"],
            LB,
            &CRYSTAL_METHOD_DEFINITION,
        )],
    ),
    e(
        "keyword",
        &[
            p(&[
                r"\b(?:__DIR__|__END_LINE__|__FILE__|__LINE__|abstract|alias|annotation|as|asm|begin|break|",
                r"case|class|def|do|else|elsif|end|ensure|enum|extend|for|fun|if|ifdef|include|",
                r"instance_sizeof|lib|macro|module|next|of|out|pointerof|private|protected|ptr|require|",
                r"rescue|return|select|self|sizeof|struct|super|then|type|typeof|undef|uninitialized|union|",
                r"unless|until|when|while|with|yield)\b",
            ]),
            pf(&[r"(\.\s*)(?:is_a|responds_to)\?"], LB),
        ],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:Array|Bignum|Binding|Class|Continuation|Dir|Exception|FalseClass|File|Fixnum|Float|",
            r"Hash|IO|Integer|MatchData|Method|Module|NilClass|Numeric|Object|Proc|Range|Regexp|Stat|",
            r"String|Struct|Symbol|TMS|Thread|ThreadGroup|Time|TrueClass)\b",
        ])],
    ),
    e("constant", &[p(&[r"\b[A-Z][A-Z0-9_]*(?:[?!]|\b)"])]),
    e(
        "number",
        &[p(&[
            r"\b(?:0b[01_]*[01]|0o[0-7_]*[0-7]|0x[\da-fA-F_]*[\da-fA-F]|",
            r"(?:\d(?:[\d_]*\d)?)(?:\.[\d_]*\d)?(?:[eE][+-]?[\d_]*\d)?)(?:_(?:[uif](?:8|16|32|64))?)?\b",
        ])],
    ),
    e("double-colon", &[pa(&[r"::"], 0, &["punctuation"])]),
    e(
        "operator",
        &[
            p(&[r"->"]),
            p(&[r"\.{2,3}|&\.|===|<?=>|[!=]?~|(?:&&|\|\||<<|>>|\*\*|[+\-*/%<>!^\&|=])=?|[?:]"]),
        ],
    ),
    e("punctuation", &[p(&[r"[(){}\[\].,;\\]"])]),
]);

pub(super) static CRYSTAL_ATTRIBUTE: Grammar = Grammar::new(&[
    e("delimiter", &[pa(&[r"^@\[|\]$"], 0, &["punctuation"])]),
    e("attribute", &[pa(&[r"^(\s*)\w+"], LB, &["class-name"])]),
    e("args", &[pi(&[r"\S(?:[\s\S]*\S)?"], 0, &CRYSTAL)]),
]);

pub(super) static CRYSTAL_EXPANSION: Grammar = Grammar::new(&[
    e("content", &[pi(&[r"^(\{.)[\s\S]+(?=.\}$)"], LB, &CRYSTAL)]),
    e(
        "delimiter",
        &[pa(&[r"^\{[\{%]|[\}%]\}$"], 0, &["operator"])],
    ),
]);

pub(super) static CRYSTAL_STRING_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_1_INTERPOLATION: Grammar = Grammar::new(&[
    e("content", &[pi(&[r"^(#\{)[\s\S]+(?=\}$)"], LB, &CRYSTAL)]),
    e("delimiter", &[pa(&[r"^#\{|\}$"], 0, &["punctuation"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_3: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pi(
            &[r"(?i)^<<[-\~]?[a-z_]\w*|\b[a-z_]\w*$"],
            0,
            &CRYSTAL_STRING_LITERAL_3_DELIMITER,
        )],
    ),
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_3_DELIMITER: Grammar = Grammar::new(&[
    e("symbol", &[p(&[r"\b\w+"])]),
    e("punctuation", &[p(&[r"^<<[-\~]?"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_4: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pi(
            &[r"(?i)^<<[-\~]?'[a-z_]\w*'|\b[a-z_]\w*$"],
            0,
            &CRYSTAL_STRING_LITERAL_4_DELIMITER,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_STRING_LITERAL_4_DELIMITER: Grammar = Grammar::new(&[
    e("symbol", &[p(&[r"\b\w+"])]),
    e("punctuation", &[p(&[r"^<<[-\~]?'|'$"])]),
]);

pub(super) static CRYSTAL_COMMAND_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("command", &[pa(&[r"[\s\S]+"], 0, &["string"])]),
]);

pub(super) static CRYSTAL_COMMAND_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("command", &[pa(&[r"[\s\S]+"], 0, &["string"])]),
]);

pub(super) static CRYSTAL_CLASS_NAME: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static CRYSTAL_REGEX_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("regex", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_REGEX_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &CRYSTAL_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("regex", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static CRYSTAL_METHOD_DEFINITION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"\b\w+$"])]),
    e("keyword", &[p(&[r"^self\b"])]),
    e("class-name", &[p(&[r"^\w+"])]),
    e("punctuation", &[p(&[r"\."])]),
]);
