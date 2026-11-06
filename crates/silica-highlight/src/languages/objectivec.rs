//! Objective-C (`objc`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-c.js`, `prism-objectivec.js` and
//! `prism-opencl.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static OBJECTIVEC: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "macro",
        &[pai(
            &[
                r"(?im)(^[\t ]*)#\s*[a-z](?:[^\r\n\\/]|/(?!\*)|/\*(?:[^*]|\*(?!/))*\*/|\\(?:\r\n|[\s\S]))*",
            ],
            LB | GR,
            &["property"],
            &OBJECTIVEC_MACRO,
        )],
    ),
    e(
        "string",
        &[pf(&[r#"@?"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:asm|auto|break|case|char|const|continue|default|do|double|else|enum|extern|float|for|",
            r"goto|if|in|inline|int|long|register|return|self|short|signed|sizeof|static|struct|super|",
            r"switch|typedef|typeof|union|unsigned|void|volatile|while)\b|(?:@interface|@end|",
            r"@implementation|@protocol|@class|@public|@protected|@private|@property|@try|@catch|",
            r"@finally|@throw|@synthesize|@dynamic|@selector)\b",
        ])],
    ),
    e(
        "constant",
        &[p(&[
            r"\b(?:EOF|NULL|SEEK_CUR|SEEK_END|SEEK_SET|__DATE__|__FILE__|__LINE__|__TIMESTAMP__|__TIME__|",
            r"__func__|stderr|stdin|stdout)\b",
        ])],
    ),
    e(
        "parallel-keyword",
        &[pa(
            &[r"\bcilk_(?:for|reducer|s(?:cope|pawn|ync))\b"],
            0,
            &["keyword"],
        )],
    ),
    e("function", &[p(&[r"(?i)\b[a-z_]\w*(?=\s*\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)(?:\b0x(?:[\da-f]+(?:\.[\da-f]*)?|\.[\da-f]+)(?:p[+-]?\d+)?|(?:\b\d+(?:\.\d*)?|",
            r"\B\.\d+)(?:e[+-]?\d+)?)[ful]{0,4}",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"-[->]?|\+\+?|!=?|<<?=?|>>?=?|==?|&&?|\|\|?|[\~^%?*/@]",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static OBJECTIVEC_MACRO: Grammar = Grammar::new(&[
    e(
        "string",
        &[
            pf(&[r"^(#\s*include\s*)<[^>]+>"], LB),
            pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR),
        ],
    ),
    e(
        "char",
        &[pf(&[r"'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n]){0,32}'"], GR)],
    ),
    e(
        "comment",
        &[pf(
            &[r"//(?:[^\r\n\\]|\\(?:\r\n?|\n|(?![\r\n])))*|/\*[\s\S]*?(?:\*/|$)"],
            GR,
        )],
    ),
    e(
        "macro-name",
        &[
            pf(&[r"(?i)(^#\s*define\s+)\w+\b(?!\()"], LB),
            pa(&[r"(?i)(^#\s*define\s+)\w+\b(?=\()"], LB, &["function"]),
        ],
    ),
    e("directive", &[pa(&[r"^(#\s*)[a-z]+"], LB, &["keyword"])]),
    e("directive-hash", &[p(&[r"^#"])]),
    e("punctuation", &[p(&[r"##|\\(?=[\r\n])"])]),
    e("expression", &[pi(&[r"\S[\s\S]*"], 0, &OBJECTIVEC)]),
]);
