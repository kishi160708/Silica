//! The loader's half of a stylesheet: custom-property declarations under
//! `:root`, `html`, `body`, `.theme-dark` and `.theme-light`, and the
//! cascade that picks one value per property for the body of a page in
//! one base theme.
//!
//! This is a property scanner, not a CSS engine (`docs/silica/Theming.md`,
//! "The loader"). Every rule whose selector is anything else — a component,
//! a descendant, a pseudo-element — is skipped without being understood,
//! and so is every declaration whose property does not begin `--`. What is
//! kept is exactly what a theme can change in Silica: the value of a token.
//! Refusing to grow a cascade beyond these five selectors is what keeps it
//! small, and that refusal is deliberate.
//!
//! The cascade is the one CSS applies to `<body class="theme-dark">`: a
//! declaration on the body element (`body`, `.theme-dark`, `body.theme-dark`)
//! beats one it would inherit from `html` or `:root`; among declarations on
//! the same element `!important` wins, then specificity, then order. That
//! is why a theme declares `.theme-dark { --color-base-00: … }` rather than
//! `body { … }`: the app's own `.theme-dark` rule would otherwise outrank
//! it. Obsidian's defaults enter the same cascade first, so a theme's
//! rules stand in the order they do there.
#![allow(dead_code)]

use crate::theme::value::{parse, Expr, Table};

/// Which element a rule declares on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `:root` or `html`: inherited by the body.
    Root,
    /// `body`, `.theme-dark`, `.theme-light` and their compounds.
    Body,
}

/// `true` when a `Themes` variant applies to the given `dark` setting.
fn scope_applies(themes: &Themes, dark: bool) -> bool {
    match themes {
        Themes::Both => true,
        Themes::Dark => dark,
        Themes::Light => !dark,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Themes {
    Both,
    Dark,
    Light,
}

/// One parsed `--name: value` declaration, with where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Expr,
    /// The declaration's source text, kept so a loader can record what the
    /// stylesheet wrote rather than what it resolves to.
    pub raw: Option<String>,
    pub scope: Scope,
    pub themes: Themes,
    /// `(ids, classes and pseudo-classes, elements)`.
    pub specificity: (u8, u8, u8),
    pub important: bool,
    /// Position in the sheets read so far: later wins among equals.
    pub order: usize,
}

impl Declaration {
    fn applies(&self, dark: bool) -> bool {
        match self.themes {
            Themes::Both => true,
            Themes::Dark => dark,
            Themes::Light => !dark,
        }
    }

    /// The cascade's ordering on one element.
    fn rank(&self) -> (bool, bool, (u8, u8, u8), usize) {
        (
            self.important,
            self.scope == Scope::Body,
            self.specificity,
            self.order,
        )
    }
}

/// Classify a selector list: the scope, theme and specificity of its most
/// specific part that the scanner understands, or `None` when no part is a
/// compound of `html`, `body`, `:root`, `.theme-dark` and `.theme-light`.
pub fn classify(selector: &str) -> Option<(Scope, Themes, (u8, u8, u8))> {
    let mut best: Option<(Scope, Themes, (u8, u8, u8))> = None;
    for part in selector.split(',') {
        let Some(c) = classify_compound(part.trim()) else {
            continue;
        };
        if best.is_none_or(|b| c.2 > b.2) {
            best = Some(c);
        }
    }
    best
}

fn classify_compound(part: &str) -> Option<(Scope, Themes, (u8, u8, u8))> {
    if part.is_empty()
        || part.contains(char::is_whitespace)
        || part.contains(['>', '+', '~', '[', '(', '#'])
    {
        return None;
    }
    let mut body = false;
    let mut theme = Themes::Both;
    let mut classes = 0u8;
    let mut elements = 0u8;

    for token in part.split('.') {
        if token.is_empty() {
            continue;
        }
        match token {
            "html" => {
                elements += 1;
            }
            "body" => {
                body = true;
                elements += 1;
            }
            ":root" => {}
            "theme-dark" => {
                classes += 1;
                theme = Themes::Dark;
            }
            "theme-light" => {
                classes += 1;
                theme = Themes::Light;
            }
            _ => classes += 1,
        }
    }

    if body || theme != Themes::Both {
        let scope = Scope::Body;
        let spec = (0, classes, elements);
        Some((scope, theme, spec))
    } else if part.contains(":root") || part.contains("html") {
        let scope = Scope::Root;
        let spec = (0, classes, elements);
        Some((scope, theme, spec))
    } else {
        None
    }
}

/// A stylesheet's declarations, grouped by the selector's scope and theme.
#[derive(Debug, Default, Clone)]
pub struct Sheet {
    decls: Vec<Declaration>,
    order: usize,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet::default()
    }

    /// Parse one CSS source, appending every `--name: value` declaration it
    /// finds under `:root`, `html`, `body`, `.theme-dark` or `.theme-light`.
    ///
    /// `dark` selects which theme-variant declarations apply: `true` keeps
    /// `Both` and `Dark`, `false` keeps `Both` and `Light`.
    pub fn parse_css(&mut self, css: &str, dark: bool) {
        // * Strip block comments, which can span lines.
        let css = strip_comments(css);
        // * Strip `@import` and `@charset` rules, which are not custom
        //   properties.
        let css = strip_at_rules(&css);
        let mut pos = 0;
        let bytes = css.as_bytes();
        while pos < bytes.len() {
            // * Find the next selector block: `selector { ... }`.
            let Some(brace) = css[pos..].find('{') else {
                break;
            };
            let brace = pos + brace;
            let selector = css[pos..brace].trim();
            let Some(close) = find_matching_brace(&css[brace..]) else {
                break;
            };
            let body = &css[brace + 1..brace + close];
            pos = brace + close + 1;

            let Some((scope, themes, spec)) = classify(selector) else {
                continue;
            };
            if !scope_applies(&themes, dark) {
                continue;
            }
            for decl in parse_declarations(body) {
                if !decl.name.starts_with("--") {
                    continue;
                }
                let decl = Declaration {
                    name: decl.name,
                    value: parse(&decl.value),
                    raw: Some(decl.value),
                    scope,
                    themes,
                    specificity: spec,
                    important: decl.important,
                    order: self.order,
                };
                self.decls.push(decl);
                self.order += 1;
            }
        }
    }

    /// Resolve every declaration into a table of `--name: Value`, applying the
    /// CSS cascade: `!important` wins, then body scope, then specificity, then
    /// later order.
    pub fn resolve(&self) -> Table {
        let mut table = Table::new();
        // * Group declarations by name, then pick the highest-ranked one.
        let mut by_name: std::collections::HashMap<String, Vec<&Declaration>> =
            std::collections::HashMap::new();
        for d in &self.decls {
            by_name.entry(d.name.clone()).or_default().push(d);
        }
        for (name, decls) in by_name {
            let best = decls.iter().max_by_key(|d| d.rank()).unwrap();
            table.set(&name, best.value.clone());
        }
        table
    }

    /// The winning declaration of every property, as its source text.
    ///
    /// A theme loader records what a stylesheet *declares*, not what it
    /// resolves to: the resolved values come from the defaults those
    /// declarations stand on, and writing them back would make every theme
    /// look like it overrode every token.
    pub fn declarations(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        let mut best: std::collections::HashMap<&str, &Declaration> =
            std::collections::HashMap::new();
        for d in &self.decls {
            match best.get(d.name.as_str()) {
                Some(held) if held.rank() >= d.rank() => {}
                _ => {
                    best.insert(&d.name, d);
                }
            }
        }
        best.into_iter().map(|(name, d)| (name, d.raw.as_deref()))
    }
}

/// Strip `/* … */` comments, which may span lines.
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.char_indices().peekable();
    while let Some((_i, c)) = chars.next() {
        if c == '/' && chars.peek().is_some_and(|(_, c2)| *c2 == '*') {
            chars.next(); // consume '*'
            let mut closed = false;
            while let Some((_, c2)) = chars.next() {
                if c2 == '*' && chars.peek().is_some_and(|(_, c3)| *c3 == '/') {
                    chars.next(); // consume '/'
                    closed = true;
                    break;
                }
            }
            if !closed {
                break;
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip `@import …;` and `@charset …;` rules.
fn strip_at_rules(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut pos = 0;
    while pos < css.len() {
        let Some(at) = css[pos..].find('@') else {
            out.push_str(&css[pos..]);
            break;
        };
        out.push_str(&css[pos..pos + at]);
        let rest = &css[pos + at..];
        let Some(semi) = rest.find(';') else {
            break;
        };
        pos = pos + at + semi + 1;
    }
    out
}

/// Find the matching closing brace, honouring nested braces and strings.
/// `css` starts with `{`; returns the index of the matching `}`.
fn find_matching_brace(css: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut quote = '\0';
    let mut escaped = false;
    for (i, c) in css.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            if c == '\\' {
                escaped = true;
            } else if c == quote {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                in_string = true;
                quote = c;
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Debug, Clone)]
struct RawDecl {
    name: String,
    value: String,
    important: bool,
}

/// Parse `--name: value;` declarations from a rule body. Each declaration
/// ends at a semicolon or the closing brace.
fn parse_declarations(body: &str) -> Vec<RawDecl> {
    let mut out = Vec::new();
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((name, value)) = part.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let mut value = value.trim();
        let important = value.ends_with("!important");
        if important {
            value = value.trim_end_matches("!important").trim();
        }
        if name.starts_with("--") && !value.is_empty() {
            out.push(RawDecl {
                name: name.to_string(),
                value: value.to_string(),
                important,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::theme::value;

    use super::*;

    fn sheet(css: &str, dark: bool) -> Sheet {
        let mut s = Sheet::new();
        s.parse_css(css, dark);
        s
    }

    #[test]
    fn root_declaration_is_kept() {
        let s = sheet(":root { --color-base-00: #1c1c1c; }", true);
        let t = s.resolve();
        match t.resolve_all().get("--color-base-00") {
            Some(value::Value::Color(c)) => assert_eq!(value::hex(*c), "#1c1c1c"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn theme_dark_declaration_applies_only_in_dark() {
        let s = sheet(".theme-dark { --accent: #ff0000; }", true);
        let t = s.resolve();
        assert!(matches!(
            t.resolve_all().get("--accent"),
            Some(value::Value::Color(_))
        ));
        let s = sheet(".theme-dark { --accent: #ff0000; }", false);
        let t = s.resolve();
        assert_eq!(t.resolve_all().get("--accent"), None);
    }

    #[test]
    fn body_beats_root_at_equal_specificity() {
        let s = sheet(":root { --x: #111111; } body { --x: #222222; }", true);
        let t = s.resolve();
        match t.resolve_all().get("--x") {
            Some(value::Value::Color(c)) => assert_eq!(value::hex(*c), "#222222"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn important_wins() {
        let s = sheet(
            "body { --x: #111111 !important; } body { --x: #222222; }",
            true,
        );
        let t = s.resolve();
        match t.resolve_all().get("--x") {
            Some(value::Value::Color(c)) => assert_eq!(value::hex(*c), "#111111"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_selector_is_skipped() {
        let s = sheet(".some-component { --x: #ff0000; }", true);
        let t = s.resolve();
        assert_eq!(t.resolve_all().get("--x"), None);
    }

    #[test]
    fn nested_braces_are_honoured() {
        let s = sheet(":root { --x: #111111; .foo { --y: #222222; } }", true);
        let t = s.resolve();
        assert_eq!(
            t.resolve_all().get("--x"),
            Some(&value::Value::Color(gpui::rgb(0x111111).into()))
        );
        assert_eq!(t.resolve_all().get("--y"), None);
    }

    #[test]
    fn comments_are_stripped() {
        let s = sheet("/* comment */ :root { --x: #111111; } /* more */", true);
        let t = s.resolve();
        match t.resolve_all().get("--x") {
            Some(value::Value::Color(c)) => assert_eq!(value::hex(*c), "#111111"),
            other => panic!("{other:?}"),
        }
    }
}
