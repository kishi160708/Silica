//! Solidity (Ethereum) (`sol`).
//!
//! Ported from Prism 1.29.0's `prism-clike.js` and `prism-solidity.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR, LB};

pub static SOLIDITY: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^\\])/\*[\s\S]*?(?:\*/|$)"], LB | GR),
            pf(&[r"(^|[^\\:])//.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[pf(
            &[r#"(["'])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1"#],
            GR,
        )],
    ),
    e(
        "class-name",
        &[pf(
            &[r"(\b(?:contract|enum|interface|library|new|struct|using)\s+)(?!\d)[\w$]+"],
            LB,
        )],
    ),
    e(
        "builtin",
        &[p(&[
            r"\b(?:address|bool|byte|u?int(?:8|16|24|32|40|48|56|64|72|80|88|96|104|112|120|128|136|144|",
            r"152|160|168|176|184|192|200|208|216|224|232|240|248|256)?|string|bytes(?:[1-9]|[12]\d|",
            r"3[0-2])?)\b",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:_|anonymous|as|assembly|assert|break|calldata|case|constant|constructor|continue|",
            r"contract|default|delete|do|else|emit|enum|event|external|for|from|function|if|import|",
            r"indexed|inherited|interface|internal|is|let|library|mapping|memory|modifier|new|payable|",
            r"pragma|private|public|pure|require|returns?|revert|selfdestruct|solidity|storage|struct|",
            r"suicide|switch|this|throw|using|var|view|while)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "version",
        &[pa(&[r"([<>]=?|\^)\d+\.\d+\.\d+\b"], LB, &["number"])],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b0x[\da-f]+\b|(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:e[+-]?\d+)?",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"=>|->|:=|=:|\*\*|\+\+|--|\|\||&&|<<=?|>>=?|[-+*/%^\&|<>!=]=?|[\~?]",
        ])],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);
