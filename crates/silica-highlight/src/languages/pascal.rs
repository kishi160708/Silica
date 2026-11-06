//! Pascal (`objectpascal`).
//!
//! Ported from Prism 1.29.0's `prism-pascal.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static PASCAL: Grammar = Grammar::new(&[
    e(
        "directive",
        &[pa(&[r"\{\$[\s\S]*?\}"], GR, &["marco", "property"])],
    ),
    e(
        "comment",
        &[pf(&[r"\(\*[\s\S]*?\*\)|\{[\s\S]*?\}|//.*"], GR)],
    ),
    e(
        "string",
        &[pf(
            &[r"(?i)(?:'(?:''|[^'\r\n])*'(?!')|#[\&$%]?[a-f\d]+)+|\^[a-z]"],
            GR,
        )],
    ),
    e(
        "asm",
        &[pi(
            &[r"(?i)(\basm\b)[\s\S]+?(?=\bend\s*[;\[])"],
            LB | GR,
            &PASCAL_ASM,
        )],
    ),
    e(
        "keyword",
        &[
            pf(
                &[
                    r"(?i)(^|[^\&])\b(?:absolute|array|asm|begin|case|const|constructor|destructor|do|downto|",
                    r"else|end|file|for|function|goto|if|implementation|inherited|inline|interface|label|nil|",
                    r"object|of|operator|packed|procedure|program|record|reintroduce|repeat|self|set|string|then|",
                    r"to|type|unit|until|uses|var|while|with)\b",
                ],
                LB,
            ),
            pf(&[r"(?i)(^|[^\&])\b(?:dispose|exit|false|new|true)\b"], LB),
            pf(
                &[
                    r"(?i)(^|[^\&])\b(?:class|dispinterface|except|exports|finalization|finally|initialization|",
                    r"inline|library|on|out|packed|property|raise|resourcestring|threadvar|try)\b",
                ],
                LB,
            ),
            pf(
                &[
                    r"(?i)(^|[^\&])\b(?:absolute|abstract|alias|assembler|bitpacked|break|cdecl|continue|cppdecl|",
                    r"cvar|default|deprecated|dynamic|enumerator|experimental|export|external|far|far16|forward|",
                    r"generic|helper|implements|index|interrupt|iochecks|local|message|name|near|nodefault|",
                    r"noreturn|nostackframe|oldfpccall|otherwise|overload|override|pascal|platform|private|",
                    r"protected|public|published|read|register|reintroduce|result|safecall|saveregisters|",
                    r"softfloat|specialize|static|stdcall|stored|strict|unaligned|unimplemented|varargs|virtual|",
                    r"write)\b",
                ],
                LB,
            ),
        ],
    ),
    e(
        "number",
        &[
            p(&[r"(?i)(?:[\&%]\d+|\$[a-f\d]+)"]),
            p(&[r"(?i)\b\d+(?:\.\d+)?(?:e[+-]?\d+)?"]),
        ],
    ),
    e(
        "operator",
        &[
            p(&[r"\.\.|\*\*|:=|<[<=>]?|>[>=]?|[+\-*/]=?|[@^=]"]),
            pf(
                &[r"(^|[^\&])\b(?:and|as|div|exclude|in|include|is|mod|not|or|shl|shr|xor)\b"],
                LB,
            ),
        ],
    ),
    e("punctuation", &[p(&[r"\(\.|\.\)|[()\[\]:;,.]"])]),
]);

pub(super) static PASCAL_ASM: Grammar = Grammar::new(&[
    e(
        "directive",
        &[pa(&[r"\{\$[\s\S]*?\}"], GR, &["marco", "property"])],
    ),
    e(
        "comment",
        &[pf(&[r"\(\*[\s\S]*?\*\)|\{[\s\S]*?\}|//.*"], GR)],
    ),
    e(
        "string",
        &[pf(
            &[r"(?i)(?:'(?:''|[^'\r\n])*'(?!')|#[\&$%]?[a-f\d]+)+|\^[a-z]"],
            GR,
        )],
    ),
    e(
        "number",
        &[
            p(&[r"(?i)(?:[\&%]\d+|\$[a-f\d]+)"]),
            p(&[r"(?i)\b\d+(?:\.\d+)?(?:e[+-]?\d+)?"]),
        ],
    ),
    e("punctuation", &[p(&[r"\(\.|\.\)|[()\[\]:;,.]"])]),
]);
