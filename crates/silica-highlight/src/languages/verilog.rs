//! Verilog.
//!
//! Ported from Prism 1.29.0's `prism-verilog.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, Grammar, GR};

pub static VERILOG: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r"//.*|/\*[\s\S]*?\*/"], GR)]),
    e(
        "string",
        &[pf(&[r#""(?:\\(?:\r\n|[\s\S])|[^"\\\r\n])*""#], GR)],
    ),
    e("kernel-function", &[pa(&[r"\B\$\w+\b"], 0, &["property"])]),
    e("constant", &[p(&[r"\B`\w+\b"])]),
    e("function", &[p(&[r"\b\w+(?=\()"])]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:alias|and|assert|assign|assume|automatic|before|begin|bind|bins|binsof|bit|break|buf|",
            r"bufif0|bufif1|byte|case|casex|casez|cell|chandle|class|clocking|cmos|config|const|",
            r"constraint|context|continue|cover|covergroup|coverpoint|cross|deassign|default|defparam|",
            r"design|disable|dist|do|edge|else|end|endcase|endclass|endclocking|endconfig|endfunction|",
            r"endgenerate|endgroup|endinterface|endmodule|endpackage|endprimitive|endprogram|endproperty|",
            r"endsequence|endspecify|endtable|endtask|enum|event|expect|export|extends|extern|final|",
            r"first_match|for|force|foreach|forever|fork|forkjoin|function|generate|genvar|highz0|highz1|",
            r"if|iff|ifnone|ignore_bins|illegal_bins|import|incdir|include|initial|inout|input|inside|",
            r"instance|int|integer|interface|intersect|join|join_any|join_none|large|liblist|library|",
            r"local|localparam|logic|longint|macromodule|matches|medium|modport|module|nand|negedge|new|",
            r"nmos|nor|noshowcancelled|not|notif0|notif1|null|or|output|package|packed|parameter|pmos|",
            r"posedge|primitive|priority|program|property|protected|pull0|pull1|pulldown|pullup|",
            r"pulsestyle_ondetect|pulsestyle_onevent|pure|rand|randc|randcase|randsequence|rcmos|real|",
            r"realtime|ref|reg|release|repeat|return|rnmos|rpmos|rtran|rtranif0|rtranif1|scalared|",
            r"sequence|shortint|shortreal|showcancelled|signed|small|solve|specify|specparam|static|",
            r"string|strong0|strong1|struct|super|supply0|supply1|table|tagged|task|this|throughout|time|",
            r"timeprecision|timeunit|tran|tranif0|tranif1|tri|tri0|tri1|triand|trior|trireg|type|typedef|",
            r"union|unique|unsigned|use|uwire|var|vectored|virtual|void|wait|wait_order|wand|weak0|weak1|",
            r"while|wildcard|wire|with|within|wor|xnor|xor)\b",
        ])],
    ),
    e(
        "important",
        &[p(&[
            r"\b(?:always|always_comb|always_ff|always_latch)\b(?: *@)?",
        ])],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\B##?\d+|(?:\b\d+)?'[odbh] ?[\da-fzx_?]+|\b(?:\d*[._])?\d+(?:e[-+]?\d+)?",
        ])],
    ),
    e("operator", &[p(&[r"[-+{}^\~%*/?=!<>\&|]+"])]),
    e("punctuation", &[p(&[r"[\[\];(),.:]"])]),
]);
