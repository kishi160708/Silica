//! Haskell (`hs`).
//!
//! Ported from Prism 1.29.0's `prism-haskell.js`. Written by
//! `tools/prism-port/port.js`, which walks the finished grammar objects;
//! change that script and re-run it rather than editing this file.

use crate::grammar::{e, p, pa, pf, pi, Grammar, GR, LB};

pub static HASKELL: Grammar = Grammar::new(&[
    e(
        "comment",
        &[pf(
            &[
                r"(?m)(^|[^-!#$%*+=?\&@|\~.:<>^\\/])(?:--(?:(?=.)[^-!#$%*+=?\&@|\~.:<>^\\/].*|$)|",
                r"\{-[\s\S]*?-\})",
            ],
            LB,
        )],
    ),
    e(
        "char",
        &[pa(
            &[
                r#"'(?:[^\\']|\\(?:[abfnrtv\\"'\&]|\^[A-Z@\[\]^_]|ACK|BEL|BS|CAN|CR|DC1|DC2|DC3|DC4|DEL|DLE|"#,
                r"EM|ENQ|EOT|ESC|ETB|ETX|FF|FS|GS|HT|LF|NAK|NUL|RS|SI|SO|SOH|SP|STX|SUB|SYN|US|VT|\d+|",
                r"o[0-7]+|x[0-9a-fA-F]+))'",
            ],
            0,
            &["string"],
        )],
    ),
    e("string", &[pf(&[r#""(?:[^\\"]|\\(?:\S|\s+\\))*""#], GR)]),
    e(
        "keyword",
        &[p(&[
            r"\b(?:case|class|data|deriving|do|else|if|in|infixl|infixr|instance|let|module|newtype|of|",
            r"primitive|then|type|where)\b",
        ])],
    ),
    e(
        "import-statement",
        &[pi(
            &[
                r"(?m)(^[\t ]*)import\s+(?:qualified\s+)?(?:[A-Z][\w']*)(?:\.[A-Z][\w']*)*(?:\s+as\s+(?:[A-Z",
                r"][\w']*)(?:\.[A-Z][\w']*)*)?(?:\s+hiding\b)?",
            ],
            LB,
            &HASKELL_IMPORT_STATEMENT,
        )],
    ),
    e(
        "builtin",
        &[p(&[
            r"\b(?:abs|acos|acosh|all|and|any|appendFile|approxRational|asTypeOf|asin|asinh|atan|atan2|",
            r"atanh|basicIORun|break|catch|ceiling|chr|compare|concat|concatMap|const|cos|cosh|curry|",
            r"cycle|decodeFloat|denominator|digitToInt|div|divMod|drop|dropWhile|either|elem|encodeFloat|",
            r"enumFrom|enumFromThen|enumFromThenTo|enumFromTo|error|even|exp|exponent|fail|filter|flip|",
            r"floatDigits|floatRadix|floatRange|floor|fmap|foldl|foldl1|foldr|foldr1|fromDouble|fromEnum|",
            r"fromInt|fromInteger|fromIntegral|fromRational|fst|gcd|getChar|getContents|getLine|group|",
            r"head|id|inRange|index|init|intToDigit|interact|ioError|isAlpha|isAlphaNum|isAscii|",
            r"isControl|isDenormalized|isDigit|isHexDigit|isIEEE|isInfinite|isLower|isNaN|isNegativeZero|",
            r"isOctDigit|isPrint|isSpace|isUpper|iterate|last|lcm|length|lex|lexDigits|lexLitChar|lines|",
            r"log|logBase|lookup|map|mapM|mapM_|max|maxBound|maximum|maybe|min|minBound|minimum|mod|",
            r"negate|not|notElem|null|numerator|odd|or|ord|otherwise|pack|pi|pred|primExitWith|print|",
            r"product|properFraction|putChar|putStr|putStrLn|quot|quotRem|range|rangeSize|read|readDec|",
            r"readFile|readFloat|readHex|readIO|readInt|readList|readLitChar|readLn|readOct|readParen|",
            r"readSigned|reads|readsPrec|realToFrac|recip|rem|repeat|replicate|return|reverse|round|",
            r"scaleFloat|scanl|scanl1|scanr|scanr1|seq|sequence|sequence_|show|showChar|showInt|showList|",
            r"showLitChar|showParen|showSigned|showString|shows|showsPrec|significand|signum|sin|sinh|",
            r"snd|sort|span|splitAt|sqrt|subtract|succ|sum|tail|take|takeWhile|tan|tanh|threadToIOResult|",
            r"toEnum|toInt|toInteger|toLower|toRational|toUpper|truncate|uncurry|undefined|unlines|until|",
            r"unwords|unzip|unzip3|userError|words|writeFile|zip|zip3|zipWith|zipWith3)\b",
        ])],
    ),
    e(
        "number",
        &[p(&[
            r"(?i)\b(?:\d+(?:\.\d+)?(?:e[+-]?\d+)?|0o[0-7]+|0x[0-9a-f]+)\b",
        ])],
    ),
    e(
        "operator",
        &[
            pf(&[r"`(?:[A-Z][\w']*\.)*[_a-z][\w']*`"], GR),
            pf(&[r"(\s)\.(?=\s)"], LB),
            p(&[
                r"[-!#$%*+=?\&@|\~:<>^\\/][-!#$%*+=?\&@|\~.:<>^\\/]*|\.[-!#$%*+=?\&@|\~.:<>^\\/]+",
            ]),
        ],
    ),
    e(
        "hvariable",
        &[pi(
            &[r"\b(?:[A-Z][\w']*\.)*[_a-z][\w']*"],
            0,
            &HASKELL_HVARIABLE,
        )],
    ),
    e(
        "constant",
        &[pi(
            &[r"\b(?:[A-Z][\w']*\.)*[A-Z][\w']*"],
            0,
            &HASKELL_CONSTANT,
        )],
    ),
    e("punctuation", &[p(&[r"[{}\[\];(),.:]"])]),
]);

pub(super) static HASKELL_IMPORT_STATEMENT: Grammar = Grammar::new(&[
    e("keyword", &[p(&[r"\b(?:as|hiding|import|qualified)\b"])]),
    e("punctuation", &[p(&[r"\."])]),
]);

pub(super) static HASKELL_HVARIABLE: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);

pub(super) static HASKELL_CONSTANT: Grammar = Grammar::new(&[e("punctuation", &[p(&[r"\."])])]);
