//! Julia.
//!
//! Ported from Prism 1.29.0's `prism-julia.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, Grammar, GR, LB};

pub static JULIA: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"(^|[^\\])(?:#=(?:[^#=]|=(?!#)|#(?!=)|#=(?:[^#=]|=(?!#)|#(?!=))*=#)*=#|#.*)"],
            LB,
        )],
    ),
    e(
        "regex",
        &[pf(&[r#"r"(?:\\.|[^"\\\r\n])*"[imsx]{0,4}"#], GR)],
    ),
    e(
        "string",
        &[pf(
            &[r#""""[\s\S]+?"""|(?:\b\w+)?"(?:\\.|[^"\\\r\n])*"|`(?:[^\\`\r\n]|\\.)*`"#],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(
            &[r"(^|[^\w'])'(?:\\[^\r\n][^'\r\n]*|[^\\\r\n])'"],
            LB | GR,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:abstract|baremodule|begin|bitstype|break|catch|ccall|const|continue|do|else|elseif|",
            r"end|export|finally|for|function|global|if|immutable|import|importall|in|let|local|macro|",
            r"module|print|println|quote|return|struct|try|type|typealias|using|while)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "number",
        &[p(&[
            r"(?i)(?:\b(?=\d)|\B(?=\.))(?:0[box])?(?:[\da-f]+(?:_[\da-f]+)*(?:\.(?:\d+(?:_\d+)*)?)?|",
            r"\.\d+(?:_\d+)*)(?:[efp][+-]?\d+(?:_\d+)*)?j?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"&&|\|\||[-+*^%÷⊻\&$\\]=?|/[/=]?|!=?=?|\|[=>]?|<(?:<=?|[=:|])?|>(?:=|>>?=?)?|==?=?|",
            r"[\~≠≤≥'√∛]",
        ])],
    ),
    e("punctuation", &[p(&[r"::?|[{}\[\]();,.?]"])]),
    e(
        "constant",
        &[p(&[r"\b(?:(?:Inf|NaN)(?:16|32|64)?|im|pi)\b|[πℯ]"])],
    ),
]);
