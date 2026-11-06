//! Ruby (`rb`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-ruby.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static RUBY: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(?m)#.*|^=begin\s[\s\S]*?^=end"], GR)]),
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
                &RUBY_STRING_LITERAL_1,
            ),
            pi(
                &[r#"("|')(?:#\{[^}]+\}|#(?!\{)|\\(?:\r\n|[\s\S])|(?!\1)[^\\#\r\n])*\1"#],
                GR,
                &RUBY_STRING_LITERAL_2,
            ),
            pai(
                &[r"(?i)<<[-\~]?([a-z_]\w*)[\r\n](?:.*[\r\n])*?[\t ]*\1"],
                GR,
                &["heredoc-string"],
                &RUBY_STRING_LITERAL_3,
            ),
            pai(
                &[r"(?i)<<[-\~]?'([a-z_]\w*)'[\r\n](?:.*[\r\n])*?[\t ]*\1"],
                GR,
                &["heredoc-string"],
                &RUBY_STRING_LITERAL_4,
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
                &RUBY_COMMAND_LITERAL_1,
            ),
            pi(
                &[r"`(?:#\{[^}]+\}|#(?!\{)|\\(?:\r\n|[\s\S])|[^\\`#\r\n])*`"],
                GR,
                &RUBY_COMMAND_LITERAL_2,
            ),
        ],
    ),
    e(
        "class-name",
        &[pi(
            &[r"(\b(?:class|module)\s+|\bcatch\s+\()[\w.\\]+|\b[A-Z_]\w*(?=\s*\.\s*new\b)"],
            LB,
            &RUBY_CLASS_NAME,
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
                &RUBY_REGEX_LITERAL_1,
            ),
            pi(
                &[
                    r"(^|[^/])/(?!/)(?:\[[^\r\n\]]+\]|\\.|[^\[/\\\r\n])+/[egimnosux]{0,6}(?=\s*(?:$|",
                    r"[\r\n,.;})#]))",
                ],
                LB | GR,
                &RUBY_REGEX_LITERAL_2,
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
            &RUBY_METHOD_DEFINITION,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:BEGIN|END|alias|and|begin|break|case|class|def|define_method|defined|do|each|else|",
            r"elsif|end|ensure|extend|for|if|in|include|module|new|next|nil|not|or|prepend|private|",
            r"protected|public|raise|redo|require|rescue|retry|return|self|super|then|throw|undef|unless|",
            r"until|when|while|yield)\b",
        ])],
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
            r"(?i)\b0x[\da-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e("double-colon", &[pa(&[r"::"], 0, &["punctuation"])]),
    e(
        "operator",
        &[p(&[
            r"\.{2,3}|&\.|===|<?=>|[!=]?~|(?:&&|\|\||<<|>>|\*\*|[+\-*/%<>!^\&|=])=?|[?:]",
        ])],
    ),
    e("punctuation", &[p(&[r"[(){}\[\].,;]"])]),
]);

pub(super) static RUBY_STRING_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_STRING_LITERAL_1_INTERPOLATION: Grammar = Grammar::new(&[
    e("content", &[pi(&[r"^(#\{)[\s\S]+(?=\}$)"], LB, &RUBY)]),
    e("delimiter", &[pa(&[r"^#\{|\}$"], 0, &["punctuation"])]),
]);

pub(super) static RUBY_STRING_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_STRING_LITERAL_3: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pi(
            &[r"(?i)^<<[-\~]?[a-z_]\w*|\b[a-z_]\w*$"],
            0,
            &RUBY_STRING_LITERAL_3_DELIMITER,
        )],
    ),
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_STRING_LITERAL_3_DELIMITER: Grammar = Grammar::new(&[
    e("symbol", &[p(&[r"\b\w+"])]),
    e("punctuation", &[p(&[r"^<<[-\~]?"])]),
]);

pub(super) static RUBY_STRING_LITERAL_4: Grammar = Grammar::new(&[
    e(
        "delimiter",
        &[pi(
            &[r"(?i)^<<[-\~]?'[a-z_]\w*'|\b[a-z_]\w*$"],
            0,
            &RUBY_STRING_LITERAL_4_DELIMITER,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_STRING_LITERAL_4_DELIMITER: Grammar = Grammar::new(&[
    e("symbol", &[p(&[r"\b\w+"])]),
    e("punctuation", &[p(&[r"^<<[-\~]?'|'$"])]),
]);

pub(super) static RUBY_COMMAND_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("command", &[pa(&[r"[\s\S]+"], 0, &["string"])]),
]);

pub(super) static RUBY_COMMAND_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("command", &[pa(&[r"[\s\S]+"], 0, &["string"])]),
]);

pub(super) static RUBY_CLASS_NAME: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static RUBY_REGEX_LITERAL_1: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("regex", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_REGEX_LITERAL_2: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\])(?:\\{2})*)#\{(?:[^{}]|\{[^{}]*\})*\}"],
            LB,
            &RUBY_STRING_LITERAL_1_INTERPOLATION,
        )],
    ),
    e("regex", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static RUBY_METHOD_DEFINITION: Grammar = Grammar::new(&[
    e("function", &[p(&[r"\b\w+$"])]),
    e("keyword", &[p(&[r"^self\b"])]),
    e("class-name", &[p(&[r"^\w+"])]),
    e("punctuation", &[p(&[r"\."])]),
]);
