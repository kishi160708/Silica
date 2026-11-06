//! Markup (`html`, `mathml`, `svg`).
//!
//! Ported from Prism 1.29.0's `prism-markup.js`, `prism-css.js`, `prism-javascript.js` and
//! `prism-actionscript.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static MARKUP: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"<!--(?:(?!<!--)[\s\S])*?-->"], GR)]),
    e("prolog", &[pf(&[r"<\?[\s\S]+?\?>"], GR)]),
    e(
        "doctype",
        &[pi(
            &[
                r#"(?i)<!DOCTYPE(?:[^>"'\[\]]|"[^"]*"|'[^']*')+(?:\[(?:[^<"'\]]|"[^"]*"|'[^']*'|<(?!!--)|"#,
                r"<!--(?:[^-]|-(?!->))*-->)*\]\s*)?>",
            ],
            GR,
            &MARKUP_DOCTYPE,
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
            &MARKUP_SCRIPT,
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
            &MARKUP_STYLE,
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
            &MARKUP_TAG,
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

pub(super) static MARKUP_DOCTYPE: Grammar = Grammar::new(&[
    e(
        "internal-subset",
        &[pi(&[r"(^[^\[]*\[)[\s\S]+(?=\]>$)"], LB | GR, &MARKUP)],
    ),
    e("string", &[pf(&[r#""[^"]*"|'[^']*'"#], GR)]),
    e("punctuation", &[p(&[r"^<!|>$|[\[\]]"])]),
    e("doctype-tag", &[p(&[r"(?i)^DOCTYPE"])]),
    e("name", &[p(&[r#"[^\s<>'"]+"#])]),
]);

pub(super) static MARKUP_SCRIPT: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &MARKUP_SCRIPT_INCLUDED_CDATA,
        )],
    ),
    e(
        "language-javascript",
        &[pi(&[r"[\s\S]+"], 0, &super::javascript::JAVASCRIPT)],
    ),
]);

pub(super) static MARKUP_SCRIPT_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-javascript",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &super::javascript::JAVASCRIPT,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static MARKUP_STYLE: Grammar = Grammar::new(&[
    e(
        "included-cdata",
        &[pi(
            &[r"(?i)<!\[CDATA\[[\s\S]*?\]\]>"],
            0,
            &MARKUP_STYLE_INCLUDED_CDATA,
        )],
    ),
    e("language-css", &[pi(&[r"[\s\S]+"], 0, &super::web::CSS)]),
]);

pub(super) static MARKUP_STYLE_INCLUDED_CDATA: Grammar = Grammar::new(&[
    e(
        "language-css",
        &[pi(
            &[r"(?i)(^<!\[CDATA\[)[\s\S]+?(?=\]\]>$)"],
            LB,
            &super::web::CSS,
        )],
    ),
    e("cdata", &[p(&[r"(?i)^<!\[CDATA\[|\]\]>$"])]),
]);

pub(super) static MARKUP_TAG: Grammar = Grammar::new(&[
    e("tag", &[pi(&[r"^</?[^\s>/]+"], 0, &MARKUP_TAG_TAG)]),
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
                &MARKUP_TAG_SPECIAL_ATTR_1,
            ),
            pi(
                &[r#"(?i)(^|["'\s])(?:style)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))"#],
                LB,
                &MARKUP_TAG_SPECIAL_ATTR_2,
            ),
        ],
    ),
    e(
        "attr-value",
        &[pi(
            &[r#"=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+)"#],
            0,
            &MARKUP_TAG_ATTR_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"/?>"])]),
    e("attr-name", &[pi(&[r"[^\s>/]+"], 0, &MARKUP_TAG_ATTR_NAME)]),
]);

pub(super) static MARKUP_TAG_TAG: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^</?"])]),
    e("namespace", &[p(&[r"^[^\s>/:]+:"])]),
]);

pub(super) static MARKUP_TAG_SPECIAL_ATTR_1: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(&[r"=[\s\S]+"], 0, &MARKUP_TAG_SPECIAL_ATTR_1_ATTR_VALUE)],
    ),
]);

pub(super) static MARKUP_TAG_SPECIAL_ATTR_1_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["javascript", "language-javascript"],
            &super::javascript::JAVASCRIPT,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static MARKUP_TAG_SPECIAL_ATTR_2: Grammar = Grammar::new(&[
    e("attr-name", &[p(&[r"^[^\s=]+"])]),
    e(
        "attr-value",
        &[pi(&[r"=[\s\S]+"], 0, &MARKUP_TAG_SPECIAL_ATTR_2_ATTR_VALUE)],
    ),
]);

pub(super) static MARKUP_TAG_SPECIAL_ATTR_2_ATTR_VALUE: Grammar = Grammar::new(&[
    e(
        "value",
        &[pai(
            &[r#"(^=\s*(["']|(?!["'])))\S[\s\S]*(?=\2$)"#],
            LB,
            &["css", "language-css"],
            &super::web::CSS,
        )],
    ),
    e(
        "punctuation",
        &[pa(&[r"^="], 0, &["attr-equals"]), p(&[r#""|'"#])],
    ),
]);

pub(super) static MARKUP_TAG_ATTR_VALUE: Grammar = Grammar::new(&[
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

pub(super) static MARKUP_TAG_ATTR_NAME: Grammar =
    Grammar::new(&[e("namespace", &[p(&[r"^[^\s>/:]+:"])])]);
