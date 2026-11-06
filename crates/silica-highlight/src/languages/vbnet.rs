//! VB.Net.
//!
//! Ported from Prism 1.29.0's `prism-basic.js`, `prism-vbnet.js` and `prism-xml-doc.js`,
//! as the grammar stands once all of them have loaded. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pai, pf, pi, Grammar, GR, LB};

pub static VBNET: Grammar = Grammar::new(&[
    e(
        "doc-comment",
        &[pai(&[r"'''.*"], GR, &["comment"], &VBNET_DOC_COMMENT)],
    ),
    e(
        "comment",
        &[
            pi(&[r"(?i)(?:!|REM\b).+"], 0, &VBNET_COMMENT_1),
            pf(&[r"(^|[^\\:])'.*"], LB | GR),
        ],
    ),
    e(
        "string",
        &[pf(&[r#"(^|[^"])"(?:""|[^"])*"(?!")"#], LB | GR)],
    ),
    e(
        "number",
        &[p(&[r"(?i)(?:\b\d+(?:\.\d*)?|\B\.\d+)(?:E[+-]?\d+)?"])],
    ),
    e(
        "keyword",
        &[p(&[
            r"(?i)(?:\b(?:ADDHANDLER|ADDRESSOF|ALIAS|AND|ANDALSO|AS|BEEP|BLOAD|BOOLEAN|BSAVE|BYREF|BYTE|",
            r"BYVAL|CALL(?: ABSOLUTE)?|CASE|CATCH|CBOOL|CBYTE|CCHAR|CDATE|CDBL|CDEC|CHAIN|CHAR|CHDIR|",
            r"CINT|CLASS|CLEAR|CLNG|CLOSE|CLS|COBJ|COM|COMMON|CONST|CONTINUE|CSBYTE|CSHORT|CSNG|CSTR|",
            r"CTYPE|CUINT|CULNG|CUSHORT|DATA|DATE|DECIMAL|DECLARE|DEF(?: FN| SEG|DBL|INT|LNG|SNG|STR)|",
            r"DEFAULT|DELEGATE|DIM|DIRECTCAST|DO|DOUBLE|ELSE|ELSEIF|END|ENUM|ENVIRON|ERASE|ERROR|EVENT|",
            r"EXIT|FALSE|FIELD|FILES|FINALLY|FOR(?: EACH)?|FRIEND|FUNCTION|GET|GETTYPE|GETXMLNAMESPACE|",
            r"GLOBAL|GOSUB|GOTO|HANDLES|IF|IMPLEMENTS|IMPORTS|IN|INHERITS|INPUT|INTEGER|INTERFACE|IOCTL|",
            r"IS|ISNOT|KEY|KILL|LET|LIB|LIKE|LINE INPUT|LOCATE|LOCK|LONG|LOOP|LSET|ME|MKDIR|MOD|MODULE|",
            r"MUSTINHERIT|MUSTOVERRIDE|MYBASE|MYCLASS|NAME|NAMESPACE|NARROWING|NEW|NEXT|NOT|NOTHING|",
            r"NOTINHERITABLE|NOTOVERRIDABLE|OBJECT|OF|OFF|ON(?: COM| ERROR| KEY| TIMER)?|OPEN|OPERATOR|",
            r"OPTION(?: BASE)?|OPTIONAL|OR|ORELSE|OUT|OVERLOADS|OVERRIDABLE|OVERRIDES|PARAMARRAY|PARTIAL|",
            r"POKE|PRIVATE|PROPERTY|PROTECTED|PUBLIC|PUT|RAISEEVENT|READ|READONLY|REDIM|REM|",
            r"REMOVEHANDLER|RESTORE|RESUME|RETURN|RMDIR|RSET|RUN|SBYTE|SELECT(?: CASE)?|SET|SHADOWS|",
            r"SHARED|SHELL|SHORT|SINGLE|SLEEP|STATIC|STEP|STOP|STRING|STRUCTURE|SUB|SWAP|SYNCLOCK|SYSTEM|",
            r"THEN|THROW|TIMER|TO|TROFF|TRON|TRUE|TRY|TRYCAST|TYPE|TYPEOF|UINTEGER|ULONG|UNLOCK|UNTIL|",
            r"USHORT|USING|VIEW PRINT|WAIT|WEND|WHEN|WHILE|WIDENING|WITH|WITHEVENTS|WRITE|WRITEONLY|XOR)|",
            r"\B(?:#CONST|#ELSE|#ELSEIF|#END|#IF))(?:\$|\b)",
        ])],
    ),
    e(
        "function",
        &[p(&[
            r"(?i)\b(?:ABS|ACCESS|ACOS|ANGLE|AREA|ARITHMETIC|ARRAY|ASIN|ASK|AT|ATN|BASE|BEGIN|BREAK|",
            r"CAUSE|CEIL|CHR|CLIP|COLLATE|COLOR|CON|COS|COSH|COT|CSC|DATE|DATUM|DEBUG|DECIMAL|DEF|DEG|",
            r"DEGREES|DELETE|DET|DEVICE|DISPLAY|DOT|ELAPSED|EPS|ERASABLE|EXLINE|EXP|EXTERNAL|EXTYPE|",
            r"FILETYPE|FIXED|FP|GO|GRAPH|HANDLER|IDN|IMAGE|IN|INT|INTERNAL|IP|IS|KEYED|LBOUND|LCASE|LEFT|",
            r"LEN|LENGTH|LET|LINE|LINES|LOG|LOG10|LOG2|LTRIM|MARGIN|MAT|MAX|MAXNUM|MID|MIN|MISSING|MOD|",
            r"NATIVE|NUL|NUMERIC|OF|OPTION|ORD|ORGANIZATION|OUTIN|OUTPUT|PI|POINT|POINTER|POINTS|POS|",
            r"PRINT|PROGRAM|PROMPT|RAD|RADIANS|RANDOMIZE|RECORD|RECSIZE|RECTYPE|RELATIVE|REMAINDER|",
            r"REPEAT|REST|RETRY|REWRITE|RIGHT|RND|ROUND|RTRIM|SAME|SEC|SELECT|SEQUENTIAL|SET|SETTER|SGN|",
            r"SIN|SINH|SIZE|SKIP|SQR|STANDARD|STATUS|STR|STREAM|STYLE|TAB|TAN|TANH|TEMPLATE|TEXT|THERE|",
            r"TIME|TIMEOUT|TRACE|TRANSFORM|TRUNCATE|UBOUND|UCASE|USE|VAL|VARIABLE|VIEWPORT|WHEN|WINDOW|",
            r"WITH|ZER|ZONEWIDTH)(?:\$|\b)",
        ])],
    ),
    e(
        "operator",
        &[p(&[
            r"(?i)<[=>]?|>=?|[+\-*/^=\&]|\b(?:AND|EQV|IMP|NOT|OR|XOR)\b",
        ])],
    ),
    e("punctuation", &[p(&[r"[,;:(){}]"])]),
]);

pub(super) static VBNET_DOC_COMMENT: Grammar = Grammar::new(&[e(
    "tag",
    &[pi(
        &[
            r#"</?(?!\d)[^\s>/=$<%]+(?:\s(?:\s*[^\s>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))|"#,
            r"(?=[\s/>])))+)?\s*/?>",
        ],
        GR,
        &super::markup::MARKUP_TAG,
    )],
)]);

pub(super) static VBNET_COMMENT_1: Grammar = Grammar::new(&[e("keyword", &[p(&[r"(?i)^REM"])])]);
