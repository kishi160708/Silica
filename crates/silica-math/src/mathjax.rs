//! MathJax compatibility pass over TeX source.
//!
//! Obsidian hands formulas to MathJax; Silica hands them to RaTeX. RaTeX
//! covers KaTeX's macro set and MathJax's is a superset, so a few
//! constructs have no meaning for it and came back as raw red source:
//!
//! - `\require{ext}` — MathJax's extension loader; RaTeX bundles the
//!   useful extensions already (`\ce` from mhchem, `\cancel`, colour), so
//!   the directive is dropped and the body keeps working.
//! - `\label`, `\ref`, `\eqref`, `\pageref` — cross-references need the
//!   document's equation numbers, which a formula does not have. The label
//!   is dropped and a reference becomes `?`, which is what MathJax prints
//!   for a reference it cannot resolve.
//! - `\bbox`, `\class`, `\style`, `\cssId` — box/CSS wrappers around a
//!   group. The wrapper is dropped and the content kept.
//! - `\unicode{…}` — prints the code point.
//! - `\newcommand`, `\renewcommand`, `\providecommand`,
//!   `\DeclareMathOperator` — user macros, collected and expanded.
//!
//! Everything else is copied byte for byte, so the engine's own error still
//! names whatever it is that RaTeX genuinely cannot set.

use std::collections::HashMap;

/// A macro from `\newcommand`/`\DeclareMathOperator`, ready to expand.
struct Macro {
    /// Number of arguments the definition takes.
    params: usize,
    /// The default for the first argument when it is given as `[default]`.
    optional: Option<String>,
    /// The replacement text, with `#1`…`#n` for arguments.
    body: String,
}

/// Rewrites `tex` so RaTeX can set it: MathJax-only commands handled, user
/// macros expanded. An input with none of those comes back unchanged.
pub fn normalize(tex: &str) -> String {
    if !tex.contains('\\') {
        return tex.to_string();
    }
    let mut macros = HashMap::new();
    let collected = collect(tex, &mut macros);
    let expanded = if macros.is_empty() {
        collected
    } else {
        expand(&collected, &macros, 0)
    };
    rewrite(&expanded, 0)
}

/// The depth past which a recursive expansion is left alone. A macro that
/// expands into itself must not become a stack overflow; MathJax reports an
/// error for it and RaTeX will too, once the text stops growing.
const DEPTH_CAP: usize = 16;

/// Everything below `i` up to the end of a balanced `{…}` group: the group's
/// inner span and the index just past its `}`. `i` sits on the `{`.
fn group(src: &[u8], i: usize) -> Option<(usize, usize)> {
    if src.get(i) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut j = i;
    while j < src.len() {
        match src[j] {
            b'\\' => j += 1, // `\{` and `\}` do not nest
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i + 1, j));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// Like [`group`] but for `[…]`, which MathJax uses for optional arguments.
fn optional(src: &[u8], i: usize) -> Option<(usize, usize)> {
    if src.get(i) != Some(&b'[') {
        return None;
    }
    let (mut depth, mut j) = (0usize, i);
    while j < src.len() {
        match src[j] {
            b'\\' => j += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i + 1, j));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// The command at `i` (which sits on a `\`): its name without the backslash
/// and the index just past it. A single non-letter is a one-character
/// command (`\\`, `\{`).
fn command(src: &[u8], i: usize) -> Option<(String, usize)> {
    if src.get(i) != Some(&b'\\') {
        return None;
    }
    let mut j = i + 1;
    if j >= src.len() {
        return Some((String::new(), j));
    }
    if src[j].is_ascii_alphabetic() {
        while j < src.len() && src[j].is_ascii_alphabetic() {
            j += 1;
        }
    } else {
        j += 1;
    }
    Some((String::from_utf8_lossy(&src[i + 1..j]).into_owned(), j))
}

/// The character at byte `i` and the byte index just past it.
fn char_at(tex: &str, i: usize) -> (char, usize) {
    let c = tex[i..].chars().next().unwrap_or('\u{fffd}');
    (c, i + c.len_utf8())
}

fn skip_ws(src: &[u8], mut i: usize) -> usize {
    while i < src.len() && src[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Pulls `\newcommand`-family declarations out of `tex`, recording them in
/// `macros`, and returns the remaining text.
fn collect(tex: &str, macros: &mut HashMap<String, Macro>) -> String {
    let src = tex.as_bytes();
    let mut out = String::with_capacity(tex.len());
    let mut i = 0usize;
    while i < src.len() {
        if src[i] != b'\\' {
            let (c, end) = char_at(tex, i);
            out.push(c);
            i = end;
            continue;
        }
        let Some((name, j)) = command(src, i) else {
            out.push('\\');
            i += 1;
            continue;
        };
        match name.as_str() {
            "newcommand" | "renewcommand" | "providecommand" => {
                match define(src, j) {
                    Some((end, def, mname)) => {
                        macros.insert(mname, def);
                        i = end;
                    }
                    None => {
                        out.push_str(&tex[i..j]);
                        i = j;
                    }
                }
            }
            "DeclareMathOperator" => match declare_operator(src, j) {
                Some((end, def, mname)) => {
                    macros.insert(mname, def);
                    i = end;
                }
                None => {
                    out.push_str(&tex[i..j]);
                    i = j;
                }
            },
            _ => {
                // A group can carry a declaration too (`{\newcommand…}`);
                // recurse so it is still collected.
                if let Some((inner, end)) = group(src, skip_ws(src, j)) {
                    let body = &tex[inner..end];
                    if body.contains("\\newcommand")
                        || body.contains("\\def")
                        || body.contains("\\require")
                    {
                        out.push_str(&tex[i..inner]);
                        out.push_str(&collect(body, macros));
                        out.push('}');
                        i = end + 1;
                        continue;
                    }
                }
                out.push_str(&tex[i..j]);
                i = j;
            }
        }
    }
    out
}

/// Reads a `\newcommand`-family definition from just past the command name:
/// `{\name}[params][default]{body}`. Returns the end index, the macro and
/// its name.
fn define(src: &[u8], i: usize) -> Option<(usize, Macro, String)> {
    let mut i = skip_ws(src, i);
    // The name is either a braced `{\name}` or a bare `\name`.
    let name = if src.get(i) == Some(&b'{') {
        let (a, b) = group(src, i)?;
        i = b + 1;
        String::from_utf8_lossy(&src[a..b]).into_owned()
    } else {
        let (n, j) = command(src, i)?;
        i = j;
        format!("\\{n}")
    };
    let name = name.trim().trim_start_matches('\\').to_string();
    if name.is_empty() {
        return None;
    }
    i = skip_ws(src, i);
    let mut params = 0usize;
    let mut optional_arg = None;
    // `\newcommand{\n}[2][default]{body}`: brackets, not braces.
    if let Some((a, b)) = optional(src, i) {
        params = String::from_utf8_lossy(&src[a..b]).trim().parse().unwrap_or(0);
        i = skip_ws(src, b + 1);
        if let Some((a, b)) = optional(src, i) {
            optional_arg = Some(String::from_utf8_lossy(&src[a..b]).into_owned());
            i = skip_ws(src, b + 1);
        }
    }
    let (a, b) = group(src, i)?;
    let body = String::from_utf8_lossy(&src[a..b]).into_owned();
    Some((
        b + 1,
        Macro {
            params,
            optional: optional_arg,
            body,
        },
        name,
    ))
}

/// `\DeclareMathOperator{\foo}{text}` is `\newcommand{\foo}{\operatorname{text}}`.
fn declare_operator(src: &[u8], i: usize) -> Option<(usize, Macro, String)> {
    let mut i = skip_ws(src, i);
    if src.get(i) == Some(&b'*') {
        i = skip_ws(src, i + 1);
    }
    let (a, b) = group(src, i)?;
    let name = String::from_utf8_lossy(&src[a..b])
        .trim()
        .trim_start_matches('\\')
        .to_string();
    let i = skip_ws(src, b + 1);
    let (c, d) = group(src, i)?;
    let text = String::from_utf8_lossy(&src[c..d]).into_owned();
    Some((
        d + 1,
        Macro {
            params: 0,
            optional: None,
            body: format!("\\operatorname{{{text}}}"),
        },
        name,
    ))
}

/// Replaces `\name` uses with their recorded bodies, up to [`DEPTH_CAP`].
fn expand(tex: &str, macros: &HashMap<String, Macro>, depth: usize) -> String {
    if depth > DEPTH_CAP {
        return tex.to_string();
    }
    let src = tex.as_bytes();
    let mut out = String::with_capacity(tex.len());
    let mut i = 0usize;
    while i < src.len() {
        match src[i] {
            b'{' => {
                if let Some((a, b)) = group(src, i) {
                    out.push('{');
                    out.push_str(&expand(&tex[a..b], macros, depth));
                    out.push('}');
                    i = b + 1;
                } else {
                    out.push('{');
                    i += 1;
                }
            }
            b'\\' => {
                let Some((name, j)) = command(src, i) else {
                    out.push('\\');
                    i += 1;
                    continue;
                };
                match macros.get(&name) {
                    Some(def) => {
                        if let Some((end, call)) = call(src, j, def) {
                            out.push_str(&expand(&call, macros, depth + 1));
                            i = end;
                        } else {
                            out.push_str(&tex[i..j]);
                            i = j;
                        }
                    }
                    None => {
                        out.push_str(&tex[i..j]);
                        i = j;
                    }
                }
            }
            _ => {
                let (c, end) = char_at(tex, i);
                out.push(c);
                i = end;
            }
        }
    }
    out
}

/// Reads a macro's arguments at `i` and returns the end index and the body
/// with `#1`…`#n` substituted.
fn call(src: &[u8], i: usize, def: &Macro) -> Option<(usize, String)> {
    let mut args: Vec<String> = Vec::new();
    let mut i = skip_ws(src, i);
    if let Some(default) = &def.optional {
        // An optional first argument in `[…]`, or its default.
        if let Some((a, b)) = optional(src, i) {
            args.push(String::from_utf8_lossy(&src[a..b]).into_owned());
            i = skip_ws(src, b + 1);
        } else {
            args.push(default.clone());
        }
    }
    while args.len() < def.params {
        i = skip_ws(src, i);
        let (a, b) = group(src, i)?;
        args.push(String::from_utf8_lossy(&src[a..b]).into_owned());
        i = b + 1;
    }
    let body = substitute(&def.body, &args);
    Some((i, body))
}

/// Substitutes `#n` in `body` with `args[n - 1]`, `##` becoming `#`.
fn substitute(body: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '#' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('#') => {
                out.push('#');
                chars.next();
            }
            Some(d) if d.is_ascii_digit() => {
                chars.next();
                let n = d.to_digit(10).unwrap_or(0) as usize;
                if n >= 1 && n <= args.len() {
                    out.push_str(&args[n - 1]);
                }
            }
            _ => out.push('#'),
        }
    }
    out
}

/// Handles the MathJax-only wrappers: `\require`, `\label`/`\ref`,
/// `\bbox`, `\class`/`\style`/`\cssId`, `\unicode`.
fn rewrite(tex: &str, depth: usize) -> String {
    if depth > DEPTH_CAP {
        return tex.to_string();
    }
    let src = tex.as_bytes();
    let mut out = String::with_capacity(tex.len());
    let mut i = 0usize;
    while i < src.len() {
        match src[i] {
            b'\\' => {
                let Some((name, j)) = command(src, i) else {
                    out.push('\\');
                    i += 1;
                    continue;
                };
                let after = skip_ws(src, j);
                match name.as_str() {
                    // The extension loader: RaTeX has the extensions it can
                    // use, so the directive goes.
                    "require" => {
                        let end = group(src, after)
                            .map(|(_, b)| b + 1)
                            .or_else(|| optional(src, after).map(|(_, b)| b + 1))
                            .or_else(|| command(src, after).map(|(_, j)| j))
                            .unwrap_or(j);
                        i = end;
                    }
                    "label" => {
                        i = group(src, after).map(|(_, b)| b + 1).unwrap_or(j);
                    }
                    "ref" | "pageref" => {
                        out.push_str("\\text{?}");
                        i = group(src, after).map(|(_, b)| b + 1).unwrap_or(j);
                    }
                    "eqref" => {
                        out.push_str("(\\text{?})");
                        i = group(src, after).map(|(_, b)| b + 1).unwrap_or(j);
                    }
                    // A box or CSS hook around a group: keep the group.
                    "bbox" => {
                        let g = optional(src, after)
                            .map(|(_, b)| skip_ws(src, b + 1))
                            .unwrap_or(after);
                        match group(src, g) {
                            Some((a, b)) => {
                                out.push('{');
                                out.push_str(&rewrite(&tex[a..b], depth + 1));
                                out.push('}');
                                i = b + 1;
                            }
                            None => i = j,
                        }
                    }
                    "class" | "style" | "cssId" => {
                        match group(src, after).and_then(|(_, b)| group(src, skip_ws(src, b + 1))) {
                            Some((a, b)) => {
                                out.push('{');
                                out.push_str(&rewrite(&tex[a..b], depth + 1));
                                out.push('}');
                                i = b + 1;
                            }
                            None => i = j,
                        }
                    }
                    "unicode" => match group(src, after) {
                        Some((a, b)) => {
                            if let Some(c) = code_point(&tex[a..b]) {
                                out.push_str(&rewrite(&c.to_string(), depth + 1));
                            }
                            i = b + 1;
                        }
                        None => i = j,
                    },
                    _ => {
                        out.push_str(&tex[i..j]);
                        i = j;
                    }
                }
            }
            b'{' => {
                if let Some((a, b)) = group(src, i) {
                    out.push('{');
                    out.push_str(&rewrite(&tex[a..b], depth + 1));
                    out.push('}');
                    i = b + 1;
                } else {
                    out.push('{');
                    i += 1;
                }
            }
            _ => {
                let (c, end) = char_at(tex, i);
                out.push(c);
                i = end;
            }
        }
    }
    out
}

/// The code point of a `\unicode{…}` argument: `x03c0`, `0x03c0` and `960`
/// all mean U+03C0.
fn code_point(arg: &str) -> Option<char> {
    let s = arg.trim();
    let n = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()?
    } else if let Some(hex) = s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
        u32::from_str_radix(hex, 16).ok()?
    } else if s.chars().all(|c| c.is_ascii_hexdigit()) && s.chars().any(|c| c.is_ascii_alphabetic()) {
        u32::from_str_radix(s, 16).ok()?
    } else {
        s.parse().ok()?
    };
    char::from_u32(n)
}

#[cfg(test)]
mod tests {
    use super::{code_point, normalize};
    use crate::typeset;

    /// The MathJax-only constructs RaTeX could not set on its own must now
    /// typeset. Each was checked against a raw engine call first.
    #[test]
    fn mathjax_only_commands_typeset() {
        for tex in [
            "\\require{mhchem}",
            "\\require{}",
            "\\require{mhchem}\\ce{H2O}",
            "\\label{eq}",
            "\\eqref{eq}",
            "\\ref{eq}",
            "\\bbox{x}",
            "\\bbox[5px,red]{x}",
            "\\class{foo}{x}",
            "\\style{color:red}{x}",
            "\\cssId{id}{x}",
            "\\unicode{x03c0}",
            "\\newcommand{\\R}{\\mathbb{R}}\\R",
            "\\newcommand{\\vv}[1]{\\vec{#1}}\\vv{v}",
            "\\renewcommand{\\R}{\\mathbb{R}}\\R",
            "\\DeclareMathOperator{\\Tr}{Tr}\\Tr",
            "x^2+\\frac{1}{2}",
        ] {
            assert!(typeset(tex, true).is_ok(), "{tex} did not typeset");
        }
    }

    /// A wrapper keeps its content: the drawing list is not empty.
    #[test]
    fn wrapper_keeps_content() {
        for tex in ["\\bbox[1px]{x}", "\\class{a}{y}", "\\unicode{x03c0}"] {
            let f = typeset(tex, false).expect("typesets");
            assert!(!f.items.is_empty(), "{tex} drew nothing");
        }
    }

    /// Macro arguments are substituted, and only declared macros expand.
    #[test]
    fn macros_expand() {
        assert_eq!(normalize("\\newcommand{\\a}{b}\\a"), "b");
        assert_eq!(normalize("\\newcommand{\\sq}[1]{#1^2}\\sq{x}"), "x^2");
        assert_eq!(normalize("\\newcommand{\\t}[2]{#2+#1}\\t{a}{b}"), "b+a");
        // `##` is a literal `#`; `#3` with only two args is left alone.
        assert_eq!(normalize("\\newcommand{\\h}{\\#}"), "");
        assert_eq!(normalize("\\DeclareMathOperator{\\Tr}{Tr}\\Tr"), "\\operatorname{Tr}");
    }

    /// A reference prints `?`, as MathJax does when it cannot resolve one.
    #[test]
    fn references_become_question_marks() {
        assert_eq!(normalize("\\ref{a}"), "\\text{?}");
        assert_eq!(normalize("\\eqref{a}"), "(\\text{?})");
    }

    /// Anything the pass does not know is copied byte for byte, so the
    /// engine's own error still names the real culprit.
    #[test]
    fn unknown_source_is_untouched() {
        for tex in ["x^2 + \\frac{1}{2}", "\\begin{matrix}a&b\\end{matrix}", "\\alpha\\beta"] {
            assert_eq!(normalize(tex), tex);
        }
    }

    #[test]
    fn unicode_forms() {
        assert_eq!(code_point("x03c0"), Some('π'));
        assert_eq!(code_point("960"), Some('π'));
        assert_eq!(code_point("0x3C0"), Some('π'));
        assert_eq!(code_point("nope"), None);
    }
}
