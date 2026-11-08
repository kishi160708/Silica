//! Nesting depth: the parser must survive pathological input. These are
//! the shapes a hostile or generated note can arrive with — deep quotes,
//! footnote-definition runs, indented lists, callouts — and the property
//! is that parsing finishes and yields something, never a stack overflow.

#[test]
fn deep_nesting_parses_without_overflow() {
    for src in [
        (0..2000)
            .map(|i| format!("{}> x\n", "> ".repeat(i)))
            .collect::<String>(),
        "[^x]: ".repeat(2000),
        (0..2000)
            .map(|i| format!("{}- a\n", " ".repeat(i)))
            .collect::<String>(),
        (0..2000)
            .map(|i| format!("{}> [!note]+ t\n", "> ".repeat(i % 9)))
            .collect::<String>(),
    ] {
        let doc = silica_markdown::parse(&src);
        assert!(!doc.blocks.is_empty());
    }
}

/// The guard that exists must not eat ordinary nesting: a quoted list
/// three deep is normal authoring, and every level still renders.
#[test]
fn ordinary_nesting_untouched() {
    let doc = silica_markdown::parse("> - one\n>   - two\n>     - three\n");
    let html = silica_markdown::html::render_html(&doc).to_string();
    assert!(html.contains("<blockquote>"), "{html}");
    assert!(html.contains("<li>one"), "{html}");
    assert!(html.contains("<li>three"), "{html}");
}
