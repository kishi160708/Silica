//! Makefile.
//!
//! Ported from Prism 1.29.0's `prism-makefile.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, Grammar, GR, LB};

pub static MAKEFILE: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(&[r"(^|[^\\])#(?:\\(?:\r\n|[\s\S])|[^\\\r\n])*"], LB)],
    ),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
    ),
    e(
        "builtin-target",
        &[pa(&[r"\.[A-Z][^:#=\s]+(?=\s*:(?!=))"], 0, &["builtin"])],
    ),
    e(
        "target",
        &[pai(
            &[r"(?m)^(?:[^:=\s]|[ \t]+(?![\s:]))+(?=\s*:(?!=))"],
            0,
            &["symbol"],
            &MAKEFILE_TARGET,
        )],
    ),
    e(
        "variable",
        &[p(&[
            r"\$+(?:(?!\$)[^(){}:#=\s]+|\([@*%<^+?][DF]\)|(?=[({]))",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"-include\b|\b(?:define|else|endef|endif|export|ifn?def|ifn?eq|include|override|private|",
            r"sinclude|undefine|unexport|vpath)\b",
        ])],
    ),
    e(
        "function",
        &[pf(
            &[
                r"(\()(?:abspath|addsuffix|and|basename|call|dir|error|eval|file|filter(?:-out)?|findstring|",
                r"firstword|flavor|foreach|guile|if|info|join|lastword|load|notdir|or|origin|patsubst|",
                r"realpath|shell|sort|strip|subst|suffix|value|warning|wildcard|word(?:list|s)?)(?=[ \t])",
            ],
            LB,
        )],
    ),
    e("operator", &[p(&[r"(?:::|[?:+!])?=|[|@]"])]),
    e("punctuation", &[p(&[r"[:;(){}]"])]),
]);

pub(super) static MAKEFILE_TARGET: Grammar = Grammar::new(&[e(
    "variable",
    &[p(&[r"\$+(?:(?!\$)[^(){}:#=\s]+|(?=[({]))"])],
)]);
