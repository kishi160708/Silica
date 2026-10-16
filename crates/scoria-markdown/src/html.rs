//! HTML rendering of a [`Document`].
//!
//! With `obsidian_attributes` off the output matches the CommonMark reference
//! implementation byte for byte on CommonMark input, which is what the spec
//! conformance tests compare against. With it on, Obsidian's reading-view
//! class names and attributes are added (`internal-link`, `tag`, `callout`,
//! `internal-embed`, ...), the hooks that themes and snippets style.

use crate::ast::*;
use std::borrow::Cow;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlOptions {
    /// Emit Obsidian's reading-view classes and attributes.
    pub obsidian_attributes: bool,
}

impl Default for HtmlOptions {
    fn default() -> HtmlOptions {
        HtmlOptions {
            obsidian_attributes: true,
        }
    }
}

/// Renders `doc` with the default (Obsidian) options.
pub fn render_html(doc: &Document<'_>) -> String {
    render_html_with(doc, &HtmlOptions::default())
}

pub fn render_html_with(doc: &Document<'_>, opts: &HtmlOptions) -> String {
    let mut r = Renderer {
        out: String::new(),
        opts,
        footnote_numbers: Vec::new(),
        inline_footnotes: Vec::new(),
        definitions: Vec::new(),
    };
    collect_footnote_definitions(&doc.blocks, &mut r.definitions);
    r.blocks(&doc.blocks, false);
    r.footnote_section();
    r.out
}

fn collect_footnote_definitions<'b, 'a>(
    blocks: &'b [Block<'a>],
    out: &mut Vec<(&'a str, &'b [Block<'a>])>,
) {
    walk_blocks(blocks, &mut |b| {
        if let BlockKind::FootnoteDefinition { label, children } = &b.kind {
            if !out.iter().any(|(l, _)| l == label) {
                out.push((label, children));
            }
        }
    });
}

struct Renderer<'r, 'a> {
    out: String,
    opts: &'r HtmlOptions,
    /// Labels in order of first reference; index + 1 is the footnote number.
    footnote_numbers: Vec<String>,
    /// Inline footnotes keyed by their generated label.
    inline_footnotes: Vec<(String, &'r [Inline<'a>])>,
    definitions: Vec<(&'a str, &'r [Block<'a>])>,
}

pub(crate) fn escape_html(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Percent-encodes a URL the way the CommonMark reference implementation does.
pub(crate) fn escape_href(s: &str, out: &mut String) {
    for &b in s.as_bytes() {
        match b {
            b'&' => out.push_str("&amp;"),
            b'\'' => out.push_str("&#x27;"),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(b as char),
            b'!' | b'#' | b'$' | b'%' | b'(' | b')' | b'*' | b'+' | b',' | b'-' | b'.' | b'/'
            | b':' | b';' | b'=' | b'?' | b'@' | b'_' | b'~' => out.push(b as char),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
}

impl<'r, 'a> Renderer<'r, 'a> {
    fn cr(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    fn blocks(&mut self, blocks: &'r [Block<'a>], tight: bool) {
        for b in blocks {
            self.block(b, tight);
        }
    }

    fn block(&mut self, block: &'r Block<'a>, tight: bool) {
        match &block.kind {
            BlockKind::Yaml { .. } => {}
            BlockKind::Paragraph { content } => {
                if tight {
                    self.inlines(content);
                } else {
                    self.cr();
                    self.out.push_str("<p>");
                    self.inlines(content);
                    self.out.push_str("</p>\n");
                }
            }
            BlockKind::Heading { level, content, .. } => {
                self.cr();
                if self.opts.obsidian_attributes {
                    let _ = write!(self.out, "<h{level} data-heading=\"");
                    escape_html(&plain_text(content), &mut self.out);
                    self.out.push_str("\">");
                } else {
                    let _ = write!(self.out, "<h{level}>");
                }
                self.inlines(content);
                let _ = writeln!(self.out, "</h{level}>");
            }
            BlockKind::ThematicBreak => {
                self.cr();
                self.out.push_str("<hr />\n");
            }
            BlockKind::CodeBlock { info, content, .. } => {
                self.cr();
                self.out.push_str("<pre><code");
                if let Some(info) = info {
                    let lang = info.split_whitespace().next().unwrap_or("");
                    if !lang.is_empty() {
                        self.out.push_str(" class=\"language-");
                        escape_html(lang, &mut self.out);
                        self.out.push('"');
                    }
                }
                self.out.push('>');
                escape_html(content, &mut self.out);
                self.out.push_str("</code></pre>\n");
            }
            BlockKind::HtmlBlock { content } => {
                self.cr();
                self.out.push_str(content);
                self.cr();
            }
            BlockKind::BlockQuote { children } => {
                self.cr();
                self.out.push_str("<blockquote>\n");
                self.blocks(children, false);
                self.cr();
                self.out.push_str("</blockquote>\n");
            }
            BlockKind::Callout {
                callout_type,
                fold,
                title,
                children,
            } => {
                self.cr();
                self.out.push_str("<div data-callout=\"");
                escape_html(&callout_type.to_lowercase(), &mut self.out);
                self.out.push('"');
                match fold {
                    Some(Fold::Collapsed) => self.out.push_str(" data-callout-fold=\"-\""),
                    Some(Fold::Expanded) => self.out.push_str(" data-callout-fold=\"+\""),
                    None => {}
                }
                self.out.push_str(" class=\"callout");
                if fold.is_some() {
                    self.out.push_str(" is-collapsible");
                }
                if *fold == Some(Fold::Collapsed) {
                    self.out.push_str(" is-collapsed");
                }
                self.out.push_str(
                    "\"><div class=\"callout-title\"><div class=\"callout-title-inner\">",
                );
                if title.is_empty() {
                    let mut chars = callout_type.chars();
                    if let Some(first) = chars.next() {
                        let shown: String = first
                            .to_uppercase()
                            .chain(chars.map(|c| c.to_ascii_lowercase()))
                            .collect();
                        escape_html(&shown, &mut self.out);
                    }
                } else {
                    self.inlines(title);
                }
                self.out.push_str("</div></div>");
                if !children.is_empty() {
                    self.out.push_str("<div class=\"callout-content\">\n");
                    self.blocks(children, false);
                    self.cr();
                    self.out.push_str("</div>");
                }
                self.out.push_str("</div>\n");
            }
            BlockKind::List { kind, tight, items } => {
                self.cr();
                match kind {
                    ListKind::Bullet { .. } => {
                        if self.opts.obsidian_attributes && items.iter().any(|i| i.task.is_some()) {
                            self.out.push_str("<ul class=\"contains-task-list\">\n");
                        } else {
                            self.out.push_str("<ul>\n");
                        }
                    }
                    ListKind::Ordered { start, .. } => {
                        if *start == 1 {
                            self.out.push_str("<ol>\n");
                        } else {
                            let _ = writeln!(self.out, "<ol start=\"{start}\">");
                        }
                    }
                }
                for item in items {
                    self.cr();
                    match item.task {
                        Some(c) if self.opts.obsidian_attributes => {
                            let _ = write!(
                                self.out,
                                "<li data-task=\"{}\" class=\"task-list-item",
                                c.to_string().replace('"', "&quot;")
                            );
                            if c != ' ' {
                                self.out.push_str(" is-checked");
                            }
                            self.out.push_str(
                                "\"><input type=\"checkbox\" class=\"task-list-item-checkbox\"",
                            );
                            if c != ' ' {
                                self.out.push_str(" checked");
                            }
                            self.out.push_str("> ");
                        }
                        Some(c) => {
                            self.out.push_str("<li><input type=\"checkbox\"");
                            if c != ' ' {
                                self.out.push_str(" checked=\"\"");
                            }
                            self.out.push_str(" disabled=\"\" /> ");
                        }
                        None => self.out.push_str("<li>"),
                    }
                    self.blocks(&item.children, *tight);
                    self.out.push_str("</li>\n");
                }
                match kind {
                    ListKind::Bullet { .. } => self.out.push_str("</ul>\n"),
                    ListKind::Ordered { .. } => self.out.push_str("</ol>\n"),
                }
            }
            BlockKind::Table {
                alignments,
                header,
                rows,
            } => {
                self.cr();
                self.out.push_str("<table>\n<thead>\n<tr>\n");
                for (i, cell) in header.iter().enumerate() {
                    self.out.push_str("<th");
                    self.alignment(alignments[i]);
                    self.out.push('>');
                    self.inlines(&cell.content);
                    self.out.push_str("</th>\n");
                }
                self.out.push_str("</tr>\n</thead>\n");
                if !rows.is_empty() {
                    self.out.push_str("<tbody>\n");
                    for row in rows {
                        self.out.push_str("<tr>\n");
                        for (i, cell) in row.iter().enumerate() {
                            self.out.push_str("<td");
                            self.alignment(alignments[i]);
                            self.out.push('>');
                            self.inlines(&cell.content);
                            self.out.push_str("</td>\n");
                        }
                        self.out.push_str("</tr>\n");
                    }
                    self.out.push_str("</tbody>\n");
                }
                self.out.push_str("</table>\n");
            }
            BlockKind::Math { content } => {
                self.cr();
                self.out.push_str("<div class=\"math math-block\">$$");
                escape_html(content.trim_end_matches('\n'), &mut self.out);
                self.out.push_str("$$</div>\n");
            }
            BlockKind::Comment { .. } => {}
            BlockKind::FootnoteDefinition { .. } => {}
        }
    }

    fn alignment(&mut self, a: Alignment) {
        match a {
            Alignment::None => {}
            Alignment::Left => self.out.push_str(" align=\"left\""),
            Alignment::Center => self.out.push_str(" align=\"center\""),
            Alignment::Right => self.out.push_str(" align=\"right\""),
        }
    }

    fn inlines(&mut self, inlines: &'r [Inline<'a>]) {
        for i in inlines {
            self.inline(i);
        }
    }

    fn inline(&mut self, inline: &'r Inline<'a>) {
        match &inline.kind {
            InlineKind::Text(t) => escape_html(t, &mut self.out),
            InlineKind::SoftBreak => self.out.push('\n'),
            InlineKind::HardBreak => self.out.push_str("<br />\n"),
            InlineKind::Code(c) => {
                self.out.push_str("<code>");
                escape_html(c, &mut self.out);
                self.out.push_str("</code>");
            }
            InlineKind::Html(h) => self.out.push_str(h),
            InlineKind::Emphasis(c) => self.wrap("em", c),
            InlineKind::Strong(c) => self.wrap("strong", c),
            InlineKind::Strikethrough(c) => self.wrap("del", c),
            InlineKind::Highlight(c) => self.wrap("mark", c),
            InlineKind::Link {
                destination,
                title,
                children,
                ..
            } => {
                self.out.push_str("<a href=\"");
                escape_href(destination, &mut self.out);
                self.out.push('"');
                if let Some(t) = title {
                    self.out.push_str(" title=\"");
                    escape_html(t, &mut self.out);
                    self.out.push('"');
                }
                if self.opts.obsidian_attributes {
                    self.out.push_str(
                        " class=\"external-link\" target=\"_blank\" rel=\"noopener nofollow\"",
                    );
                }
                self.out.push('>');
                self.inlines(children);
                self.out.push_str("</a>");
            }
            InlineKind::Image {
                destination,
                title,
                children,
            } => {
                self.out.push_str("<img src=\"");
                escape_href(destination, &mut self.out);
                self.out.push_str("\" alt=\"");
                escape_html(&plain_text(children), &mut self.out);
                self.out.push('"');
                if let Some(t) = title {
                    self.out.push_str(" title=\"");
                    escape_html(t, &mut self.out);
                    self.out.push('"');
                }
                self.out.push_str(" />");
            }
            InlineKind::Wikilink { body, embed } => {
                let link = crate::parse_linktext(body);
                let target = body.split('|').next().unwrap_or("").trim();
                if *embed {
                    self.out.push_str("<span class=\"internal-embed\" src=\"");
                    escape_html(target, &mut self.out);
                    self.out.push_str("\" alt=\"");
                    escape_html(link.alias.unwrap_or(target), &mut self.out);
                    self.out.push_str("\"></span>");
                } else {
                    self.out.push_str("<a data-href=\"");
                    escape_html(target, &mut self.out);
                    self.out.push_str("\" href=\"");
                    escape_html(target, &mut self.out);
                    self.out.push_str(
                        "\" class=\"internal-link\" target=\"_blank\" rel=\"noopener nofollow\">",
                    );
                    let shown = match link.alias {
                        Some(a) => Cow::Borrowed(a),
                        None => Cow::Owned(wikilink_display(&link)),
                    };
                    escape_html(&shown, &mut self.out);
                    self.out.push_str("</a>");
                }
            }
            InlineKind::Tag(name) => {
                self.out.push_str("<a href=\"#");
                escape_html(name, &mut self.out);
                self.out
                    .push_str("\" class=\"tag\" target=\"_blank\" rel=\"noopener nofollow\">#");
                escape_html(name, &mut self.out);
                self.out.push_str("</a>");
            }
            InlineKind::Math { content, display } => {
                let class = if *display {
                    "math math-block"
                } else {
                    "math math-inline"
                };
                let _ = write!(self.out, "<span class=\"{class}\">");
                let delim = if *display { "$$" } else { "$" };
                self.out.push_str(delim);
                escape_html(content, &mut self.out);
                self.out.push_str(delim);
                self.out.push_str("</span>");
            }
            InlineKind::Comment(_) => {}
            InlineKind::FootnoteReference(label) => {
                let n = self.footnote_number(label);
                self.footnote_ref(n);
            }
            InlineKind::InlineFootnote(children) => {
                let label = format!("inline-{}", self.inline_footnotes.len() + 1);
                self.inline_footnotes.push((label.clone(), children));
                let n = self.footnote_number(&label);
                self.footnote_ref(n);
            }
        }
    }

    fn wrap(&mut self, tag: &str, children: &'r [Inline<'a>]) {
        let _ = write!(self.out, "<{tag}>");
        self.inlines(children);
        let _ = write!(self.out, "</{tag}>");
    }

    fn footnote_number(&mut self, label: &str) -> usize {
        if let Some(i) = self.footnote_numbers.iter().position(|l| l == label) {
            return i + 1;
        }
        self.footnote_numbers.push(label.to_string());
        self.footnote_numbers.len()
    }

    fn footnote_ref(&mut self, n: usize) {
        let _ = write!(
            self.out,
            "<sup id=\"fnref-{n}\" class=\"footnote-ref\"><a href=\"#fn-{n}\" class=\"footnote-link\">[{n}]</a></sup>"
        );
    }

    fn footnote_section(&mut self) {
        if self.footnote_numbers.is_empty() {
            return;
        }
        self.cr();
        self.out.push_str("<section class=\"footnotes\">\n<ol>\n");
        let labels = self.footnote_numbers.clone();
        for (i, label) in labels.iter().enumerate() {
            let n = i + 1;
            let _ = writeln!(self.out, "<li id=\"fn-{n}\" class=\"footnote\">");
            if let Some((_, inlines)) = self.inline_footnotes.iter().find(|(l, _)| l == label) {
                let inlines = *inlines;
                self.out.push_str("<p>");
                self.inlines(inlines);
                self.backref(n);
                self.out.push_str("</p>\n");
            } else if let Some((_, blocks)) = self.definitions.iter().find(|(l, _)| *l == label) {
                let blocks = *blocks;
                // Append the backlink inside the last paragraph when there is one.
                let split = blocks.len().saturating_sub(1);
                self.blocks(&blocks[..split], false);
                match blocks.last() {
                    Some(Block {
                        kind: BlockKind::Paragraph { content },
                        ..
                    }) => {
                        self.cr();
                        self.out.push_str("<p>");
                        self.inlines(content);
                        self.backref(n);
                        self.out.push_str("</p>\n");
                    }
                    Some(last) => {
                        self.block(last, false);
                        self.backref(n);
                        self.out.push('\n');
                    }
                    None => {
                        self.backref(n);
                        self.out.push('\n');
                    }
                }
            } else {
                self.backref(n);
                self.out.push('\n');
            }
            self.out.push_str("</li>\n");
        }
        self.out.push_str("</ol>\n</section>\n");
    }

    fn backref(&mut self, n: usize) {
        let _ = write!(
            self.out,
            " <a href=\"#fnref-{n}\" class=\"footnote-backref footnote-link\">\u{21a9}\u{fe0e}</a>"
        );
    }
}

/// Display text of a wikilink without an alias: `Note`, `Note > Heading`, `Note > ^id`.
fn wikilink_display(link: &crate::LinkText<'_>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !link.path.is_empty() {
        parts.push(link.path.to_string());
    }
    match &link.subpath {
        Some(crate::Subpath::Headings(hs)) => parts.extend(hs.iter().map(|h| h.to_string())),
        Some(crate::Subpath::Block(b)) => parts.push(format!("^{b}")),
        None => {}
    }
    parts.join(" > ")
}
