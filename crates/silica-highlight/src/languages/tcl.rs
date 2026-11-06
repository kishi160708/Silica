//! Tcl.
//!
//! Ported from Prism 1.29.0's `prism-tcl.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR, LB};

pub static TCL: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(^|[^\\])#.*"], LB)]),
    e(
        "string",
        &[pf(&[r#""(?:[^"\\\r\n]|\\(?:\r\n|[\s\S]))*""#], GR)],
    ),
    e(
        "variable",
        &[
            pf(&[r"(\$)(?:::)?(?:[a-zA-Z0-9]+::)*\w+"], LB),
            pf(&[r"(\$)\{[^}]+\}"], LB),
            pf(
                &[r"(?m)(^[\t ]*set[ \t]+)(?:::)?(?:[a-zA-Z0-9]+::)*\w+"],
                LB,
            ),
        ],
    ),
    e("function", &[pf(&[r"(?m)(^[\t ]*proc[ \t]+)\S+"], LB)]),
    e(
        "builtin",
        &[
            pf(
                &[
                    r"(?m)(^[\t ]*)(?:break|class|continue|error|eval|exit|for|foreach|if|proc|return|switch|",
                    r"while)\b",
                ],
                LB,
            ),
            p(&[r"\b(?:else|elseif)\b"]),
        ],
    ),
    e(
        "scope",
        &[pa(
            &[r"(?m)(^[\t ]*)(?:global|upvar|variable)\b"],
            LB,
            &["constant"],
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(?m)(^[\t ]*|\[)(?:Safe_Base|Tcl|after|append|apply|array|auto_(?:execok|import|load|",
                r"mkindex|qualify|reset)|automkindex_old|bgerror|binary|catch|cd|chan|clock|close|concat|dde|",
                r"dict|encoding|eof|exec|expr|fblocked|fconfigure|fcopy|file(?:event|name)?|flush|gets|glob|",
                r"history|http|incr|info|interp|join|lappend|lassign|lindex|linsert|list|llength|load|lrange|",
                r"lrepeat|lreplace|lreverse|lsearch|lset|lsort|math(?:func|op)|memory|msgcat|namespace|open|",
                r"package|parray|pid|pkg_mkIndex|platform|puts|pwd|re_syntax|read|refchan|regexp|registry|",
                r"regsub|rename|scan|seek|set|socket|source|split|string|subst|tcl(?:_endOfWord|_findLibrary|",
                r"startOf(?:Next|Previous)Word|test|vars|wordBreak(?:After|Before))|tell|time|tm|trace|",
                r"unknown|unload|unset|update|uplevel|vwait)\b",
            ],
            LB,
        )],
    ),
    e(
        "operator",
        &[p(&[
            r"!=?|\*\*?|==|&&?|\|\|?|<[=<]?|>[=>]?|[-+\~/%?^]|\b(?:eq|in|ne|ni)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}()\[\]]"])]),
]);
