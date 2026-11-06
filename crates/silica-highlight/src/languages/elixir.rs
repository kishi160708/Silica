//! Elixir.
//!
//! Ported from Prism 1.29.0's `prism-elixir.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static ELIXIR: Grammar = Grammar::new(&[
    e(
        "doc",
        &[pi(
            &[
                r#"@(?:doc|moduledoc)\s+(?:("""|''')[\s\S]*?\1|("|')(?:\\(?:\r\n|[\s\S])|(?!\2)[^\\\r\n])*\2)"#,
            ],
            0,
            &ELIXIR_DOC,
        )],
    ),
    e("comment", &[pf(&[r"#.*"], GR)]),
    e(
        "regex",
        &[pf(
            &[
                r#"~[rR](?:("""|''')(?:\\[\s\S]|(?!\1)[^\\])+\1|([/|"'])(?:\\.|(?!\2)[^\\\r\n])+\2|\((?:\\.|"#,
                r"[^\\)\r\n])+\)|\[(?:\\.|[^\\\]\r\n])+\]|\{(?:\\.|[^\\}\r\n])+\}|<(?:\\.|",
                r"[^\\>\r\n])+>)[uismxfr]*",
            ],
            GR,
        )],
    ),
    e(
        "string",
        &[
            pi(
                &[
                    r#"~[cCsSwW](?:("""|''')(?:\\[\s\S]|(?!\1)[^\\])+\1|([/|"'])(?:\\.|(?!\2)[^\\\r\n])+\2|"#,
                    r"\((?:\\.|[^\\)\r\n])+\)|\[(?:\\.|[^\\\]\r\n])+\]|\{(?:\\.|#\{[^}]+\}|#(?!\{)|",
                    r"[^#\\}\r\n])+\}|<(?:\\.|[^\\>\r\n])+>)[csa]?",
                ],
                GR,
                &ELIXIR_STRING_1,
            ),
            pi(&[r#"("""|''')[\s\S]*?\1"#], GR, &ELIXIR_STRING_2),
            pi(
                &[r#"("|')(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
                GR,
                &ELIXIR_STRING_3,
            ),
        ],
    ),
    e("atom", &[pa(&[r"(^|[^:]):\w+"], LB, &["symbol"])]),
    e("module", &[pa(&[r"\b[A-Z]\w*\b"], 0, &["class-name"])]),
    e("attr-name", &[p(&[r"\b\w+\??:(?!:)"])]),
    e("argument", &[pa(&[r"(^|[^\&])&\d+"], LB, &["variable"])]),
    e("attribute", &[pa(&[r"@\w+"], 0, &["variable"])]),
    e(
        "function",
        &[p(&[r"\b[_a-zA-Z]\w*[?!]?(?:(?=\s*(?:\.\s*)?\()|(?=/\d))"])],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b(?:0[box][a-f\d_]+|\d[\d_]*)(?:\.[\d_]+)?(?:e[+-]?[\d_]+)?\b",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:after|alias|and|case|catch|cond|def(?:callback|delegate|exception|impl|macro|module|n|",
            r"np|p|protocol|struct)?|do|else|end|fn|for|if|import|not|or|quote|raise|require|rescue|try|",
            r"unless|unquote|use|when)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|nil|true)\b"])]),
    e(
        "operator",
        &[
            p(&[
                r"\bin\b|&&?|\|[|>]?|\\\\|::|\.\.\.?|\+\+?|-[->]?|<[-=>]|>=|!==?|\B!|=(?:==?|[>\~])?|[*/^]",
            ]),
            pf(&[r"([^<])<(?!<)"], LB),
            pf(&[r"([^>])>(?!>)"], LB),
        ],
    ),
    e("punctuation", &[p(&[r"<<|>>|[.,%\[\]{}()]"])]),
]);

pub(super) static ELIXIR_DOC: Grammar = Grammar::new(&[
    e("attribute", &[p(&[r"^@\w+"])]),
    e("string", &[p(&[r#"['"][\s\S]+"#])]),
]);

pub(super) static ELIXIR_STRING_1: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(&[r"#\{[^}]+\}"], 0, &ELIXIR_STRING_1_INTERPOLATION)],
)]);

pub(super) static ELIXIR_STRING_1_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e("delimiter", &[pa(&[r"^#\{|\}$"], 0, &["punctuation"])])],
    &ELIXIR,
);

pub(super) static ELIXIR_STRING_2: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(&[r"#\{[^}]+\}"], 0, &ELIXIR_STRING_2_INTERPOLATION)],
)]);

pub(super) static ELIXIR_STRING_2_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e("delimiter", &[pa(&[r"^#\{|\}$"], 0, &["punctuation"])])],
    &ELIXIR,
);

pub(super) static ELIXIR_STRING_3: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(&[r"#\{[^}]+\}"], 0, &ELIXIR_STRING_3_INTERPOLATION)],
)]);

pub(super) static ELIXIR_STRING_3_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e("delimiter", &[pa(&[r"^#\{|\}$"], 0, &["punctuation"])])],
    &ELIXIR,
);
