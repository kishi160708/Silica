//! Fortran.
//!
//! Ported from Prism 1.29.0's `prism-fortran.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static FORTRAN: Grammar = Grammar::new(&[
    e(
        "quoted-number",
        &[pa(&[r#"(?i)[BOZ](['"])[A-F0-9]+\1"#], 0, &["number"])],
    ),
    e(
        "string",
        &[pi(
            &[
                r#"(?:\b\w+_)?(['"])(?:\1\1|&(?:\r\n?|\n)(?:[ \t]*!.*(?:\r\n?|\n)|(?![ \t]*!))|(?!\1).)*(?:\1|"#,
                r"&)",
            ],
            0,
            &FORTRAN_STRING,
        )],
    ),
    e("comment", &[pf(&[r"!.*"], GR)]),
    e("boolean", &[p(&[r"(?i)\.(?:FALSE|TRUE)\.(?:_\w+)?"])]),
    e(
        "number",
        &[p(&[
            r"(?i)(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:[ED][+-]?\d+)?(?:_\w+)?",
        ])],
    ),
    e(
        "keyword",
        &[
            p(&[r"(?i)\b(?:CHARACTER|COMPLEX|DOUBLE ?PRECISION|INTEGER|LOGICAL|REAL)\b"]),
            p(&[
                r"(?i)\b(?:END ?)?(?:BLOCK ?DATA|DO|FILE|FORALL|FUNCTION|IF|INTERFACE|MODULE(?! PROCEDURE)|",
                r"PROGRAM|SELECT|SUBROUTINE|TYPE|WHERE)\b",
            ]),
            p(&[
                r"(?i)\b(?:ALLOCATABLE|ALLOCATE|BACKSPACE|CALL|CASE|CLOSE|COMMON|CONTAINS|CONTINUE|CYCLE|",
                r"DATA|DEALLOCATE|DIMENSION|DO|END|EQUIVALENCE|EXIT|EXTERNAL|FORMAT|GO ?TO|",
                r"IMPLICIT(?: NONE)?|INQUIRE|INTENT|INTRINSIC|MODULE PROCEDURE|NAMELIST|NULLIFY|OPEN|",
                r"OPTIONAL|PARAMETER|POINTER|PRINT|PRIVATE|PUBLIC|READ|RETURN|REWIND|SAVE|SELECT|STOP|TARGET|",
                r"WHILE|WRITE)\b",
            ]),
            p(&[
                r"(?i)\b(?:ASSIGNMENT|DEFAULT|ELEMENTAL|ELSE|ELSEIF|ELSEWHERE|ENTRY|IN|INCLUDE|INOUT|KIND|",
                r"NULL|ONLY|OPERATOR|OUT|PURE|RECURSIVE|RESULT|SEQUENCE|STAT|THEN|USE)\b",
            ]),
        ],
    ),
    e(
        "operator",
        &[
            p(&[r"(?i)\*\*|//|=>|[=/]=|[<>]=?|::|[+\-*=%]|\.[A-Z]+\."]),
            pf(&[r"(^|(?!\().)/(?!\))"], LB),
        ],
    ),
    e("punctuation", &[p(&[r"\(/|/\)|[(),;:\&]"])]),
]);

pub(super) static FORTRAN_STRING: Grammar =
    Grammar::new(&[e("comment", &[pf(&[r"(&(?:\r\n?|\n)\s*)!.*"], LB)])]);
