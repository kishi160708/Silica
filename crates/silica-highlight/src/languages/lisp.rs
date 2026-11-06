//! Lisp (`elisp`, `emacs`, `emacs-lisp`).
//!
//! Ported from Prism 1.29.0's `prism-lisp.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static LISP: Grammar = Grammar::new(&[
    e("heading", &[pa(&[r";;;.*"], 0, &["comment", "title"])]),
    e("comment", &[p(&[r";.*"])]),
    e("string", &[pi(&[r#""(?:[^"\\]|\\.)*""#], GR, &LISP_STRING)]),
    e(
        "quoted-symbol",
        &[pa(
            &[r"#?'(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            0,
            &["variable", "symbol"],
        )],
    ),
    e(
        "lisp-property",
        &[pa(&[r":(?!\d)[-+*/\~!@$%^=<>{}\w]+"], 0, &["property"])],
    ),
    e(
        "splice",
        &[pa(
            &[r",@?(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            0,
            &["symbol", "variable"],
        )],
    ),
    e(
        "keyword",
        &[
            pf(
                &[
                    r"(\()(?:and|(?:cl-)?letf|cl-loop|cond|cons|error|if|(?:lexical-)?let\*?|message|not|null|or|",
                    r"provide|require|setq|unless|use-package|when|while)(?=\s)",
                ],
                LB,
            ),
            pf(
                &[r"(\()(?:append|by|collect|concat|do|finally|for|in|return)(?=\s)"],
                LB,
            ),
        ],
    ),
    e(
        "declare",
        &[pa(&[r"(\()(?:declare)(?=[\s\)])"], LB, &["keyword"])],
    ),
    e(
        "interactive",
        &[pa(&[r"(\()(?:interactive)(?=[\s\)])"], LB, &["keyword"])],
    ),
    e("boolean", &[pf(&[r"([\s(\[])(?:nil|t)(?=[\s)])"], LB)]),
    e(
        "number",
        &[pf(&[r"([\s(\[])(?:[-+]?\d+(?:\.\d*)?)(?=[\s)])"], LB)],
    ),
    e(
        "defvar",
        &[pi(
            &[r"(\()def(?:const|custom|group|var)\s+(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            LB,
            &LISP_DEFVAR,
        )],
    ),
    e(
        "defun",
        &[pi(
            &[
                r"(\()(?:cl-)?(?:defmacro|defun\*?)\s+(?!\d)[-+*/\~!@$%^=<>{}\w]+\s+\((?:[^()]|\((?:[^()]|",
                r"\((?:[^()]|\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\))*\))*\))*\)",
            ],
            LB | GR,
            &LISP_DEFUN,
        )],
    ),
    e(
        "lambda",
        &[pi(
            &[
                r"(\()lambda\s+\(\s*(?:&?(?!\d)[-+*/\~!@$%^=<>{}\w]+(?:\s+&?(?!\d)[-+*/\~!@$%^=<>{}\w]+)*\s*",
                r")?\)",
            ],
            LB | GR,
            &LISP_LAMBDA,
        )],
    ),
    e("car", &[pf(&[r"(\()(?!\d)[-+*/\~!@$%^=<>{}\w]+"], LB)]),
    e(
        "punctuation",
        &[p(&[r"(?:['`,]?\(|[)\[\]])"]), pf(&[r"(\s)\.(?=\s)"], LB)],
    ),
]);

pub(super) static LISP_STRING: Grammar = Grammar::new(&[
    e("argument", &[p(&[r"[-A-Z]+(?=[.,\s])"])]),
    e("symbol", &[p(&[r"`(?!\d)[-+*/\~!@$%^=<>{}\w]+'"])]),
]);

pub(super) static LISP_DEFVAR: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^def[a-z]+"])]),
    e("variable", &[p(&[r"(?!\d)[-+*/\~!@$%^=<>{}\w]+"])]),
]);

pub(super) static LISP_DEFUN: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^(?:cl-)?def\S+"])]),
    e(
        "arguments",
        &[pi(
            &[
                r"(\()(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                r"\([^()]*\))*\))*\))*\))*\))*(?=\))",
            ],
            LB,
            &LISP_DEFUN_ARGUMENTS,
        )],
    ),
    e(
        "function",
        &[pf(&[r"(^\s)(?!\d)[-+*/\~!@$%^=<>{}\w]+"], LB)],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS: Grammar = Grammar::new(&[
    e(
        "rest-vars",
        &[pi(
            &[r"&(?:body|rest)\s+\S+(?:\s+\S+)*"],
            0,
            &LISP_DEFUN_ARGUMENTS_REST_VARS,
        )],
    ),
    e(
        "other-marker-vars",
        &[pi(
            &[r"&(?:aux|optional)\s+\S+(?:\s+\S+)*"],
            0,
            &LISP_DEFUN_ARGUMENTS_REST_VARS,
        )],
    ),
    e(
        "keys",
        &[pi(
            &[r"&key\s+\S+(?:\s+\S+)*(?:\s+&allow-other-keys)?"],
            0,
            &LISP_DEFUN_ARGUMENTS_REST_VARS,
        )],
    ),
    e(
        "argument",
        &[pa(&[r"(?!\d)[-+*/\~!@$%^=<>{}\w]+"], 0, &["variable"])],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
    e(
        "sublist",
        &[pi(
            &[
                r"(\()(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                r"\([^()]*\))*\))*\))*\))*\))*(?=\))",
            ],
            LB,
            &LISP_DEFUN_ARGUMENTS_SUBLIST,
        )],
    ),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS: Grammar = Grammar::with_rest(
    &[
        e("lisp-marker", &[p(&[r"&(?!\d)[-+*/\~!@$%^=<>{}\w]+"])]),
        e(
            "varform",
            &[pi(
                &[
                    r"\((?!\d)[-+*/\~!@$%^=<>{}\w]+\s+(?=\S)(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                    r"\([^()]*\))*\))*\))*\))*\))*\)",
                ],
                0,
                &LISP_DEFUN_ARGUMENTS_REST_VARS_REST,
            )],
        ),
        e(
            "argument",
            &[pa(
                &[r"(^|[\s(])(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
                LB,
                &["variable"],
            )],
        ),
    ],
    &LISP_DEFUN_ARGUMENTS_REST_VARS_REST,
);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS_REST: Grammar = Grammar::new(&[
    e("heading", &[pa(&[r";;;.*"], 0, &["comment", "title"])]),
    e("comment", &[p(&[r";.*"])]),
    e(
        "string",
        &[pi(
            &[r#""(?:[^"\\]|\\.)*""#],
            GR,
            &LISP_DEFUN_ARGUMENTS_REST_VARS_REST_STRING,
        )],
    ),
    e(
        "quoted-symbol",
        &[pa(
            &[r"#?'(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            0,
            &["variable", "symbol"],
        )],
    ),
    e(
        "lisp-property",
        &[pa(&[r":(?!\d)[-+*/\~!@$%^=<>{}\w]+"], 0, &["property"])],
    ),
    e(
        "splice",
        &[pa(
            &[r",@?(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            0,
            &["symbol", "variable"],
        )],
    ),
    e(
        "keyword",
        &[
            pf(
                &[
                    r"(\()(?:and|(?:cl-)?letf|cl-loop|cond|cons|error|if|(?:lexical-)?let\*?|message|not|null|or|",
                    r"provide|require|setq|unless|use-package|when|while)(?=\s)",
                ],
                LB,
            ),
            pf(
                &[r"(\()(?:append|by|collect|concat|do|finally|for|in|return)(?=\s)"],
                LB,
            ),
        ],
    ),
    e(
        "declare",
        &[pa(&[r"(\()(?:declare)(?=[\s\)])"], LB, &["keyword"])],
    ),
    e(
        "interactive",
        &[pa(&[r"(\()(?:interactive)(?=[\s\)])"], LB, &["keyword"])],
    ),
    e("boolean", &[pf(&[r"([\s(\[])(?:nil|t)(?=[\s)])"], LB)]),
    e(
        "number",
        &[pf(&[r"([\s(\[])(?:[-+]?\d+(?:\.\d*)?)(?=[\s)])"], LB)],
    ),
    e(
        "defvar",
        &[pi(
            &[r"(\()def(?:const|custom|group|var)\s+(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
            LB,
            &LISP_DEFUN_ARGUMENTS_REST_VARS_REST_DEFVAR,
        )],
    ),
    e(
        "defun",
        &[pi(
            &[
                r"(\()(?:cl-)?(?:defmacro|defun\*?)\s+(?!\d)[-+*/\~!@$%^=<>{}\w]+\s+\((?:[^()]|\((?:[^()]|",
                r"\((?:[^()]|\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\))*\))*\))*\)",
            ],
            LB | GR,
            &LISP_DEFUN_ARGUMENTS_REST_VARS_REST_DEFUN,
        )],
    ),
    e(
        "lambda",
        &[pi(
            &[
                r"(\()lambda\s+\(\s*(?:&?(?!\d)[-+*/\~!@$%^=<>{}\w]+(?:\s+&?(?!\d)[-+*/\~!@$%^=<>{}\w]+)*\s*",
                r")?\)",
            ],
            LB | GR,
            &LISP_DEFUN_ARGUMENTS_REST_VARS_REST_LAMBDA,
        )],
    ),
    e("car", &[pf(&[r"(\()(?!\d)[-+*/\~!@$%^=<>{}\w]+"], LB)]),
    e(
        "punctuation",
        &[p(&[r"(?:['`,]?\(|[)\[\]])"]), pf(&[r"(\s)\.(?=\s)"], LB)],
    ),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS_REST_STRING: Grammar = Grammar::new(&[
    e("argument", &[p(&[r"[-A-Z]+(?=[.,\s])"])]),
    e("symbol", &[p(&[r"`(?!\d)[-+*/\~!@$%^=<>{}\w]+'"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS_REST_DEFVAR: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^def[a-z]+"])]),
    e("variable", &[p(&[r"(?!\d)[-+*/\~!@$%^=<>{}\w]+"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS_REST_DEFUN: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^(?:cl-)?def\S+"])]),
    e(
        "function",
        &[pf(&[r"(^\s)(?!\d)[-+*/\~!@$%^=<>{}\w]+"], LB)],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_REST_VARS_REST_LAMBDA: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^lambda"])]),
    e(
        "arguments",
        &[pi(
            &[
                r"(\()(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                r"\([^()]*\))*\))*\))*\))*\))*(?=\))",
            ],
            LB,
            &LISP_DEFUN_ARGUMENTS,
        )],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_SUBLIST: Grammar = Grammar::new(&[
    e(
        "rest-vars",
        &[pi(
            &[r"&(?:body|rest)\s+\S+(?:\s+\S+)*"],
            0,
            &LISP_DEFUN_ARGUMENTS_SUBLIST_REST_VARS,
        )],
    ),
    e(
        "other-marker-vars",
        &[pi(
            &[r"&(?:aux|optional)\s+\S+(?:\s+\S+)*"],
            0,
            &LISP_DEFUN_ARGUMENTS_SUBLIST_REST_VARS,
        )],
    ),
    e(
        "keys",
        &[pi(
            &[r"&key\s+\S+(?:\s+\S+)*(?:\s+&allow-other-keys)?"],
            0,
            &LISP_DEFUN_ARGUMENTS_SUBLIST_REST_VARS,
        )],
    ),
    e(
        "argument",
        &[pa(&[r"(?!\d)[-+*/\~!@$%^=<>{}\w]+"], 0, &["variable"])],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);

pub(super) static LISP_DEFUN_ARGUMENTS_SUBLIST_REST_VARS: Grammar = Grammar::with_rest(
    &[
        e("lisp-marker", &[p(&[r"&(?!\d)[-+*/\~!@$%^=<>{}\w]+"])]),
        e(
            "varform",
            &[pi(
                &[
                    r"\((?!\d)[-+*/\~!@$%^=<>{}\w]+\s+(?=\S)(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                    r"\([^()]*\))*\))*\))*\))*\))*\)",
                ],
                0,
                &LISP,
            )],
        ),
        e(
            "argument",
            &[pa(
                &[r"(^|[\s(])(?!\d)[-+*/\~!@$%^=<>{}\w]+"],
                LB,
                &["variable"],
            )],
        ),
    ],
    &LISP,
);

pub(super) static LISP_LAMBDA: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"^lambda"])]),
    e(
        "arguments",
        &[pi(
            &[
                r"(\()(?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|\((?:[^()]|",
                r"\([^()]*\))*\))*\))*\))*\))*(?=\))",
            ],
            LB,
            &LISP_DEFUN_ARGUMENTS_SUBLIST,
        )],
    ),
    e("punctuation", &[p(&[r"[()]"])]),
]);
