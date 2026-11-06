//! F#.
//!
//! Ported from Prism 1.29.0's `prism-clike.js`, `prism-fsharp.js` and `prism-xml-doc.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pf, pi, Grammar, GR, LB};

pub static FSHARP: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(
            &[r"///.*"],
            GR,
            &["comment"],
            &super::csharp::CSHARP_DOC_COMMENT,
        )],
    ),
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])\(\*(?!\))[\s\S]*?\*\)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e("annotation", &[pi(&[r"\[<.+?>\]"], GR, &FSHARP_ANNOTATION)]),
    e(
        "char",
        &[pf(
            &[r"'(?:[^\\']|\\(?:.|\d{3}|x[a-fA-F\d]{2}|u[a-fA-F\d]{4}|U[a-fA-F\d]{8}))'B?"],
            GR,
        )],
    ),
    e(
        "string",
        &[pf(
            &[r#"(?:"""[\s\S]*?"""|@"(?:""|[^"])*"|"(?:\\[\s\S]|[^\\"])*")B?"#],
            GR,
        )],
    ),
    e(
        "class-name",
        &[pi(
            &[
                r"(\b(?:exception|inherit|interface|new|of|type)\s+|\w\s*:\s*|\s:\??>\s*)[.\w]+\b(?:\s*(?:->|",
                r"\*)\s*[.\w]+\b)*(?!\s*[:.])",
            ],
            LB,
            &FSHARP_CLASS_NAME,
        )],
    ),
    e(
        "preprocessor",
        &[pai(
            &[r"(?m)(^[\t ]*)#.*"],
            LB,
            &["property"],
            &FSHARP_PREPROCESSOR,
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:let|return|use|yield)(?:!\B|\b)|\b(?:abstract|and|as|asr|assert|atomic|base|begin|",
            r"break|checked|class|component|const|constraint|constructor|continue|default|delegate|do|",
            r"done|downcast|downto|eager|elif|else|end|event|exception|extern|external|false|finally|",
            r"fixed|for|fun|function|functor|global|if|in|include|inherit|inline|interface|internal|land|",
            r"lazy|lor|lsl|lsr|lxor|match|member|method|mixin|mod|module|mutable|namespace|new|not|null|",
            r"object|of|open|or|override|parallel|private|process|protected|public|pure|rec|sealed|",
            r"select|sig|static|struct|tailcall|then|to|trait|true|try|type|upcast|val|virtual|void|",
            r"volatile|when|while|with)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "number",
        &[
            p(&[r"\b0x[\da-fA-F]+(?:LF|lf|un)?\b"]),
            p(&[r"\b0b[01]+(?:uy|y)?\b"]),
            p(&[r"(?i)(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:[fm]|e[+-]?\d+)?\b"]),
            p(&[r"\b\d+(?:[IlLsy]|UL|u[lsy]?)?\b"]),
        ],
    ),
    e(
        "operator",
        &[p(&[
            r"([<>\~\&^])\1\1|([*.:<>\&])\2|<-|->|[!=:]=|<?\|{1,3}>?|\??(?:<=|>=|<>|[-+*/%=<>])\??|",
            r"[!?^\&]|~[+\~-]|:>|:\?>?",
        ])],
    ),
    e(
        "computation-expression",
        &[pa(&[r"(?i)\b[_a-z]\w*(?=\s*\{)"], 0, &["keyword"])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static FSHARP_ANNOTATION: Grammar = Grammar::new(&[
    e("punctuation", &[p(&[r"^\[<|>\]$"])]),
    e("class-name", &[pf(&[r"^\w+$|(^|;\s*)[A-Z]\w*(?=\()"], LB)]),
    e("annotation-content", &[pi(&[r"[\s\S]+"], 0, &FSHARP)]),
]);

pub(super) static FSHARP_CLASS_NAME: Grammar = Grammar::new(&[
    e("operator", &[p(&[r"->|\*"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static FSHARP_PREPROCESSOR: Grammar = Grammar::new(&[e(
    "directive",
    &[pa(
        &[r"(^#)\b(?:else|endif|if|light|line|nowarn)\b"],
        LB,
        &["keyword"],
    )],
)]);
