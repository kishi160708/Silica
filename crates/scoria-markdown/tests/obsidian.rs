//! Obsidian Flavored Markdown: the extensions on top of CommonMark, checked
//! both as rendered HTML and as tree shape with source spans.

use _markdown::{
    parse, render_html, render_html_with, Block, BlockKind, Fold, HtmlOptions, Inline, InlineKind,
    ListKind, Span,
};

fn html(src: &str) -> String {
    render_html(&parse(src))
}

fn plain_html(src: &str) -> String {
    render_html_with(&parse(src), &HtmlOptions::commonmark())
}

fn paragraph_inlines(src: &str) -> Vec<Inline<'_>> {
    let doc = parse(src);
    match doc.blocks.into_iter().next().map(|b| b.kind) {
        Some(BlockKind::Paragraph { content }) => content,
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

#[test]
fn wikilinks_and_embeds() {
    let src = "See [[Note#Head|alias]] and ![[pic.png|200]].";
    let inlines = paragraph_inlines(src);
    let links: Vec<_> = inlines
        .iter()
        .filter_map(|i| match &i.kind {
            InlineKind::Wikilink { body, embed } => Some((body.as_ref(), *embed, i.span)),
            _ => None,
        })
        .collect();
    assert_eq!(
        links,
        vec![
            ("Note#Head|alias", false, Span::new(4, 23)),
            ("pic.png|200", true, Span::new(28, 44)),
        ]
    );
    assert_eq!(&src[4..23], "[[Note#Head|alias]]");
    assert_eq!(&src[28..44], "![[pic.png|200]]");
    assert_eq!(
        html(src),
        "<p>See <a data-tooltip-position=\"top\" aria-label=\"Note#Head\" data-href=\"Note#Head\" href=\"Note#Head\" class=\"internal-link\" target=\"_blank\" rel=\"noopener nofollow\">alias</a> and <span alt=\"pic.png\" src=\"pic.png\" class=\"internal-embed\"></span>.</p>\n"
    );
}

#[test]
fn wikilinks_inside_code_and_links_without_alias() {
    assert_eq!(
        html("`[[not a link]]` and [[A#B]]"),
        "<p><code>[[not a link]]</code> and <a data-href=\"A#B\" href=\"A#B\" class=\"internal-link\" target=\"_blank\" rel=\"noopener nofollow\">A &gt; B</a></p>\n"
    );
    // Wikilinks are disabled in CommonMark mode and parse as bracketed text.
    assert_eq!(
        render_html_with(
            &_markdown::parse_with("[[x]]", &_markdown::Options::commonmark()),
            &HtmlOptions::commonmark()
        ),
        "<p>[[x]]</p>\n"
    );
}

#[test]
fn tags() {
    let inlines = paragraph_inlines("a #tag/nested, #1a but not #123 or x#y or # alone");
    let tags: Vec<_> = inlines
        .iter()
        .filter_map(|i| match &i.kind {
            InlineKind::Tag(name) => Some(*name),
            _ => None,
        })
        .collect();
    assert_eq!(tags, vec!["tag/nested", "1a"]);
    assert_eq!(
        html("#tag"),
        "<p><a href=\"#tag\" class=\"tag\" target=\"_blank\" rel=\"noopener nofollow\">#tag</a></p>\n"
    );
    // `#` followed by a space is a heading, not a tag.
    assert_eq!(html("# Title"), "<h1 data-heading=\"Title\">Title</h1>\n");
}

#[test]
fn block_ids() {
    let doc = parse("para one ^abc-1\n\n- item ^li\n\n| a |\n| - |\n| b |\n\n^tbl\n");
    let ids: Vec<_> = doc
        .blocks
        .iter()
        .map(|b| b.id.map(|i| (i.id, i.span)))
        .collect();
    assert_eq!(ids[0], Some(("abc-1", Span::new(9, 15))));
    assert_eq!(ids[1], None);
    assert_eq!(ids[2], Some(("tbl", Span::new(48, 52))));
    let BlockKind::List { items, .. } = &doc.blocks[1].kind else {
        panic!("list")
    };
    assert_eq!(items[0].children[0].id.map(|i| i.id), Some("li"));
    // The id is not part of the paragraph text.
    assert_eq!(html("para one ^abc-1"), "<p>para one</p>\n");
    assert_eq!(doc.blocks.len(), 3);
}

#[test]
fn callouts() {
    let src = "> [!tip]- Fold *me*\n> body line\n>\n> - list\n\n> [!WARNING]\n> only body\n\n> plain quote";
    let doc = parse(src);
    let BlockKind::Callout {
        callout_type,
        fold,
        title,
        children,
    } = &doc.blocks[0].kind
    else {
        panic!("callout, got {:?}", doc.blocks[0].kind)
    };
    assert_eq!(*callout_type, "tip");
    assert_eq!(*fold, Some(Fold::Collapsed));
    assert_eq!(_markdown::plain_text(title), "Fold me");
    assert_eq!(children.len(), 2);
    assert!(matches!(children[0].kind, BlockKind::Paragraph { .. }));
    assert!(matches!(children[1].kind, BlockKind::List { .. }));
    assert_eq!(doc.blocks[0].span, Span::new(0, 42));
    let BlockKind::Callout {
        callout_type,
        title,
        ..
    } = &doc.blocks[1].kind
    else {
        panic!("callout")
    };
    assert_eq!(*callout_type, "WARNING");
    assert!(title.is_empty());
    assert!(matches!(doc.blocks[2].kind, BlockKind::BlockQuote { .. }));
    assert_eq!(
        html("> [!note] Hi\n> there"),
        "<div data-callout-metadata=\"\" data-callout-fold=\"\" data-callout=\"note\" class=\"callout\"><div class=\"callout-title\"><div class=\"callout-title-inner\">Hi</div></div><div class=\"callout-content\">\n<p>there</p>\n</div></div>\n"
    );
    assert_eq!(
        html("> [!warning]"),
        "<div data-callout-metadata=\"\" data-callout-fold=\"\" data-callout=\"warning\" class=\"callout\"><div class=\"callout-title\"><div class=\"callout-title-inner\">Warning</div></div></div>\n"
    );
}

#[test]
fn highlight_strikethrough_and_math() {
    assert_eq!(
        plain_html("==hi== ~~gone~~ $x^2$ $$a+b$$ not $5 and $6"),
        "<p><mark>hi</mark> <del>gone</del> <span class=\"math math-inline\">x^2</span> <span class=\"math math-block\">a+b</span> not $5 and $6</p>\n"
    );
    assert_eq!(
        plain_html("$$\n\\int_0^1 x\n$$\n"),
        "<div class=\"math math-block\">\\int_0^1 x</div>\n"
    );
    let doc = parse("$$\nx\n$$");
    assert_eq!(doc.blocks[0].span, Span::new(0, 7));
    assert!(matches!(&doc.blocks[0].kind, BlockKind::Math { content } if content == "x\n"));
}

#[test]
fn comments() {
    assert_eq!(plain_html("a %%hidden%% b"), "<p>a  b</p>\n");
    let doc = parse("%%\nblock\ncomment\n%%\nafter");
    assert!(
        matches!(&doc.blocks[0].kind, BlockKind::Comment { content } if *content == "\nblock\ncomment\n")
    );
    assert_eq!(doc.blocks[0].span, Span::new(0, 19));
    assert!(matches!(doc.blocks[1].kind, BlockKind::Paragraph { .. }));
    assert_eq!(plain_html("%%\nblock\n%%\nafter"), "<p>after</p>\n");
}

#[test]
fn tables_with_escaped_pipes() {
    let src = "| a | b |\n| :- | -: |\n| [[x\\|y]] | `c\\|d` |\n";
    let doc = parse(src);
    let BlockKind::Table { header, rows, .. } = &doc.blocks[0].kind else {
        panic!("table")
    };
    assert_eq!(header.len(), 2);
    assert_eq!(rows.len(), 1);
    // The body keeps `\|`; the resolvable target drops the backslash.
    assert!(
        matches!(&rows[0][0].content[0].kind, InlineKind::Wikilink { body, .. } if body == r"x\|y")
    );
    assert_eq!(_markdown::link_target(r"x\|y"), "x");
    assert_eq!(_markdown::parse_linktext(r"x\|y").alias, Some("y"));
    assert!(matches!(&rows[0][1].content[0].kind, InlineKind::Code(c) if c == "c\\|d"));
    assert_eq!(rows[0][0].span, Span::new(24, 32));
    assert_eq!(&src[24..32], "[[x\\|y]]");
    assert_eq!(
        plain_html("| a |\n| - |\n| 1 |\n"),
        "<table>\n<thead>\n<tr>\n<th>a</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>1</td>\n</tr>\n</tbody>\n</table>\n"
    );
}

#[test]
fn task_lists() {
    let doc = parse("- [ ] open\n- [x] done\n- [?] custom\n- [ ]\n- plain");
    let BlockKind::List { items, kind, .. } = &doc.blocks[0].kind else {
        panic!("list")
    };
    assert_eq!(*kind, ListKind::Bullet { marker: b'-' });
    let tasks: Vec<_> = items.iter().map(|i| i.task).collect();
    // `- [ ]` with nothing after it is plain text in Obsidian.
    assert_eq!(tasks, vec![Some(' '), Some('x'), Some('?'), None, None]);
    assert_eq!(items[1].marker, Span::new(11, 12));
    assert_eq!(
        plain_html("- [x] done"),
        "<ul>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> done</li>\n</ul>\n"
    );
    assert_eq!(
        html("- [ ] a"),
        "<ul class=\"contains-task-list\">\n<li data-task=\"\" class=\"task-list-item\"><input type=\"checkbox\" class=\"task-list-item-checkbox\">a</li>\n</ul>\n"
    );
}

#[test]
fn footnotes() {
    let src = "Text[^1] and inline^[note here].\n\n[^1]: The *def*.\n";
    assert_eq!(
        plain_html(src),
        "<p>Text<sup data-footnote-id=\"fnref-1\" class=\"footnote-ref\" id=\"fnref-1\"><a data-footref=\"1\" href=\"#fn-1\" class=\"footnote-link\" target=\"_blank\" rel=\"noopener nofollow\">[1]</a></sup> and inline<sup data-footnote-id=\"fnref-2\" class=\"footnote-ref\" id=\"fnref-2\"><a data-footref=\"inline-1\" href=\"#fn-2\" class=\"footnote-link\" target=\"_blank\" rel=\"noopener nofollow\">[2]</a></sup>.</p>\n<section class=\"footnotes\"><hr><ol>\n<li data-footnote-id=\"fn-1\" id=\"fn-1\">\n<p>The <em>def</em>.<a href=\"#fnref-1\" class=\"footnote-backref footnote-link\" target=\"_blank\" rel=\"noopener nofollow\">\u{21a9}\u{fe0e}</a></p>\n</li>\n<li data-footnote-id=\"fn-2\" id=\"fn-2\">note here<a href=\"#fnref-2\" class=\"footnote-backref footnote-link\" target=\"_blank\" rel=\"noopener nofollow\">\u{21a9}\u{fe0e}</a></li>\n</ol></section>\n"
    );
    let doc = parse(src);
    assert_eq!(doc.footnote_labels, vec!["1"]);
    assert!(matches!(
        &doc.blocks[1].kind,
        BlockKind::FootnoteDefinition { label: "1", .. }
    ));
    // An undefined reference renders as its label, as Obsidian does.
    assert_eq!(plain_html("x[^nope]"), "<p>xnope</p>\n");
}

#[test]
fn frontmatter_block() {
    let src = "---\ntitle: T\n---\n# H\n";
    let doc = parse(src);
    assert!(matches!(&doc.blocks[0].kind, BlockKind::Yaml { yaml } if *yaml == "title: T\n"));
    assert_eq!(doc.blocks[0].span, Span::new(0, 16));
    assert_eq!(doc.blocks[1].span, Span::new(17, 20));
    assert_eq!(html(src), "<h1 data-heading=\"H\">H</h1>\n");
}

#[test]
fn bare_urls() {
    assert_eq!(
        plain_html("see https://example.com/a?b=1. or www.x.org/y (http://a.b/c)"),
        "<p>see <a href=\"https://example.com/a?b=1\">https://example.com/a?b=1</a>. or <a href=\"http://www.x.org/y\">www.x.org/y</a> (<a href=\"http://a.b/c\">http://a.b/c</a>)</p>\n"
    );
}

#[test]
fn spans_cover_blocks_and_inlines() {
    let src = "# Head\n\npara *em* text\n\n- a\n- b\n\n```rs\nlet x;\n```\n";
    let doc = parse(src);
    let spans: Vec<_> = doc.blocks.iter().map(|b| b.span.of(src)).collect();
    assert_eq!(
        spans,
        vec!["# Head", "para *em* text", "- a\n- b", "```rs\nlet x;\n```"]
    );
    let BlockKind::Paragraph { content } = &doc.blocks[1].kind else {
        panic!()
    };
    assert_eq!(content[1].span.of(src), "*em*");
    let BlockKind::List { items, .. } = &doc.blocks[2].kind else {
        panic!()
    };
    assert_eq!(items[1].span.of(src), "- b");
    let BlockKind::CodeBlock { info, content, .. } = &doc.blocks[3].kind else {
        panic!()
    };
    assert_eq!(info.as_deref(), Some("rs"));
    assert_eq!(content, "let x;\n");
}

#[test]
fn nested_structure_spans() {
    let src = "> quote\n> - item\n>   more\n";
    let doc = parse(src);
    let Block {
        kind: BlockKind::BlockQuote { children },
        span,
        ..
    } = &doc.blocks[0]
    else {
        panic!()
    };
    assert_eq!(span.of(src), "> quote\n> - item\n>   more");
    let BlockKind::List { items, .. } = &children[1].kind else {
        panic!()
    };
    assert_eq!(items[0].span.of(src), "- item\n>   more");
}

#[test]
fn crlf_and_bom_sources() {
    let src = "\u{feff}---\r\ntags: [a]\r\n---\r\n# H\r\n\r\ntext [[L]]\r\n";
    let doc = parse(src);
    assert!(matches!(doc.blocks[0].kind, BlockKind::Yaml { .. }));
    assert!(matches!(
        &doc.blocks[1].kind,
        BlockKind::Heading { level: 1, .. }
    ));
    let BlockKind::Paragraph { content } = &doc.blocks[2].kind else {
        panic!()
    };
    assert_eq!(content[1].span.of(src), "[[L]]");
}

/// Every CommonMark spec input, plus tricky multibyte and edge inputs, must
/// parse and render under the Obsidian options without panicking.
#[test]
fn obsidian_options_never_panic() {
    let spec = include_str!("data/commonmark-0.31.2.json");
    let mut inputs: Vec<String> = Vec::new();
    for chunk in spec.split("\"markdown\":").skip(1) {
        let s = chunk.trim_start();
        let mut out = String::new();
        let mut chars = s[1..].chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Some(ch) =
                            u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                        {
                            out.push(ch);
                        }
                    }
                    Some(o) => out.push(o),
                    None => {}
                },
                _ => out.push(c),
            }
        }
        inputs.push(out);
    }
    assert!(inputs.len() >= 600);
    inputs.extend(
        [
            "[[",
            "![[",
            "[[]]",
            "[[é|ü]] #tägß ^",
            "$",
            "$$",
            "$$\n",
            "%%",
            "%%\n",
            "==",
            "~~",
            "> [!",
            "> [!]",
            "> [!x",
            "| é |\n|-|\n| \\| |",
            "- [ ]",
            "- [é] x",
            "^[",
            "^[]",
            "[^",
            "[^]",
            "[^x]: ",
            "---\n",
            "---\nk: v",
            "\u{feff}---\n---\n",
            "a\u{0}b",
            "https://",
            "www.",
            "www.x",
            "<https://é>",
            "*é*_ü_",
            "`é`",
            "#\u{1f600}",
        ]
        .map(String::from),
    );
    for input in &inputs {
        let doc = parse(input);
        let _ = render_html(&doc);
        for i in 0..=input.len() {
            if input.is_char_boundary(i) {
                let doc = parse(&input[..i]);
                let _ = render_html(&doc);
            }
        }
    }
}
