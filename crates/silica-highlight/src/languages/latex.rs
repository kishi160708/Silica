//! LaTeX (`context`, `tex`).
//!
//! Ported from Prism 1.29.0's `prism-latex.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, Grammar, LB};

pub static LATEX: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"%.*"])]),
    e(
        "cdata",
        &[pf(
            &[r"(\\begin\{((?:lstlisting|verbatim)\*?)\})[\s\S]*?(?=\\end\{\2\})"],
            LB,
        )],
    ),
    e(
        "equation",
        &[
            pai(
                &[
                    r"\$\$(?:\\[\s\S]|[^\\$])+\$\$|\$(?:\\[\s\S]|[^\\$])+\$|\\\([\s\S]*?\\\)|\\\[[\s\S]*?\\\]",
                ],
                0,
                &["string"],
                &LATEX_EQUATION_1,
            ),
            pai(
                &[
                    r"(\\begin\{((?:align|eqnarray|equation|gather|math|multline)\*?)\})[\s\S]*?(?=\\end\{\2\})",
                ],
                LB,
                &["string"],
                &LATEX_EQUATION_1,
            ),
        ],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(\\(?:begin|cite|documentclass|end|label|ref|usepackage)(?:\[[^\]]+\])?\{)[^}]+(?=\})",
            ],
            LB,
        )],
    ),
    e("url", &[pf(&[r"(\\url\{)[^}]+(?=\})"], LB)]),
    e(
        "headline",
        &[pa(
            &[
                r"(\\(?:chapter|frametitle|paragraph|part|section|subparagraph|subsection|subsubparagraph|",
                r"subsubsection|subsubsubparagraph)\*?(?:\[[^\]]+\])?\{)[^}]+(?=\})",
            ],
            LB,
            &["class-name"],
        )],
    ),
    e(
        "function",
        &[pa(&[r"(?i)\\(?:[^a-z()\[\]]|[a-z*]+)"], 0, &["selector"])],
    ),
    e("punctuation", &[p(&[r"[\[\]{}\&]"])]),
]);

pub(super) static LATEX_EQUATION_1: Grammar = Grammar::new(&[e(
    "equation-command",
    &[pa(&[r"(?i)\\(?:[^a-z()\[\]]|[a-z*]+)"], 0, &["regex"])],
)]);
