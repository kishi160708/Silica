//! GraphQL.
//!
//! Ported from Prism 1.29.0's `prism-graphql.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static GRAPHQL: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"#.*"])]),
    e(
        "description",
        &[pai(
            &[r#"(?i)(?:"""(?:[^"]|(?!""")")*"""|"(?:\\.|[^\\"\r\n])*")(?=\s*[a-z_])"#],
            GR,
            &["string"],
            &GRAPHQL_DESCRIPTION,
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#""""(?:[^"]|(?!""")")*"""|"(?:\\.|[^\\"\r\n])*""#],
            GR,
        )],
    ),
    e(
        "number",
        &[p(&[r"(?i)(?:\B-|\b)\d+(?:\.\d+)?(?:e[+-]?\d+)?\b"])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("variable", &[p(&[r"(?i)\$[a-z_]\w*"])]),
    e("directive", &[pa(&[r"(?i)@[a-z_]\w*"], 0, &["function"])]),
    e(
        "attr-name",
        &[pf(
            &[r#"(?i)\b[a-z_]\w*(?=\s*(?:\((?:[^()"]|"(?:\\.|[^\\"\r\n])*")*\))?:)"#],
            GR,
        )],
    ),
    e(
        "atom-input",
        &[pa(&[r"\b[A-Z]\w*Input\b"], 0, &["class-name"])],
    ),
    e("scalar", &[p(&[r"\b(?:Boolean|Float|ID|Int|String)\b"])]),
    e("constant", &[p(&[r"\b[A-Z][A-Z_\d]*\b"])]),
    e(
        "class-name",
        &[pf(
            &[r"(\b(?:enum|implements|interface|on|scalar|type|union)\s+|&\s*|:\s*|\[)[A-Z_]\w*"],
            LB,
        )],
    ),
    e(
        "fragment",
        &[pa(
            &[r"(\bfragment\s+|\.{3}\s*(?!on\b))[a-zA-Z_]\w*"],
            LB,
            &["function"],
        )],
    ),
    e(
        "definition-mutation",
        &[pa(&[r"(\bmutation\s+)[a-zA-Z_]\w*"], LB, &["function"])],
    ),
    e(
        "definition-query",
        &[pa(&[r"(\bquery\s+)[a-zA-Z_]\w*"], LB, &["function"])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:directive|enum|extend|fragment|implements|input|interface|mutation|on|query|",
            r"repeatable|scalar|schema|subscription|type|union)\b",
        ])],
    ),
    e("operator", &[p(&[r"[!=|\&]|\.{3}"])]),
    e("property-query", &[p(&[r"\w+(?=\s*\()"])]),
    e("object", &[p(&[r"\w+(?=\s*\{)"])]),
    e("punctuation", &[p(&[r"[!(){}\[\]:=,]"])]),
    e("property", &[p(&[r"\w+"])]),
]);

pub(super) static GRAPHQL_DESCRIPTION: Grammar = Grammar::new(&[e(
    "language-markdown",
    &[pi(
        &[r#"(^"(?:"")?)(?!\1)[\s\S]+(?=\1$)"#],
        LB,
        &super::markdown::MARKDOWN,
    )],
)]);
