//! Python and Rust.
//!
//! Ported from Prism's `components/prism-{python,rust}.js`.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

// ---- Python ---------------------------------------------------------------

static PY_FORMAT_SPEC: Grammar = Grammar::new(&[
    e("format-spec", &[pf(&[r"(:)[^:(){}]+(?=\}$)"], LB)]),
    e(
        "conversion-option",
        &[pa(&[r"![sra](?=[:}]$)"], 0, &["punctuation"])],
    ),
]);

/// The inside of an f-string's `{…}`. Prism sets its `rest` to Python
/// itself, so an expression in a format field is highlighted as code.
static PY_INTERPOLATION_INSIDE: Grammar = Grammar::with_rest(PY_FORMAT_SPEC.entries, &PYTHON);

static PY_STRING_INTERPOLATION: Grammar = Grammar::new(&[
    e(
        "interpolation",
        &[pi(
            &[
                r"((?:^|[^{])(?:\{\{)*)\{(?!\{)(?:[^{}]|\{(?!\{)(?:[^{}]|\{(?!\{)(?:[^{}])+\})+\})+\}",
            ],
            LB,
            &PY_INTERPOLATION_INSIDE,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

static PY_DECORATOR_INSIDE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

/// `prism-python.js`. `py` is the same grammar.
pub static PYTHON: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"(^|[^\\])#.*"], LB | GR)]),
    e(
        "string-interpolation",
        &[pi(
            &[r#"(?i)(?:f|fr|rf)(?:("""|''')[\s\S]*?\1|("|')(?:\\.|(?!\2)[^\\\r\n])*\2)"#],
            GR,
            &PY_STRING_INTERPOLATION,
        )],
    ),
    e(
        "triple-quoted-string",
        &[pa(
            &[r#"(?i)(?:[rub]|br|rb)?("""|''')[\s\S]*?\1"#],
            GR,
            &["string"],
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#"(?i)(?:[rub]|br|rb)?("|')(?:\\.|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
    ),
    e(
        "function",
        &[pf(&[r"((?:^|\s)def[ \t]+)[a-zA-Z_]\w*(?=\s*\()"], LB)],
    ),
    e("class-name", &[pf(&[r"(?i)(\bclass\s+)\w+"], LB)]),
    e(
        "decorator",
        &[pai(
            &[r"(?m)(^[\t ]*)@\w+(?:\.\w+)*"],
            LB,
            &["annotation", "punctuation"],
            &PY_DECORATOR_INSIDE,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:_(?=\s*:)|and|as|assert|async|await|break|case|class|continue|def|del|elif|else|except|exec|",
            r"finally|for|from|global|if|import|in|is|lambda|match|nonlocal|not|or|pass|print|raise|return|try|",
            r"while|with|yield)\b",
        ])],
    ),
    e(
        "builtin",
        &[p(&[
            r"\b(?:__import__|abs|all|any|apply|ascii|basestring|bin|bool|buffer|bytearray|bytes|callable|chr|",
            r"classmethod|cmp|coerce|compile|complex|delattr|dict|dir|divmod|enumerate|eval|execfile|file|filter|",
            r"float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|intern|isinstance|",
            r"issubclass|iter|len|list|locals|long|map|max|memoryview|min|next|object|oct|open|ord|pow|property|",
            r"range|raw_input|reduce|reload|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|",
            r"super|tuple|type|unichr|unicode|vars|xrange|zip)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:False|None|True)\b"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0(?:b(?:_?[01])+|o(?:_?[0-7])+|x(?:_?[a-f0-9])+)\b|",
            r"(?:\b\d+(?:_\d+)*(?:\.(?:\d+(?:_\d+)*)?)?|\B\.\d+(?:_\d+)*)(?:e[+-]?\d+(?:_\d+)*)?j?(?!\w)",
        ])],
    ),
    e(
        "operator",
        &[p(&[r"[-+%=]=?|!=|:=|\*\*?=?|//?=?|<[<=>]?|>[=>]?|[&|^~]"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

// ---- Rust -----------------------------------------------------------------

/// Prism nests `/* … */` four deep by substituting the pattern into itself
/// twice and then closing it off with a class that matches nothing. Written
/// out here, because the result is what the engine needs and the
/// substitution is not something a `static` can do.
const RUST_COMMENT_1: &str = r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|[^\s\S])*\*/";
const RUST_COMMENT_2: &str = concat!(
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|",
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|[^\s\S])*\*/",
    r")*\*/",
);
const RUST_COMMENT_4: &str = concat!(
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|",
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|",
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|",
    r"/\*(?:[^*/]|\*(?!/)|/(?!\*)|[^\s\S])*\*/",
    r")*\*/",
    r")*\*/",
    r")*\*/",
);

static RUST_STRING_ONLY: Grammar = Grammar::new(&[e(
    "string",
    &[pf(
        &[r#"b?"(?:\\[\s\S]|[^\\"])*"|b?r(#*)"(?:[^"]|"(?!\1))*"\1"#],
        GR,
    )],
)]);

static RUST_CLOSURE_PUNCT: Grammar = Grammar::with_rest(
    &[e(
        "closure-punctuation",
        &[pa(&[r"^\||\|$"], 0, &["punctuation"])],
    )],
    &RUST,
);

static RUST_NAMESPACE_INSIDE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"::"])])]);

/// `prism-rust.js`.
pub static RUST: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])", RUST_COMMENT_4], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[pf(
            &[r#"b?"(?:\\[\s\S]|[^\\"])*"|b?r(#*)"(?:[^"]|"(?!\1))*"\1"#],
            GR,
        )],
    ),
    e(
        "char",
        &[pf(
            &[r"b?'(?:\\(?:x[0-7][\da-fA-F]|u\{(?:[\da-fA-F]_*){1,6}\}|.)|[^\\\r\n\t'])'"],
            GR,
        )],
    ),
    e(
        "attribute",
        &[pai(
            &[r#"#!?\[(?:[^\[\]"]|"(?:\\[\s\S]|[^\\"])*")*\]"#],
            GR,
            &["attr-name"],
            &RUST_STRING_ONLY,
        )],
    ),
    e(
        "closure-params",
        &[pi(
            &[r"([=(,:]\s*|\bmove\s*)\|[^|]*\||\|[^|]*\|(?=\s*(?:\{|->))"],
            LB | GR,
            &RUST_CLOSURE_PUNCT,
        )],
    ),
    e("lifetime-annotation", &[pa(&[r"'\w+"], 0, &["symbol"])]),
    e(
        "fragment-specifier",
        &[pa(&[r"(\$\w+:)[a-z]+"], LB, &["punctuation"])],
    ),
    e("variable", &[p(&[r"\$\w+"])]),
    e(
        "function-definition",
        &[pa(&[r"(\bfn\s+)\w+"], LB, &["function"])],
    ),
    e(
        "type-definition",
        &[pa(
            &[r"(\b(?:enum|struct|trait|type|union)\s+)\w+"],
            LB,
            &["class-name"],
        )],
    ),
    e(
        "module-declaration",
        &[
            pa(&[r"(\b(?:crate|mod)\s+)[a-z][a-z_\d]*"], LB, &["namespace"]),
            pai(
                &[
                    r"(\b(?:crate|self|super)\s*)::\s*[a-z][a-z_\d]*\b(?:\s*::(?:\s*[a-z][a-z_\d]*\s*::)*)?",
                ],
                LB,
                &["namespace"],
                &RUST_NAMESPACE_INSIDE,
            ),
        ],
    ),
    e(
        "keyword",
        &[
            p(&[
                r"\b(?:Self|abstract|as|async|await|become|box|break|const|continue|crate|do|dyn|else|enum|",
                r"extern|final|fn|for|if|impl|in|let|loop|macro|match|mod|move|mut|override|priv|pub|ref|",
                r"return|self|static|struct|super|trait|try|type|typeof|union|unsafe|unsized|use|virtual|",
                r"where|while|yield)\b",
            ]),
            p(&[r"\b(?:bool|char|f(?:32|64)|[ui](?:8|16|32|64|128|size)|str)\b"]),
        ],
    ),
    e("function", &[p(&[r"\b[a-z_]\w*(?=\s*(?:::\s*<|\())"])]),
    e("macro", &[pa(&[r"\b\w+!"], 0, &["property"])]),
    e("constant", &[p(&[r"\b[A-Z_][A-Z_\d]+\b"])]),
    e("class-name", &[p(&[r"\b[A-Z]\w*\b"])]),
    e(
        "namespace",
        &[pi(
            &[r"(?:\b[a-z][a-z_\d]*\s*::\s*)*\b[a-z][a-z_\d]*\s*::(?!\s*<)"],
            0,
            &RUST_NAMESPACE_INSIDE,
        )],
    ),
    e(
        "number",
        &[p(&[
            r"\b(?:0x[\dA-Fa-f](?:_?[\dA-Fa-f])*|0o[0-7](?:_?[0-7])*|0b[01](?:_?[01])*|",
            r"(?:(?:\d(?:_?\d)*)?\.)?\d(?:_?\d)*(?:[Ee][+-]?\d+)?)(?:_?(?:f32|f64|[iu](?:8|16|32|64|size)?))?\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("punctuation", &[p(&[r"->|\.\.=|\.{1,3}|::|[{}\[\];(),:]"])]),
    e(
        "operator",
        &[p(&[r"[-+*/%!^]=?|=[=>]?|&[&=]?|\|[|=]?|<<?=?|>>?=?|[@?]"])],
    ),
]);

/// Prism keeps the unused single- and double-nesting steps around as it
/// builds the four-deep comment; naming them documents the derivation and
/// keeps the constants from reading as dead weight.
#[allow(dead_code)]
const _DERIVATION: (&str, &str) = (RUST_COMMENT_1, RUST_COMMENT_2);
