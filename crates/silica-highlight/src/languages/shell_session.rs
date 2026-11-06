//! Shell session (`sh-session`, `shellsession`).
//!
//! Ported from Prism 1.29.0's `prism-bash.js` and `prism-shell-session.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pai, pi, Grammar, GR, LB};

pub static SHELL_SESSION: Grammar = Grammar::new(&[
    e(
        "command",
        &[pi(
            &[
                r#"(?m)^(?:[^\s@:$#%*!/\\]+@[^\r\n@:$#%*!/\\]+(?::[^\x00-\x1F$#%*?"<>:;|]+)?|"#,
                r#"[/\~.][^\x00-\x1F$#%*?"<>@:;|]*)?[$#%](?=\s)(?:[^\\\r\n \t'"<$]|[ \t](?:(?!#)|#.*$)|"#,
                r#"\\(?:[^\r]|\r\n?)|\$(?!')|<(?!<)|"(?:\\[\s\S]|\$\([^)]+\)|\$(?!\()|`[^`]+`|[^"\\`$])*"|"#,
                r#"'[^']*'|\$'(?:[^'\\]|\\[\s\S])*'|<<-?\s*(["']?)(\w+)\1\s[\s\S]*?[\r\n]\2)+"#,
            ],
            GR,
            &SHELL_SESSION_COMMAND,
        )],
    ),
    e("output", &[p(&[r".(?:.*(?:[\r\n]|.$))*"])]),
]);

pub(super) static SHELL_SESSION_COMMAND: Grammar = Grammar::new(&[
    e(
        "info",
        &[pai(
            &[r"^[^#$%]+"],
            0,
            &["punctuation"],
            &SHELL_SESSION_COMMAND_INFO,
        )],
    ),
    e(
        "bash",
        &[pai(
            &[r"(^[$#%]\s*)\S[\s\S]*"],
            LB,
            &["language-bash"],
            &super::bash::BASH,
        )],
    ),
    e("shell-symbol", &[pa(&[r"^[$#%]"], 0, &["important"])]),
]);

pub(super) static SHELL_SESSION_COMMAND_INFO: Grammar = Grammar::new(&[
    e("user", &[p(&[r"^[^\s@:$#%*!/\\]+@[^\r\n@:$#%*!/\\]+"])]),
    e("punctuation", &[p(&[r":"])]),
    e("path", &[p(&[r"[\s\S]+"])]),
]);
