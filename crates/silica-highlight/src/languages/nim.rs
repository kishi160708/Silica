//! Nim.
//!
//! Ported from Prism 1.29.0's `prism-nim.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, pi, Grammar, GR, LB};

pub static NIM: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"#.*"], GR)]),
    e(
        "string",
        &[pf(
            &[
                r#"(?:\b(?!\d)(?:\w|\\x[89a-fA-F][0-9a-fA-F])+)?(?:"""[\s\S]*?"""(?!")|"(?:\\[\s\S]|""|"#,
                r#"[^"\\])*")"#,
            ],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\d+|x[\da-fA-F]{0,2}|.)|[^'])'"], GR)],
    ),
    e(
        "function",
        &[pi(
            &[r"(?:(?!\d)(?:\w|\\x[89a-fA-F][0-9a-fA-F])+|`[^`\r\n]+`)\*?(?:\[[^\]]+\])?(?=\s*\()"],
            GR,
            &NIM_FUNCTION,
        )],
    ),
    e("identifier", &[pi(&[r"`[^`\r\n]+`"], GR, &NIM_IDENTIFIER)]),
    e(
        "number",
        &[p(&[
            r"\b(?:0[xXoObB][\da-fA-F_]+|\d[\d_]*(?:(?!\.\.)\.[\d_]*)?(?:[eE][+-]?\d[\d_]*)?)(?:'?[iuf]\d",
            r"*)?",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:addr|as|asm|atomic|bind|block|break|case|cast|concept|const|continue|converter|defer|",
            r"discard|distinct|do|elif|else|end|enum|except|export|finally|for|from|func|generic|if|",
            r"import|include|interface|iterator|let|macro|method|mixin|nil|object|out|proc|ptr|raise|ref|",
            r"return|static|template|try|tuple|type|using|var|when|while|with|without|yield)\b",
        ])],
    ),
    e(
        "operator",
        &[pf(
            &[
                r"(?m)(^|[({\[](?=\.\.)|(?![({\[]\.).)(?:(?:[=+\-*/<>@$\~\&%|!?^:\\]|\.\.|\.(?![)}\]]))+|",
                r"\b(?:and|div|in|is|isnot|mod|not|notin|of|or|shl|shr|xor)\b)",
            ],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[({\[]\.|\.[)}\]]|[`(){}\[\],:]"])]),
]);

pub(super) static NIM_FUNCTION: Grammar = Grammar::new(&[e("operator", &[p(&[r"\*$"])])]);

pub(super) static NIM_IDENTIFIER: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"`"])])]);
