//! Git.
//!
//! Ported from Prism 1.29.0's `prism-git.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pi, Grammar};

pub static GIT: Grammar = Grammar::new(&[
    e("comment", &[p(&[r"(?m)^#.*"])]),
    e("deleted", &[p(&[r"(?m)^[-–].*"])]),
    e("inserted", &[p(&[r"(?m)^\+.*"])]),
    e("string", &[p(&[r#"("|')(?:\\.|(?!\1)[^\\\r\n])*\1"#])]),
    e("command", &[pi(&[r"(?m)^.*\$ git .*$"], 0, &GIT_COMMAND)]),
    e("coord", &[p(&[r"(?m)^@@.*@@$"])]),
    e("commit-sha1", &[p(&[r"(?m)^commit \w{40}$"])]),
]);

pub(super) static GIT_COMMAND: Grammar = Grammar::new(&[e("parameter", &[p(&[r"\s--?\w+"])])]);
