//! WebAssembly.
//!
//! Ported from Prism 1.29.0's `prism-wasm.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, pi, Grammar, GR};

pub static WASM: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"\(;[\s\S]*?;\)"]), pf(&[r";;.*"], GR)]),
    e("string", &[pf(&[r#""(?:\\[\s\S]|[^"\\])*""#], GR)]),
    e(
        "keyword",
        &[
            pi(&[r"\b(?:align|offset)="], 0, &WASM_KEYWORD_1),
            pi(
                &[
                    r"\b(?:(?:f32|f64|i32|i64)(?:\.(?:abs|add|and|ceil|clz|const|convert_[su]/i(?:32|64)|",
                    r"copysign|ctz|demote/f64|div(?:_[su])?|eqz?|extend_[su]/i32|floor|ge(?:_[su])?|gt(?:_[su])?|",
                    r"le(?:_[su])?|load(?:(?:8|16|32)_[su])?|lt(?:_[su])?|max|min|mul|neg?|nearest|or|popcnt|",
                    r"promote/f32|reinterpret/[fi](?:32|64)|rem_[su]|rot[lr]|shl|shr_[su]|sqrt|store(?:8|16|32)?|",
                    r"sub|trunc(?:_[su]/f(?:32|64))?|wrap/i64|xor))?|memory\.(?:grow|size))\b",
                ],
                0,
                &WASM_KEYWORD_2,
            ),
            p(&[
                r"\b(?:anyfunc|block|br(?:_if|_table)?|call(?:_indirect)?|data|drop|elem|else|end|export|",
                r"func|get_(?:global|local)|global|if|import|local|loop|memory|module|mut|nop|offset|param|",
                r"result|return|select|set_(?:global|local)|start|table|tee_local|then|type|unreachable)\b",
            ]),
        ],
    ),
    e("variable", &[p(&[r"\$[\w!#$%\&'*+\-./:<=>?@\\^`|\~]+"])]),
    e(
        "number",
        &[p(&[
            r"[+-]?\b(?:\d(?:_?\d)*(?:\.\d(?:_?\d)*)?(?:[eE][+-]?\d(?:_?\d)*)?|",
            r"0x[\da-fA-F](?:_?[\da-fA-F])*(?:\.[\da-fA-F](?:_?[\da-fA-D])*)?(?:[pP][+-]?\d(?:_?\d)*)?)\b",
            r"|\binf\b|\bnan(?::0x[\da-fA-F](?:_?[\da-fA-D])*)?\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);

pub(super) static WASM_KEYWORD_1: Grammar = Grammar::new(&[e("operator", &[p(&[r"="])])]);

pub(super) static WASM_KEYWORD_2: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
