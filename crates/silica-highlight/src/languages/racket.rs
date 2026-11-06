//! Racket (`rkt`).
//!
//! Ported from Prism 1.29.0's `prism-scheme.js` and `prism-racket.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR, LB};

pub static RACKET: Grammar = Grammar::new(&[
    e(
        "comment",
        &[p(&[
            r";.*|#;\s*(?:\((?:[^()]|\([^()]*\))*\)|\[(?:[^\[\]]|\[[^\[\]]*\])*\])|#\|(?:[^#|]|#(?!\|)|\|",
            r"(?!#)|#\|(?:[^#|]|#(?!\|)|\|(?!#))*\|#)*\|#",
        ])],
    ),
    e("lang", &[pa(&[r"(?m)^#lang.+"], GR, &["keyword"])]),
    e("string", &[pf(&[r#""(?:[^"\\]|\\.)*""#], GR)]),
    e("symbol", &[pf(&[r"'[^()\[\]#'\s]+"], GR)]),
    e(
        "char",
        &[pf(
            &[r"#\\(?:[ux][a-fA-F\d]+\b|[-a-zA-Z]+\b|[\x{10000}-\x{10FFFF}]|\S)"],
            GR,
        )],
    ),
    e(
        "lambda-parameter",
        &[pf(&[r"([(\[]lambda\s+[(\[])[^()\[\]'\s]+"], LB)],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"((?:^|[^'`#])[(\[])(?:begin|case(?:-lambda)?|cond(?:-expand)?|define(?:-library|-macro|",
                r"-record-type|-syntax|-values)?|defmacro|delay(?:-force)?|do|else|except|export|guard|if|",
                r"import|include(?:-ci|-library-declarations)?|lambda|let(?:rec)?(?:-syntax|-values|\*)?|",
                r"let\*-values|only|parameterize|prefix|(?:quasi-?)?quote|rename|set!|syntax-(?:case|rules)|",
                r"unless|unquote(?:-splicing)?|when)(?=[()\[\]\s]|$)",
            ],
            LB,
        )],
    ),
    e(
        "builtin",
        &[pf(
            &[
                r"((?:^|[^'`#])[(\[])(?:abs|and|append|apply|assoc|ass[qv]|binary-port\?|boolean=?\?|",
                r"bytevector(?:-append|-copy|-copy!|-length|-u8-ref|-u8-set!|\?)?|caar|cadr|",
                r"call-with-(?:current-continuation|port|values)|call/cc|car|cdar|cddr|cdr|ceiling|",
                r"char(?:->integer|-ready\?|\?|<\?|<=\?|=\?|>\?|>=\?)|close-(?:input-port|output-port|port)|",
                r"complex\?|cons|current-(?:error|input|output)-port|denominator|dynamic-wind|eof-object\??|",
                r"eq\?|equal\?|eqv\?|error|error-object(?:-irritants|-message|\?)|eval|even\?|",
                r"exact(?:-integer-sqrt|-integer\?|\?)?|expt|features|file-error\?|floor(?:-quotient|",
                r"-remainder|/)?|flush-output-port|for-each|gcd|get-output-(?:bytevector|string)|inexact\??|",
                r"input-port(?:-open\?|\?)|integer(?:->char|\?)|lcm|length|list(?:->string|->vector|-copy|",
                r"-ref|-set!|-tail|\?)?|make-(?:bytevector|list|parameter|string|vector)|map|max|member|memq|",
                r"memv|min|modulo|negative\?|newline|not|null\?|number(?:->string|\?)|numerator|odd\?|",
                r"open-(?:input|output)-(?:bytevector|string)|or|output-port(?:-open\?|\?)|pair\?|peek-char|",
                r"peek-u8|port\?|positive\?|procedure\?|quotient|raise|raise-continuable|rational\?|",
                r"rationalize|read-(?:bytevector|bytevector!|char|error\?|line|string|u8)|real\?|remainder|",
                r"reverse|round|set-c[ad]r!|square|string(?:->list|->number|->symbol|->utf8|->vector|-append|",
                r"-copy|-copy!|-fill!|-for-each|-length|-map|-ref|-set!|\?|<\?|<=\?|=\?|>\?|>=\?)?|substring|",
                r"symbol(?:->string|\?|=\?)|syntax-error|textual-port\?|truncate(?:-quotient|-remainder|/)?|",
                r"u8-ready\?|utf8->string|values|vector(?:->list|->string|-append|-copy|-copy!|-fill!|",
                r"-for-each|-length|-map|-ref|-set!|\?)?|with-exception-handler|write-(?:bytevector|char|",
                r"string|u8)|zero\?)(?=[()\[\]\s]|$)",
            ],
            LB,
        )],
    ),
    e(
        "operator",
        &[pf(
            &[r"((?:^|[^'`#])[(\[])(?:[-+*%/]|[<>]=?|=>?)(?=[()\[\]\s]|$)"],
            LB,
        )],
    ),
    e(
        "number",
        &[pf(
            &[
                r"(?i)(^|[()\[\]\s])(?:(?:(?:#d(?:#[ei])?|#[ei](?:#d)?)?(?:(?:[+-]?(?:\d+(?:/\d+)|",
                r"(?:\d+(?:\.\d*)?|\.\d+)(?:[esfdl][+-]?\d+)?)|[+-](?:inf|nan)\.0)(?:@(?:[+-]?(?:\d+(?:/\d+)|",
                r"(?:\d+(?:\.\d*)?|\.\d+)(?:[esfdl][+-]?\d+)?)|[+-](?:inf|nan)\.0)|(?:[+-](?:(?:\d+(?:/\d+)|",
                r"(?:\d+(?:\.\d*)?|\.\d+)(?:[esfdl][+-]?\d+)?)|(?:inf|nan)\.0)?i))?|(?:[+-](?:(?:\d+(?:/\d+)|",
                r"(?:\d+(?:\.\d*)?|\.\d+)(?:[esfdl][+-]?\d+)?)|(?:inf|nan)\.0)?i)))|(?:#[box](?:#[ei])?|",
                r"(?:#[ei])?#[box](?:(?:[+-]?(?:[0-9a-f]+(?:/[0-9a-f]+)?)|[+-](?:inf|",
                r"nan)\.0)(?:@(?:[+-]?(?:[0-9a-f]+(?:/[0-9a-f]+)?)|[+-](?:inf|nan)\.0)|",
                r"(?:[+-](?:(?:[0-9a-f]+(?:/[0-9a-f]+)?)|(?:inf|nan)\.0)?i))?|",
                r"(?:[+-](?:(?:[0-9a-f]+(?:/[0-9a-f]+)?)|(?:inf|nan)\.0)?i))))(?=[()\[\]\s]|$)",
            ],
            LB,
        )],
    ),
    e(
        "boolean",
        &[pf(
            &[r"(^|[()\[\]\s])#(?:[ft]|false|true)(?=[()\[\]\s]|$)"],
            LB,
        )],
    ),
    e(
        "function",
        &[pf(
            &[r"((?:^|[^'`#])[(\[])(?:[^|()\[\]'\s]+|\|(?:[^\\|]|\\.)*\|)(?=[()\[\]\s]|$)"],
            LB,
        )],
    ),
    e(
        "identifier",
        &[pf(
            &[r"(^|[()\[\]\s])\|(?:[^\\|]|\\.)*\|(?=[()\[\]\s]|$)"],
            LB | GR,
        )],
    ),
    e("punctuation", &[p(&[r"[()\[\]']"])]),
]);
