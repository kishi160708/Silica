//! Batch.
//!
//! Ported from Prism 1.29.0's `prism-batch.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, LB};

pub static BATCH: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            p(&[r"(?m)^::.*"]),
            pf(
                &[r"(?im)((?:^|[\&(])[ \t]*)rem\b(?:[^^\&)\r\n]|\^(?:\r\n|[\s\S]))*"],
                LB,
            ),
        ],
    ),
    e("label", &[pa(&[r"(?m)^:.*"], 0, &["property"])]),
    e(
        "command",
        &[
            pi(
                &[
                    r#"(?im)((?:^|[\&(])[ \t]*)for(?: /[a-z?](?:[ :](?:"[^"]*"|"#,
                    r#"[^\s"/]\S*))?)* \S+ in \([^)]+\) do"#,
                ],
                LB,
                &BATCH_COMMAND_1,
            ),
            pi(
                &[
                    r#"(?im)((?:^|[\&(])[ \t]*)if(?: /[a-z?](?:[ :](?:"[^"]*"|"#,
                    r#"[^\s"/]\S*))?)* (?:not )?(?:cmdextversion \d+|defined \w+|errorlevel \d+|exist \S+|"#,
                    r#"(?:"[^"]*"|(?!")(?:(?!==)\S)+)?(?:==| (?:equ|geq|gtr|leq|lss|neq) )(?:"[^"]*"|[^\s"]\S*))"#,
                ],
                LB,
                &BATCH_COMMAND_2,
            ),
            pi(&[r"(?im)((?:^|[\&()])[ \t]*)else\b"], LB, &BATCH_COMMAND_3),
            pi(
                &[
                    r#"(?im)((?:^|[\&(])[ \t]*)set(?: /[a-z](?:[ :](?:"[^"]*"|[^\s"/]\S*))?)* (?:[^^\&)\r\n]|"#,
                    r"\^(?:\r\n|[\s\S]))*",
                ],
                LB,
                &BATCH_COMMAND_4,
            ),
            pi(
                &[
                    r#"(?m)((?:^|[\&(])[ \t]*@?)\w+\b(?:"(?:[\\"]"|[^"])*"(?!")|[^"^\&)\r\n]|\^(?:\r\n|[\s\S]))*"#,
                ],
                LB,
                &BATCH_COMMAND_5,
            ),
        ],
    ),
    e("operator", &[p(&[r"[\&@]"])]),
    e("punctuation", &[p(&[r"[()']"])]),
]);

pub(super) static BATCH_COMMAND_1: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"(?i)\b(?:do|in)\b|^for\b"])]),
    e("string", &[p(&[r#""(?:[\\"]"|[^"])*"(?!")"#])]),
    e(
        "parameter",
        &[pai(
            &[r"(?im)/[a-z?]+(?=[ :]|$):?|-[a-z]\b|--[a-z-]+\b"],
            0,
            &["attr-name"],
            &BATCH_COMMAND_1_PARAMETER,
        )],
    ),
    e("variable", &[p(&[r"%%?[\~:\w]+%?|!\S+!"])]),
    e("number", &[p(&[r"(?:\b|-)\d+\b"])]),
    e("punctuation", &[p(&[r"[()',]"])]),
]);

pub(super) static BATCH_COMMAND_1_PARAMETER: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r":"])])]);

pub(super) static BATCH_COMMAND_2: Grammar = Grammar::new(&[
    e(
        "keyword",
        &[p(&[
            r"(?i)\b(?:cmdextversion|defined|errorlevel|exist|not)\b|^if\b",
        ])],
    ),
    e("string", &[p(&[r#""(?:[\\"]"|[^"])*"(?!")"#])]),
    e(
        "parameter",
        &[pai(
            &[r"(?im)/[a-z?]+(?=[ :]|$):?|-[a-z]\b|--[a-z-]+\b"],
            0,
            &["attr-name"],
            &BATCH_COMMAND_1_PARAMETER,
        )],
    ),
    e("variable", &[p(&[r"%%?[\~:\w]+%?|!\S+!"])]),
    e("number", &[p(&[r"(?:\b|-)\d+\b"])]),
    e(
        "operator",
        &[p(&[r"(?i)\^|==|\b(?:equ|geq|gtr|leq|lss|neq)\b"])],
    ),
]);

pub(super) static BATCH_COMMAND_3: Grammar = Grammar::new(&[e("keyword", &[p(&[r"(?i)^else\b"])])]);

pub(super) static BATCH_COMMAND_4: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"(?i)^set\b"])]),
    e("string", &[p(&[r#""(?:[\\"]"|[^"])*"(?!")"#])]),
    e(
        "parameter",
        &[pai(
            &[r"(?im)/[a-z?]+(?=[ :]|$):?|-[a-z]\b|--[a-z-]+\b"],
            0,
            &["attr-name"],
            &BATCH_COMMAND_1_PARAMETER,
        )],
    ),
    e(
        "variable",
        &[
            p(&[r"%%?[\~:\w]+%?|!\S+!"]),
            p(&[r"\w+(?=(?:[*/%+\-\&^|]|<<|>>)?=)"]),
        ],
    ),
    e("number", &[p(&[r"(?:\b|-)\d+\b"])]),
    e("operator", &[p(&[r"[*/%+\-\&^|]=?|<<=?|>>=?|[!\~_=]"])]),
    e("punctuation", &[p(&[r"[()',]"])]),
]);

pub(super) static BATCH_COMMAND_5: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^\w+\b"])]),
    e("string", &[p(&[r#""(?:[\\"]"|[^"])*"(?!")"#])]),
    e(
        "parameter",
        &[pai(
            &[r"(?im)/[a-z?]+(?=[ :]|$):?|-[a-z]\b|--[a-z-]+\b"],
            0,
            &["attr-name"],
            &BATCH_COMMAND_1_PARAMETER,
        )],
    ),
    e("label", &[pa(&[r"(?m)(^\s*):\S+"], LB, &["property"])]),
    e("variable", &[p(&[r"%%?[\~:\w]+%?|!\S+!"])]),
    e("number", &[p(&[r"(?:\b|-)\d+\b"])]),
    e("operator", &[p(&[r"\^"])]),
]);
