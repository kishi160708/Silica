//! Sass (Sass).
//!
//! Ported from Prism 1.29.0's `prism-css.js`, `prism-sass.js` and `prism-css-extras.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static SASS: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[r"(?m)^([ \t]*)/[/*].*(?:(?:\r?\n|\r)\1[ \t].+)*"],
            LB | GR,
        )],
    ),
    e(
        "atrule-line",
        &[pi(&[r"(?m)^(?:[ \t]*)[@+=].+"], GR, &SASS_ATRULE_LINE)],
    ),
    e(
        "url",
        &[pi(
            &[
                r#"(?i)\burl\((?:(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')|"#,
                r#"(?:[^\\\r\n()"']|\\[\s\S])*)\)"#,
            ],
            GR,
            &SASS_URL,
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
        "variable",
        &[pf(
            &[
                r"(?i)(^|[^-\w\xA0-\x{FFFF}])--(?!\s)[-_a-z\xA0-\x{FFFF}](?:(?!\s)[-\w\xA0-\x{FFFF}])*",
            ],
            LB,
        )],
    ),
    e(
        "variable-line",
        &[pi(&[r"(?m)^[ \t]*\$.+"], GR, &SASS_VARIABLE_LINE)],
    ),
    e(
        "property-line",
        &[pi(
            &[r"(?m)^[ \t]*(?:[^:\s]+ *:.*|:[^:\s].*)"],
            GR,
            &SASS_PROPERTY_LINE,
        )],
    ),
    e("operator", &[pf(&[r"(\s)[+\-*/](?=\s)"], LB)]),
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
                &SASS_COLOR_2,
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
        "function",
        &[pf(&[r"(?i)(^|[^-a-z0-9])[-a-z0-9]+(?=\()"], LB)],
    ),
    e(
        "selector",
        &[pf(
            &[
                r"(?m)^([ \t]*)\S(?:,[^,\r\n]+|[^,\r\n]*)(?:,[^,\r\n]+)*(?:,(?:\r?\n|",
                r"\r)\1[ \t]+\S(?:,[^,\r\n]+|[^,\r\n]*)(?:,[^,\r\n]+)*)*",
            ],
            LB | GR,
        )],
    ),
    e("punctuation", &[p(&[r"[(){};:,]"])]),
]);

pub(super) static SASS_ATRULE_LINE: Grammar =
    Grammar::new(&[e("atrule", &[p(&[r"(?:@[\w-]+|[+=])"])])]);

pub(super) static SASS_URL: Grammar = Grammar::new(&[
    e("function", &[p(&[r"(?i)^url"])]),
    e("punctuation", &[p(&[r"^\(|\)$"])]),
    e(
        "string",
        &[pa(
            &[r#"^(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')$"#],
            0,
            &["url"],
        )],
    ),
]);

pub(super) static SASS_VARIABLE_LINE: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r":"])]),
    e("variable", &[p(&[r"\$[-\w]+|#\{\$[-\w]+\}"])]),
    e(
        "operator",
        &[
            p(&[r"[+*/%]|[=!]=|<=?|>=?|\b(?:and|not|or)\b"]),
            pf(&[r"(\s)-(?=\s)"], LB),
        ],
    ),
]);

pub(super) static SASS_PROPERTY_LINE: Grammar = Grammar::new(&[
    e(
        "property",
        &[p(&[r"[^:\s]+(?=\s*:)"]), pf(&[r"(:)[^:\s]+"], LB)],
    ),
    e("punctuation", &[p(&[r":"])]),
    e("variable", &[p(&[r"\$[-\w]+|#\{\$[-\w]+\}"])]),
    e(
        "operator",
        &[
            p(&[r"[+*/%]|[=!]=|<=?|>=?|\b(?:and|not|or)\b"]),
            pf(&[r"(\s)-(?=\s)"], LB),
        ],
    ),
    e("important", &[p(&[r"(?i)!important\b"])]),
]);

pub(super) static SASS_COLOR_2: Grammar = Grammar::new(&[
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e("function", &[p(&[r"[\w-]+(?=\()"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);
