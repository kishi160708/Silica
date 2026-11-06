//! Groovy.
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-groovy.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static GROOVY: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("shebang", &[pa(&[r"#!.+"], GR, &["comment"])]),
    e(
        "interpolation-string",
        &[pi(
            &[
                r#""""(?:[^\\]|\\[\s\S])*?"""|(["/])(?:\\.|(?!\1)[^\\\r\n])*\1|\$/(?:[^/$]|\$(?:[/$]|"#,
                r"(?![/$]))|/(?!\$))*/\$",
            ],
            GR,
            &GROOVY_INTERPOLATION_STRING,
        )],
    ),
    e(
        "string",
        &[pf(
            &[r"'''(?:[^\\]|\\[\s\S])*?'''|'(?:\\.|[^\\'\r\n])*'"],
            GR,
        )],
    ),
    e(
        "class-name",
        &[pi(
            &[r"(\b(?:class|extends|implements|instanceof|interface|new)\s+)[\w.\\]+"],
            LB,
            &GROOVY_CLASS_NAME,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:abstract|as|assert|boolean|break|byte|case|catch|char|class|const|continue|def|",
            r"default|do|double|else|enum|extends|final|finally|float|for|goto|if|implements|import|in|",
            r"instanceof|int|interface|long|native|new|package|private|protected|public|return|short|",
            r"static|strictfp|super|switch|synchronized|this|throw|throws|trait|transient|try|void|",
            r"volatile|while)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "annotation",
        &[pa(&[r"(^|[^.])@\w+"], LB, &["punctuation"])],
    ),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b(?:0b[01_]+|0x[\da-f_]+(?:\.[\da-f_p\-]+)?|",
            r"[\d_]+(?:\.[\d_]+)?(?:e[+-]?\d+)?)[glidf]?\b",
        ])],
    ),
    e(
        "operator",
        &[pf(
            &[
                r"(^|[^.])(?:~|==?~?|\?[.:]?|\*(?:[.=]|\*=?)?|\.[@\&]|\.\.<|\.\.(?!\.)|-[-=>]?|\+[+=]?|!=?|",
                r"<(?:<=?|=>?)?|>(?:>>?=?|=)?|&[\&=]?|\|[|=]?|/=?|\^=?|%=?)",
            ],
            LB,
        )],
    ),
    e(
        "spock-block",
        &[p(&[
            r"\b(?:and|cleanup|expect|given|setup|then|when|where):",
        ])],
    ),
    e("punctuation", &[p(&[r"\.+|[{}\[\];(),:$]"])]),
]);

pub(super) static GROOVY_INTERPOLATION_STRING: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[r"((?:^|[^\\$])(?:\\{2})*)\$(?:\w+|\{[^{}]*\})"],
            LB,
            &GROOVY_INTERPOLATION_STRING_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static GROOVY_INTERPOLATION_STRING_INTERPOLATION: Grammar = Grammar::new(&[
    e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{?|\}$"], 0, &["punctuation"])],
    ),
    e("expression", &[pi(&[r"[\s\S]+"], 0, &GROOVY)]),
]);

pub(super) static GROOVY_CLASS_NAME: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);
