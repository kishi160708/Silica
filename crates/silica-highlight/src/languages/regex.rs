//! Regex.
//!
//! Ported from Prism 1.29.0's `prism-regex.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pi, Grammar, LB};

pub static REGEX: Grammar = Grammar::new(&[
    e(
        "char-class",
        &[pi(
            &[r"((?:^|[^\\])(?:\\\\)*)\[(?:[^\\\]]|\\[\s\S])*\]"],
            LB,
            &REGEX_CHAR_CLASS,
        )],
    ),
    e(
        "special-escape",
        &[pa(&[r"\\[\\(){}\[\]^$+*?|.]"], 0, &["escape"])],
    ),
    e(
        "char-set",
        &[pa(&[r"(?i)\.|\\[wsd]|\\p\{[^{}]+\}"], 0, &["class-name"])],
    ),
    e(
        "backreference",
        &[
            pa(&[r"\\(?![123][0-7]{2})[1-9]"], 0, &["keyword"]),
            pai(&[r"\\k<[^<>']+>"], 0, &["keyword"], &REGEX_BACKREFERENCE_2),
        ],
    ),
    e("anchor", &[pa(&[r"[$^]|\\[ABbGZz]"], 0, &["function"])]),
    e(
        "escape",
        &[p(&[
            r"\\(?:x[\da-fA-F]{2}|u[\da-fA-F]{4}|u\{[\da-fA-F]+\}|0[0-7]{0,2}|[123][0-7]{2}|c[a-zA-Z]|.)",
        ])],
    ),
    e(
        "group",
        &[
            pai(
                &[r"\((?:\?(?:<[^<>']+>|'[^<>']+'|[>:]|<?[=!]|[idmnsuxU]+(?:-[idmnsuxU]+)?:?))?"],
                0,
                &["punctuation"],
                &REGEX_GROUP_1,
            ),
            pa(&[r"\)"], 0, &["punctuation"]),
        ],
    ),
    e(
        "quantifier",
        &[pa(&[r"(?:[+*?]|\{\d+(?:,\d*)?\})[?+]?"], 0, &["number"])],
    ),
    e("alternation", &[pa(&[r"\|"], 0, &["keyword"])]),
]);

pub(super) static REGEX_CHAR_CLASS: Grammar = Grammar::new(&[
    e(
        "char-class-negation",
        &[pa(&[r"(^\[)\^"], LB, &["operator"])],
    ),
    e(
        "char-class-punctuation",
        &[pa(&[r"^\[|\]$"], 0, &["punctuation"])],
    ),
    e(
        "range",
        &[pi(
            &[
                r"(?:[^\\-]|\\(?:x[\da-fA-F]{2}|u[\da-fA-F]{4}|u\{[\da-fA-F]+\}|0[0-7]{0,2}|[123][0-7]{2}|",
                r"c[a-zA-Z]|.))-(?:[^\\-]|\\(?:x[\da-fA-F]{2}|u[\da-fA-F]{4}|u\{[\da-fA-F]+\}|0[0-7]{0,2}|",
                r"[123][0-7]{2}|c[a-zA-Z]|.))",
            ],
            0,
            &REGEX_CHAR_CLASS_RANGE,
        )],
    ),
    e(
        "special-escape",
        &[pa(&[r"\\[\\(){}\[\]^$+*?|.]"], 0, &["escape"])],
    ),
    e(
        "char-set",
        &[pa(&[r"(?i)\\[wsd]|\\p\{[^{}]+\}"], 0, &["class-name"])],
    ),
    e(
        "escape",
        &[p(&[
            r"\\(?:x[\da-fA-F]{2}|u[\da-fA-F]{4}|u\{[\da-fA-F]+\}|0[0-7]{0,2}|[123][0-7]{2}|c[a-zA-Z]|.)",
        ])],
    ),
]);

pub(super) static REGEX_CHAR_CLASS_RANGE: Grammar = Grammar::new(&[
    e(
        "escape",
        &[p(&[
            r"\\(?:x[\da-fA-F]{2}|u[\da-fA-F]{4}|u\{[\da-fA-F]+\}|0[0-7]{0,2}|[123][0-7]{2}|c[a-zA-Z]|.)",
        ])],
    ),
    e("range-punctuation", &[pa(&[r"-"], 0, &["operator"])]),
]);

pub(super) static REGEX_BACKREFERENCE_2: Grammar = Grammar::new(&[e(
    "group-name",
    &[pa(&[r"(<|')[^<>']+(?=[>']$)"], LB, &["variable"])],
)]);

pub(super) static REGEX_GROUP_1: Grammar = Grammar::new(&[e(
    "group-name",
    &[pa(&[r"(<|')[^<>']+(?=[>']$)"], LB, &["variable"])],
)]);
