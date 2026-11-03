//! The data formats: JSON, YAML, TOML, INI, diff and Nix.
//!
//! Ported from Prism's `components/prism-{json,yaml,toml,ini,diff,nix}.js`,
//! key for key and in the same order, which is the order the engine tries
//! them in. JavaScript's regex flags become inline groups: `/…/i` is
//! `(?i)…` and `/…/m` is `(?m)…`.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

// ---- JSON -----------------------------------------------------------------

/// `prism-json.js`. `webmanifest` is the same grammar.
pub static JSON: Grammar = Grammar::new(&[
    e(
        "property",
        &[pf(&[r#"(^|[^\\])"(?:\\.|[^\\"\r\n])*"(?=\s*:)"#], LB | GR)],
    ),
    e(
        "string",
        &[pf(&[r#"(^|[^\\])"(?:\\.|[^\\"\r\n])*"(?!\s*:)"#], LB | GR)],
    ),
    e("comment", &[pf(&[r"//.*|/\*[\s\S]*?(?:\*/|$)"], GR)]),
    e("number", &[p(&[r"(?i)-?\b\d+(?:\.\d+)?(?:e[+-]?\d+)?\b"])]),
    e("punctuation", &[p(&[r"[{}\[\],]"])]),
    e("operator", &[p(&[r":"])]),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("null", &[pa(&[r"\bnull\b"], 0, &["keyword"])]),
]);

// ---- YAML -----------------------------------------------------------------
// Prism builds these out of shared fragments; the fragments are `const`s
// here and the patterns are the same composition.

const ANCHOR_OR_ALIAS: &str = r"[*&][^\s\[\]{},]+";
const TAG: &str =
    r"!(?:<[\w\-%#;/?:@&=+$,.!~*'()\[\]]+>|(?:[a-zA-Z\d-]*!)?[\w\-%#;/?:@&=+$.~*'()]+)?";
/// Prism's `properties`: a tag, an anchor, or one then the other.
const PROPS: &str = concat!(
    r"(?:!(?:<[\w\-%#;/?:@&=+$,.!~*'()\[\]]+>|(?:[a-zA-Z\d-]*!)?[\w\-%#;/?:@&=+$.~*'()]+)?",
    r"(?:[ \t]+[*&][^\s\[\]{},]+)?|[*&][^\s\[\]{},]+",
    r"(?:[ \t]+!(?:<[\w\-%#;/?:@&=+$,.!~*'()\[\]]+>|(?:[a-zA-Z\d-]*!)?[\w\-%#;/?:@&=+$.~*'()]+)?)?)",
);
/// Prism's `plainKey`, with `<PLAIN>` expanded. The surrogate range of the
/// original is dropped: a `&str` cannot hold one, so the class is the same.
const PLAIN_KEY: &str = concat!(
    r##"(?:[^\s\x00-\x08\x0e-\x1f!"#%&'*,\-:>?@\[\]`{|}\x7f-\x84\x86-\x9f\u{fffe}\u{ffff}]"##,
    r"|[?:-][^\s\x00-\x08\x0e-\x1f,\[\]{}\x7f-\x84\x86-\x9f\u{fffe}\u{ffff}])",
    r"(?:[ \t]*(?:(?![#:])[^\s\x00-\x08\x0e-\x1f,\[\]{}\x7f-\x84\x86-\x9f\u{fffe}\u{ffff}]",
    r"|:[^\s\x00-\x08\x0e-\x1f,\[\]{}\x7f-\x84\x86-\x9f\u{fffe}\u{ffff}]))*",
);
const YAML_STRING: &str = r#""(?:[^"\\\r\n]|\\.)*"|'(?:[^'\\\r\n]|\\.)*'"#;
/// The front of Prism's `createValuePattern`: the lookbehind group.
const VALUE_HEAD: &str = r"(?m)([:\-,\[{]\s*(?:\s";
const VALUE_MID: &str = r"[ \t]+)?)(?:";
/// Its tail: a value ends at a comma, a bracket, a comment or the line.
const VALUE_TAIL: &str = r")(?=[ \t]*(?:$|,|\]|\}|(?:[\r\n]\s*)?#))";

/// `prism-yaml.js`. `yml` is the same grammar.
pub static YAML: Grammar = Grammar::new(&[
    e(
        "scalar",
        &[pa(
            &[
                r"([\-:]\s*(?:\s",
                PROPS,
                r"[ \t]+)?[|>])[ \t]*(?:((?:\r?\n|\r)[ \t]+)\S[^\r\n]*(?:\2[^\r\n]+)*)",
            ],
            LB,
            &["string"],
        )],
    ),
    e("comment", &[p(&[r"#.*"])]),
    e(
        "key",
        &[pa(
            &[
                r"((?:^|[:\-,\[{\r\n?])[ \t]*(?:",
                PROPS,
                r"[ \t]+)?)(?:",
                PLAIN_KEY,
                r"|",
                YAML_STRING,
                r")(?=\s*:\s)",
            ],
            LB | GR,
            &["atrule"],
        )],
    ),
    e(
        "directive",
        &[pa(&[r"(?m)(^[ \t]*)%.+"], LB, &["important"])],
    ),
    e(
        "datetime",
        &[pa(
            &[
                VALUE_HEAD,
                PROPS,
                VALUE_MID,
                r"\d{4}-\d\d?-\d\d?(?:[tT]|[ \t]+)\d\d?:\d{2}:\d{2}(?:\.\d*)?(?:[ \t]*(?:Z|[-+]\d\d?(?::\d{2})?))?|\d{4}-\d{2}-\d{2}|\d\d?:\d{2}(?::\d{2}(?:\.\d*)?)?",
                VALUE_TAIL,
            ],
            LB,
            &["number"],
        )],
    ),
    e(
        "boolean",
        &[pa(
            &[
                r"(?i)",
                VALUE_HEAD,
                PROPS,
                VALUE_MID,
                r"false|true",
                VALUE_TAIL,
            ],
            LB,
            &["important"],
        )],
    ),
    e(
        "null",
        &[pa(
            &[r"(?i)", VALUE_HEAD, PROPS, VALUE_MID, r"null|~", VALUE_TAIL],
            LB,
            &["important"],
        )],
    ),
    e(
        "string",
        &[pf(
            &[VALUE_HEAD, PROPS, VALUE_MID, YAML_STRING, VALUE_TAIL],
            LB | GR,
        )],
    ),
    e(
        "number",
        &[pf(
            &[
                r"(?i)",
                VALUE_HEAD,
                PROPS,
                VALUE_MID,
                r"[+-]?(?:0x[\da-f]+|0o[0-7]+|(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?|\.inf|\.nan)",
                VALUE_TAIL,
            ],
            LB,
        )],
    ),
    e("tag", &[p(&[TAG])]),
    e("important", &[p(&[ANCHOR_OR_ALIAS])]),
    e("punctuation", &[p(&[r"---|[:\[\]{}\-,|>?]|\.\.\."])]),
]);

// ---- TOML -----------------------------------------------------------------

/// Prism's `key` fragment, which both `table` and `key` are built from.
const TOML_KEY: &str = r#"(?:[\w-]+|'[^'\n\r]*'|"(?:\\.|[^\\"\r\n])*")"#;

/// `prism-toml.js`.
pub static TOML: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"#.*"], GR)]),
    e(
        "table",
        &[pa(
            &[
                r"(?m)(^[\t ]*\[\s*(?:\[\s*)?)",
                TOML_KEY,
                r"(?:\s*\.\s*",
                TOML_KEY,
                r")*(?=\s*\])",
            ],
            LB | GR,
            &["class-name"],
        )],
    ),
    e(
        "key",
        &[pa(
            &[
                r"(?m)(^[\t ]*|[{,]\s*)",
                TOML_KEY,
                r"(?:\s*\.\s*",
                TOML_KEY,
                r")*(?=\s*=)",
            ],
            LB | GR,
            &["property"],
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#""""(?:\\[\s\S]|[^\\])*?"""|'''[\s\S]*?'''|'[^'\n\r]*'|"(?:\\.|[^\\"\r\n])*""#],
            GR,
        )],
    ),
    e(
        "date",
        &[
            pa(
                &[
                    r"(?i)\b\d{4}-\d{2}-\d{2}(?:[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?\b",
                ],
                0,
                &["number"],
            ),
            pa(&[r"\b\d{2}:\d{2}:\d{2}(?:\.\d+)?\b"], 0, &["number"]),
        ],
    ),
    e(
        "number",
        &[p(&[
            r"(?:\b0(?:x[\da-zA-Z]+(?:_[\da-zA-Z]+)*|o[0-7]+(?:_[0-7]+)*|b[10]+(?:_[10]+)*))\b",
            r"|[-+]?\b\d+(?:_\d+)*(?:\.\d+(?:_\d+)*)?(?:[eE][+-]?\d+(?:_\d+)*)?\b|[-+]?\b(?:inf|nan)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("punctuation", &[p(&[r"[.,=\[\]{}]"])]),
]);

// ---- INI ------------------------------------------------------------------

static INI_SECTION: Grammar = Grammar::new(&[
    e(
        "section-name",
        &[pa(
            &[r"(^\[[ \f\t\v]*)[^ \f\t\v\]]+(?:[ \f\t\v]+[^ \f\t\v\]]+)*"],
            LB,
            &["selector"],
        )],
    ),
    e("punctuation", &[p(&[r"\[|\]"])]),
]);

static INI_VALUE: Grammar = Grammar::new(&[e("inner-value", &[pf(&[r#"^("|').+(?=\1$)"#], LB)])]);

/// `prism-ini.js`, which mimics the Win32 API's parser.
pub static INI: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(?m)(^[ \f\t\v]*)[#;][^\n\r]*"], LB)]),
    e(
        "section",
        &[pi(&[r"(?m)(^[ \f\t\v]*)\[[^\n\r\]]*\]?"], LB, &INI_SECTION)],
    ),
    e(
        "key",
        &[pa(
            &[r"(?m)(^[ \f\t\v]*)[^ \f\n\r\t\v=]+(?:[ \f\t\v]+[^ \f\n\r\t\v=]+)*(?=[ \f\t\v]*=)"],
            LB,
            &["attr-name"],
        )],
    ),
    e(
        "value",
        &[pai(
            &[r"(=[ \f\t\v]*)[^ \f\n\r\t\v]+(?:[ \f\t\v]+[^ \f\n\r\t\v]+)*"],
            LB,
            &["attr-value"],
            &INI_VALUE,
        )],
    ),
    e("punctuation", &[p(&[r"="])]),
]);

// ---- diff -----------------------------------------------------------------
// Prism builds one entry per line prefix from a table; written out here,
// since the alias of each is what the theme paints.

macro_rules! diff_block {
    ($name:ident, $alias:literal) => {
        static $name: Grammar = Grammar::new(&[
            e("line", &[pf(&[r"(.)(?=[\s\S]).*(?:\r\n?|\n)?"], LB)]),
            e("prefix", &[pa(&[r"[\s\S]"], 0, &[$alias])]),
        ]);
    };
}

diff_block!(DIFF_DELETED, "deleted");
diff_block!(DIFF_DELETED_ARROW, "deleted");
diff_block!(DIFF_INSERTED, "inserted");
diff_block!(DIFF_INSERTED_ARROW, "inserted");
diff_block!(DIFF_UNCHANGED, "unchanged");
diff_block!(DIFF_BANG, "diff");

/// `prism-diff.js`. `coord` comes first, then one entry per prefix.
pub static DIFF: Grammar = Grammar::new(&[
    e(
        "coord",
        &[
            p(&[r"(?m)^(?:\*{3}|-{3}|\+{3}).*$"]),
            p(&[r"(?m)^@@.*@@$"]),
            p(&[r"(?m)^\d.*$"]),
        ],
    ),
    e(
        "deleted-sign",
        &[pai(
            &[r"(?m)^(?:[-].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &["deleted"],
            &DIFF_DELETED,
        )],
    ),
    e(
        "deleted-arrow",
        &[pai(
            &[r"(?m)^(?:[<].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &["deleted"],
            &DIFF_DELETED_ARROW,
        )],
    ),
    e(
        "inserted-sign",
        &[pai(
            &[r"(?m)^(?:[+].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &["inserted"],
            &DIFF_INSERTED,
        )],
    ),
    e(
        "inserted-arrow",
        &[pai(
            &[r"(?m)^(?:[>].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &["inserted"],
            &DIFF_INSERTED_ARROW,
        )],
    ),
    e(
        "unchanged",
        &[pi(
            &[r"(?m)^(?:[ ].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &DIFF_UNCHANGED,
        )],
    ),
    e(
        "diff",
        &[pai(
            &[r"(?m)^(?:[!].*(?:\r\n?|\n|(?![\s\S])))+"],
            0,
            &["bold"],
            &DIFF_BANG,
        )],
    ),
]);

// ---- Nix ------------------------------------------------------------------

/// `prism-nix.js`'s `interpolation`, whose body is Nix again.
static NIX_INTERPOLATION: Grammar = Grammar::with_rest(
    &[e(
        "interpolation-punctuation",
        &[pa(&[r"^\$\{|\}$"], 0, &["punctuation"])],
    )],
    &NIX,
);

static NIX_STRING: Grammar = Grammar::new(&[e(
    "interpolation",
    &[pi(
        &[r"(^|(?:^|(?!\$\{).)*?[^\\])\$\{(?:[^{}]|\{[^}]*\})*\}"],
        LB,
        &NIX_INTERPOLATION,
    )],
)]);

/// `prism-nix.js`.
pub static NIX: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"/\*[\s\S]*?\*/|#.*"], GR)]),
    e(
        "string",
        &[pi(
            &[r#""(?:[^"\\]|\\[\s\S])*"|''(?:(?!'')[\s\S]|''(?:'|\\|\$\{))*''"#],
            GR,
            &NIX_STRING,
        )],
    ),
    e(
        "url",
        &[p(&[
            r"[a-z][a-z0-9+.-]*:[\w%/?:@&=+$,\\.!~*'-]+|",
            r"[a-zA-Z0-9%/?:@&=+$,\\.!~*'-]*(?:/[a-zA-Z0-9%/?:@&=+$,\\.!~*'-]+)+",
        ])],
    ),
    e("antiquotation", &[pa(&[r"\$(?=\{)"], 0, &["important"])]),
    e("number", &[p(&[r"\b\d+\b"])]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:assert|builtins|else|if|in|inherit|let|null|or|rec|then|with)\b",
        ])],
    ),
    e(
        "function",
        &[p(&[
            r"\b(?:abort|add|all|any|attrNames|attrValues|baseNameOf|compareVersions|concatLists|concatMap|",
            r"deepSeq|derivation|dirOf|div|elem(?:At)?|fetch(?:git|url|Tarball)|filter(?:Source)?|",
            r"foldl'|fromJSON|genList|getAttr|getEnv|hasAttr|hashString|head|import|intersectAttrs|isAttrs|",
            r"isBool|isFunction|isInt|isList|isNull|isString|length|lessThan|listToAttrs|map|mul|parseDrvName|",
            r"pathExists|read(?:Dir|File)|removeAttrs|replaceStrings|seq|sort|stringLength|sub(?:string)?|tail|",
            r"throw|to(?:File|JSON|Path|String|XML)|trace|typeOf)\b|\bfoldl'\B",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "operator",
        &[p(&[r"[=!<>]=?|\+\+?|\|\||&&|/\\|\\/|->|[?@]"])],
    ),
    e("punctuation", &[p(&[r"[{}()\[\].,:;]"])]),
]);
