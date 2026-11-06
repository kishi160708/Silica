//! Docker (`dockerfile`).
//!
//! Ported from Prism 1.29.0's `prism-docker.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, pi, Grammar, GR, LB};

pub static DOCKER: Grammar = Grammar::new(&[
    e(
        "instruction",
        &[pi(
            &[
                r"(?im)(^[ \t]*)(?:ADD|ARG|CMD|COPY|ENTRYPOINT|ENV|EXPOSE|FROM|HEALTHCHECK|LABEL|MAINTAINER|",
                r"ONBUILD|RUN|SHELL|STOPSIGNAL|USER|VOLUME|WORKDIR)(?=\s)(?:\\.|[^\r\n\\])*(?:\\$(?:\s|",
                r"#.*$)*(?![\s#])(?:\\.|[^\r\n\\])*)*",
            ],
            LB | GR,
            &DOCKER_INSTRUCTION,
        )],
    ),
    e("comment", &[pf(&[r"(?m)(^[ \t]*)#.*"], LB | GR)]),
]);

pub(super) static DOCKER_INSTRUCTION: Grammar = Grammar::new(&[
    e(
        "options",
        &[pi(
            &[
                r"(?i)(^(?:ONBUILD(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                r"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                r"\\[\r\n])))?\w+(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                r#"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n])))--[\w-]+=(?:"(?:[^"\\\r\n]|\\(?:\r\n|"#,
                r#"[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|[\s\S]))*'|(?!["'])(?:[^\s\\]|"#,
                r"\\.)+)(?:(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                r#"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))--[\w-]+=(?:"(?:[^"\\\r\n]|\\(?:\r\n|"#,
                r#"[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|[\s\S]))*'|(?!["'])(?:[^\s\\]|\\.)+))*"#,
            ],
            LB | GR,
            &DOCKER_INSTRUCTION_OPTIONS,
        )],
    ),
    e(
        "keyword",
        &[
            pf(
                &[
                    r"(?i)(^(?:ONBUILD(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                    r"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n])))?HEALTHCHECK(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n]))?|\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r#"\\[\r\n]))(?:--[\w-]+=(?:"(?:[^"\\\r\n]|\\(?:\r\n|[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|"#,
                    r#"[\s\S]))*'|(?!["'])(?:[^\s\\]|\\.)+)(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|"#,
                    r"#.*(?!.))*(?![\s#]|\\[\r\n]))?|\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n])))*)(?:CMD|NONE)\b",
                ],
                LB | GR,
            ),
            pf(
                &[
                    r"(?i)(^(?:ONBUILD(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                    r"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n])))?FROM(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                    r#"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))(?:--[\w-]+=(?:"(?:[^"\\\r\n]|"#,
                    r#"\\(?:\r\n|[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|[\s\S]))*'|(?!["'])(?:[^\s\\]|"#,
                    r"\\.)+)(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                    r"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n])))*(?!--)[^ \t\\]+(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|",
                    r"\\[\r\n]))?|\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n])))AS",
                ],
                LB | GR,
            ),
            pf(
                &[
                    r"(?i)(^ONBUILD(?:[ \t]+(?![ \t])(?:\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n]))?|",
                    r"\\[\r\n](?:\s|\\[\r\n]|#.*(?!.))*(?![\s#]|\\[\r\n])))\w+",
                ],
                LB | GR,
            ),
            pf(&[r"^\w+"], GR),
        ],
    ),
    e("comment", &[pf(&[r"(?m)(^[ \t]*)#.*"], LB | GR)]),
    e(
        "string",
        &[pf(
            &[r#""(?:[^"\\\r\n]|\\(?:\r\n|[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|[\s\S]))*'"#],
            GR,
        )],
    ),
    e("variable", &[p(&[r#"\$(?:\w+|\{[^{}"'\\]*\})"#])]),
    e("operator", &[p(&[r"(?m)\\$"])]),
]);

pub(super) static DOCKER_INSTRUCTION_OPTIONS: Grammar = Grammar::new(&[
    e("property", &[pf(&[r"(^|\s)--[\w-]+"], LB)]),
    e(
        "string",
        &[
            pf(
                &[r#""(?:[^"\\\r\n]|\\(?:\r\n|[\s\S]))*"|'(?:[^'\\\r\n]|\\(?:\r\n|[\s\S]))*'"#],
                GR,
            ),
            pf(&[r#"(=)(?!["'])(?:[^\s\\]|\\.)+"#], LB),
        ],
    ),
    e("operator", &[p(&[r"(?m)\\$"])]),
    e("punctuation", &[p(&[r"="])]),
]);
