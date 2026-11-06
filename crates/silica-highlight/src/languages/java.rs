//! Java.
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-java.js`, `prism-javadoc.js` and
//! `prism-javadoclike.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static JAVA: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"(^|[^\\])/\*\*[^/][\s\S]*?(?:\*/|$)"],
            LB,
            &["comment"],
            &JAVA_DOC_COMMENT,
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
        "triple-quoted-string",
        &[pa(
            &[r#""""[ \t]*[\r\n](?:(?:"|"")?(?:\\.|[^"\\]))*""""#],
            GR,
            &["string"],
        )],
    ),
    e("char", &[pf(&[r"'(?:\\.|[^'\\\r\n]){1,6}'"], GR)]),
    e(
        "string",
        &[pf(&[r#"(^|[^\\])"(?:\\.|[^"\\\r\n])*""#], LB | GR)],
    ),
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
            &JAVA_GENERICS,
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
                &JAVA_IMPORT_1,
            ),
            pai(
                &[
                    r"(\bimport\s+static\s+)(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*(?:\w+|\*)(?=\s*;)",
                ],
                LB,
                &["static"],
                &JAVA_IMPORT_2,
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
            &JAVA_NAMESPACE,
        )],
    ),
    e(
        "class-name",
        &[
            pi(
                &[
                    r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z](?:[\d_A-Z]*[a-z]\w*)?\b",
                ],
                LB,
                &JAVA_GENERICS_CLASS_NAME,
            ),
            pi(
                &[
                    r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z]\w*(?=\s+\w+\s*[;,=()]|",
                    r"\s*(?:\[[\s,]*\]\s*)?::\s*new\b)",
                ],
                LB,
                &JAVA_GENERICS_CLASS_NAME,
            ),
            pi(
                &[
                    r"(\b(?:class|enum|extends|implements|instanceof|interface|new|record|",
                    r"throws)\s+)(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z]\w*\b",
                ],
                LB,
                &JAVA_GENERICS_CLASS_NAME,
            ),
        ],
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
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "function",
        &[p(&[r"\b\w+(?=\()"]), pf(&[r"(::\s*)[a-z_]\w*"], LB)],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b0b[01][01_]*L?\b|\b0x(?:\.[\da-f_p+-]+|[\da-f_]+(?:\.[\da-f_p+-]+)?)\b|",
            r"(?:\b\d[\d_]*(?:\.[\d_]*)?|\B\.\d[\d_]*)(?:e[+-]?\d[\d_]*)?[dfl]?",
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
    e("constant", &[p(&[r"\b[A-Z][A-Z_\d]+\b"])]),
]);

pub(super) static JAVA_DOC_COMMENT: Grammar = Grammar::new(&[
    e(
        "parameter",
        &[pf(
            &[r"(?m)(^[\t ]*(?:/{3}|\*|/\*\*)\s*@(?:arg|arguments|param)\s+)\w+"],
            LB,
        )],
    ),
    e(
        "reference",
        &[pi(
            &[
                r"(@(?:exception|link|linkplain|see|throws|",
                r"value)\s+(?:\*\s*)?)(?:(?:\b[a-zA-Z]\w+\s*\.\s*)*\b[A-Z]\w*(?:\s*#\s*\w+(?:\s*\([^()]*\))?",
                r")?|#\s*\w+(?:\s*\([^()]*\))?)",
            ],
            LB,
            &JAVA_DOC_COMMENT_REFERENCE,
        )],
    ),
    e(
        "class-name",
        &[pi(
            &[r"(@param\s+)<[A-Z]\w*>"],
            LB,
            &JAVA_DOC_COMMENT_CLASS_NAME,
        )],
    ),
    e(
        "code-section",
        &[
            pi(
                &[
                    r"(\{@code\s+(?!\s))(?:[^\s{}]|\s+(?![\s}])|\{(?:[^{}]|\{(?:[^{}]|\{(?:[^{}]|",
                    r"\{[^{}]*\})*\})*\})*\})+(?=\s*\})",
                ],
                LB,
                &JAVA_DOC_COMMENT_CODE_SECTION_1,
            ),
            pi(
                &[r"(<(code|pre|tt)>(?!<code>)\s*)\S(?:\S|\s+\S)*?(?=\s*</\2>)"],
                LB,
                &JAVA_DOC_COMMENT_CODE_SECTION_2,
            ),
        ],
    ),
    e(
        "tag",
        &[pi(
            &[
                r#"</?(?!\d)[^\s>/=$<%]+(?:\s(?:\s*[^\s>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))|"#,
                r"(?=[\s/>])))+)?\s*/?>",
            ],
            GR,
            &super::markup::MARKUP_TAG,
        )],
    ),
    e(
        "entity",
        &[
            pa(&[r"(?i)&[\da-z]{1,8};"], 0, &["named-entity"]),
            p(&[r"(?i)&#x?[\da-f]{1,8};"]),
        ],
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

pub(super) static JAVA_DOC_COMMENT_REFERENCE: Grammar = Grammar::new(&[
    e("function", &[pf(&[r"(#\s*)\w+(?=\s*\()"], LB)]),
    e("field", &[pf(&[r"(#\s*)\w+"], LB)]),
    e(
        "namespace",
        &[pi(
            &[r"\b(?:[a-z]\w*\s*\.\s*)+"],
            0,
            &JAVA_DOC_COMMENT_REFERENCE_NAMESPACE,
        )],
    ),
    e("class-name", &[p(&[r"\b[A-Z]\w*"])]),
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
    e("punctuation", &[p(&[r"[#()\[\],.]"])]),
]);

pub(super) static JAVA_DOC_COMMENT_REFERENCE_NAMESPACE: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JAVA_DOC_COMMENT_CLASS_NAME: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"[.<>]"])])]);

pub(super) static JAVA_DOC_COMMENT_CODE_SECTION_1: Grammar = Grammar::new(&[e(
    "code",
    &[pai(
        &[r"(?m)(^(?:[\t ]*(?:\*\s*)*))[^*\s].*$"],
        LB,
        &["language-java"],
        &JAVA,
    )],
)]);

pub(super) static JAVA_DOC_COMMENT_CODE_SECTION_2: Grammar = Grammar::new(&[e(
    "line",
    &[pi(
        &[r"(?m)(^(?:[\t ]*(?:\*\s*)*))[^*\s].*$"],
        LB,
        &JAVA_DOC_COMMENT_CODE_SECTION_2_LINE,
    )],
)]);

pub(super) static JAVA_DOC_COMMENT_CODE_SECTION_2_LINE: Grammar = Grammar::new(&[
    e(
        "tag",
        &[pi(
            &[
                r#"</?(?!\d)[^\s>/=$<%]+(?:\s(?:\s*[^\s>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))|"#,
                r"(?=[\s/>])))+)?\s*/?>",
            ],
            GR,
            &super::markup::MARKUP_TAG,
        )],
    ),
    e(
        "entity",
        &[
            pa(&[r"(?i)&[\da-z]{1,8};"], 0, &["named-entity"]),
            p(&[r"(?i)&#x?[\da-f]{1,8};"]),
        ],
    ),
    e("code", &[pai(&[r".+"], 0, &["language-java"], &JAVA)]),
]);

pub(super) static JAVA_GENERICS: Grammar = Grammar::new(&[
    e(
        "class-name",
        &[pi(
            &[r"(^|[^\w.])(?:[a-z]\w*\s*\.\s*)*(?:[A-Z]\w*\s*\.\s*)*[A-Z](?:[\d_A-Z]*[a-z]\w*)?\b"],
            LB,
            &JAVA_GENERICS_CLASS_NAME,
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

pub(super) static JAVA_GENERICS_CLASS_NAME: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &JAVA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static JAVA_GENERICS_CLASS_NAME_NAMESPACE: Grammar =
    Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static JAVA_IMPORT_1: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &JAVA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("punctuation", &[p(&[r"\."])]),
    e("operator", &[p(&[r"\*"])]),
    e("class-name", &[p(&[r"\w+"])]),
]);

pub(super) static JAVA_IMPORT_2: Grammar = Grammar::new(&[
    e(
        "namespace",
        &[pi(
            &[r"^[a-z]\w*(?:\s*\.\s*[a-z]\w*)*(?:\s*\.)?"],
            0,
            &JAVA_GENERICS_CLASS_NAME_NAMESPACE,
        )],
    ),
    e("static", &[p(&[r"\b\w+$"])]),
    e("punctuation", &[p(&[r"\."])]),
    e("operator", &[p(&[r"\*"])]),
    e("class-name", &[p(&[r"\w+"])]),
]);

pub(super) static JAVA_NAMESPACE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
