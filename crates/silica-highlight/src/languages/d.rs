//! D.
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-d.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static D: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"^\s*#!.+"], GR),
            pf(
                &[
                    r"(^|[^\\])(?:/\+(?:/\+(?:[^+]|\+(?!/))*\+/|(?!/\+)[\s\S])*?\+/|//.*|/\*[\s\S]*?\*/)",
                ],
                LB | GR,
            ),
        ],
    ),
    e("char", &[p(&[r"'(?:\\(?:\W|\w+)|[^\\])'"])]),
    e(
        "string",
        &[
            pf(
                &[
                    r#"(?m)\b[rx]"(?:\\[\s\S]|[^\\"])*"[cwd]?|\bq"(?:\[[\s\S]*?\]|\([\s\S]*?\)|<[\s\S]*?>|"#,
                    r#"\{[\s\S]*?\})"|\bq"((?!\d)\w+)$[\s\S]*?^\1"|\bq"(.)[\s\S]*?\2"|(["`])(?:\\[\s\S]|"#,
                    r"(?!\3)[^\\])*\3[cwd]?",
                ],
                GR,
            ),
            pa(&[r"\bq\{(?:\{[^{}]*\}|[^{}])*\}"], GR, &["token-string"]),
        ],
    ),
    e(
        "class-name",
        &[pi(
            &[r"(\b(?:class|extends|implements|instanceof|interface|new)\s+)[\w.\\]+"],
            LB,
            &D_CLASS_NAME,
        )],
    ),
    e("property", &[p(&[r"\B@\w*"])]),
    e(
        "keyword",
        &[p(&[
            r"\$|\b(?:__(?:(?:DATE|EOF|FILE|FUNCTION|LINE|MODULE|PRETTY_FUNCTION|TIMESTAMP|TIME|VENDOR|",
            r"VERSION)__|gshared|parameters|traits|vector)|abstract|alias|align|asm|assert|auto|body|",
            r"bool|break|byte|case|cast|catch|cdouble|cent|cfloat|char|class|const|continue|creal|dchar|",
            r"debug|default|delegate|delete|deprecated|do|double|dstring|else|enum|export|extern|false|",
            r"final|finally|float|for|foreach|foreach_reverse|function|goto|idouble|if|ifloat|immutable|",
            r"import|inout|int|interface|invariant|ireal|lazy|long|macro|mixin|module|new|nothrow|null|",
            r"out|override|package|pragma|private|protected|ptrdiff_t|public|pure|real|ref|return|scope|",
            r"shared|short|size_t|static|string|struct|super|switch|synchronized|template|this|throw|",
            r"true|try|typedef|typeid|typeof|ubyte|ucent|uint|ulong|union|unittest|ushort|version|void|",
            r"volatile|wchar|while|with|wstring)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "register",
        &[pa(
            &[
                r"\b(?:[ABCD][LHX]|E?(?:BP|DI|SI|SP)|[BS]PL|[ECSDGF]S|CR[0234]|[DS]IL|DR[012367]|E[ABCD]X|",
                r"X?MM[0-7]|R(?:1[0-5]|[89])[BWD]?|R[ABCD]X|R[BS]P|R[DS]I|TR[3-7]|XMM(?:1[0-5]|[89])|",
                r"YMM(?:1[0-5]|\d))\b|\bST(?:\([0-7]\)|\b)",
            ],
            0,
            &["variable"],
        )],
    ),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[
            p(&[r"(?i)\b0x\.?[a-f\d_]+(?:(?!\.\.)\.[a-f\d_]*)?(?:p[+-]?[a-f\d_]+)?[ulfi]{0,4}"]),
            pf(
                &[
                    r"(?i)((?:\.\.)?)(?:\b0b\.?|\b|\.)\d[\d_]*(?:(?!\.\.)\.[\d_]*)?(?:e[+-]?\d[\d_]*)?[ulfi]{0,4",
                    r"}",
                ],
                LB,
            ),
        ],
    ),
    e(
        "operator",
        &[p(&[
            r"\|[|=]?|&[\&=]?|\+[+=]?|-[-=]?|\.?\.\.|=[>=]?|!(?:i[ns]\b|<>?=?|>=?|=)?|\bi[ns]\b|",
            r"(?:<[<>]?|>>?>?|\^\^|[*/%^\~])=?",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static D_CLASS_NAME: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"[.\\]"])])]);
