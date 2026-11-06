//! Zig.
//!
//! Ported from Prism 1.29.0's `prism-zig.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static ZIG: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pa(&[r"//[/!].*"], 0, &["doc-comment"]), p(&[r"/{2}.*"])],
    ),
    e(
        "string",
        &[
            pf(&[r#"(^|[^\\@])c?"(?:[^"\\\r\n]|\\.)*""#], LB | GR),
            pf(
                &[r"([\r\n])([ \t]+c?\\{2}).*(?:(?:\r\n?|\n)\2.*)*"],
                LB | GR,
            ),
        ],
    ),
    e(
        "char",
        &[pf(
            &[
                r"(^|[^\\])'(?:[^'\\\r\n]|[\x{10000}-\x{10FFFF}]|\\(?:.|x[a-fA-F\d]{2}|",
                r"u\{[a-fA-F\d]{1,6}\}))'",
            ],
            LB | GR,
        )],
    ),
    e("builtin", &[p(&[r"\B@(?!\d)\w+(?=\s*\()"])]),
    e(
        "label",
        &[pf(
            &[r"(\b(?:break|continue)\s*:\s*)\w+\b|\b(?!\d)\w+\b(?=\s*:\s*(?:\{|while\b))"],
            LB,
        )],
    ),
    e(
        "class-name",
        &[
            p(&[r"\b(?!\d)\w+(?=\s*=\s*(?:(?:extern|packed)\s+)?(?:enum|struct|union)\s*[({])"]),
            pi(
                &[
                    r"(:\s*)(?!\s)(?:!?\s*(?:(?:\?|\bpromise->|(?:\[[^\[\]]*\]|\*(?!\*)|",
                    r"\*\*)(?:\s*align\s*\((?:[^()]|\([^()]*\))*\)|\s*const\b|\s*volatile\b|",
                    r"\s*allowzero\b)*)\s*)*(?:\bpromise\b|(?:\berror\.)?\b(?!\b(?:align|allowzero|and|anyframe|",
                    r"anytype|asm|async|await|break|cancel|catch|comptime|const|continue|defer|else|enum|",
                    r"errdefer|error|export|extern|fn|for|if|inline|linksection|nakedcc|noalias|nosuspend|null|",
                    r"or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|suspend|switch|test|",
                    r"threadlocal|try|undefined|union|unreachable|usingnamespace|var|volatile|",
                    r"while)\b)(?!\d)\w+\b(?:\.\b(?!\b(?:align|allowzero|and|anyframe|anytype|asm|async|await|",
                    r"break|cancel|catch|comptime|const|continue|defer|else|enum|errdefer|error|export|extern|fn|",
                    r"for|if|inline|linksection|nakedcc|noalias|nosuspend|null|or|orelse|packed|promise|pub|",
                    r"resume|return|stdcallcc|struct|suspend|switch|test|threadlocal|try|undefined|union|",
                    r"unreachable|usingnamespace|var|volatile|while)\b)(?!\d)\w+\b)*(?!\s+\b(?!\b(?:align|",
                    r"allowzero|and|anyframe|anytype|asm|async|await|break|cancel|catch|comptime|const|continue|",
                    r"defer|else|enum|errdefer|error|export|extern|fn|for|if|inline|linksection|nakedcc|noalias|",
                    r"nosuspend|null|or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|suspend|switch|",
                    r"test|threadlocal|try|undefined|union|unreachable|usingnamespace|var|volatile|",
                    r"while)\b)(?!\d)\w+\b)))+(?=\s*(?:align\s*\((?:[^()]|\([^()]*\))*\)\s*)?[=;,)])|",
                    r"(?!\s)(?:!?\s*(?:(?:\?|\bpromise->|(?:\[[^\[\]]*\]|\*(?!\*)|\*\*)(?:\s*align\s*\((?:[^()]|",
                    r"\([^()]*\))*\)|\s*const\b|\s*volatile\b|\s*allowzero\b)*)\s*)*(?:\bpromise\b|",
                    r"(?:\berror\.)?\b(?!\b(?:align|allowzero|and|anyframe|anytype|asm|async|await|break|cancel|",
                    r"catch|comptime|const|continue|defer|else|enum|errdefer|error|export|extern|fn|for|if|",
                    r"inline|linksection|nakedcc|noalias|nosuspend|null|or|orelse|packed|promise|pub|resume|",
                    r"return|stdcallcc|struct|suspend|switch|test|threadlocal|try|undefined|union|unreachable|",
                    r"usingnamespace|var|volatile|while)\b)(?!\d)\w+\b(?:\.\b(?!\b(?:align|allowzero|and|",
                    r"anyframe|anytype|asm|async|await|break|cancel|catch|comptime|const|continue|defer|else|",
                    r"enum|errdefer|error|export|extern|fn|for|if|inline|linksection|nakedcc|noalias|nosuspend|",
                    r"null|or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|suspend|switch|test|",
                    r"threadlocal|try|undefined|union|unreachable|usingnamespace|var|volatile|",
                    r"while)\b)(?!\d)\w+\b)*(?!\s+\b(?!\b(?:align|allowzero|and|anyframe|anytype|asm|async|await|",
                    r"break|cancel|catch|comptime|const|continue|defer|else|enum|errdefer|error|export|extern|fn|",
                    r"for|if|inline|linksection|nakedcc|noalias|nosuspend|null|or|orelse|packed|promise|pub|",
                    r"resume|return|stdcallcc|struct|suspend|switch|test|threadlocal|try|undefined|union|",
                    r"unreachable|usingnamespace|var|volatile|while)\b)(?!\d)\w+\b)))+(?=\s*(?:align\s*\((?:[^()",
                    r"]|\([^()]*\))*\)\s*)?\{)",
                ],
                LB,
                &ZIG,
            ),
            pi(
                &[
                    r"(\)\s*)(?!\s)(?:!?\s*(?:(?:\?|\bpromise->|(?:\[[^\[\]]*\]|\*(?!\*)|",
                    r"\*\*)(?:\s*align\s*\((?:[^()]|\([^()]*\))*\)|\s*const\b|\s*volatile\b|",
                    r"\s*allowzero\b)*)\s*)*(?:\bpromise\b|(?:\berror\.)?\b(?!\b(?:align|allowzero|and|anyframe|",
                    r"anytype|asm|async|await|break|cancel|catch|comptime|const|continue|defer|else|enum|",
                    r"errdefer|error|export|extern|fn|for|if|inline|linksection|nakedcc|noalias|nosuspend|null|",
                    r"or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|suspend|switch|test|",
                    r"threadlocal|try|undefined|union|unreachable|usingnamespace|var|volatile|",
                    r"while)\b)(?!\d)\w+\b(?:\.\b(?!\b(?:align|allowzero|and|anyframe|anytype|asm|async|await|",
                    r"break|cancel|catch|comptime|const|continue|defer|else|enum|errdefer|error|export|extern|fn|",
                    r"for|if|inline|linksection|nakedcc|noalias|nosuspend|null|or|orelse|packed|promise|pub|",
                    r"resume|return|stdcallcc|struct|suspend|switch|test|threadlocal|try|undefined|union|",
                    r"unreachable|usingnamespace|var|volatile|while)\b)(?!\d)\w+\b)*(?!\s+\b(?!\b(?:align|",
                    r"allowzero|and|anyframe|anytype|asm|async|await|break|cancel|catch|comptime|const|continue|",
                    r"defer|else|enum|errdefer|error|export|extern|fn|for|if|inline|linksection|nakedcc|noalias|",
                    r"nosuspend|null|or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|suspend|switch|",
                    r"test|threadlocal|try|undefined|union|unreachable|usingnamespace|var|volatile|",
                    r"while)\b)(?!\d)\w+\b)))+(?=\s*(?:align\s*\((?:[^()]|\([^()]*\))*\)\s*)?;)",
                ],
                LB,
                &ZIG,
            ),
        ],
    ),
    e(
        "builtin-type",
        &[pa(
            &[
                r"\b(?:anyerror|bool|c_u?(?:int|long|longlong|short)|c_longdouble|c_void|comptime_(?:float|",
                r"int)|f(?:16|32|64|128)|[iu](?:8|16|32|64|128|size)|noreturn|type|void)\b",
            ],
            0,
            &["keyword"],
        )],
    ),
    e(
        "keyword",
        &[p(&[
            r"\b(?:align|allowzero|and|anyframe|anytype|asm|async|await|break|cancel|catch|comptime|",
            r"const|continue|defer|else|enum|errdefer|error|export|extern|fn|for|if|inline|linksection|",
            r"nakedcc|noalias|nosuspend|null|or|orelse|packed|promise|pub|resume|return|stdcallcc|struct|",
            r"suspend|switch|test|threadlocal|try|undefined|union|unreachable|usingnamespace|var|",
            r"volatile|while)\b",
        ])],
    ),
    e("function", &[p(&[r"\b(?!\d)\w+(?=\s*\()"])]),
    e(
        "number",
        &[p(&[
            r"\b(?:0b[01]+|0o[0-7]+|0x[a-fA-F\d]+(?:\.[a-fA-F\d]*)?(?:[pP][+-]?[a-fA-F\d]+)?|",
            r"\d+(?:\.\d*)?(?:[eE][+-]?\d+)?)\b",
        ])],
    ),
    e("boolean", &[p(&[r"\b(?:false|true)\b"])]),
    e(
        "operator",
        &[p(&[
            r"\.[*?]|\.{2,3}|[-=]>|\*\*|\+\+|\|\||(?:<<|>>|[-+*]%|[-+*/%^\&|<>!=])=?|[?\~]",
        ])],
    ),
    e("punctuation", &[p(&[r"[.:,;(){}\[\]]"])]),
]);
