//! Clojure.
//!
//! Ported from Prism 1.29.0's `prism-clojure.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pf, Grammar, GR, LB};

pub static CLOJURE: Grammar = Grammar::new(&[
    e("comment", &[pf(&[r";.*"], GR)]),
    e("string", &[pf(&[r#""(?:[^"\\]|\\.)*""#], GR)]),
    e("char", &[p(&[r"\\\w+"])]),
    e(
        "symbol",
        &[pf(&[r"(^|[\s()\[\]{},])::?[\w*+!?'<>=/.-]+"], LB)],
    ),
    e(
        "keyword",
        &[pf(
            &[
                r"(\()(?:-|->|->>|\.|\.\.|\*|/|\+|<|<=|=|==|>|>=|accessor|agent|agent-errors|aget|alength|",
                r"all-ns|alter|and|append-child|apply|array-map|aset|aset-boolean|aset-byte|aset-char|",
                r"aset-double|aset-float|aset-int|aset-long|aset-short|assert|assoc|await|await-for|bean|",
                r"binding|bit-and|bit-not|bit-or|bit-shift-left|bit-shift-right|bit-xor|boolean|branch\?|",
                r"butlast|byte|cast|char|children|class|clear-agent-errors|comment|commute|comp|comparator|",
                r"complement|concat|cond|conj|cons|constantly|construct-proxy|contains\?|count|create-ns|",
                r"create-struct|cycle|dec|declare|def|def-|definline|definterface|defmacro|defmethod|",
                r"defmulti|defn|defn-|defonce|defproject|defprotocol|defrecord|defstruct|deftype|deref|",
                r"difference|disj|dissoc|distinct|do|doall|doc|dorun|doseq|dosync|dotimes|doto|double|down|",
                r"drop|drop-while|edit|end\?|ensure|eval|every\?|false\?|ffirst|file-seq|filter|find|",
                r"find-doc|find-ns|find-var|first|float|flush|fn|fnseq|for|frest|gensym|get|get-proxy-class|",
                r"hash-map|hash-set|identical\?|identity|if|if-let|if-not|import|in-ns|inc|index|",
                r"insert-child|insert-left|insert-right|inspect-table|inspect-tree|instance\?|int|interleave|",
                r"intersection|into|into-array|iterate|join|key|keys|keyword|keyword\?|last|lazy-cat|",
                r"lazy-cons|left|lefts|let|line-seq|list|list\*|load|load-file|locking|long|loop|macroexpand|",
                r"macroexpand-1|make-array|make-node|map|map-invert|map\?|mapcat|max|max-key|memfn|merge|",
                r"merge-with|meta|min|min-key|monitor-enter|name|namespace|neg\?|new|newline|next|nil\?|node|",
                r"not|not-any\?|not-every\?|not=|ns|ns-imports|ns-interns|ns-map|ns-name|ns-publics|",
                r"ns-refers|ns-resolve|ns-unmap|nth|nthrest|or|parse|partial|path|peek|pop|pos\?|pr|pr-str|",
                r"print|print-str|println|println-str|prn|prn-str|project|proxy|proxy-mappings|quot|quote|",
                r"rand|rand-int|range|re-find|re-groups|re-matcher|re-matches|re-pattern|re-seq|read|",
                r"read-line|recur|reduce|ref|ref-set|refer|rem|remove|remove-method|remove-ns|rename|",
                r"rename-keys|repeat|replace|replicate|resolve|rest|resultset-seq|reverse|rfirst|right|",
                r"rights|root|rrest|rseq|second|select|select-keys|send|send-off|seq|seq-zip|seq\?|set|set!|",
                r"short|slurp|some|sort|sort-by|sorted-map|sorted-map-by|sorted-set|special-symbol\?|",
                r"split-at|split-with|str|string\?|struct|struct-map|subs|subvec|symbol|symbol\?|sync|take|",
                r"take-nth|take-while|test|throw|time|to-array|to-array-2d|tree-seq|true\?|try|union|up|",
                r"update-proxy|val|vals|var|var-get|var-set|var\?|vector|vector-zip|vector\?|when|when-first|",
                r"when-let|when-not|with-local-vars|with-meta|with-open|with-out-str|xml-seq|xml-zip|zero\?|",
                r"zipmap|zipper)(?=[\s)]|$)",
            ],
            LB,
        )],
    ),
    e("boolean", &[p(&[r"\b(?:false|nil|true)\b"])]),
    e(
        "number",
        &[pf(
            &[
                r"(?i)(^|[^\w$@])(?:\d+(?:[/.]\d+)?(?:e[+-]?\d+)?|0x[a-f0-9]+|",
                r"[1-9]\d?r[a-z0-9]+)[lmn]?(?![\w$@])",
            ],
            LB,
        )],
    ),
    e(
        "function",
        &[pf(&[r"((?:^|[^'])\()[\w*+!?'<>=/.-]+(?=[\s)]|$)"], LB)],
    ),
    e("operator", &[p(&[r"[#@^`\~]"])]),
    e("punctuation", &[p(&[r"[{}\[\](),]"])]),
]);
