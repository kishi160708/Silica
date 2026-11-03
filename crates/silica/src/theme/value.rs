//! CSS values for the theme registry: the subset of the value grammar that
//! Obsidian's custom properties are written in, parsed once and resolved
//! against a table of other properties.
//!
//! A theme is a table of `--name: value` declarations, and a value is a
//! literal (`#1c1c1c`, `13px`, `0.85`), a reference (`var(--text-muted)`, with
//! an optional fallback), a colour function (`hsl()`, `rgba()`,
//! `color-mix(in oklch, …)`), `calc()` arithmetic, a keyword (`inherit`,
//! `transparent`, `none`), a shorthand sequence (`1px solid transparent`) or
//! a comma list (shadow layers). Nothing here is a cascade: a selector never
//! appears, only names and values.
//!
//! Resolution follows the CSS rules that matter for these values: a `var()`
//! substitutes the referenced property's value (or its fallback when the
//! property is missing); `calc()` adds like units and scales by unitless
//! numbers; `hsl()` takes `calc()` channels, as Obsidian's accent shades do;
//! `color-mix(in oklch, C p%, transparent)` is `C` at alpha `p`, the form
//! every wash in the default theme uses, and a mix of two opaque colours
//! interpolates in OKLCH.

use std::collections::HashMap;

use gpui::{hsla, Hsla, Rgba};

/// A parsed declaration value.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `var(--name)` or `var(--name, fallback)`.
    Var(String, Option<Box<Expr>>),
    /// A number with its unit.
    Num(f32, Unit),
    /// A `#rrggbb[aa]` literal or a named colour.
    Color(Hsla),
    /// A function call with its comma-separated arguments.
    Func(String, Vec<Expr>),
    /// A bare identifier: `inherit`, `none`, `solid`, `inset`, `in`, `oklch`.
    Ident(String),
    /// A space-separated sequence of terms.
    Seq(Vec<Expr>),
    /// A comma-separated list.
    List(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    None,
    Px,
    Em,
    Rem,
    Percent,
    Deg,
}

/// A resolved value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(Hsla),
    /// A unitless number, or degrees.
    Number(f32),
    /// `0..=100`.
    Percent(f32),
    Length(Length),
    /// `inherit`, `none`, `initial`, `unset`, `currentcolor`, `solid`, …
    Keyword(String),
    Shadow(Vec<ShadowLayer>),
    Seq(Vec<Value>),
    /// A reference to a property the table has not got, with no fallback.
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Px(f32),
    Em(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Hsla,
    pub inset: bool,
}

// ---- parsing ---------------------------------------------------------------

/// Parse a declaration value. Anything the grammar does not cover parses as
/// an identifier or a sequence of them rather than failing: the registry's
/// kinds decide what a value has to be.
pub fn parse(src: &str) -> Expr {
    let mut p = Parser {
        s: src.as_bytes(),
        i: 0,
    };
    p.list()
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    /// Comma-separated list of sequences; a single item is returned bare.
    fn list(&mut self) -> Expr {
        let mut items = vec![self.seq()];
        loop {
            self.ws();
            if self.peek() == Some(b',') {
                self.i += 1;
                items.push(self.seq());
            } else {
                break;
            }
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Expr::List(items)
        }
    }

    /// Space-separated terms up to a comma or a closing paren.
    fn seq(&mut self) -> Expr {
        let mut terms = Vec::new();
        loop {
            self.ws();
            match self.peek() {
                None | Some(b',') | Some(b')') => break,
                Some(_) => match self.term() {
                    Some(t) => terms.push(t),
                    None => break,
                },
            }
        }
        if terms.len() == 1 {
            terms.pop().unwrap()
        } else {
            Expr::Seq(terms)
        }
    }

    fn term(&mut self) -> Option<Expr> {
        self.ws();
        let c = self.peek()?;
        if c == b'#' {
            self.i += 1;
            let start = self.i;
            while matches!(self.peek(), Some(b) if b.is_ascii_hexdigit()) {
                self.i += 1;
            }
            let hex = std::str::from_utf8(&self.s[start..self.i]).ok()?;
            return Some(
                hex_color(hex)
                    .map(Expr::Color)
                    .unwrap_or_else(|| Expr::Ident(format!("#{hex}"))),
            );
        }
        if c.is_ascii_digit()
            || c == b'.'
            || ((c == b'-' || c == b'+')
                && matches!(self.s.get(self.i + 1), Some(b) if b.is_ascii_digit() || *b == b'.'))
        {
            return self.number();
        }
        if c == b'/' {
            self.i += 1;
            return Some(Expr::Ident("/".into()));
        }
        if c.is_ascii_alphabetic() || c == b'-' || c == b'_' {
            let start = self.i;
            while matches!(
                self.peek(),
                Some(b) if b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
            ) {
                self.i += 1;
            }
            let name = std::str::from_utf8(&self.s[start..self.i])
                .ok()?
                .to_string();
            if self.peek() == Some(b'(') {
                self.i += 1;
                let args = self.args(&name);
                self.ws();
                if self.peek() == Some(b')') {
                    self.i += 1;
                }
                let lower = name.to_ascii_lowercase();
                if lower == "var" {
                    let mut it = args.into_iter();
                    let target = match it.next() {
                        Some(Expr::Ident(n)) => n,
                        Some(other) => return Some(Expr::Func("var".into(), vec![other])),
                        None => return Some(Expr::Func("var".into(), vec![])),
                    };
                    let fallback = it.next().map(Box::new);
                    return Some(Expr::Var(target, fallback));
                }
                return Some(Expr::Func(lower, args));
            }
            let lower = name.to_ascii_lowercase();
            return Some(match named_color(&lower) {
                Some(c) => Expr::Color(c),
                None => Expr::Ident(name),
            });
        }
        // Something the grammar does not know: skip it as an identifier.
        self.i += 1;
        Some(Expr::Ident((c as char).to_string()))
    }

    /// Function arguments: comma-separated, each a sequence. `calc()` and
    /// its arithmetic cousins keep their content as one infix expression.
    fn args(&mut self, name: &str) -> Vec<Expr> {
        if matches!(name.to_ascii_lowercase().as_str(), "calc") {
            return vec![self.infix()];
        }
        let mut args = Vec::new();
        loop {
            self.ws();
            if matches!(self.peek(), None | Some(b')')) {
                break;
            }
            if matches!(name.to_ascii_lowercase().as_str(), "min" | "max" | "clamp") {
                args.push(self.infix());
            } else {
                args.push(self.seq());
            }
            self.ws();
            if self.peek() == Some(b',') {
                self.i += 1;
            } else {
                break;
            }
        }
        args
    }

    /// `a + b * c` with the usual precedence, parenthesised groups included,
    /// encoded as `Func("+", [a, b])` nodes.
    fn infix(&mut self) -> Expr {
        let mut lhs = self.product();
        loop {
            self.ws();
            match self.peek() {
                Some(op @ (b'+' | b'-')) => {
                    self.i += 1;
                    let rhs = self.product();
                    lhs = Expr::Func((op as char).to_string(), vec![lhs, rhs]);
                }
                _ => return lhs,
            }
        }
    }

    fn product(&mut self) -> Expr {
        let mut lhs = self.atom();
        loop {
            self.ws();
            match self.peek() {
                Some(op @ (b'*' | b'/')) => {
                    self.i += 1;
                    let rhs = self.atom();
                    lhs = Expr::Func((op as char).to_string(), vec![lhs, rhs]);
                }
                _ => return lhs,
            }
        }
    }

    fn atom(&mut self) -> Expr {
        self.ws();
        if self.peek() == Some(b'(') {
            self.i += 1;
            let e = self.infix();
            self.ws();
            if self.peek() == Some(b')') {
                self.i += 1;
            }
            return e;
        }
        self.term().unwrap_or(Expr::Ident(String::new()))
    }

    fn number(&mut self) -> Option<Expr> {
        let start = self.i;
        if matches!(self.peek(), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while matches!(self.peek(), Some(b) if b.is_ascii_digit() || b == b'.') {
            self.i += 1;
        }
        if matches!(self.peek(), Some(b'e' | b'E'))
            && matches!(self.s.get(self.i + 1), Some(b) if b.is_ascii_digit() || *b == b'-')
        {
            self.i += 1;
            if self.peek() == Some(b'-') {
                self.i += 1;
            }
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        let v: f32 = text.parse().ok()?;
        let ustart = self.i;
        while matches!(
            self.peek(),
            Some(b) if b.is_ascii_alphabetic() || b == b'%'
        ) {
            self.i += 1;
        }
        let unit = match std::str::from_utf8(&self.s[ustart..self.i])
            .ok()?
            .to_ascii_lowercase()
            .as_str()
        {
            "" => Unit::None,
            "px" => Unit::Px,
            "em" => Unit::Em,
            "rem" => Unit::Rem,
            "%" => Unit::Percent,
            "deg" => Unit::Deg,
            _ => Unit::None,
        };
        Some(Expr::Num(v, unit))
    }
}

fn hex_color(hex: &str) -> Option<Hsla> {
    let v = u32::from_str_radix(hex, 16).ok()?;
    let rgba = match hex.len() {
        3 => {
            let r = (v >> 8) & 0xf;
            let g = (v >> 4) & 0xf;
            let b = v & 0xf;
            Rgba {
                r: (r * 17) as f32 / 255.,
                g: (g * 17) as f32 / 255.,
                b: (b * 17) as f32 / 255.,
                a: 1.,
            }
        }
        4 => {
            let r = (v >> 12) & 0xf;
            let g = (v >> 8) & 0xf;
            let b = (v >> 4) & 0xf;
            let a = v & 0xf;
            Rgba {
                r: (r * 17) as f32 / 255.,
                g: (g * 17) as f32 / 255.,
                b: (b * 17) as f32 / 255.,
                a: (a * 17) as f32 / 255.,
            }
        }
        6 => gpui::rgb(v),
        8 => gpui::rgba(v),
        _ => return None,
    };
    Some(rgba.into())
}

fn named_color(name: &str) -> Option<Hsla> {
    Some(match name {
        "transparent" => hsla(0., 0., 0., 0.),
        "white" => gpui::rgb(0xffffff).into(),
        "black" => gpui::rgb(0x000000).into(),
        "gray" | "grey" => gpui::rgb(0x808080).into(),
        "red" => gpui::rgb(0xff0000).into(),
        "blue" => gpui::rgb(0x0000ff).into(),
        "green" => gpui::rgb(0x008000).into(),
        "yellow" => gpui::rgb(0xffff00).into(),
        "orange" => gpui::rgb(0xffa500).into(),
        "purple" => gpui::rgb(0x800080).into(),
        "pink" => gpui::rgb(0xffc0cb).into(),
        "cyan" | "aqua" => gpui::rgb(0x00ffff).into(),
        "magenta" | "fuchsia" => gpui::rgb(0xff00ff).into(),
        "silver" => gpui::rgb(0xc0c0c0).into(),
        "maroon" => gpui::rgb(0x800000).into(),
        "navy" => gpui::rgb(0x000080).into(),
        "teal" => gpui::rgb(0x008080).into(),
        "olive" => gpui::rgb(0x808000).into(),
        "lime" => gpui::rgb(0x00ff00).into(),
        _ => return None,
    })
}

// ---- resolution ------------------------------------------------------------

/// The most `var()` references a single resolution may follow.
///
/// A real theme chains three or four deep at most — `--nav-item-color` is
/// `--text-muted` is `--color-base-70` is a hex. The limit is generous enough
/// that no hand-written theme reaches it and small enough that a file built to
/// recurse cannot exhaust the stack.
const MAX_DEPTH: usize = 64;

/// The declarations a resolution reads from: later entries win, as in a
/// stylesheet where the theme's `body` block follows the app's.
#[derive(Debug, Default, Clone)]
pub struct Table {
    decls: HashMap<String, Expr>,
}

impl Table {
    pub fn new() -> Table {
        Table::default()
    }

    /// Declare `name` (with or without the leading `--`) as `value`.
    pub fn set(&mut self, name: &str, value: Expr) {
        self.decls.insert(canonical(name), value);
    }

    pub fn set_str(&mut self, name: &str, value: &str) {
        self.set(name, parse(value));
    }

    /// Resolve every declaration. A reference to a missing property without a
    /// fallback resolves to [`Value::Invalid`]; a cycle resolves to it too.
    pub fn resolve_all(&self) -> Resolved {
        let mut r = Resolver {
            table: self,
            memo: HashMap::new(),
            stack: Vec::new(),
        };
        let mut out = HashMap::new();
        for name in self.decls.keys() {
            let v = r.property(name);
            out.insert(name.clone(), v);
        }
        Resolved { values: out }
    }
}

fn canonical(name: &str) -> String {
    if name.starts_with("--") {
        name.to_string()
    } else {
        format!("--{name}")
    }
}

/// Every property of a [`Table`], resolved.
#[derive(Debug, Default, Clone)]
pub struct Resolved {
    values: HashMap<String, Value>,
}

impl Resolved {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(&canonical(name))
    }
}

struct Resolver<'a> {
    table: &'a Table,
    memo: HashMap<String, Value>,
    stack: Vec<String>,
}

impl<'a> Resolver<'a> {
    fn property(&mut self, name: &str) -> Value {
        if let Some(v) = self.memo.get(name) {
            return v.clone();
        }
        if self.stack.iter().any(|n| n == name) {
            return Value::Invalid;
        }
        // * A cycle is caught above, but a long acyclic chain is not: a theme
        // * can declare `--a: var(--b)` a thousand times over, and resolving it
        // * would recurse as deep as the file is long. `stack` is the recursion
        // * the resolver is actually in, so its length is the depth to bound.
        if self.stack.len() >= MAX_DEPTH {
            return Value::Invalid;
        }
        let Some(expr) = self.table.decls.get(name) else {
            return Value::Invalid;
        };
        self.stack.push(name.to_string());
        let v = self.eval(expr);
        self.stack.pop();
        self.memo.insert(name.to_string(), v.clone());
        v
    }

    fn eval(&mut self, e: &Expr) -> Value {
        match e {
            Expr::Var(name, fallback) => {
                let v = self.property(name);
                match (v, fallback) {
                    (Value::Invalid, Some(f)) => self.eval(f),
                    (v, _) => v,
                }
            }
            Expr::Num(v, Unit::None) => Value::Number(*v),
            Expr::Num(v, Unit::Deg) => Value::Number(*v),
            Expr::Num(v, Unit::Percent) => Value::Percent(*v),
            Expr::Num(v, Unit::Px) => Value::Length(Length::Px(*v)),
            Expr::Num(v, Unit::Em | Unit::Rem) => Value::Length(Length::Em(*v)),
            Expr::Color(c) => Value::Color(*c),
            Expr::Ident(i) => Value::Keyword(i.to_ascii_lowercase()),
            Expr::Func(name, args) => self.func(name, args),
            Expr::Seq(terms) => {
                let vals: Vec<Value> = terms.iter().map(|t| self.eval(t)).collect();
                match shadow_layer(&vals) {
                    Some(layer) => Value::Shadow(vec![layer]),
                    None => Value::Seq(vals),
                }
            }
            Expr::List(items) => {
                let vals: Vec<Value> = items.iter().map(|t| self.eval(t)).collect();
                if !vals.is_empty() && vals.iter().all(|v| matches!(v, Value::Shadow(_))) {
                    let mut layers = Vec::new();
                    for v in vals {
                        if let Value::Shadow(l) = v {
                            layers.extend(l);
                        }
                    }
                    return Value::Shadow(layers);
                }
                Value::Seq(vals)
            }
        }
    }

    fn func(&mut self, name: &str, args: &[Expr]) -> Value {
        match name {
            "+" | "-" | "*" | "/" => {
                let a = self.eval(&args[0]);
                let b = self.eval(&args[1]);
                arith(name, a, b)
            }
            "calc" => self.eval(&args[0]),
            "min" | "max" => {
                let vals: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
                let mut best: Option<Value> = None;
                for v in vals {
                    best = Some(match best {
                        None => v,
                        Some(b) => match (scalar(&b), scalar(&v)) {
                            (Some(x), Some(y)) => {
                                if (name == "max") == (y > x) {
                                    v
                                } else {
                                    b
                                }
                            }
                            _ => b,
                        },
                    });
                }
                best.unwrap_or(Value::Invalid)
            }
            "rgb" | "rgba" => {
                let ch = self.channels(args);
                if ch.len() < 3 {
                    return Value::Invalid;
                }
                let f = |v: &Value| match v {
                    Value::Number(n) => n / 255.,
                    Value::Percent(p) => p / 100.,
                    _ => 0.,
                };
                let a = ch.get(3).map(alpha).unwrap_or(1.);
                Value::Color(
                    Rgba {
                        r: f(&ch[0]),
                        g: f(&ch[1]),
                        b: f(&ch[2]),
                        a,
                    }
                    .into(),
                )
            }
            "hsl" | "hsla" => {
                let ch = self.channels(args);
                if ch.len() < 3 {
                    return Value::Invalid;
                }
                let h = match &ch[0] {
                    Value::Number(n) => *n,
                    _ => 0.,
                };
                let pct = |v: &Value| match v {
                    Value::Percent(p) => p / 100.,
                    Value::Number(n) => *n,
                    _ => 0.,
                };
                let a = ch.get(3).map(alpha).unwrap_or(1.);
                Value::Color(hsla(
                    h.rem_euclid(360.) / 360.,
                    pct(&ch[1]).clamp(0., 1.),
                    pct(&ch[2]).clamp(0., 1.),
                    a,
                ))
            }
            "color-mix" => {
                let mut colors: Vec<(Hsla, Option<f32>)> = Vec::new();
                let mut space = String::from("oklch");
                for arg in args {
                    match self.eval(arg) {
                        Value::Seq(terms) => {
                            if let [Value::Keyword(k), Value::Keyword(s), ..] = terms.as_slice() {
                                if k == "in" {
                                    space = s.clone();
                                    continue;
                                }
                            }
                            let mut c = None;
                            let mut p = None;
                            for t in terms {
                                match t {
                                    Value::Color(x) => c = Some(x),
                                    Value::Percent(x) => p = Some(x / 100.),
                                    _ => {}
                                }
                            }
                            if let Some(c) = c {
                                colors.push((c, p));
                            }
                        }
                        Value::Color(c) => colors.push((c, None)),
                        _ => {}
                    }
                }
                if colors.len() != 2 {
                    return Value::Invalid;
                }
                let (c1, p1) = colors[0];
                let (c2, p2) = colors[1];
                let (p1, p2) = match (p1, p2) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) => (a, 1. - a),
                    (None, Some(b)) => (1. - b, b),
                    (None, None) => (0.5, 0.5),
                };
                Value::Color(mix(&space, c1, p1, c2, p2))
            }
            "var" => Value::Invalid,
            _ => Value::Keyword(format!("{name}()")),
        }
    }

    /// The channels of a colour function, in the legacy comma form or the
    /// modern space form with `/ alpha`.
    fn channels(&mut self, args: &[Expr]) -> Vec<Value> {
        let mut out = Vec::new();
        for a in args {
            match a {
                Expr::Seq(terms) => {
                    let mut slash = false;
                    for t in terms {
                        if let Expr::Ident(i) = t {
                            if i == "/" {
                                slash = true;
                                continue;
                            }
                            if i.eq_ignore_ascii_case("none") {
                                out.push(Value::Number(0.));
                                continue;
                            }
                        }
                        let v = self.eval(t);
                        if slash {
                            while out.len() < 3 {
                                out.push(Value::Number(0.));
                            }
                        }
                        out.push(v);
                    }
                }
                other => out.push(self.eval(other)),
            }
        }
        out
    }
}

fn alpha(v: &Value) -> f32 {
    match v {
        Value::Number(n) => n.clamp(0., 1.),
        Value::Percent(p) => (p / 100.).clamp(0., 1.),
        _ => 1.,
    }
}

fn scalar(v: &Value) -> Option<f32> {
    match v {
        Value::Number(n) | Value::Percent(n) => Some(*n),
        Value::Length(Length::Px(n)) | Value::Length(Length::Em(n)) => Some(*n),
        _ => None,
    }
}

#[allow(clippy::redundant_guards)]
fn arith(op: &str, a: Value, b: Value) -> Value {
    use Length as L;
    use Value::{Invalid, Number, Percent};
    match op {
        "+" | "-" => {
            let sign = if op == "+" { 1. } else { -1. };
            match (a, b) {
                (Number(x), Number(y)) => Number(x + sign * y),
                (Percent(x), Percent(y)) => Percent(x + sign * y),
                (Value::Length(L::Px(x)), Value::Length(L::Px(y))) => {
                    Value::Length(L::Px(x + sign * y))
                }
                (Value::Length(L::Em(x)), Value::Length(L::Em(y))) => {
                    Value::Length(L::Em(x + sign * y))
                }
                (Value::Length(l), Number(y)) if y == 0. => Value::Length(l),
                (Number(x), Value::Length(l)) if x == 0. => match l {
                    L::Px(v) => Value::Length(L::Px(sign * v)),
                    L::Em(v) => Value::Length(L::Em(sign * v)),
                },
                _ => Invalid,
            }
        }
        "*" => match (a, b) {
            (Number(x), Number(y)) => Number(x * y),
            (Percent(x), Number(y)) | (Number(y), Percent(x)) => Percent(x * y),
            (Value::Length(L::Px(x)), Number(y)) | (Number(y), Value::Length(L::Px(x))) => {
                Value::Length(L::Px(x * y))
            }
            (Value::Length(L::Em(x)), Number(y)) | (Number(y), Value::Length(L::Em(x))) => {
                Value::Length(L::Em(x * y))
            }
            _ => Invalid,
        },
        "/" => match (a, b) {
            (_, Number(y)) if y == 0. => Invalid,
            (Number(x), Number(y)) => Number(x / y),
            (Percent(x), Number(y)) => Percent(x / y),
            (Value::Length(L::Px(x)), Number(y)) => Value::Length(L::Px(x / y)),
            (Value::Length(L::Em(x)), Number(y)) => Value::Length(L::Em(x / y)),
            _ => Invalid,
        },
        _ => Invalid,
    }
}

/// A sequence of two to four lengths and a colour, `inset` optional, is a
/// shadow layer. Lengths and the colour may come in either order. Without a
/// colour the sequence is not a shadow: `10px 10px 0 0` is a radius.
fn shadow_layer(vals: &[Value]) -> Option<ShadowLayer> {
    let mut lengths = Vec::new();
    let mut color = None;
    let mut inset = false;
    for v in vals {
        match v {
            Value::Length(Length::Px(p)) => lengths.push(*p),
            Value::Number(n) if *n == 0. => lengths.push(0.),
            Value::Color(c) => {
                if color.is_some() {
                    return None;
                }
                color = Some(*c);
            }
            Value::Keyword(k) if k == "inset" => inset = true,
            _ => return None,
        }
    }
    if !(2..=4).contains(&lengths.len()) || color.is_none() {
        return None;
    }
    Some(ShadowLayer {
        x: lengths[0],
        y: lengths[1],
        blur: lengths.get(2).copied().unwrap_or(0.),
        spread: lengths.get(3).copied().unwrap_or(0.),
        color: color.unwrap_or_else(|| gpui::rgb(0).into()),
        inset,
    })
}

// ---- colour spaces ---------------------------------------------------------

/// `color-mix()`: premultiplied interpolation, so a mix with `transparent`
/// keeps the other colour's hue and takes its share as alpha.
fn mix(space: &str, c1: Hsla, p1: f32, c2: Hsla, p2: f32) -> Hsla {
    let total = p1 + p2;
    let (w1, w2) = if total <= 0. {
        (0.5, 0.5)
    } else {
        (p1 / total, p2 / total)
    };
    let a = c1.a * w1 + c2.a * w2;
    if a <= 0. {
        return hsla(0., 0., 0., 0.);
    }
    let q1 = c1.a * w1 / a;
    let q2 = c2.a * w2 / a;
    let r1 = c1.to_rgb();
    let r2 = c2.to_rgb();
    let rgb = match space {
        "oklch" | "oklab" => {
            let (l1, a1, b1) = oklab(r1);
            let (l2, a2, b2) = oklab(r2);
            let (l, aa, bb) = if space == "oklch" {
                let (c1, h1) = (a1.hypot(b1), b1.atan2(a1));
                let (c2, h2) = (a2.hypot(b2), b2.atan2(a2));
                let (h1, h2) = if c1 < 1e-4 {
                    (h2, h2)
                } else if c2 < 1e-4 {
                    (h1, h1)
                } else {
                    (h1, h2)
                };
                let mut dh = h2 - h1;
                if dh > std::f32::consts::PI {
                    dh -= 2. * std::f32::consts::PI;
                } else if dh < -std::f32::consts::PI {
                    dh += 2. * std::f32::consts::PI;
                }
                let h = h1 + dh * q2;
                let c = c1 * q1 + c2 * q2;
                (l1 * q1 + l2 * q2, c * h.cos(), c * h.sin())
            } else {
                (l1 * q1 + l2 * q2, a1 * q1 + a2 * q2, b1 * q1 + b2 * q2)
            };
            oklab_to_rgb(l, aa, bb)
        }
        _ => Rgba {
            r: r1.r * q1 + r2.r * q2,
            g: r1.g * q1 + r2.g * q2,
            b: r1.b * q1 + r2.b * q2,
            a: 1.,
        },
    };
    let mut out: Hsla = Rgba { a: 1., ..rgb }.into();
    out.a = a;
    out
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}

fn oklab(c: Rgba) -> (f32, f32, f32) {
    let r = srgb_to_linear(c.r);
    let g = srgb_to_linear(c.g);
    let b = srgb_to_linear(c.b);
    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let l = l.cbrt();
    let m = m.cbrt();
    let s = s.cbrt();
    (
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    )
}

fn oklab_to_rgb(l: f32, a: f32, b: f32) -> Rgba {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
    let g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
    let bb = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;
    Rgba {
        r: linear_to_srgb(r).clamp(0., 1.),
        g: linear_to_srgb(g).clamp(0., 1.),
        b: linear_to_srgb(bb).clamp(0., 1.),
        a: 1.,
    }
}

#[cfg(test)]
pub fn hex(c: Hsla) -> String {
    let r = c.to_rgb();
    let ch = |v: f32| (v * 255.).round().clamp(0., 255.) as u8;
    if (r.a - 1.).abs() < 1e-3 {
        format!("#{:02x}{:02x}{:02x}", ch(r.r), ch(r.g), ch(r.b))
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            ch(r.r),
            ch(r.g),
            ch(r.b),
            ch(r.a)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(decls: &[(&str, &str)]) -> Table {
        let mut t = Table::new();
        for (k, v) in decls {
            t.set_str(k, v);
        }
        t
    }

    fn color(t: &Table, name: &str) -> String {
        match t.resolve_all().get(name) {
            Some(Value::Color(c)) => hex(*c),
            other => panic!("{name}: {other:?}"),
        }
    }

    #[test]
    fn literals_parse() {
        assert_eq!(parse("13px"), Expr::Num(13., Unit::Px));
        assert_eq!(parse("0.875em"), Expr::Num(0.875, Unit::Em));
        assert_eq!(parse("30%"), Expr::Num(30., Unit::Percent));
        assert_eq!(parse("inherit"), Expr::Ident("inherit".into()));
        assert_eq!(parse("#1C1C1C"), Expr::Color(gpui::rgb(0x1c1c1c).into()));
        assert_eq!(
            parse("var(--text-muted)"),
            Expr::Var("--text-muted".into(), None)
        );
        assert_eq!(
            parse("var(--text-highlight-bg, var(--highlight-background-yellow))"),
            Expr::Var(
                "--text-highlight-bg".into(),
                Some(Box::new(Expr::Var(
                    "--highlight-background-yellow".into(),
                    None
                )))
            )
        );
    }

    #[test]
    fn var_chains_resolve_through_the_table() {
        let t = table(&[
            ("--color-base-70", "#b3b3b3"),
            ("--text-muted", "var(--color-base-70)"),
            ("--nav-item-color", "var(--text-muted)"),
        ]);
        assert_eq!(color(&t, "--nav-item-color"), "#b3b3b3");
        assert_eq!(color(&t, "nav-item-color"), "#b3b3b3");
    }

    #[test]
    fn a_missing_reference_takes_its_fallback_or_is_invalid() {
        let t = table(&[
            ("--highlight-background-yellow", "#e0de71"),
            (
                "--highlight-background",
                "var(--text-highlight-bg, var(--highlight-background-yellow))",
            ),
            (
                "--graph-line",
                "var(--color-base-35, var(--background-modifier-border-focus))",
            ),
            ("--background-modifier-border-focus", "#555555"),
            ("--lost", "var(--nothing)"),
        ]);
        assert_eq!(color(&t, "--highlight-background"), "#e0de71");
        assert_eq!(color(&t, "--graph-line"), "#555555");
        assert_eq!(t.resolve_all().get("--lost"), Some(&Value::Invalid));
    }

    #[test]
    fn a_cycle_is_invalid_not_a_hang() {
        let t = table(&[("--a", "var(--b)"), ("--b", "var(--a)")]);
        assert_eq!(t.resolve_all().get("--a"), Some(&Value::Invalid));
    }

    /// A chain that is acyclic but deeper than [`MAX_DEPTH`] stops rather than
    /// recursing once per declaration in the file.
    #[test]
    fn a_chain_deeper_than_the_limit_stops() {
        let mut t = Table::new();
        // 200 links: `--c0` -> `--c1` -> … -> `--c199` -> a literal.
        for i in 0..200 {
            t.set_str(&format!("--c{i}"), &format!("var(--c{})", i + 1));
        }
        t.set_str("--c200", "#123456");
        let r = t.resolve_all();
        // The head of the chain is past the limit and resolves to nothing;
        // the literal at the end and its near neighbours are still fine.
        assert_eq!(r.get("--c0"), Some(&Value::Invalid));
        assert!(matches!(r.get("--c200"), Some(Value::Color(_))));
        // A chain inside the limit still resolves.
        let mut short = Table::new();
        short.set_str("--a", "var(--b)");
        short.set_str("--b", "var(--c)");
        short.set_str("--c", "#abcdef");
        assert!(matches!(
            short.resolve_all().get("--a"),
            Some(Value::Color(_))
        ));
    }

    #[test]
    fn hsl_takes_calc_channels() {
        let t = table(&[
            ("--accent-h", "258"),
            ("--accent-s", "88%"),
            ("--accent-l", "66%"),
            ("--color-accent", "hsl(var(--accent-h), var(--accent-s), var(--accent-l))"),
            (
                "--color-accent-1",
                "hsl(calc(var(--accent-h) - 3), calc(var(--accent-s) * 1.02), calc(var(--accent-l) * 1.15))",
            ),
        ]);
        let r = t.resolve_all();
        match r.get("--color-accent") {
            Some(Value::Color(c)) => {
                assert!(
                    (c.h * 360. - 258.).abs() < 0.01
                        && (c.s - 0.88).abs() < 0.001
                        && (c.l - 0.66).abs() < 0.001,
                    "{c:?}"
                )
            }
            other => panic!("{other:?}"),
        }
        match r.get("--color-accent-1") {
            Some(Value::Color(c)) => {
                assert!(
                    (c.h * 360. - 255.).abs() < 0.01
                        && (c.s - 0.8976).abs() < 0.001
                        && (c.l - 0.759).abs() < 0.001,
                    "{c:?}"
                )
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn calc_adds_like_units_and_scales() {
        let t = table(&[
            ("--font-weight", "400"),
            ("--bold-modifier", "200"),
            (
                "--bold-weight",
                "calc(var(--font-weight) + var(--bold-modifier))",
            ),
            ("--x", "calc(1px * 3)"),
            ("--y", "calc((2em + 1em) / 2)"),
            ("--z", "max(13px, 1em)"),
        ]);
        let r = t.resolve_all();
        assert_eq!(r.get("--bold-weight"), Some(&Value::Number(600.)));
        assert_eq!(r.get("--x"), Some(&Value::Length(Length::Px(3.))));
        assert_eq!(r.get("--y"), Some(&Value::Length(Length::Em(1.5))));
        assert_eq!(r.get("--z"), Some(&Value::Length(Length::Px(13.))));
    }

    #[test]
    fn a_mix_with_transparent_is_the_colour_at_that_alpha() {
        let t = table(&[
            ("--mono-100", "white"),
            (
                "--background-modifier-hover",
                "color-mix(in oklch, var(--mono-100) 6.7%, transparent)",
            ),
            ("--interactive-accent", "hsl(258, 88%, 66%)"),
            (
                "--tag-background",
                "color-mix(in oklch, var(--interactive-accent) 10%, transparent)",
            ),
        ]);
        let r = t.resolve_all();
        match r.get("--background-modifier-hover") {
            Some(Value::Color(c)) => {
                assert!((c.a - 0.067).abs() < 1e-4 && c.l > 0.999, "{c:?}")
            }
            other => panic!("{other:?}"),
        }
        match r.get("--tag-background") {
            Some(Value::Color(c)) => {
                assert!((c.a - 0.1).abs() < 1e-4, "{c:?}");
                assert!((c.h * 360. - 258.).abs() < 0.5, "{c:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_mix_of_two_opaque_colours_interpolates_in_oklch() {
        let t = table(&[("--m", "color-mix(in oklch, white 50%, black)")]);
        match t.resolve_all().get("--m") {
            Some(Value::Color(c)) => assert_eq!(hex(*c), "#636363", "{c:?}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rgba_and_modern_syntax() {
        let t = table(&[
            ("--a", "rgba(10, 10, 10, 0.4)"),
            ("--b", "rgb(255 0 0 / 50%)"),
            ("--c", "hsl(0 100% 50% / 0.5)"),
        ]);
        let r = t.resolve_all();
        match r.get("--a") {
            Some(Value::Color(c)) => assert_eq!(hex(*c), "#0a0a0a66"),
            other => panic!("{other:?}"),
        }
        match r.get("--b") {
            Some(Value::Color(c)) => assert_eq!(hex(*c), "#ff000080"),
            other => panic!("{other:?}"),
        }
        match r.get("--c") {
            Some(Value::Color(c)) => assert_eq!(hex(*c), "#ff000080"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn shadows_are_layers() {
        let t = table(&[(
            "--shadow-s",
            "0px 1px 2px rgba(0, 0, 0, 0.121), 0px 3.4px 6.7px rgba(0, 0, 0, 0.179), inset 0 0 0 1px #333",
        )]);
        match t.resolve_all().get("--shadow-s") {
            Some(Value::Shadow(layers)) => {
                assert_eq!(layers.len(), 3);
                assert_eq!(
                    (layers[1].x, layers[1].y, layers[1].blur, layers[1].spread),
                    (0., 3.4, 6.7, 0.)
                );
                assert!((layers[1].color.a - 0.179).abs() < 1e-4);
                assert!(layers[2].inset && layers[2].spread == 1.);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn shorthands_stay_sequences() {
        let t = table(&[("--b", "1px solid transparent"), ("--n", "none")]);
        let r = t.resolve_all();
        match r.get("--b") {
            Some(Value::Seq(v)) => {
                assert_eq!(v[0], Value::Length(Length::Px(1.)));
                assert_eq!(v[1], Value::Keyword("solid".into()));
                assert!(matches!(v[2], Value::Color(c) if c.a == 0.));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(r.get("--n"), Some(&Value::Keyword("none".into())));
    }
}
