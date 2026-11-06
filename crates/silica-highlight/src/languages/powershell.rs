//! PowerShell.
//!
//! Ported from Prism 1.29.0's `prism-powershell.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, pi, Grammar, GR, LB};

pub static POWERSHELL: Grammar = Grammar::new(&[
    e(
        "comment",
        &[
            pf(&[r"(^|[^`])<#[\s\S]*?#>"], LB),
            pf(&[r"(^|[^`])#.*"], LB),
        ],
    ),
    e(
        "string",
        &[
            pi(&[r#""(?:`[\s\S]|[^`"])*""#], GR, &POWERSHELL_STRING_1),
            pf(&[r"'(?:[^']|'')*'"], GR),
        ],
    ),
    e(
        "namespace",
        &[p(&[
            r"(?i)\[[a-z](?:\[(?:\[[^\]]*\]|[^\[\]])*\]|[^\[\]])*\]",
        ])],
    ),
    e("boolean", &[p(&[r"(?i)\$(?:false|true)\b"])]),
    e("variable", &[p(&[r"\$\w+\b"])]),
    e(
        "function",
        &[
            p(&[
                r"(?i)\b(?:Add|Approve|Assert|Backup|Block|Checkpoint|Clear|Close|Compare|Complete|Compress|",
                r"Confirm|Connect|Convert|ConvertFrom|ConvertTo|Copy|Debug|Deny|Disable|Disconnect|Dismount|",
                r"Edit|Enable|Enter|Exit|Expand|Export|Find|ForEach|Format|Get|Grant|Group|Hide|Import|",
                r"Initialize|Install|Invoke|Join|Limit|Lock|Measure|Merge|Move|New|Open|Optimize|Out|Ping|",
                r"Pop|Protect|Publish|Push|Read|Receive|Redo|Register|Remove|Rename|Repair|Request|Reset|",
                r"Resize|Resolve|Restart|Restore|Resume|Revoke|Save|Search|Select|Send|Set|Show|Skip|Sort|",
                r"Split|Start|Step|Stop|Submit|Suspend|Switch|Sync|Tee|Test|Trace|Unblock|Undo|Uninstall|",
                r"Unlock|Unprotect|Unpublish|Unregister|Update|Use|Wait|Watch|Where|Write)-[a-z]+\b",
            ]),
            p(&[
                r"(?i)\b(?:ac|cat|chdir|clc|cli|clp|clv|compare|copy|cp|cpi|cpp|cvpa|dbp|del|diff|dir|ebp|",
                r"echo|epal|epcsv|epsn|erase|fc|fl|ft|fw|gal|gbp|gc|gci|gcs|gdr|gi|gl|gm|gp|gps|group|gsv|gu|",
                r"gv|gwmi|iex|ii|ipal|ipcsv|ipsn|irm|iwmi|iwr|kill|lp|ls|measure|mi|mount|move|mp|mv|nal|ndr|",
                r"ni|nv|ogv|popd|ps|pushd|pwd|rbp|rd|rdr|ren|ri|rm|rmdir|rni|rnp|rp|rv|rvpa|rwmi|sal|saps|",
                r"sasv|sbp|sc|select|set|shcm|si|sl|sleep|sls|sort|sp|spps|spsv|start|sv|swmi|tee|trcm|type|",
                r"write)\b",
            ]),
        ],
    ),
    e(
        "keyword",
        &[p(&[
            r"(?i)\b(?:Begin|Break|Catch|Class|Continue|Data|Define|Do|DynamicParam|Else|ElseIf|End|Exit|",
            r"Filter|Finally|For|ForEach|From|Function|If|InlineScript|Parallel|Param|Process|Return|",
            r"Sequence|Switch|Throw|Trap|Try|Until|Using|Var|While|Workflow)\b",
        ])],
    ),
    e(
        "operator",
        &[pf(
            &[
                r"(?i)(^|\W)(?:!|-(?:b?(?:and|x?or)|as|(?:Not)?(?:Contains|In|Like|Match)|eq|ge|gt|",
                r"is(?:Not)?|Join|le|lt|ne|not|Replace|sh[lr])\b|-[-=]?|\+[+=]?|[*/%]=?)",
            ],
            LB,
        )],
    ),
    e("punctuation", &[p(&[r"[|{}\[\];(),.]"])]),
]);

pub(super) static POWERSHELL_STRING_1: Grammar = Grammar::new(&[
    e(
        "function",
        &[pi(
            &[r"(^|[^`])\$\((?:\$\([^\r\n()]*\)|(?!\$\()[^\r\n)])*\)"],
            LB,
            &POWERSHELL,
        )],
    ),
    e("boolean", &[p(&[r"(?i)\$(?:false|true)\b"])]),
    e("variable", &[p(&[r"\$\w+\b"])]),
]);
