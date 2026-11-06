//! VHDL.
//!
//! Ported from Prism 1.29.0's `prism-vhdl.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, Grammar};

pub static VHDL: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"--.+"])]),
    e(
        "vhdl-vectors",
        &[pa(
            &[r#"(?i)\b[oxb]"[\da-f_]+"|"[01uxzwlh-]+""#],
            0,
            &["number"],
        )],
    ),
    e(
        "quoted-function",
        &[pa(&[r#""\S+?"(?=\()"#], 0, &["function"])],
    ),
    e("string", &[p(&[r#""(?:[^\\"\r\n]|\\(?:\r\n|[\s\S]))*""#])]),
    e("attribute", &[pa(&[r"\b'\w+"], 0, &["attr-name"])]),
    e(
        "keyword",
        &[p(&[
            r"(?i)\b(?:access|after|alias|all|architecture|array|assert|attribute|begin|block|body|",
            r"buffer|bus|case|component|configuration|constant|disconnect|downto|else|elsif|end|entity|",
            r"exit|file|for|function|generate|generic|group|guarded|if|impure|in|inertial|inout|is|label|",
            r"library|linkage|literal|loop|map|new|next|null|of|on|open|others|out|package|port|",
            r"postponed|private|procedure|process|pure|range|record|register|reject|report|return|select|",
            r"severity|shared|signal|subtype|then|to|transport|type|unaffected|units|until|use|variable|",
            r"view|wait|when|while|with)\b",
        ])],
    ),
    e("boolean", &[p(&[r"(?i)\b(?:false|true)\b"])]),
    e("function", &[p(&[r"\w+(?=\()"])]),
    e(
        "number",
        &[p(&[
            r"(?i)'[01uxzwlh-]'|\b(?:\d+#[\da-f_.]+#|\d[\d_.]*)(?:e[-+]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"(?i)[<>]=?|:=|[-+*/\&=]|\b(?:abs|and|mod|nand|nor|not|or|rem|rol|ror|sla|sll|sra|srl|xnor|",
            r"xor)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);
