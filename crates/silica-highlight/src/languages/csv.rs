//! CSV.
//!
//! Ported from Prism 1.29.0's `prism-csv.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, Grammar};

pub static CSV: Grammar = Grammar::new(&[
    e("value", &[p(&[r#"[^\r\n,"]+|"(?:[^"]|"")*"(?!")"#])]),
    e("punctuation", &[p(&[r","])]),
]);
