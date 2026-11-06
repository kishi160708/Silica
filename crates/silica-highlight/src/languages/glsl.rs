//! GLSL.
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-c.js`, `prism-glsl.js` and
//! `prism-opencl.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static GLSL: Grammar = Grammar::new(&[
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
            &GLSL_MACRO,
        )],
    ),
    e(
        "string",
        &[pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e(
        "class-name",
        &[pf(
            &[r"(\b(?:enum|struct)\s+(?:__attribute__\s*\(\([\s\S]*?\)\)\s*)?)\w+|\b[a-z]\w*_t\b"],
            LB,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:active|asm|atomic_uint|attribute|[ibdu]?vec[234]|bool|break|buffer|case|cast|centroid|",
            r"class|coherent|common|const|continue|d?mat[234](?:x[234])?|default|discard|do|double|else|",
            r"enum|extern|external|false|filter|fixed|flat|float|for|fvec[234]|goto|half|highp|hvec[234]|",
            r"[iu]?sampler2DMS(?:Array)?|[iu]?sampler2DRect|[iu]?samplerBuffer|[iu]?samplerCube|",
            r"[iu]?samplerCubeArray|[iu]?sampler[123]D|[iu]?sampler[12]DArray|[iu]?image2DMS(?:Array)?|",
            r"[iu]?image2DRect|[iu]?imageBuffer|[iu]?imageCube|[iu]?imageCubeArray|[iu]?image[123]D|",
            r"[iu]?image[12]DArray|if|in|inline|inout|input|int|interface|invariant|layout|long|lowp|",
            r"mediump|namespace|noinline|noperspective|out|output|partition|patch|precise|precision|",
            r"public|readonly|resource|restrict|return|sample|sampler[12]DArrayShadow|sampler[12]DShadow|",
            r"sampler2DRectShadow|sampler3DRect|samplerCubeArrayShadow|samplerCubeShadow|shared|short|",
            r"sizeof|smooth|static|struct|subroutine|superp|switch|template|this|true|typedef|uint|",
            r"uniform|union|unsigned|using|varying|void|volatile|while|writeonly)\b",
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
        &[p(&[r">>=?|<<=?|->|([-+\&|:])\1|[?:\~]|[-+*/%\&|^!=<>]=?"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static GLSL_MACRO: Grammar = Grammar::new(&[
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
    e("expression", &[pi(&[r"\S[\s\S]*"], 0, &GLSL)]),
]);
