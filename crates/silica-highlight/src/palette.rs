//! Which `--code-*` token a run is painted from.
//!
//! Obsidian runs two highlighters over one palette: Prism in the reading
//! view (`span.token.keyword`) and CodeMirror in the editor (`.cm-keyword`),
//! and both resolve to the same eleven `--code-*` custom properties the
//! theme registry already carries.
//!
//! The table below is Obsidian 1.14.5's, read by
//! `tools/obsidian-probe/code-highlight.mjs` (step s10) on 2026-11-02. Its
//! `.token` rules are not reachable through `document.styleSheets` — a dump
//! found 184 CodeMirror rules and none of Prism's, with 39 token spans on
//! screen — so the question was put to the renderer instead: a span of each
//! candidate class was placed inside a real rendered block and its computed
//! colour read back and matched to the variable that holds it. **Thirty-three
//! classes are styled; every other class inherits `--code-normal`**, which is
//! what the block's `code` element carries. The record is
//! `results/2026-11-02-code-highlight.json`.
//!
//! Silica paints the editor from this same table. Obsidian's two views do
//! not agree with each other — a JSON key is `--code-property` in the
//! reading view and `--code-function` in the editor, and a Python decorator
//! is punctuation in one and `--code-comment` in the other — because they
//! run different tokenizers. One tokenizer is the whole point of this
//! crate, so Silica's two views agree, and both match Obsidian's rendered
//! block.

/// The `--code-*` token a run is painted from. A name in the theme
/// registry, not a colour: the theme decides what each one is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Paint {
    /// `--code-normal`, which the block carries and an unstyled class keeps.
    Normal,
    Comment,
    Keyword,
    String,
    Function,
    Value,
    Operator,
    Punctuation,
    Property,
    Tag,
    Important,
}

impl Paint {
    /// The custom property this is, for the doc comments and the tests.
    pub fn property(self) -> &'static str {
        match self {
            Paint::Normal => "--code-normal",
            Paint::Comment => "--code-comment",
            Paint::Keyword => "--code-keyword",
            Paint::String => "--code-string",
            Paint::Function => "--code-function",
            Paint::Value => "--code-value",
            Paint::Operator => "--code-operator",
            Paint::Punctuation => "--code-punctuation",
            Paint::Property => "--code-property",
            Paint::Tag => "--code-tag",
            Paint::Important => "--code-important",
        }
    }
}

/// How a run is painted: a palette entry, and whatever emphasis goes with
/// it. Obsidian gives weight 700 to `important` and `bold`, and italics to
/// `italic`; nothing else is anything but upright and regular.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub paint: Paint,
    pub bold: bool,
    pub italic: bool,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            paint: Paint::Normal,
            bold: false,
            italic: false,
        }
    }
}

/// The thirty-one classes Obsidian gives a colour to.
//
// `bold` and `italic` are not here: they set weight and style only, and
// leave the colour the block's own.
const COLOURED: &[(&str, Paint)] = &[
    ("comment", Paint::Comment),
    ("prolog", Paint::Comment),
    ("doctype", Paint::Comment),
    ("cdata", Paint::Comment),
    ("punctuation", Paint::Punctuation),
    ("property", Paint::Property),
    ("variable", Paint::Property),
    ("parameter", Paint::Property),
    ("entity", Paint::Property),
    ("url", Paint::Property),
    ("tag", Paint::Tag),
    ("constant", Paint::Tag),
    ("deleted", Paint::Tag),
    // Indistinguishable from `--code-tag` by colour — the two properties
    // hold the same value in both base themes — but s06 recorded
    // `.token.symbol`'s neighbours in the same rule, so it is grouped here.
    ("symbol", Paint::Tag),
    ("boolean", Paint::Value),
    ("number", Paint::Value),
    ("selector", Paint::String),
    ("attr-name", Paint::String),
    ("string", Paint::String),
    ("char", Paint::String),
    ("inserted", Paint::String),
    // `--code-operator` equals `--code-tag` in both base themes, so only
    // the declaration told the two apart: s06 saw `.token.operator` win
    // with `var(--code-operator)`.
    ("operator", Paint::Operator),
    ("atrule", Paint::Function),
    ("attr-value", Paint::Function),
    ("function", Paint::Function),
    ("class-name", Paint::Function),
    ("builtin", Paint::Function),
    ("property-access", Paint::Function),
    ("keyword", Paint::Keyword),
    ("regex", Paint::Important),
    ("important", Paint::Important),
];

/// How Obsidian paints a token with this type and these aliases.
///
/// A span carries its type then its aliases, and every rule is a single
/// class of equal specificity, so document order decides a conflict. The
/// one conflict the probe saw is `important`, which beats the colour of
/// whatever it is an alias of (`boolean important` is the important
/// orange, not the value purple), so it is tried first; no other pair of
/// coloured classes was ever observed on one span.
pub fn style_of(kind: &str, alias: &[&str]) -> Style {
    let names = std::iter::once(kind).chain(alias.iter().copied());
    let mut style = Style::default();
    let mut found = false;
    for name in names {
        match name {
            "important" => {
                style.paint = Paint::Important;
                style.bold = true;
                found = true;
            }
            "bold" => style.bold = true,
            "italic" => style.italic = true,
            _ if !found => {
                if let Some((_, paint)) = COLOURED.iter().find(|(n, _)| *n == name) {
                    style.paint = *paint;
                    found = true;
                }
            }
            _ => {}
        }
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every class step s10 found a style for, with the property it
    /// resolved to. A disagreement here is a disagreement with Obsidian.
    #[test]
    fn every_styled_class_paints_as_obsidian_paints_it() {
        let cases: &[(&str, Paint)] = &[
            ("atrule", Paint::Function),
            ("attr-name", Paint::String),
            ("attr-value", Paint::Function),
            ("boolean", Paint::Value),
            ("builtin", Paint::Function),
            ("cdata", Paint::Comment),
            ("char", Paint::String),
            ("class-name", Paint::Function),
            ("comment", Paint::Comment),
            ("constant", Paint::Tag),
            ("deleted", Paint::Tag),
            ("doctype", Paint::Comment),
            ("entity", Paint::Property),
            ("function", Paint::Function),
            ("important", Paint::Important),
            ("inserted", Paint::String),
            ("keyword", Paint::Keyword),
            ("number", Paint::Value),
            ("operator", Paint::Operator),
            ("parameter", Paint::Property),
            ("prolog", Paint::Comment),
            ("property", Paint::Property),
            ("property-access", Paint::Function),
            ("punctuation", Paint::Punctuation),
            ("regex", Paint::Important),
            ("selector", Paint::String),
            ("string", Paint::String),
            ("symbol", Paint::Tag),
            ("tag", Paint::Tag),
            ("url", Paint::Property),
            ("variable", Paint::Property),
        ];
        for (name, want) in cases {
            assert_eq!(style_of(name, &[]).paint, *want, "token {name}");
        }
        // `bold` and `italic` carry weight and slant, not colour.
        assert_eq!(style_of("bold", &[]).paint, Paint::Normal);
        assert!(style_of("bold", &[]).bold);
        assert_eq!(style_of("italic", &[]).paint, Paint::Normal);
        assert!(style_of("italic", &[]).italic);
    }

    /// Every class s10 found *no* style for keeps the block's own colour.
    #[test]
    fn an_unstyled_class_keeps_the_block_s_colour() {
        for name in [
            "namespace",
            "prefix",
            "unchanged",
            "coord",
            "interpolation",
            "template-string",
            "maybe-class-name",
            "key",
            "macro",
            "arrow",
            "annotation",
            "decorator",
            "directive",
            "control-flow",
            "module",
            "expression",
            "rule",
            "combinator",
            "pseudo-class",
            "pseudo-element",
            "unit",
            "hexcode",
            "color",
            "line",
            "name",
            "script",
            "style",
            "doctype-tag",
            "attr-equals",
            "triple-quoted-string",
            "shebang",
            "title",
            "list",
            "table",
            "null",
            "nil",
        ] {
            let got = style_of(name, &[]);
            assert_eq!(got.paint, Paint::Normal, "token {name}");
            assert!(!got.bold && !got.italic, "token {name}");
        }
    }

    /// The combinations the probe recorded on real spans, each with the
    /// `--code-*` Obsidian painted it.
    #[test]
    fn the_recorded_combinations_resolve_as_recorded() {
        let cases: &[(&str, &[&str], Paint)] = &[
            // `important` beats the type it is an alias of.
            ("boolean", &["important"], Paint::Important),
            ("null", &["important"], Paint::Important),
            ("shebang", &["important"], Paint::Important),
            ("title", &["important"], Paint::Important),
            // An unstyled type with a styled alias takes the alias.
            ("annotation", &["punctuation"], Paint::Punctuation),
            ("key", &["property"], Paint::Property),
            ("key", &["atrule"], Paint::Function),
            ("macro", &["property"], Paint::Property),
            ("parameter", &["variable"], Paint::Property),
            ("entity", &["named-entity"], Paint::Property),
            ("prefix", &["deleted"], Paint::Tag),
            ("prefix", &["inserted"], Paint::String),
            ("prefix", &["unchanged"], Paint::Normal),
            ("arrow", &["operator"], Paint::Operator),
            ("directive", &["keyword"], Paint::Keyword),
            ("keyword", &["control-flow"], Paint::Keyword),
            ("null", &["keyword"], Paint::Keyword),
            ("builtin", &["class-name"], Paint::Function),
            ("table", &["class-name"], Paint::Function),
            ("triple-quoted-string", &["string"], Paint::String),
            ("template-punctuation", &["string"], Paint::String),
            ("deleted-sign", &["deleted"], Paint::Tag),
            ("assign-left", &["variable"], Paint::Property),
            ("function-definition", &["function"], Paint::Function),
            ("type-definition", &["class-name"], Paint::Function),
            ("attribute", &["attr-name"], Paint::String),
            (
                "interpolation-punctuation",
                &["punctuation"],
                Paint::Punctuation,
            ),
            ("double-colon", &["punctuation"], Paint::Punctuation),
            (
                "decorator",
                &["annotation", "punctuation"],
                Paint::Punctuation,
            ),
            ("for-or-select", &["variable"], Paint::Property),
        ];
        for (kind, alias, want) in cases {
            assert_eq!(style_of(kind, alias).paint, *want, "token {kind} {alias:?}");
        }
        assert!(style_of("boolean", &["important"]).bold);
        // `regex` is the important colour without the weight.
        assert_eq!(style_of("regex", &[]).paint, Paint::Important);
        assert!(!style_of("regex", &[]).bold);
    }
}
