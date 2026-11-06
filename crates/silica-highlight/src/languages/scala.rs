//! Scala.
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-java.js`, `prism-scala.js`,
//! `prism-javadoc.js` and `prism-javadoclike.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static SCALA: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &SCALA_DOC_COMMENT,
        )],
    ),
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string-interpolation",
        &[pi(
            &[
                r#"(?i)\b[a-z]\w*(?:"""(?:[^$]|\$(?:[^{]|\{(?:[^{}]|\{[^{}]*\})*\}))*?"""|"(?:[^$"\r\n]|"#,
                r#"\$(?:[^{]|\{(?:[^{}]|\{[^{}]*\})*\}))*")"#,
            ],
            GR,
            &SCALA_STRING_INTERPOLATION,
        )],
    ),
    e(
        "triple-quoted-string",
        &[pa(&[r#""""[\s\S]*?""""#], GR, &["string"])],
    ),
    e("char", &[pf(&[r"'(?:\\.|[^'\\\r\n]){1,6}'"], GR)]),
    e("string", &[pf(&[r#"("|')(?:\\.|(?!\1)[^\\\r\n])*\1"#], GR)]),
    e(
        "annotation",
        &[pa(&[r"(^|[^.])@\w+(?:\s*\.\s*\w+)*"], LB, &["punctuation"])],
    ),
    e(
        "generics",
        &[pi(
            &[
                r"<(?:[\w\s,.?]|&(?!&)|<(?:[\w\s,.?]|&(?!&)|<(?:[\w\s,.?]|&(?!&)|<(?:[\w\s,.?]|",
                r"&(?!&))*>)*>)*>)*>",
            ],
            0,
            &SCALA_GENERICS,
        )],
    ),
    e(
        "import",
        &[
            pi(
                &[
                    r"(\bimport\s+)(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*(?:[A-Z]\w*|\*)(?=\s*;)",
                ],
                LB,
                &SCALA_IMPORT_1,
            ),
            pai(
                &[
                    r"(\bimport\s+static\s+)(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*(?:\w+|\*)(?=\s*;)",
                ],
                LB,
                &["static"],
                &SCALA_IMPORT_2,
            ),
        ],
    ),
    e(
        "namespace",
        &[pi(
            &[
                r"(\b(?:exports|import(?:\s+static)?|module|open|opens|package|provides|requires|to|",
                r"transitive|uses|with)\s+)(?!\b(?:abstract|assert|boolean|break|byte|case|catch|char|class|",
                r"const|continue|default|do|double|else|enum|exports|extends|final|finally|float|for|goto|if|",
                r"implements|import|instanceof|int|interface|long|module|native|new|non-sealed|null|open|",
                r"opens|package|permits|private|protected|provides|public|",
                r"record(?!\s*[(){}\[\]<>=%\~.:,;?+\-*/\&|^])|requires|return|sealed|short|static|strictfp|",
                r"super|switch|synchronized|this|throw|throws|to|transient|transitive|try|uses|var|void|",
                r"volatile|while|with|yield)\b)[a-z]\w*(?:\.[a-z]\w*)*\.?",
            ],
            LB,
            &SCALA_NAMESPACE,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"<-|=>|\b(?:abstract|case|catch|class|def|derives|do|else|enum|extends|extension|final|",
            r"finally|for|forSome|given|if|implicit|import|infix|inline|lazy|match|new|null|object|",
            r"opaque|open|override|package|private|protected|return|sealed|self|super|this|throw|trait|",
            r"transparent|try|type|using|val|var|while|with|yield)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x(?:[\da-f]*\.)?[\da-f]+|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e\d+)?[dfl]?",
        ])],
    ),
    e(
        "operator",
        &[pf(
            &[r"(?m)(^|[^.])(?:<<=?|>>>?=?|->|--|\+\+|&&|\|\||::|[?:\~]|[-+*/%\&|^!=<>]=?)"],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
    e(
        "builtin",
        &[p(&[
            r"\b(?:Any|AnyRef|AnyVal|Boolean|Byte|Char|Double|Float|Int|Long|Nothing|Short|String|",
            r"Unit)\b",
        ])],
    ),
    e("symbol", &[p(&[r"'[^\d\s\\]\w*"])]),
]);

pub(super) static SCALA_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*@(?:arg|arguments|param)\s+)\w+"],
            LB,
        )],
    ),
    e(
        "keyword",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*|\{)@[a-z][a-zA-Z-]+\b"],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[{}]"])]),
]);

pub(super) static SCALA_STRING_INTERPOLATION: Grammar = Grammar::new(&[
    e("id", &[pa(&[r"^\w+"], GR, &["function"])]),
    e("escape", &[pa(&[r#"\\\$"|\$[$"]"#], GR, &["symbol"])]),
    e(
        "interpolation",
        &[pi(
            &[r"\$(?:\w+|\{(?:[^{}]|\{[^{}]*\})*\})"],
            GR,
            &SCALA_STRING_INTERPOLATION_INTERPOLATION,
        )],
    ),
    e("string", &[p(&[r"[\s\S]+"])]),
]);

pub(super) static SCALA_STRING_INTERPOLATION_INTERPOLATION: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\$\{?|\}$"])]),
    e("expression", &[pi(&[r"[\s\S]+"], 0, &SCALA)]),
]);

pub(super) static SCALA_GENERICS: Grammar = Grammar::new(&[
    e(
        "class-name",
        &[pi(
            &[r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z](?:[\d_A-Z]*[a-z]\w*)?\b"],
            LB,
            &SCALA_GENERICS_CLASS_NAME,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:abstract|assert|boolean|break|byte|case|catch|char|class|const|continue|default|do|",
            r"double|else|enum|exports|extends|final|finally|float|for|goto|if|implements|import|",
            r"instanceof|int|interface|long|module|native|new|non-sealed|null|open|opens|package|permits|",
            r"private|protected|provides|public|record(?!\s*[(){}\[\]<>=%\~.:,;?+\-*/\&|^])|requires|",
            r"return|sealed|short|static|strictfp|super|switch|synchronized|this|throw|throws|to|",
            r"transient|transitive|try|uses|var|void|volatile|while|with|yield)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[<>(),.:]"])]),
    e("operator", &[p(&[r"[?\&|]"])]),
]);

pub(super) static SCALA_GENERICS_CLASS_NAME: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &SCALA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static SCALA_GENERICS_CLASS_NAME_NAMESPACE: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static SCALA_IMPORT_1: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &SCALA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("punctuation", &[p(&[r"\."])]),
    e("operator", &[p(&[r"\*"])]),
    e("class-name", &[p(&[r"\w+"])]),
]);

pub(super) static SCALA_IMPORT_2: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &SCALA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("static", &[p(&[r"\b\w+$"])]),
    e("punctuation", &[p(&[r"\."])]),
    e("operator", &[p(&[r"\*"])]),
    e("class-name", &[p(&[r"\w+"])]),
]);

pub(super) static SCALA_NAMESPACE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
