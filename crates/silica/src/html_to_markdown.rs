//! HTML to Markdown, for "Convert pasted HTML to Markdown".
//!
//! A page's clipboard carries `text/html` beside `text/plain`, and the plain
//! copy has lost everything the page expressed in markup — a heading comes
//! back as a bare line, a list as lines with no marker. With `autoConvertHtml`
//! on, Obsidian reads the HTML and writes Markdown instead; this is that.
//!
//! It is a small converter on purpose. The input is one page's clipboard, not
//! a document to round-trip: it handles the block elements a page is written
//! with (headings, paragraphs, lists, quotes, code, rules, tables) and the
//! inline ones that carry meaning (emphasis, code, links, images), and drops
//! what Markdown cannot say — attributes, scripts, styles, layout tables.
//! Anything it does not know is unwrapped to its text rather than dropped.

/// Convert `html` to Markdown. `None` when the input holds no markup worth
/// converting — a plain string that happens to be on the clipboard as
/// `text/html`, or an empty document.
pub fn convert(html: &str) -> Option<String> {
    if !looks_like_html(html) {
        return None;
    }
    let mut p = Parser::new(html);
    let nodes = p.parse_nodes(None);
    let mut out = String::new();
    write_blocks(&nodes, &mut out, 0);
    let trimmed = tidy(&out);
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Whether `s` is markup rather than a string that merely arrived as
/// `text/html`: a tag, and not one of the entities a page writes into a
/// plain-text clipboard by mistake.
fn looks_like_html(s: &str) -> bool {
    let s = s.trim_start();
    s.starts_with('<') && s.contains('>')
}

// ---------------------------------------------------------------- the tree

#[derive(Debug, PartialEq)]
enum Node {
    Text(String),
    Element(Element),
}

#[derive(Debug, PartialEq)]
struct Element {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

impl Element {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Elements whose content is not text and never becomes Markdown.
const SKIP: &[&str] = &["script", "style", "head", "title", "noscript", "template"];

/// Elements that open a line of their own.
const BLOCK: &[&str] = &[
    "p", "div", "section", "article", "header", "footer", "main", "aside", "nav", "figure",
    "figcaption", "form", "fieldset", "address", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol",
    "li", "blockquote", "pre", "hr", "table", "thead", "tbody", "tfoot", "tr", "td", "th", "dl",
    "dt", "dd", "details", "summary",
];

/// Elements that are their own line but hold no text of their own.
const VOID_BLOCK: &[&str] = &["hr", "br"];

fn is_block(tag: &str) -> bool {
    BLOCK.contains(&tag)
}

// ---------------------------------------------------------------- the parser

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Parser<'a> {
        Parser {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    /// Parses until `close` (a tag name) or the end, consuming the closing
    /// tag. `None` means "to the end of the input".
    fn parse_nodes(&mut self, close: Option<&str>) -> Vec<Node> {
        let mut out: Vec<Node> = Vec::new();
        while self.pos < self.src.len() {
            if self.starts_with("<!--") {
                self.skip_past("-->");
                continue;
            }
            if self.starts_with("<!") || self.starts_with("<?") {
                self.skip_past(">");
                continue;
            }
            if self.starts_with("</") {
                let name = self.peek_tag_name(2);
                self.skip_past(">");
                if close.is_some_and(|c| c.eq_ignore_ascii_case(&name)) {
                    return out;
                }
                // A stray close for a different element: its content has
                // already been read, so there is nothing to do.
                continue;
            }
            if self.starts_with("<") {
                if let Some(el) = self.parse_element() {
                    if SKIP.contains(&el.tag.as_str()) {
                        continue;
                    }
                    out.push(Node::Element(el));
                }
                continue;
            }
            out.push(Node::Text(self.parse_text()));
        }
        out
    }

    /// `<tag attr="v">children</tag>` or `<tag/>`.
    fn parse_element(&mut self) -> Option<Element> {
        let open_len = self.peek_tag_name(1).len();
        if open_len == 0 {
            // A stray `<`: it is text.
            self.pos += 1;
            return None;
        }
        let tag = self.peek_tag_name(1).to_ascii_lowercase();
        let (attrs, self_closing) = self.parse_attrs();
        if self_closing {
            return Some(Element {
                tag,
                attrs,
                children: Vec::new(),
            });
        }
        // `br` and `hr` need no close tag; nor do the ones a browser closes
        // on its own. Treating them as empty is what Markdown wants.
        if VOID_BLOCK.contains(&tag.as_str()) {
            return Some(Element {
                tag,
                attrs,
                children: Vec::new(),
            });
        }
        let children = self.parse_nodes(Some(&tag));
        Some(Element {
            tag,
            attrs,
            children,
        })
    }

    /// Reads `<tag` … `>` and answers the attributes plus whether the tag
    /// closed itself. Leaves `pos` after the `>`.
    fn parse_attrs(&mut self) -> (Vec<(String, String)>, bool) {
        let mut attrs = Vec::new();
        self.pos += 1; // the '<'
        // Skip the tag name, stopping at the `>` or `/` that ends it: a tag
        // with no attributes has no whitespace to stop on.
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if is_space(c) || c == b'>' || c == b'/' {
                break;
            }
            self.pos += 1;
        }
        loop {
            while self.pos < self.src.len() && is_space(self.src[self.pos]) {
                self.pos += 1;
            }
            if self.pos >= self.src.len() {
                return (attrs, false);
            }
            match self.src[self.pos] {
                b'>' => {
                    self.pos += 1;
                    return (attrs, false);
                }
                b'/' => {
                    self.pos += 1;
                    if self.pos < self.src.len() && self.src[self.pos] == b'>' {
                        self.pos += 1;
                        return (attrs, true);
                    }
                }
                _ => {
                    let name = self.read_name();
                    while self.pos < self.src.len() && is_space(self.src[self.pos]) {
                        self.pos += 1;
                    }
                    if self.pos < self.src.len() && self.src[self.pos] == b'=' {
                        self.pos += 1;
                        while self.pos < self.src.len() && is_space(self.src[self.pos]) {
                            self.pos += 1;
                        }
                        let value = self.read_value();
                        if !name.is_empty() {
                            attrs.push((name.to_ascii_lowercase(), value));
                        }
                    } else if !name.is_empty() {
                        attrs.push((name.to_ascii_lowercase(), String::new()));
                    }
                }
            }
        }
    }

    fn read_name(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if is_space(c) || c == b'=' || c == b'>' || c == b'/' {
                break;
            }
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.src[start..self.pos]).into_owned()
    }

    fn read_value(&mut self) -> String {
        if self.pos >= self.src.len() {
            return String::new();
        }
        let quote = self.src[self.pos];
        if quote == b'"' || quote == b'\'' {
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos] != quote {
                self.pos += 1;
            }
            let v = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
            if self.pos < self.src.len() {
                self.pos += 1;
            }
            decode_entities(&v)
        } else {
            let start = self.pos;
            while self.pos < self.src.len() && !is_space(self.src[self.pos]) && self.src[self.pos] != b'>'
            {
                self.pos += 1;
            }
            decode_entities(&String::from_utf8_lossy(&self.src[start..self.pos]))
        }
    }

    fn parse_text(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos] != b'<' {
            self.pos += 1;
        }
        decode_entities(&String::from_utf8_lossy(&self.src[start..self.pos]))
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s.as_bytes())
    }

    /// The tag name starting `skip` bytes ahead of the `<`.
    fn peek_tag_name(&self, skip: usize) -> String {
        let mut i = self.pos + skip;
        let start = i;
        while i < self.src.len() {
            let c = self.src[i];
            if is_space_or_end(c) || c == b'>' || c == b'/' {
                break;
            }
            i += 1;
        }
        String::from_utf8_lossy(&self.src[start..i]).into_owned()
    }

    fn skip_past(&mut self, needle: &str) {
        match find(&self.src[self.pos..], needle.as_bytes()) {
            Some(ix) => self.pos += ix + needle.len(),
            None => self.pos = self.src.len(),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

fn is_space_or_end(c: u8) -> bool {
    is_space(c)
}

/// The named entities that show up in prose, plus numeric ones. Anything
/// else is left as written rather than guessed at.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some((text, len)) = entity_at(&bytes[i..]) {
                out.push_str(&text);
                i += len;
                continue;
            }
        }
        // Copy one whole character, not one byte.
        let ch = s[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn entity_at(bytes: &[u8]) -> Option<(String, usize)> {
    let semi = bytes.iter().take(12).position(|&b| b == b';')?;
    let name = std::str::from_utf8(&bytes[1..semi]).ok()?;
    let ch = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
    } else if let Some(dec) = name.strip_prefix('#') {
        char::from_u32(dec.parse::<u32>().ok()?)?
    } else {
        match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            "nbsp" => '\u{a0}',
            "mdash" => '—',
            "ndash" => '–',
            "hellip" => '…',
            "ldquo" => '“',
            "rdquo" => '”',
            "lsquo" => '‘',
            "rsquo" => '’',
            "copy" => '©',
            "reg" => '®',
            "trade" => '™',
            "deg" => '°',
            "middot" => '·',
            "bull" => '•',
            "laquo" => '«',
            "raquo" => '»',
            "times" => '×',
            "divide" => '÷',
            "minus" => '−',
            "plusmn" => '±',
            "frac12" => '½',
            "euro" => '€',
            "pound" => '£',
            "yen" => '¥',
            "cent" => '¢',
            "sect" => '§',
            "para" => '¶',
            "dagger" => '†',
            "prime" => '′',
            "Prime" => '″',
            "larr" => '←',
            "rarr" => '→',
            "uarr" => '↑',
            "darr" => '↓',
            "harr" => '↔',
            "infin" => '∞',
            "ne" => '≠',
            "le" => '≤',
            "ge" => '≥',
            "asymp" => '≈',
            "sum" => '∑',
            "prod" => '∏',
            "radic" => '√',
            "int" => '∫',
            "part" => '∂',
            "nabla" => '∇',
            "isin" => '∈',
            "notin" => '∉',
            "cap" => '∩',
            "cup" => '∪',
            "and" => '∧',
            "or" => '∨',
            "forall" => '∀',
            "exist" => '∃',
            "empty" => '∅',
            "alpha" => 'α',
            "beta" => 'β',
            "gamma" => 'γ',
            "delta" => 'δ',
            "epsilon" => 'ε',
            "lambda" => 'λ',
            "mu" => 'μ',
            "pi" => 'π',
            "sigma" => 'σ',
            "tau" => 'τ',
            "phi" => 'φ',
            "omega" => 'ω',
            "Delta" => 'Δ',
            "Omega" => 'Ω',
            "Sigma" => 'Σ',
            "Pi" => 'Π',
            _ => return None,
        }
    };
    Some((ch.to_string(), semi + 1))
}

// ---------------------------------------------------------------- the writer

/// Writes `nodes` as Markdown blocks, each separated by one blank line.
fn write_blocks(nodes: &[Node], out: &mut String, depth: usize) {
    let mut inline = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => inline.push_str(t),
            Node::Element(el) if is_block(&el.tag) => {
                flush_inline(&mut inline, out);
                write_block(el, out, depth);
            }
            Node::Element(el) => write_inline(el, &mut inline),
        }
    }
    flush_inline(&mut inline, out);
}

/// Emits a pending run of inline content as its own paragraph.
fn flush_inline(inline: &mut String, out: &mut String) {
    let text = collapse(inline);
    inline.clear();
    let text = text.trim();
    if !text.is_empty() {
        push_block(out, text);
    }
}

fn push_block(out: &mut String, text: &str) {
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(text);
}

fn write_block(el: &Element, out: &mut String, depth: usize) {
    match el.tag.as_str() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = el.tag.as_bytes()[1] - b'0';
            let text = inline_of(&el.children);
            let text = text.trim();
            if !text.is_empty() {
                push_block(out, &format!("{} {text}", "#".repeat(level as usize)));
            }
        }
        "hr" => push_block(out, "---"),
        "pre" => write_pre(el, out),
        "blockquote" => {
            let mut inner = String::new();
            write_blocks(&el.children, &mut inner, depth);
            let quoted: Vec<String> = inner
                .lines()
                .map(|l| {
                    if l.is_empty() {
                        ">".to_string()
                    } else {
                        format!("> {l}")
                    }
                })
                .collect();
            if !quoted.is_empty() {
                push_block(out, &quoted.join("\n"));
            }
        }
        "ul" | "ol" => write_list(el, out, depth),
        "table" => write_table(el, out),
        // A definition list has no Markdown of its own: the term becomes a
        // bold line and the definitions sit under it.
        "dl" => {
            for child in &el.children {
                if let Node::Element(e) = child {
                    match e.tag.as_str() {
                        "dt" => {
                            let t = inline_of(&e.children);
                            if !t.trim().is_empty() {
                                push_block(out, &format!("**{}**", t.trim()));
                            }
                        }
                        "dd" => {
                            let t = inline_of(&e.children);
                            if !t.trim().is_empty() {
                                push_block(out, t.trim());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        "li" => {
            // An `li` outside a list: write it as a bullet rather than
            // dropping the marker.
            let text = inline_of(&el.children);
            if !text.trim().is_empty() {
                push_block(out, &format!("- {}", text.trim()));
            }
        }
        _ => {
            // `p`, `div`, `section` and the rest: their content is blocks of
            // its own. A paragraph is one line.
            let mut inner = String::new();
            write_blocks(&el.children, &mut inner, depth);
            let inner = inner.trim();
            if !inner.is_empty() {
                push_block(out, inner);
            }
        }
    }
}

fn write_pre(el: &Element, out: &mut String) {
    // `<pre><code class="language-rust">` names the fence's language.
    let mut lang = String::new();
    if let Some(Node::Element(code)) = el
        .children
        .iter()
        .find(|n| matches!(n, Node::Element(e) if e.tag == "code"))
    {
        if let Some(class) = code.attr("class") {
            if let Some(name) = class.strip_prefix("language-") {
                lang = name.split_whitespace().next().unwrap_or("").to_string();
            }
        }
    }
    let mut body = raw_text(&el.children);
    while body.ends_with('\n') {
        body.pop();
    }
    push_block(out, &format!("```{lang}\n{body}\n```"));
}

fn write_list(el: &Element, out: &mut String, depth: usize) {
    let ordered = el.tag == "ol";
    let mut items: Vec<String> = Vec::new();
    for child in &el.children {
        let Node::Element(li) = child else { continue };
        if li.tag != "li" {
            continue;
        }
        let mut body = String::new();
        // The item's own text comes first, then any nested list indented
        // under it.
        let mut inline = String::new();
        for node in &li.children {
            match node {
                Node::Text(t) => inline.push_str(t),
                Node::Element(e) if e.tag == "ul" || e.tag == "ol" => {
                    flush_inline_in_place(&mut inline, &mut body);
                    let mut nested = String::new();
                    write_list(e, &mut nested, depth + 1);
                    if !nested.trim().is_empty() {
                        if !body.is_empty() {
                            body.push('\n');
                        }
                        for line in nested.lines() {
                            body.push_str("    ");
                            body.push_str(line);
                            body.push('\n');
                        }
                        while body.ends_with('\n') {
                            body.pop();
                        }
                    }
                }
                Node::Element(e) if is_block(&e.tag) => {
                    flush_inline_in_place(&mut inline, &mut body);
                    let mut inner = String::new();
                    write_block(e, &mut inner, depth + 1);
                    if !inner.trim().is_empty() {
                        if !body.is_empty() {
                            body.push('\n');
                        }
                        body.push_str(inner.trim());
                    }
                }
                Node::Element(e) => write_inline(e, &mut inline),
            }
        }
        flush_inline_in_place(&mut inline, &mut body);
        let body = body.trim();
        if body.is_empty() {
            continue;
        }
        let marker = if ordered { "1." } else { "-" };
        let mut lines = body.lines();
        let first = lines.next().unwrap_or("");
        let mut item = format!("{marker} {first}");
        // A continuation line is indented to the marker's width, but a
        // line that already starts with spaces is a nested list's own
        // indent and keeps it.
        for line in lines {
            item.push('\n');
            if line.starts_with(' ') {
                item.push_str(line);
            } else {
                item.push_str("   ");
                item.push_str(line);
            }
        }
        items.push(item);
    }
    if !items.is_empty() {
        push_block(out, &items.join("\n"));
    }
}

/// The list-item counterpart of [`flush_inline`], appending into a body
/// rather than emitting a block.
fn flush_inline_in_place(inline: &mut String, body: &mut String) {
    let text = collapse(inline);
    inline.clear();
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(text);
}

fn write_table(el: &Element, out: &mut String) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut header_rows = 0usize;
    for section in &el.children {
        match section {
            Node::Element(s) if s.tag == "thead" || s.tag == "tbody" || s.tag == "tfoot" => {
                for row in &s.children {
                    if let Node::Element(tr) = row {
                        if tr.tag == "tr" {
                            let cells = table_row(tr);
                            if s.tag == "thead" && header_rows == rows.len() {
                                header_rows += 1;
                            }
                            rows.push(cells);
                        }
                    }
                }
            }
            Node::Element(tr) if tr.tag == "tr" => rows.push(table_row(tr)),
            _ => {}
        }
    }
    rows.retain(|r| !r.is_empty());
    if rows.is_empty() {
        return;
    }
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if cols == 0 {
        return;
    }
    // A `th` row is the header even without a `<thead>`.
    let has_th = el.children.iter().any(|n| {
        matches!(n, Node::Element(s) if s.tag == "thead")
    });
    let header_at = if has_th { 0 } else { header_rows };
    let mut lines: Vec<String> = Vec::new();
    for (ix, row) in rows.iter().enumerate() {
        let mut cells = row.clone();
        cells.resize(cols, String::new());
        let joined = cells
            .iter()
            .map(|c| pipe_safe(c.trim()))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("| {joined} |"));
        if ix == header_at {
            lines.push(format!(
                "| {} |",
                (0..cols).map(|_| "---").collect::<Vec<_>>().join(" | ")
            ));
        }
    }
    push_block(out, &lines.join("\n"));
}

fn table_row(tr: &Element) -> Vec<String> {
    tr.children
        .iter()
        .filter_map(|n| match n {
            Node::Element(td) if td.tag == "td" || td.tag == "th" => {
                Some(inline_of(&td.children))
            }
            _ => None,
        })
        .collect()
}

/// A cell's text with the pipe escaped, which is the one character GFM
/// tables cannot hold raw.
fn pipe_safe(cell: &str) -> String {
    cell.replace('|', "\\|")
}

/// Writes an inline element into `out`.
fn write_inline(el: &Element, out: &mut String) {
    match el.tag.as_str() {
        "br" => out.push(BREAK),
        "strong" | "b" => wrap(el, out, "**"),
        "em" | "i" => wrap(el, out, "*"),
        "del" | "s" | "strike" => wrap(el, out, "~~"),
        "code" | "kbd" | "samp" | "tt" => {
            let text = raw_text(&el.children);
            let text = text.trim();
            if !text.is_empty() {
                // A backtick inside the code needs a longer fence.
                let fence = if text.contains('`') { "``" } else { "`" };
                out.push_str(&format!("{fence}{text}{fence}"));
            }
        }
        "a" => {
            let text = collapsed_inline(&el.children);
            let href = el.attr("href").unwrap_or("").trim();
            if text.is_empty() {
                return;
            }
            if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
                out.push_str(&text);
            } else if is_image_target(href) && text == href {
                out.push_str(&format!("![]({})", escape_target(href)));
            } else {
                out.push_str(&format!("[{}]({})", text, escape_target(href)));
            }
        }
        "img" => {
            let src = el.attr("src").unwrap_or("").trim();
            if src.is_empty() {
                return;
            }
            let alt = el.attr("alt").unwrap_or("").trim();
            out.push_str(&format!("![{}]({})", alt, escape_target(src)));
        }
        "sup" => {
            // Markdown has no superscript; the caret reads the same in text.
            let text = collapsed_inline(&el.children);
            if !text.is_empty() {
                out.push_str(&format!("^{text}^"));
            }
        }
        "sub" => {
            let text = collapsed_inline(&el.children);
            if !text.is_empty() {
                out.push_str(&format!("~{text}~"));
            }
        }
        "input" => {
            if el.attr("type") == Some("checkbox") {
                let checked = el.attrs.iter().any(|(k, _)| k == "checked");
                out.push_str(if checked { "[x] " } else { "[ ] " });
            }
        }
        _ => {
            // A span, a font, a label: its text, without the wrapper.
            for child in &el.children {
                match child {
                    Node::Text(t) => out.push_str(t),
                    Node::Element(e) if is_block(&e.tag) => {
                        out.push('\n');
                        out.push_str(&collapsed_inline(&e.children));
                        out.push('\n');
                    }
                    Node::Element(e) => write_inline(e, out),
                }
            }
        }
    }
}

fn wrap(el: &Element, out: &mut String, marker: &str) {
    let text = collapsed_inline(&el.children);
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    // Leading or trailing spaces inside emphasis are lost by every Markdown
    // parser; move them outside the markers.
    let lead = collapsed_inline(&el.children).len() - collapsed_inline(&el.children).trim_start().len();
    let spaced = " ".repeat(lead);
    out.push_str(&spaced);
    out.push_str(marker);
    out.push_str(text);
    out.push_str(marker);
}

fn is_image_target(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".bmp"]
        .iter()
        .any(|ext| lower.contains(ext))
}

/// A link or image target: spaces and parentheses would end the target
/// early, so they are escaped or encoded.
fn escape_target(href: &str) -> String {
    href.replace(' ', "%20").replace(')', "%29")
}

/// The plain text of `nodes`, with inline markup flattened.
fn inline_of(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) if e.tag == "br" => out.push(' '),
            Node::Element(e) => out.push_str(&inline_of(&e.children)),
        }
    }
    collapse(&out)
}

/// The same, with the inline markup kept: what goes inside an emphasis or a
/// link.
fn collapsed_inline(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => write_inline(e, &mut out),
        }
    }
    collapse(&out)
}

/// The text of `nodes` verbatim: `<pre>` keeps its spaces and newlines.
fn raw_text(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => out.push_str(&raw_text(&e.children)),
        }
    }
    out
}

/// The character `<br>` leaves behind: a newline the source cannot spell,
/// so collapsing the page's own whitespace cannot eat it.
const BREAK: char = '\u{0}';

/// Runs of whitespace become one space, the way a browser lays text out.
/// The page's own newlines are whitespace too — a paragraph does not break
/// because its source did — but a [`BREAK`] is a real line break and is
/// turned back into a newline here.
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for ch in s.chars() {
        if ch == BREAK {
            out.push('\n');
            space = true;
        } else if ch.is_whitespace() {
            if !space {
                out.push(' ');
                space = true;
            }
        } else {
            out.push(ch);
            space = false;
        }
    }
    out
}

/// Trims every line, drops the blank ones a page's whitespace left behind,
/// and keeps exactly one blank line between blocks.
fn tidy(s: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut blank = false;
    for line in s.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            if !out.is_empty() {
                blank = true;
            }
            continue;
        }
        if blank && !out.is_empty() {
            out.push(String::new());
            blank = false;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(html: &str) -> String {
        convert(html).expect("markup was not recognised")
    }

    #[test]
    fn headings_and_paragraphs() {
        assert_eq!(md("<h1>Title</h1><p>Body.</p>"), "# Title\n\nBody.");
        assert_eq!(md("<h3>Deep</h3>"), "### Deep");
    }

    #[test]
    fn emphasis_and_code() {
        assert_eq!(
            md("<p><strong>bold</strong> and <em>it</em> and <code>x = 1</code></p>"),
            "**bold** and *it* and `x = 1`"
        );
    }

    #[test]
    fn links_keep_their_target() {
        assert_eq!(
            md(r#"<p><a href="https://example.com/a">text</a></p>"#),
            "[text](https://example.com/a)"
        );
        // An anchor to nowhere, or a script, keeps only its text.
        assert_eq!(md(r##"<p><a href="#top">top</a></p>"##), "top");
        assert_eq!(md(r##"<p><a href="javascript:x()">bad</a></p>"##), "bad");
    }

    #[test]
    fn lists_nest() {
        assert_eq!(
            md("<ul><li>one</li><li>two<ul><li>inner</li></ul></li></ul>"),
            "- one\n- two\n    - inner"
        );
        assert_eq!(md("<ol><li>first</li><li>second</li></ol>"), "1. first\n1. second");
    }

    #[test]
    fn pre_keeps_its_language_and_whitespace() {
        assert_eq!(
            md("<pre><code class=\"language-rust\">fn main() {\n    1\n}</code></pre>"),
            "```rust\nfn main() {\n    1\n}\n```"
        );
        // No language: a bare fence.
        assert_eq!(md("<pre>plain</pre>"), "```\nplain\n```");
    }

    #[test]
    fn quote_is_prefixed_per_line() {
        assert_eq!(
            md("<blockquote><p>one</p><p>two</p></blockquote>"),
            "> one\n>\n> two"
        );
    }

    #[test]
    fn table_becomes_gfm() {
        assert_eq!(
            md("<table><thead><tr><th>A</th><th>B</th></tr></thead>\
                <tbody><tr><td>1</td><td>2</td></tr></tbody></table>"),
            "| A | B |\n| --- | --- |\n| 1 | 2 |"
        );
        // A pipe in a cell is escaped, or the row would gain a column.
        assert_eq!(
            md("<table><tr><th>A</th></tr><tr><td>a|b</td></tr></table>"),
            "| A |\n| --- |\n| a\\|b |"
        );
    }

    #[test]
    fn images_and_breaks() {
        assert_eq!(
            md(r#"<p><img src="a.png" alt="pic"></p>"#),
            "![pic](a.png)"
        );
        assert_eq!(md("<p>one<br>two</p>"), "one\ntwo");
        assert_eq!(md("<p>a</p><hr><p>b</p>"), "a\n\n---\n\nb");
    }

    #[test]
    fn entities_are_decoded() {
        assert_eq!(md("<p>a &amp; b &lt; c &#8212; d</p>"), "a & b < c — d");
        // An entity the table does not know stays as written.
        assert_eq!(md("<p>&weird;</p>"), "&weird;");
    }

    #[test]
    fn unknown_elements_are_unwrapped_not_dropped() {
        assert_eq!(
            md(r#"<div class="x"><span style="color:red">kept</span></div>"#),
            "kept"
        );
    }

    #[test]
    fn scripts_and_styles_never_reach_the_output() {
        assert_eq!(
            md("<style>p{color:red}</style><script>x()</script><p>only</p>"),
            "only"
        );
    }

    #[test]
    fn a_plain_string_is_not_converted() {
        // A page that puts its plain copy on the clipboard as text/html.
        assert_eq!(convert("just some text"), None);
        assert_eq!(convert(""), None);
        assert_eq!(convert("   \n  "), None);
    }

    #[test]
    fn whitespace_collapses_the_way_a_browser_draws_it() {
        assert_eq!(md("<p>a\n      b</p>"), "a b");
        // But a <br> is a real break.
        assert_eq!(md("<p>a<br>\n     b</p>"), "a\nb");
    }
}
