//! A small JSON reader and writer for the annotation sidecar files.
//!
//! The sidecar is plain JSON, and the crate stays dependency-free, so this
//! module carries just enough of the format to round-trip one sidecar:
//! objects, arrays, strings, numbers, booleans and null. The parser is
//! strict — malformed input is an error carrying the byte offset, never a
//! silent guess — and caps nesting so a hostile file cannot exhaust the
//! stack.

use std::fmt;

/// How deep a sidecar may nest before the parser gives up. A sidecar only
/// ever holds an object of arrays of flat objects, so 64 is already far
/// past anything a human writes.
const MAX_DEPTH: usize = 64;

/// One parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The entry for `key` when this value is an object, else `None`.
    pub fn entry(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// The text when this value is a string, else `None`.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The number when this value is a finite number, else `None`.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(value) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    /// The value as a whole number, when it is finite, not negative and has
    /// no fraction. Used for page indices and ids.
    pub fn as_whole(&self) -> Option<u64> {
        let value = self.as_number()?;
        if value < 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
            return None;
        }
        Some(value as u64)
    }

    /// The elements when this value is an array, else `None`.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// Parse `text` into one value. Anything other than whitespace after the
/// value is an error, so a truncated file never loads half an annotation
/// list.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser { text, pos: 0 };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.peek().is_some() {
        return parser.err("trailing content after the value");
    }
    Ok(value)
}

/// Write `value` with two-space indentation, so a sidecar stays readable
/// and hand-editable. `Value::Number` values that are not finite are
/// written as `null`, since JSON has no infinity or NaN.
pub fn write(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, 0, &mut out);
    out.push('\n');
    out
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn err<T>(&self, why: &str) -> Result<T, String> {
        Err(format!("{why} at byte {}", self.pos))
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return self.err("nesting is too deep");
        }
        match self.peek() {
            None => self.err("expected a value"),
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => Ok(Value::Text(self.text_value()?)),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(_) => self.err("expected a value"),
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.text[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(value)
        } else {
            self.err("expected a literal")
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.pos += 1; // '{'
        let mut entries: Vec<(String, Value)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Value::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return self.err("expected an object key");
            }
            let key = self.text_value()?;
            self.skip_whitespace();
            if self.bump() != Some(':') {
                return self.err("expected ':' after an object key");
            }
            self.skip_whitespace();
            let value = self.value(depth + 1)?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(Value::Object(entries)),
                _ => return self.err("expected ',' or '}' in an object"),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(Value::Array(items)),
                _ => return self.err("expected ',' or ']' in an array"),
            }
        }
    }

    fn text_value(&mut self) -> Result<String, String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let c = match self.bump() {
                Some(c) => c,
                None => return self.err("unterminated string"),
            };
            match c {
                '"' => return Ok(out),
                '\\' => out.push(self.escape()?),
                c if (c as u32) < 0x20 => return self.err("unescaped control character"),
                c => out.push(c),
            }
        }
    }

    /// The character behind a backslash, with `\u` handling surrogate pairs.
    fn escape(&mut self) -> Result<char, String> {
        let c = match self.bump() {
            Some(c) => c,
            None => return self.err("unterminated escape"),
        };
        let simple = match c {
            '"' => Some('"'),
            '\\' => Some('\\'),
            '/' => Some('/'),
            'b' => Some('\u{08}'),
            'f' => Some('\u{0c}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            _ => None,
        };
        if let Some(c) = simple {
            return Ok(c);
        }
        if c != 'u' {
            return self.err("unknown escape");
        }
        let first = self.hex4()?;
        if !(0xd800..0xdc00).contains(&first) {
            return match char::from_u32(first) {
                Some(c) => Ok(c),
                None => self.err("not a character"),
            };
        }
        // A high surrogate must be followed by its low half.
        if self.peek() == Some('\\') {
            let mark = self.pos;
            self.pos += 1;
            if self.bump() == Some('u') {
                let second = self.hex4()?;
                if (0xdc00..0xe000).contains(&second) {
                    let combined = 0x1_0000 + ((first - 0xd800) << 10) + (second - 0xdc00);
                    if let Some(c) = char::from_u32(combined) {
                        return Ok(c);
                    }
                }
                return self.err("invalid surrogate pair");
            }
            self.pos = mark;
        }
        self.err("lone high surrogate")
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = match self.bump() {
                Some(c) => c,
                None => return self.err("truncated \\u escape"),
            };
            let digit = digit
                .to_digit(16)
                .ok_or_else(|| format!("bad hex digit at byte {}", self.pos))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-') {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        let raw = &self.text[start..self.pos];
        if !number_is_valid(raw) {
            return self.err("expected a number");
        }
        match raw.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(Value::Number(value)),
            _ => self.err("expected a number"),
        }
    }
}

/// True for the spellings of a number that hold at least one digit in
/// every part: `1`, `-1.5`, `2e10`. Rust's own parser is looser (it takes
/// `1.` and `+1`), and a sidecar written by hand should not lean on that.
fn number_is_valid(raw: &str) -> bool {
    let digits: Vec<char> = raw.chars().collect();
    let mut at = 0usize;
    if digits.first() == Some(&'-') {
        at += 1;
    }
    at += count_digits(&digits[at..]);
    if at == 0 {
        return false; // no integer part at all
    }
    if digits.get(at) == Some(&'.') {
        at += 1;
        let before = at;
        at += count_digits(&digits[at..]);
        if at == before {
            return false;
        }
    }
    if matches!(digits.get(at), Some('e') | Some('E')) {
        at += 1;
        if matches!(digits.get(at), Some('+') | Some('-')) {
            at += 1;
        }
        let before = at;
        at += count_digits(&digits[at..]);
        if at == before {
            return false;
        }
    }
    at == digits.len()
}

fn count_digits(digits: &[char]) -> usize {
    digits.iter().take_while(|c| c.is_ascii_digit()).count()
}

fn write_value(value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&format_number(*number)),
        Value::Text(text) => {
            out.push('"');
            escape_into(text, out);
            out.push('"');
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                indent(depth + 1, out);
                write_value(item, depth + 1, out);
                out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
            }
            indent(depth, out);
            out.push(']');
        }
        Value::Object(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, entry)) in entries.iter().enumerate() {
                indent(depth + 1, out);
                out.push('"');
                escape_into(key, out);
                out.push_str("\": ");
                write_value(entry, depth + 1, out);
                out.push_str(if index + 1 < entries.len() {
                    ",\n"
                } else {
                    "\n"
                });
            }
            indent(depth, out);
            out.push('}');
        }
    }
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// A number as JSON text: whole values without a `.0` suffix, everything
/// else in shortest round-trip form. Non-finite values become `null`.
fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".to_string();
    }
    if value == value.trunc() && value.abs() < 9_007_199_254_740_992.0 {
        return format!("{}", value as i64);
    }
    format!("{value}")
}

fn escape_into(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        write_value(self, 0, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(entries: Vec<(&str, Value)>) -> Value {
        Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    #[test]
    fn round_trip_keeps_every_part() {
        let value = object(vec![
            ("version", Value::Number(1.0)),
            ("file", Value::Text("paper.pdf".into())),
            ("empty", Value::Array(Vec::new())),
            ("flag", Value::Bool(true)),
            ("none", Value::Null),
            (
                "annotations",
                Value::Array(vec![object(vec![
                    ("page", Value::Number(2.0)),
                    ("kind", Value::Text("text".into())),
                    (
                        "rect",
                        object(vec![
                            ("x0", Value::Number(-12.5)),
                            ("y0", Value::Number(0.0)),
                            ("x1", Value::Number(120.25)),
                            ("y1", Value::Number(30.0)),
                        ]),
                    ),
                ])]),
            ),
        ]);
        let text = write(&value);
        let parsed = parse(&text).expect("written JSON must parse back");
        assert_eq!(parsed, value);
        assert!(
            text.contains("\n  \"version\": 1,"),
            "pretty output: {text}"
        );
    }

    #[test]
    fn strings_carry_escapes_and_unicode() {
        let value =
            Value::Text("quote \" backslash \\ newline \n tab \t control \u{1} emoji 🙂".into());
        let parsed = parse(&write(&value)).expect("must parse");
        assert_eq!(parsed, value);
        let text = write(&value);
        assert!(text.contains("\\u0001"), "control escaped: {text}");
    }

    #[test]
    fn surrogate_pairs_combine() {
        let parsed = parse("\"\\ud83d\\ude42\"").expect("must parse");
        assert_eq!(parsed, Value::Text("🙂".into()));
        assert!(parse("\"\\ud83d\"").is_err(), "lone high surrogate");
    }

    #[test]
    fn numbers_keep_whole_and_fractional_forms() {
        assert_eq!(write(&Value::Number(-3.0)), "-3\n");
        assert_eq!(parse("-3").unwrap(), Value::Number(-3.0));
        assert_eq!(parse("1e3").unwrap(), Value::Number(1000.0));
        assert_eq!(parse("0.5").unwrap(), Value::Number(0.5));
        assert!(parse("-").is_err());
        assert!(parse("1.").is_err());
    }

    #[test]
    fn entry_lookup_and_coercions() {
        let value = object(vec![
            ("page", Value::Number(3.0)),
            ("kind", Value::Text("highlight".into())),
            ("bad", Value::Number(-1.0)),
            ("fraction", Value::Number(1.5)),
            ("list", Value::Array(vec![Value::Null])),
        ]);
        assert_eq!(value.entry("page").unwrap().as_whole(), Some(3));
        assert_eq!(value.entry("kind").unwrap().as_text(), Some("highlight"));
        assert_eq!(value.entry("missing").and_then(Value::as_whole), None);
        assert_eq!(value.entry("bad").unwrap().as_whole(), None);
        assert_eq!(value.entry("fraction").unwrap().as_whole(), None);
        assert_eq!(value.entry("page").unwrap().as_array(), None);
        assert_eq!(
            value
                .entry("list")
                .unwrap()
                .as_array()
                .map(|items| items.len()),
            Some(1)
        );
        assert_eq!(Value::Number(f64::INFINITY).as_number(), None);
    }

    #[test]
    fn malformed_input_is_rejected_with_a_position() {
        for bad in [
            "",
            "   ",
            "{",
            "{\"a\":}",
            "{\"a\" 1}",
            "[1,]",
            "[1 2]",
            "nope",
            "\"unterminated",
            "{} trailing",
            "1 2",
            "{\"a\": 1}{\"b\": 2}",
        ] {
            let error = parse(bad)
                .err()
                .unwrap_or_else(|| panic!("{bad:?} must not parse"));
            assert!(error.contains("at byte"), "error names a position: {error}");
        }
    }

    #[test]
    fn deep_nesting_is_capped() {
        let mut text = String::new();
        for _ in 0..(MAX_DEPTH + 2) {
            text.push('[');
        }
        let error = parse(&text).expect_err("deep nesting must be refused");
        assert!(error.contains("too deep"), "{error}");
    }
}
