//! XML (`atom`, `rss`, `ssml`).
//!
//! Ported from Prism 1.29.0's `prism-markup.js`, `prism-css.js`, `prism-javascript.js` and
//! `prism-actionscript.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static XML: Grammar = Grammar::new(&[
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
            &XML_DOCTYPE,
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
            &XML_TAG,
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

pub(super) static XML_DOCTYPE: Grammar = Grammar::new(&[
    e(
        "internal-subset",
        &[pi(&[r"(^[^\[]*\[)[\s\S]+(?=\]>$)"], LB | GR, &XML)],
    ),
    e("string", &[pf(&[r#""[^"]*"|'[^']*'"#], GR)]),
    e("punctuation", &[p(&[r"^<!|>$|[\[\]]"])]),
    e("doctype-tag", &[p(&[r"(?i)^DOCTYPE"])]),
    e("name", &[p(&[r#"[^\s<>'"]+"#])]),
]);

pub(super) static XML_TAG: Grammar = Grammar::new(&[
    e("tag", &[pi(&[r"^</?[^\s>/]+"], 0, &XML_TAG_TAG)]),
    e("special-attr", &[]),
    e(
        "attr-value",
        &[pi(
            &[r#"=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+)"#],
            0,
            &XML_TAG_ATTR_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"/?>"])]),
    e("attr-name", &[pi(&[r"[^\s>/]+"], 0, &XML_TAG_ATTR_NAME)]),
]);

pub(super) static XML_TAG_TAG: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^</?"])]),
    e("namespace", &[p(&[r"^[^\s>/:]+:"])]),
]);

pub(super) static XML_TAG_ATTR_VALUE: Grammar = Grammar::new(&[
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

pub(super) static XML_TAG_ATTR_NAME: Grammar =
    Grammar::new(&[e("namespace", &[p(&[r"^[^\s>/:]+:"])])]);
