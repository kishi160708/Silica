//! CSS, with the `css-extras` component Obsidian loads over it.
//!
//! Ported from Prism's `components/prism-css.js` and
//! `components/prism-css-extras.js`. The extras component replaces
//! `selector`'s `inside`, gives the at-rule's `selector-function-argument`
//! that same inside, and inserts `variable` before `property` and
//! `operator`, `hexcode`, `color`, `entity`, `unit` and `number` before
//! `function` — so the order below is the order those insertions leave.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

/// Prism's shared `string` fragment: a quoted run with escapes.
const CSS_STRING: &str =
    r#"(?:"(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*"|'(?:\\(?:\r\n|[\s\S])|[^'\\\r\n])*')"#;
/// The same thing as `css-extras` writes it, with a back-reference.
const EXTRAS_STRING: &str = r#"("|')(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#;

// ---- the pieces `css-extras` shares ---------------------------------------

static ATTR_NAMESPACE_INSIDE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\|$"])])]);

static SELECTOR_ATTRIBUTE_INSIDE: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\[|\]$"])]),
    e(
        "case-sensitivity",
        &[pa(&[r"(?i)(\s)[si]$"], LB, &["keyword"])],
    ),
    e(
        "namespace",
        &[pi(
            &[r"^(\s*)(?:(?!\s)[-*\w\x{00A0}-\x{FFFF}])*\|(?!=)"],
            LB,
            &ATTR_NAMESPACE_INSIDE,
        )],
    ),
    e(
        "attr-name",
        &[pf(&[r"^(\s*)(?:(?!\s)[-\w\x{00A0}-\x{FFFF}])+"], LB)],
    ),
    e(
        "attr-value",
        &[
            p(&[EXTRAS_STRING]),
            pf(&[r"(=\s*)(?:(?!\s)[-\w\x{00A0}-\x{FFFF}])+(?=\s*$)"], LB),
        ],
    ),
    e("operator", &[p(&[r"[|~*^$]?="])]),
]);

static NTH_INSIDE: Grammar = Grammar::new(&[
    e("number", &[p(&[r"[\dn]+"])]),
    e("operator", &[p(&[r"[+-]"])]),
]);

/// `css-extras`'s `selectorInside`, which both `selector` and the at-rule's
/// `selector-function-argument` are given.
static SELECTOR_INSIDE: Grammar = Grammar::new(&[
    e(
        "pseudo-element",
        &[p(&[
            r":(?:after|before|first-letter|first-line|selection)|::[-\w]+",
        ])],
    ),
    e("pseudo-class", &[p(&[r":[-\w]+"])]),
    e("class", &[p(&[r"\.[-\w]+"])]),
    e("id", &[p(&[r"#[-\w]+"])]),
    e(
        "attribute",
        &[pi(
            &[r#"\[(?:[^\[\]"']|"#, EXTRAS_STRING, r")*\]"],
            GR,
            &SELECTOR_ATTRIBUTE_INSIDE,
        )],
    ),
    e(
        "n-th",
        &[
            pi(
                &[r"(\(\s*)[+-]?\d*[\dn](?:\s*[+-]\s*\d+)?(?=\s*\))"],
                LB,
                &NTH_INSIDE,
            ),
            pf(&[r"(?i)(\(\s*)(?:even|odd)(?=\s*\))"], LB),
        ],
    ),
    e("combinator", &[p(&[r">|\+|~|\|\|"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);

static COLOR_FUNCTION_INSIDE: Grammar = Grammar::new(&[
    e("unit", &[pf(&[r"(\b\d+)(?:%|[a-z]+(?![\w-]))"], LB)]),
    e(
        "number",
        &[pf(&[r"(^|[^\w.-])-?(?:\d+(?:\.\d+)?|\.\d+)"], LB)],
    ),
    e("function", &[p(&[r"[\w-]+(?=\()"])]),
    e("punctuation", &[p(&[r"[(),]"])]),
]);

/// The inside of `@media (…)` and friends. Prism gives it `rest: css`, so
/// what follows an at-rule is highlighted as CSS again, and `css-extras`
/// replaces its `selector-function-argument`'s inside with the selector's.
static CSS_ATRULE_INSIDE: Grammar = Grammar::with_rest(
    &[
        e("rule", &[p(&[r"^@[\w-]+"])]),
        e(
            "selector-function-argument",
            &[pai(
                &[
                    r"(\bselector\s*\(\s*(?![\s)]))(?:[^()\s]|\s+(?![\s)])|\((?:[^()]|\([^()]*\))*\))+(?=\s*\))",
                ],
                LB,
                &["selector"],
                &SELECTOR_INSIDE,
            )],
        ),
        e(
            "keyword",
            &[pf(&[r"(^|[^\w-])(?:and|not|only|or)(?![\w-])"], LB)],
        ),
    ],
    &CSS,
);

static CSS_URL_INSIDE: Grammar = Grammar::new(&[
    e("function", &[p(&[r"(?i)^url"])]),
    e("punctuation", &[p(&[r"^\(|\)$"])]),
    e("string", &[pa(&[r"^", CSS_STRING, r"$"], 0, &["url"])]),
]);

/// `prism-css.js` with `prism-css-extras.js` applied.
pub static CSS: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"/\*[\s\S]*?\*/"])]),
    e(
        "atrule",
        &[pi(
            &[
                r#"@[\w-](?:[^;{\s"']|\s+(?!\s)|"#,
                CSS_STRING,
                r")*?(?:;|(?=\s*\{))",
            ],
            0,
            &CSS_ATRULE_INSIDE,
        )],
    ),
    e(
        "url",
        &[pi(
            &[
                r"(?i)\burl\((?:",
                CSS_STRING,
                r#"|(?:[^\\\r\n()"']|\\[\s\S])*)\)"#,
            ],
            GR,
            &CSS_URL_INSIDE,
        )],
    ),
    e(
        "selector",
        &[pi(
            &[
                r#"(^|[{}\s])[^{}\s](?:[^{};"'\s]|\s+(?![\s{])|"#,
                CSS_STRING,
                r")*(?=\s*\{)",
            ],
            LB,
            &SELECTOR_INSIDE,
        )],
    ),
    e("string", &[pf(&[CSS_STRING], GR)]),
    // `insertBefore('css', 'property', { variable })`.
    e(
        "variable",
        &[pf(
            &[
                r"(?i)(^|[^-\w\x{00A0}-\x{FFFF}])--(?!\s)[-_a-z\x{00A0}-\x{FFFF}](?:(?!\s)[-\w\x{00A0}-\x{FFFF}])*",
            ],
            LB,
        )],
    ),
    e(
        "property",
        &[pf(
            &[
                r"(?i)(^|[^-\w\x{00A0}-\x{FFFF}])(?!\s)[-_a-z\x{00A0}-\x{FFFF}](?:(?!\s)[-\w\x{00A0}-\x{FFFF}])*(?=\s*:)",
            ],
            LB,
        )],
    ),
    e("important", &[p(&[r"(?i)!important\b"])]),
    // `insertBefore('css', 'function', { operator, hexcode, color, entity,
    // unit, number })`.
    e("operator", &[pf(&[r"(\s)[+\-*/](?=\s)"], LB)]),
    e("hexcode", &[pa(&[r"(?i)\B#[\da-f]{3,8}\b"], 0, &["color"])]),
    e(
        "color",
        &[
            pf(
                &[
                    r"(?i)(^|[^\w-])(?:AliceBlue|AntiqueWhite|Aqua|Aquamarine|Azure|Beige|Bisque|Black|",
                    r"BlanchedAlmond|Blue|BlueViolet|Brown|BurlyWood|CadetBlue|Chartreuse|Chocolate|Coral|",
                    r"CornflowerBlue|Cornsilk|Crimson|Cyan|DarkBlue|DarkCyan|DarkGoldenRod|DarkGr[ae]y|",
                    r"DarkGreen|DarkKhaki|DarkMagenta|DarkOliveGreen|DarkOrange|DarkOrchid|DarkRed|DarkSalmon|",
                    r"DarkSeaGreen|DarkSlateBlue|DarkSlateGr[ae]y|DarkTurquoise|DarkViolet|DeepPink|",
                    r"DeepSkyBlue|DimGr[ae]y|DodgerBlue|FireBrick|FloralWhite|ForestGreen|Fuchsia|Gainsboro|",
                    r"GhostWhite|Gold|GoldenRod|Gr[ae]y|Green|GreenYellow|HoneyDew|HotPink|IndianRed|Indigo|",
                    r"Ivory|Khaki|Lavender|LavenderBlush|LawnGreen|LemonChiffon|LightBlue|LightCoral|LightCyan|",
                    r"LightGoldenRodYellow|LightGr[ae]y|LightGreen|LightPink|LightSalmon|LightSeaGreen|",
                    r"LightSkyBlue|LightSlateGr[ae]y|LightSteelBlue|LightYellow|Lime|LimeGreen|Linen|Magenta|",
                    r"Maroon|MediumAquaMarine|MediumBlue|MediumOrchid|MediumPurple|MediumSeaGreen|",
                    r"MediumSlateBlue|MediumSpringGreen|MediumTurquoise|MediumVioletRed|MidnightBlue|MintCream|",
                    r"MistyRose|Moccasin|NavajoWhite|Navy|OldLace|Olive|OliveDrab|Orange|OrangeRed|Orchid|",
                    r"PaleGoldenRod|PaleGreen|PaleTurquoise|PaleVioletRed|PapayaWhip|PeachPuff|Peru|Pink|Plum|",
                    r"PowderBlue|Purple|RebeccaPurple|Red|RosyBrown|RoyalBlue|SaddleBrown|Salmon|SandyBrown|",
                    r"SeaGreen|SeaShell|Sienna|Silver|SkyBlue|SlateBlue|SlateGr[ae]y|Snow|SpringGreen|",
                    r"SteelBlue|Tan|Teal|Thistle|Tomato|Transparent|Turquoise|Violet|Wheat|White|WhiteSmoke|",
                    r"Yellow|YellowGreen)(?![\w-])",
                ],
                LB,
            ),
            pi(
                &[
                    r"(?i)\b(?:hsl|rgb)\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*\)\B|",
                    r"\b(?:hsl|rgb)a\(\s*\d{1,3}\s*,\s*\d{1,3}%?\s*,\s*\d{1,3}%?\s*,\s*(?:0|0?\.\d+|1)\s*\)\B",
                ],
                0,
                &COLOR_FUNCTION_INSIDE,
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
    e("punctuation", &[p(&[r"[(){};:,]"])]),
]);
