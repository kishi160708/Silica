//! Markdown (`md`).
//!
//! Ported from Prism 1.29.0's `prism-markup.js`, `prism-markdown.js`, `prism-css.js`,
//! `prism-javascript.js` and `prism-actionscript.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static MARKDOWN: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"<!--(?:(?!<!--)[\s\S])*?-->"], GR)]),
    e(
        "front-matter-block",
        &[pi(
            &[r"(^(?:\s*[\r\n])?)---(?!.)[\s\S]*?[\r\n]---(?!.)"],
            LB | GR,
            &MARKDOWN_FRONT_MATTER_BLOCK,
        )],
    ),
    e(
        "blockquote",
        &[pa(&[r"(?m)^>(?:[\t ]*>)*"], 0, &["punctuation"])],
    ),
    e(
        "table",
        &[pi(
            &[
                r"(?m)^\|?(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|",
                r"`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S]))\|",
                r"?[ \t]*:?-{3,}:?[ \t]*(?:\|[ \t]*:?-{3,}:?[ \t]*)+\|?(?:\n|\r\n?)(?:\|?(?:\\.|",
                r"``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|`(?!`))+``|",
                r"`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S])))*",
            ],
            0,
            &MARKDOWN_TABLE,
        )],
    ),
    e(
        "code",
        &[
            pa(
                &[
                    r"((?:^|\n)[ \t]*\n|(?:^|\r\n?)[ \t]*\r\n?)(?: {4}|\t).+(?:(?:\n|\r\n?)(?: {4}|\t).+)*",
                ],
                LB,
                &["keyword"],
            ),
            pi(&[r"(?m)^```[\s\S]*?^```$"], GR, &MARKDOWN_CODE_2),
        ],
    ),
    e(
        "title",
        &[
            pai(
                &[r"(?m)\S.*(?:\n|\r\n?)(?:==+|--+)(?=[ \t]*$)"],
                0,
                &["important"],
                &MARKDOWN_TITLE_1,
            ),
            pai(&[r"(?m)(^\s*)#.+"], LB, &["important"], &MARKDOWN_TITLE_2),
        ],
    ),
    e(
        "hr",
        &[pa(
            &[r"(?m)(^\s*)([*-])(?:[\t ]*\2){2,}(?=\s*$)"],
            LB,
            &["punctuation"],
        )],
    ),
    e(
        "list",
        &[pa(
            &[r"(?m)(^\s*)(?:[*+-]|\d+\.)(?=[\t ].)"],
            LB,
            &["punctuation"],
        )],
    ),
    e(
        "url-reference",
        &[pai(
            &[
                r#"!?\[[^\]]+\]:[\t ]+(?:\S+|<(?:\\.|[^>\\])+>)(?:[\t ]+(?:"(?:\\.|[^"\\])*"|'(?:\\.|"#,
                r"[^'\\])*'|\((?:\\.|[^)\\])*\)))?",
            ],
            0,
            &["url"],
            &MARKDOWN_URL_REFERENCE,
        )],
    ),
    e(
        "bold",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+_)+__\b|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*)+\*\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD,
        )],
    ),
    e(
        "italic",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+__)+_\b|\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*\*)+\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC,
        )],
    ),
    e(
        "strike",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:(~~?)(?:(?!~)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\2)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE,
        )],
    ),
    e(
        "code-snippet",
        &[pa(
            &[r"(^|[^\\`])(?:``[^`\r\n]+(?:`[^`\r\n]+)*``(?!`)|`[^`\r\n]+`(?!`))"],
            LB | GR,
            &["code", "keyword"],
        )],
    ),
    e(
        "url",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:!?\[(?:(?!\])(?:\\.|[^\\\n\r]|(?:\n|",
                r#"\r\n?)(?![\r\n])))+\](?:\([^\s)]+(?:[\t ]+"(?:\\.|[^"\\])*")?\)|[ \t]?\[(?:(?!\])(?:\\.|"#,
                r"[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\]))",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL,
        )],
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
            &MARKDOWN_DOCTYPE,
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
            &MARKDOWN_SCRIPT,
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
            &MARKDOWN_STYLE,
        )],
    ),
    e("cdata", &[pf(&[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"], GR)]),
    e(
        "tag",
        &[pi(
            &[
                r#"</?(?!\d)[^\s>/=$<%]+(?:\s(?:\s*[^\s>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))|"#,
                r"(?=[\s/>])))+)?\s*/?>",
            ],
            GR,
            &MARKDOWN_TAG,
        )],
    ),
    e(
        "entity",
        &[
            pa(&[r"(?i)&[\da-z]{1,8};"], 0, &["named-entity"]),
            p(&[r"(?i)&#x?[\da-f]{1,8};"]),
        ],
    ),
]);

pub(super) static MARKDOWN_FRONT_MATTER_BLOCK: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^---|---$"])]),
    e(
        "front-matter",
        &[pai(
            &[r"\S+(?:\s+\S+)*"],
            0,
            &["yaml", "language-yaml"],
            &super::data::YAML,
        )],
    ),
]);

pub(super) static MARKDOWN_TABLE: Grammar = Grammar::new(&[
    e(
        "table-data-rows",
        &[pi(
            &[
                r"^(\|?(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|",
                r"`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S]))\|",
                r"?[ \t]*:?-{3,}:?[ \t]*(?:\|[ \t]*:?-{3,}:?[ \t]*)+\|?(?:\n|\r\n?))(?:\|?(?:\\.|",
                r"``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|`(?!`))+``|",
                r"`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S])))*$",
            ],
            LB,
            &MARKDOWN_TABLE_TABLE_DATA_ROWS,
        )],
    ),
    e(
        "table-line",
        &[pi(
            &[
                r"^(\|?(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|",
                r"`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S])))\|",
                r"?[ \t]*:?-{3,}:?[ \t]*(?:\|[ \t]*:?-{3,}:?[ \t]*)+\|?(?:\n|\r\n?)$",
            ],
            LB,
            &MARKDOWN_TABLE_TABLE_LINE,
        )],
    ),
    e(
        "table-header-row",
        &[pi(
            &[
                r"^\|?(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+(?:\|(?:\\.|``(?:[^`\r\n]|",
                r"`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+)+\|?(?:(?:\n|\r\n?)|(?![\s\S]))$",
            ],
            0,
            &MARKDOWN_TABLE_TABLE_HEADER_ROW,
        )],
    ),
]);

pub(super) static MARKDOWN_TABLE_TABLE_DATA_ROWS: Grammar = Grammar::new(&[
    e(
        "table-data",
        &[pi(
            &[r"(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+"],
            0,
            &MARKDOWN,
        )],
    ),
    e("punctuation", &[p(&[r"\|"])]),
]);

pub(super) static MARKDOWN_TABLE_TABLE_LINE: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\||:?-{3,}:?"])])]);

pub(super) static MARKDOWN_TABLE_TABLE_HEADER_ROW: Grammar = Grammar::new(&[
    e(
        "table-header",
        &[pai(
            &[r"(?:\\.|``(?:[^`\r\n]|`(?!`))+``|`[^`\r\n]+`|[^\\|\r\n`])+"],
            0,
            &["important"],
            &MARKDOWN,
        )],
    ),
    e("punctuation", &[p(&[r"\|"])]),
]);

pub(super) static MARKDOWN_CODE_2: Grammar = Grammar::new(&[
    e(
        "code-block",
        &[pf(
            &[r"(?m)^(```.*(?:\n|\r\n?))[\s\S]+?(?=(?:\n|\r\n?)^```$)"],
            LB,
        )],
    ),
    e("code-language", &[pf(&[r"^(```).+"], LB)]),
    e("punctuation", &[p(&[r"```"])]),
]);

pub(super) static MARKDOWN_TITLE_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"==+$|--+$"])])]);

pub(super) static MARKDOWN_TITLE_2: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"^#+|#+$"])])]);

pub(super) static MARKDOWN_URL_REFERENCE: Grammar = Grammar::new(&[
    e("variable", &[pf(&[r"^(!?\[)[^\]]+"], LB)]),
    e(
        "string",
        &[p(&[
            r#"(?:"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\((?:\\.|[^)\\])*\))$"#,
        ])],
    ),
    e("punctuation", &[p(&[r"^[\[\]!:]|[<>]"])]),
]);

pub(super) static MARKDOWN_BOLD: Grammar = Grammar::new(&[
    e(
        "content",
        &[pi(&[r"(^..)[\s\S]+(?=..$)"], LB, &MARKDOWN_BOLD_CONTENT)],
    ),
    e("punctuation", &[p(&[r"\*\*|__"])]),
]);

pub(super) static MARKDOWN_BOLD_CONTENT: Grammar = Grammar::new(&[
    e(
        "url",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:!?\[(?:(?!\])(?:\\.|[^\\\n\r]|(?:\n|",
                r#"\r\n?)(?![\r\n])))+\](?:\([^\s)]+(?:[\t ]+"(?:\\.|[^"\\])*")?\)|[ \t]?\[(?:(?!\])(?:\\.|"#,
                r"[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\]))",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL,
        )],
    ),
    e(
        "italic",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+__)+_\b|\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*\*)+\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC,
        )],
    ),
    e(
        "strike",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:(~~?)(?:(?!~)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\2)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE,
        )],
    ),
    e(
        "code-snippet",
        &[pa(
            &[r"(^|[^\\`])(?:``[^`\r\n]+(?:`[^`\r\n]+)*``(?!`)|`[^`\r\n]+`(?!`))"],
            LB | GR,
            &["code", "keyword"],
        )],
    ),
]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL: Grammar = Grammar::new(&[
    e("operator", &[p(&[r"^!"])]),
    e(
        "content",
        &[pi(
            &[r"(^\[)[^\]]+(?=\])"],
            LB,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT,
        )],
    ),
    e("variable", &[pf(&[r"(^\][ \t]?\[)[^\]]+(?=\]$)"], LB)]),
    e("url", &[pf(&[r"(^\]\()[^\s)]+"], LB)]),
    e(
        "string",
        &[pf(&[r#"(^[ \t]+)"(?:\\.|[^"\\])*"(?=\)$)"#], LB)],
    ),
]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL_CONTENT: Grammar = Grammar::new(&[
    e(
        "bold",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+_)+__\b|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*)+\*\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD,
        )],
    ),
    e(
        "italic",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+__)+_\b|\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*\*)+\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC,
        )],
    ),
    e(
        "strike",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:(~~?)(?:(?!~)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\2)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE,
        )],
    ),
    e(
        "code-snippet",
        &[pa(
            &[r"(^|[^\\`])(?:``[^`\r\n]+(?:`[^`\r\n]+)*``(?!`)|`[^`\r\n]+`(?!`))"],
            LB | GR,
            &["code", "keyword"],
        )],
    ),
]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC: Grammar = Grammar::new(&[
    e(
        "content",
        &[pi(
            &[r"(^.)[\s\S]+(?=.$)"],
            LB,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT,
        )],
    ),
    e("punctuation", &[p(&[r"[*_]"])]),
]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT: Grammar = Grammar::new(&[
    e(
        "url",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:!?\[(?:(?!\])(?:\\.|[^\\\n\r]|(?:\n|",
                r#"\r\n?)(?![\r\n])))+\](?:\([^\s)]+(?:[\t ]+"(?:\\.|[^"\\])*")?\)|[ \t]?\[(?:(?!\])(?:\\.|"#,
                r"[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\]))",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL,
        )],
    ),
    e(
        "bold",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:\b__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                r"_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+_)+__\b|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                r"(?:\n|\r\n?)(?![\r\n]))|\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*)+\*\*)",
            ],
            LB | GR,
            &MARKDOWN_BOLD,
        )],
    ),
    e(
        "strike",
        &[pi(
            &[
                r"((?:^|[^\\])(?:\\{2})*)(?:(~~?)(?:(?!~)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\2)",
            ],
            LB | GR,
            &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE,
        )],
    ),
    e(
        "code-snippet",
        &[pa(
            &[r"(^|[^\\`])(?:``[^`\r\n]+(?:`[^`\r\n]+)*``(?!`)|`[^`\r\n]+`(?!`))"],
            LB | GR,
            &["code", "keyword"],
        )],
    ),
]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE: Grammar =
    Grammar::new(&[
        e(
            "content",
            &[pi(
                &[r"(^~~?)[\s\S]+(?=\1$)"],
                LB,
                &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE_CONTENT,
            )],
        ),
        e("punctuation", &[p(&[r"~~?"])]),
    ]);

pub(super) static MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC_CONTENT_STRIKE_CONTENT: Grammar =
    Grammar::new(&[
        e(
            "url",
            &[pi(
                &[
                    r"((?:^|[^\\])(?:\\{2})*)(?:!?\[(?:(?!\])(?:\\.|[^\\\n\r]|(?:\n|",
                    r#"\r\n?)(?![\r\n])))+\](?:\([^\s)]+(?:[\t ]+"(?:\\.|[^"\\])*")?\)|[ \t]?\[(?:(?!\])(?:\\.|"#,
                    r"[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\]))",
                ],
                LB | GR,
                &MARKDOWN_BOLD_CONTENT_URL,
            )],
        ),
        e(
            "bold",
            &[pi(
                &[
                    r"((?:^|[^\\])(?:\\{2})*)(?:\b__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                    r"_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+_)+__\b|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                    r"(?:\n|\r\n?)(?![\r\n]))|\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*)+\*\*)",
                ],
                LB | GR,
                &MARKDOWN_BOLD,
            )],
        ),
        e(
            "italic",
            &[pi(
                &[
                    r"((?:^|[^\\])(?:\\{2})*)(?:\b_(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n]))|",
                    r"__(?:(?!_)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+__)+_\b|\*(?:(?!\*)(?:\\.|[^\\\n\r]|",
                    r"(?:\n|\r\n?)(?![\r\n]))|\*\*(?:(?!\*)(?:\\.|[^\\\n\r]|(?:\n|\r\n?)(?![\r\n])))+\*\*)+\*)",
                ],
                LB | GR,
                &MARKDOWN_BOLD_CONTENT_URL_CONTENT_ITALIC,
            )],
        ),
        e(
            "code-snippet",
            &[pa(
                &[r"(^|[^\\`])(?:``[^`\r\n]+(?:`[^`\r\n]+)*``(?!`)|`[^`\r\n]+`(?!`))"],
                LB | GR,
                &["code", "keyword"],
            )],
        ),
    ]);

pub(super) static MARKDOWN_DOCTYPE: Grammar = Grammar::new(&[
    e(
        "internal-subset",
        &[pi(&[r"(^[^\[]*\[)[\s\S]+(?=\]>$)"], LB | GR, &MARKDOWN)],
    ),
    e("string", &[pf(&[r#""[^"]*"|'[^']*'"#], GR)]),
    e("punctuation", &[p(&[r"^<!|>$|[\[\]]"])]),
    e("doctype-tag", &[p(&[r"(?i)^DOCTYPE"])]),
    e("name", &[p(&[r#"[^\s<>'"]+"#])]),
]);

pub(super) static MARKDOWN_SCRIPT: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &MARKDOWN_SCRIPT_INCLUDED_CDATA,
        )],
    ),
    e(
        "language-javascript",
        &[pi(
            &[r"[\s\S]+"],
            0,
            &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
]);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-javascript",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT: Grammar = Grammar::new(&[
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
        &[pi(
            &[r"`(?:\\[\s\S]|\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}|(?!\$\{)[^\\`])*`"],
            GR,
            &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING,
        )],
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
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_CLASS_NAME_1,
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
            &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_REGEX,
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
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
            ),
            pi(
                &[
                    r"(?i)(^|[^$\w\xA0-\x{FFFF}])(?!\s)[_$a-z\xA0-\x{FFFF}](?:(?!\s)[$\w\xA0-\x{FFFF}])*(?=\s*=>",
                    r")",
                ],
                LB,
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
            ),
            pi(
                &[r"(\(\s*)(?!\s)(?:[^()\s]|\s+(?![\s)])|\([^()]*\))+(?=\s*\)\s*=>)"],
                LB,
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
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
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
            ),
        ],
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
]);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING: Grammar =
    Grammar::new(&[
        e("template-punctuation", &[pa(&[r"^`|`$"], 0, &["string"])]),
        e(
            "interpolation",
            &[pi(
                &[r"((?:^|[^\\])(?:\\{2})*)\$\{(?:[^{}]|\{(?:[^{}]|\{[^}]*\})*\})+\}"],
                LB,
                &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_INTERPOLATION,
            )],
        ),
        e("string", &[p(&[r"[\s\S]+"])]),
    ]);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_TEMPLATE_STRING_INTERPOLATION: Grammar = Grammar::with_rest(
    &[
    e("interpolation-punctuation", &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])]),
    ],
    &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_CLASS_NAME_1: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);

pub(super) static MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT_REGEX: Grammar =
    Grammar::new(&[
        e(
            "regex-source",
            &[pa(&[r"^(/)[\s\S]+(?=/[a-z]*$)"], LB, &["language-regex"])],
        ),
        e("regex-delimiter", &[p(&[r"^/|/$"])]),
        e("regex-flags", &[p(&[r"^[a-z]+$"])]),
    ]);

pub(super) static MARKDOWN_STYLE: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &MARKDOWN_STYLE_INCLUDED_CDATA,
        )],
    ),
    e(
        "language-css",
        &[pi(
            &[r"[\s\S]+"],
            0,
            &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
        )],
    ),
]);

pub(super) static MARKDOWN_STYLE_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-css",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"/\*[\s\S]*?\*/"])]),
    e(
        "atrule",
        &[pi(
            &[
                r#"@[\w-](?:[^;{\s"']|\s+(?!\s)|(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|"#,
                r"[^'\\\r\n])*'))*?(?:;|(?=\s*\{))",
            ],
            0,
            &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE,
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
            &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_URL,
        )],
    ),
    e(
        "selector",
        &[pf(
            &[
                r#"(^|[{}\s])[^{}\s](?:[^{};"'\s]|\s+(?![\s{])|(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|"#,
                r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*'))*(?=\s*\{)",
            ],
            LB,
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
        "property",
        &[pf(
            &[
                r"(?i)(^|[^-\w\xA0-\x{FFFF}])(?!\s)[-_a-z\xA0-\x{FFFF}](?:(?!\s)[-\w\xA0-\x{FFFF}])*(?=\s*:)",
            ],
            LB,
        )],
    ),
    e("important", &[p(&[r"(?i)!important\b"])]),
    e(
        "function",
        &[pf(&[r"(?i)(^|[^-a-z0-9])[-a-z0-9]+(?=\()"], LB)],
    ),
    e("punctuation", &[p(&[r"[(){};:,]"])]),
]);

pub(super) static MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_ATRULE: Grammar = Grammar::with_rest(
    &[
        e("rule", &[p(&[r"^@[\w-]+"])]),
        e(
            "selector-function-argument",
            &[pa(
                &[
                    r"(\bselector\s*\(\s*(?![\s)]))(?:[^()\s]|\s+(?![\s)])|\((?:[^()]|\([^()]*\))*\))+(?=\s*\))",
                ],
                LB,
                &["selector"],
            )],
        ),
        e(
            "keyword",
            &[pf(&[r"(^|[^\w-])(?:and|not|only|or)(?![\w-])"], LB)],
        ),
    ],
    &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
);

pub(super) static MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS_URL: Grammar = Grammar::new(&[
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

pub(super) static MARKDOWN_TAG: Grammar = Grammar::new(&[
    e("tag", &[pi(&[r"^</?[^\s>/]+"], 0, &MARKDOWN_TAG_TAG)]),
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
                &MARKDOWN_TAG_SPECIAL_ATTR_1,
            ),
            pi(
                &[r#"(?i)(^|["'\s])(?:style)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))"#],
                LB,
                &MARKDOWN_TAG_SPECIAL_ATTR_2,
            ),
        ],
    ),
    e(
        "attr-value",
        &[pi(
            &[r#"=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+)"#],
            0,
            &MARKDOWN_TAG_ATTR_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"/?>"])]),
    e(
        "attr-name",
        &[pi(&[r"[^\s>/]+"], 0, &MARKDOWN_TAG_ATTR_NAME)],
    ),
]);

pub(super) static MARKDOWN_TAG_TAG: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^</?"])]),
    e("namespace", &[p(&[r"^[^\s>/:]+:"])]),
]);

pub(super) static MARKDOWN_TAG_SPECIAL_ATTR_1: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(
            &[r"=[\s\S]+"],
            0,
            &MARKDOWN_TAG_SPECIAL_ATTR_1_ATTR_VALUE,
        )],
    ),
]);

pub(super) static MARKDOWN_TAG_SPECIAL_ATTR_1_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["javascript", "language-javascript"],
            &MARKDOWN_SCRIPT_INCLUDED_CDATA_LANGUAGE_JAVASCRIPT,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static MARKDOWN_TAG_SPECIAL_ATTR_2: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(
            &[r"=[\s\S]+"],
            0,
            &MARKDOWN_TAG_SPECIAL_ATTR_2_ATTR_VALUE,
        )],
    ),
]);

pub(super) static MARKDOWN_TAG_SPECIAL_ATTR_2_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["css", "language-css"],
            &MARKDOWN_STYLE_INCLUDED_CDATA_LANGUAGE_CSS,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static MARKDOWN_TAG_ATTR_VALUE: Grammar = Grammar::new(&[
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

pub(super) static MARKDOWN_TAG_ATTR_NAME: Grammar =
    Grammar::new(&[e("namespace", &[p(&[r"^[^\s>/:]+:"])])]);
