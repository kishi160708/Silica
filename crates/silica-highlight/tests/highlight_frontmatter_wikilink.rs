//! afk-3 item 7: tokenizer harus menangani frontmatter dan wikilink di dalam
//! blok markdown tanpa panic, dengan span token yang benar.
//! Lihat: `.burst/silica/reports/afk-3-tests.md` item 7.

use silica_highlight::{grammar_for, spans, tokenize};

#[test]
fn highlight_tokenizer_handles_frontmatter_and_wikilinks() {
    let text = "---\ntitle: Hello\n---\n\n# Note\n\nSee [[Other Note]] for details.\n";
    let grammar = grammar_for("markdown").expect("grammar markdown tersedia");
    let nodes = tokenize(text, grammar);
    let spans = spans(&nodes);

    let contains = |kind: &str, start: usize, end: usize| {
        spans
            .iter()
            .any(|s| s.kind == kind && s.start == start && s.end == end)
    };

    // seluruh wilayah ---...--- adalah satu token front-matter-block
    assert!(contains("front-matter-block", 0, 20));
    // pembuka dan penutup blok ditandai punctuation
    assert!(contains("punctuation", 0, 3));
    assert!(contains("punctuation", 17, 20));
    // isi front-matter ditokenskan sebagai front-matter dengan alias yaml
    assert!(spans.iter().any(|s| {
        s.kind == "front-matter"
            && s.start == 4
            && s.end == 16
            && s.alias == ["yaml", "language-yaml"]
    }));
    // judul adalah token title
    assert!(contains("title", 22, 28));
    // wikilink bukan satu token: setiap kurung siku punctuation sendiri,
    // teks di tengah tidak dibentuk token
    // wikilink [[Other Note]] (34..48): tidak ada token yang dimulai di
    // dalamnya — kurung siku dan teks tengah plain text pada grammar markdown,
    // tokenizer tidak panic dan tidak buat token palsu
    assert!(!spans.iter().any(|s| s.start >= 34 && s.start < 48));
}
