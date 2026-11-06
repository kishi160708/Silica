//! Perl.
//!
//! Ported from Prism 1.29.0's `prism-perl.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR, LB};

pub static PERL: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(?m)(^\s*)=\w[\s\S]*?=cut.*"], LB | GR),
            pf(&[r"(^|[^\\$])#.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[
            pf(
                &[
                    r"\b(?:q|qq|qw|qx)(?![a-zA-Z0-9])\s*(?:([^a-zA-Z0-9\s{(\[<])(?:(?!\1)[^\\]|\\[\s\S])*\1|",
                    r"([a-zA-Z0-9])(?:(?!\2)[^\\]|\\[\s\S])*\2|(?:\((?:[^()\\]|\\[\s\S])*\)|\{(?:[^{}\\]|",
                    r"\\[\s\S])*\}|\[(?:[^\[\]\\]|\\[\s\S])*\]|<(?:[^<>\\]|\\[\s\S])*>))",
                ],
                GR,
            ),
            pf(&[r#"("|`)(?:(?!\1)[^\\]|\\[\s\S])*\1"#], GR),
            pf(&[r"'(?:[^'\\\r\n]|\\.)*'"], GR),
        ],
    ),
    e(
        "regex",
        &[
            pf(
                &[
                    r"\b(?:m|qr)(?![a-zA-Z0-9])\s*(?:([^a-zA-Z0-9\s{(\[<])(?:(?!\1)[^\\]|\\[\s\S])*\1|",
                    r"([a-zA-Z0-9])(?:(?!\2)[^\\]|\\[\s\S])*\2|(?:\((?:[^()\\]|\\[\s\S])*\)|\{(?:[^{}\\]|",
                    r"\\[\s\S])*\}|\[(?:[^\[\]\\]|\\[\s\S])*\]|<(?:[^<>\\]|\\[\s\S])*>))[msixpodualngc]*",
                ],
                GR,
            ),
            pf(
                &[
                    r"(^|[^-])\b(?:s|tr|y)(?![a-zA-Z0-9])\s*(?:([^a-zA-Z0-9\s{(\[<])(?:(?!\2)[^\\]|",
                    r"\\[\s\S])*\2(?:(?!\2)[^\\]|\\[\s\S])*\2|([a-zA-Z0-9])(?:(?!\3)[^\\]|",
                    r"\\[\s\S])*\3(?:(?!\3)[^\\]|\\[\s\S])*\3|(?:\((?:[^()\\]|\\[\s\S])*\)|\{(?:[^{}\\]|",
                    r"\\[\s\S])*\}|\[(?:[^\[\]\\]|\\[\s\S])*\]|<(?:[^<>\\]|\\[\s\S])*>)\s*(?:\((?:[^()\\]|",
                    r"\\[\s\S])*\)|\{(?:[^{}\\]|\\[\s\S])*\}|\[(?:[^\[\]\\]|\\[\s\S])*\]|<(?:[^<>\\]|",
                    r"\\[\s\S])*>))[msixpodualngcer]*",
                ],
                LB | GR,
            ),
            pf(
                &[
                    r"/(?:[^/\\\r\n]|\\.)*/[msixpodualngc]*(?=\s*(?:$|[\r\n,.;})\&|\-+*\~<>!?^]|(?:and|cmp|eq|ge|",
                    r"gt|le|lt|ne|not|or|x|xor)\b))",
                ],
                GR,
            ),
        ],
    ),
    e(
        "variable",
        &[
            p(&[r"[\&*$@%]\{\^[A-Z]+\}"]),
            p(&[r"[\&*$@%]\^[A-Z_]"]),
            p(&[r"[\&*$@%]#?(?=\{)"]),
            p(&[r"[\&*$@%]#?(?:(?:::)*'?(?!\d)[\w$]+(?![\w$]))+(?:::)*"]),
            p(&[r"[\&*$@%]\d+"]),
            p(&[r##"(?!%=)[$@%][!"#$%\&'()*+,\-./:;<=>?@\[\\\]^_`{|}\~]"##]),
        ],
    ),
    e(
        "filehandle",
        &[pa(&[r"<(?![<=])\S*?>|\b_\b"], 0, &["symbol"])],
    ),
    e(
        "v-string",
        &[pa(&[r"v\d+(?:\.\d+)*|\d+(?:\.\d+){2,}"], 0, &["string"])],
    ),
    e("function", &[pf(&[r"(\bsub[ \t]+)\w+"], LB)]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:any|break|continue|default|delete|die|do|else|elsif|eval|for|foreach|given|goto|if|",
            r"last|local|my|next|our|package|print|redo|require|return|say|state|sub|switch|undef|unless|",
            r"until|use|when|while)\b",
        ])],
    ),
    e(
        "number",
        &[p(&[
            r"\b(?:0x[\dA-Fa-f](?:_?[\dA-Fa-f])*|0b[01](?:_?[01])*|",
            r"(?:(?:\d(?:_?\d)*)?\.)?\d(?:_?\d)*(?:[Ee][+-]?\d+)?)\b",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"-[rwxoRWXOezsfdlpSbctugkTBMAC]\b|\+[+=]?|-[-=>]?|\*\*?=?|//?=?|=[=\~>]?|~[\~=]?|\|\|?=?|",
            r"&&?=?|<(?:=>?|<=?)?|>>?=?|![\~=]?|[%^]=?|\.(?:=|\.\.?)?|[\\?]|\bx(?:=|\b)|\b(?:and|cmp|eq|",
            r"ge|gt|le|lt|ne|not|or|xor)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),:]"])]),
]);
