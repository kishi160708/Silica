//! MATLAB.
//!
//! Ported from Prism 1.29.0's `prism-matlab.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, Grammar, GR};

pub static MATLAB: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"%\{[\s\S]*?\}%"]), p(&[r"%.+"])]),
    e("string", &[pf(&[r"\B'(?:''|[^'\r\n])*'"], GR)]),
    e(
        "number",
        &[p(&[
            r"(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:[eE][+-]?\d+)?(?:[ij])?|\b[ij]\b",
        ])],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:NaN|break|case|catch|continue|else|elseif|end|for|function|if|inf|otherwise|parfor|",
            r"pause|pi|return|switch|try|while)\b",
        ])],
    ),
    e("function", &[p(&[r"\b(?!\d)\w+(?=\s*\()"])]),
    e(
        "operator",
        &[p(&[r"\.?[*^/\\']|[+\-:@]|[<>=\~]=?|&&?|\|\|?"])],
    ),
    e("punctuation", &[p(&[r"\.{3}|[.,;\[\](){}!]"])]),
]);
