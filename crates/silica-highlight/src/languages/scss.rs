//! Sass (SCSS).
//!
//! Ported from Prism 1.29.0's `prism-css.js`, `prism-scss.js` and `prism-css-extras.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static SCSS: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(^|[^\\])(?:/\*[\s\S]*?\*/|//.*)"], LB)]),
    e(
        "keyword",
        &[
            p(&[
                r"(?i)@(?:content|debug|each|else(?: if)?|extend|for|forward|function|if|import|include|",
                r"mixin|return|use|warn|while)\b",
            ]),
            pf(&[r"( )(?:from|through)(?= )"], LB),
        ],
    ),
    e(
        "atrule",
        &[pi(
            &[r"@[\w-](?:\([^()]+\)|[^()\s]|\s+(?!\s))*?(?=\s+[{;])"],
            0,
            &SCSS_ATRULE,
        )],
    ),
    e("url", &[p(&[r"(?i)(?:[-a-z]+-)?url(?=\()"])]),
    e(
        "selector",
        &[pi(
            &[
                r"(?=\S)[^@;{}()]?(?:[^@;{}()\s]|\s+(?!\s)|#\{\$[-\w]+\})+(?=\s*\{(?:\}|\s|",
                r"[^}][^:{}]*[:{][^}]))",
            ],
            0,
            &SCSS_SELECTOR,
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#"(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')"#],
            GR,
        )],
    ),
    e(
        "property",
        &[pi(
            &[r"(?:[-\w]|\$[-\w]|#\{\$[-\w]+\})+(?=\s*:)"],
            0,
            &SCSS_PROPERTY,
        )],
    ),
    e("variable", &[p(&[r"\$[-\w]+|#\{\$[-\w]+\}"])]),
    e("important", &[p(&[r"(?i)!important\b"])]),
    e("hexcode", &[pa(&[r"(?i)\B#[\da-f]{3,8}\b"], 0, &["color"])]),
    e(
        "color",
        &[
            pf(
                &[
                    r"(?i)(^|[^\w-])(?:AliceBlue|AntiqueWhite|Aqua|Aquamarine|Azure|Beige|Bisque|Black|",
                    r"BlanchedAlmond|Blue|BlueViolet|Brown|BurlyWood|CadetBlue|Chartreuse|Chocolate|Coral|",
                    r"CornflowerBlue|Cornsilk|Crimson|Cyan|DarkBlue|DarkCyan|DarkGoldenRod|DarkGr[ae]y|DarkGreen|",
                    r"DarkKhaki|DarkMagenta|DarkOliveGreen|DarkOrange|DarkOrchid|DarkRed|DarkSalmon|DarkSeaGreen|",
                    r"DarkSlateBlue|DarkSlateGr[ae]y|DarkTurquoise|DarkViolet|DeepPink|DeepSkyBlue|DimGr[ae]y|",
                    r"DodgerBlue|FireBrick|FloralWhite|ForestGreen|Fuchsia|Gainsboro|GhostWhite|Gold|GoldenRod|",
                    r"Gr[ae]y|Green|GreenYellow|HoneyDew|HotPink|IndianRed|Indigo|Ivory|Khaki|Lavender|",
                    r"LavenderBlush|LawnGreen|LemonChiffon|LightBlue|LightCoral|LightCyan|LightGoldenRodYellow|",
                    r"LightGr[ae]y|LightGreen|LightPink|LightSalmon|LightSeaGreen|LightSkyBlue|LightSlateGr[ae]y|",
                    r"LightSteelBlue|LightYellow|Lime|LimeGreen|Linen|Magenta|Maroon|MediumAquaMarine|MediumBlue|",
                    r"MediumOrchid|MediumPurple|MediumSeaGreen|MediumSlateBlue|MediumSpringGreen|MediumTurquoise|",
                    r"MediumVioletRed|MidnightBlue|MintCream|MistyRose|Moccasin|NavajoWhite|Navy|OldLace|Olive|",
                    r"OliveDrab|Orange|OrangeRed|Orchid|PaleGoldenRod|PaleGreen|PaleTurquoise|PaleVioletRed|",
                    r"PapayaWhip|PeachPuff|Peru|Pink|Plum|PowderBlue|Purple|RebeccaPurple|Red|RosyBrown|",
                    r"RoyalBlue|SaddleBrown|Salmon|SandyBrown|SeaGreen|SeaShell|Sienna|Silver|SkyBlue|SlateBlue|",
                    r"SlateGr[ae]y|Snow|SpringGreen|SteelBlue|Tan|Teal|Thistle|Tomato|Transparent|Turquoise|",
                    r"Violet|Wheat|White|WhiteSmoke|Yellow|YellowGreen)(?![\w-])",
                ],
                LB,
            ),
            pi(
                &[
                    r"(?i)\b(?:hsl|rgb)\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*\)\B|\b(?:hsl|",
                    r"rgb)a\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*,\s*(?:0|0?\.\d+|1)\s*\)\B",
                ],
                0,
                &SCSS_COLOR_2,
            ),
        ],
    ),
    e("entity", &[p(&[r"(?i)\\[\da-f]{1,8}"])]),
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e(
        "module-modifier",
        &[pa(&[r"(?i)\b(?:as|hide|show|with)\b"], 0, &["keyword"])],
    ),
    e("placeholder", &[pa(&[r"%[-\w]+"], 0, &["selector"])]),
    e(
        "statement",
        &[pa(&[r"(?i)\B!(?:default|optional)\b"], 0, &["keyword"])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("null", &[pa(&[r"\bnull\b"], 0, &["keyword"])]),
    e(
        "operator",
        &[pf(&[r"(\s)(?:[-+*/%]|[=!]=|<=?|>=?|and|not|or)(?=\s)"], LB)],
    ),
    e(
        "function",
        &[pf(&[r"(?i)(^|[^-a-z0-9])[-a-z0-9]+(?=\()"], LB)],
    ),
    e("punctuation", &[p(&[r"[(){};:,]"])]),
]);

pub(super) static SCSS_ATRULE: Grammar =
    Grammar::with_rest(&[e("rule", &[p(&[r"@[\w-]+"])])], &SCSS);

pub(super) static SCSS_SELECTOR: Grammar = Grammar::new(&[
    e("parent", &[pa(&[r"&"], 0, &["important"])]),
    e("placeholder", &[p(&[r"%[-\w]+"])]),
    e("variable", &[p(&[r"\$[-\w]+|#\{\$[-\w]+\}"])]),
]);

pub(super) static SCSS_PROPERTY: Grammar =
    Grammar::new(&[e("variable", &[p(&[r"\$[-\w]+|#\{\$[-\w]+\}"])])]);

pub(super) static SCSS_COLOR_2: Grammar = Grammar::new(&[
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e("function", &[p(&[r"[\w-]+(?=\()"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);
