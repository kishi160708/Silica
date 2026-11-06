//! The vault exported as a static HTML site: every note rendered through the
//! same parser the reading view uses, links rewritten to the exported pages,
//! assets copied beside them.
//!
//! The point is publishing without a plugin: one command writes a folder a
//! browser (or a static host) can serve straight away. It is a snapshot, not
//! a sync — run it again and the folder is rebuilt.
//!
//! Links: a resolved wikilink becomes a relative URL to the exported page —
//! relative, never root-relative, so the folder works opened straight from
//! disk with no server. An unresolved link loses its `href` (the anchor
//! stays as text, `data-href` kept for round-tripping); an external link is
//! left exactly as the renderer wrote it. An embed of a *note* renders
//! nothing in the reading view's HTML either, so it is left alone too — the
//! ceiling here is images, audio and video.

use silica_formats::FileKind;
use silica_vault::VaultFile;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// What one export pass did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteReport {
    pub notes: usize,
    pub links: usize,
    pub unresolved: usize,
    pub output: PathBuf,
}

/// Directories of the vault that are never published.
const SKIPPED_DIRS: [&str; 3] = [".obsidian", ".trash", ".git"];

/// Exports the vault at `root` (whose files are `files`) into `out`,
/// replacing whatever an earlier pass wrote there.
pub fn export(root: &Path, files: &[VaultFile], out: &Path) -> Result<SiteReport, String> {
    let notes: Vec<&VaultFile> = files
        .iter()
        .filter(|f| f.kind == FileKind::Markdown)
        .collect();
    if notes.is_empty() {
        return Err("The vault has no notes to export.".into());
    }
    let map = link_map(&notes);
    let assets = asset_paths(files);
    std::fs::create_dir_all(out).map_err(|e| format!("Cannot create {}: {e}", out.display()))?;

    let mut report = SiteReport {
        notes: 0,
        links: 0,
        unresolved: 0,
        output: out.to_path_buf(),
    };

    // * Every note renders to `out/<same folders>/<stem>.html`. A page's
    //   depth is its folder's, which is what relative URLs are built from.
    for note in &notes {
        let rel = note.path.replace('\\', "/");
        let depth = rel.matches('/').count();
        let text = silica_vault::read_verified(root, Path::new(&rel), u64::MAX)
            .map_err(|e| format!("Cannot read {rel}: {e}"))?;
        let body = rewrite(&render(&text), &map, &assets, depth);
        report.links += body.links;
        report.unresolved += body.unresolved;
        write_page(out, &page_path(&rel), &title_of(&rel), depth, &body.html)?;
        report.notes += 1;
    }

    copy_assets(root, files, out)?;
    std::fs::write(out.join("style.css"), STYLE)
        .map_err(|e| format!("Cannot write style.css: {e}"))?;

    // * `index.html` is the note called README, index or Home when there is
    //   one, else the first note — rendered like any other, at depth 0.
    let home = ["README.md", "index.md", "Home.md"]
        .iter()
        .find_map(|name| notes.iter().find(|n| n.path.eq_ignore_ascii_case(name)))
        .unwrap_or(&notes[0]);
    let text = silica_vault::read_verified(root, Path::new(&home.path), u64::MAX)
        .map_err(|e| format!("Cannot read {}: {e}", home.path))?;
    let body = rewrite(&render(&text), &map, &assets, 0);
    write_page(out, "index.html", &title_of(&home.path), 0, &body.html)?;
    Ok(report)
}

/// Parses and renders one note the way the reading view would.
fn render(text: &str) -> String {
    let doc = silica_markdown::parse(text);
    silica_markdown::html::render_html(&doc)
}

/// A page's `<title>`: its name without the extension.
fn title_of(rel: &str) -> String {
    rel.rsplit('/')
        .next()
        .unwrap_or(rel)
        .strip_suffix(".md")
        .unwrap_or_else(|| rel.rsplit('/').next().unwrap_or(rel))
        .to_string()
}

/// Wraps a rendered body in the one document shape every page shares.
fn write_page(out: &Path, name: &str, title: &str, depth: usize, body: &str) -> Result<(), String> {
    let escaped_title = escape_title(title);
    let page = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{escaped_title}</title>\n<link rel=\"stylesheet\" href=\"{}style.css\">\n\
         </head>\n<body>\n{body}\n</body>\n</html>\n",
        "../".repeat(depth),
    );
    let path = out.join(name);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, page).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}

/// The link table: every way a note can be named — full path with and
/// without the extension, bare stem with and without — lower-cased, mapped
/// to the note's vault path. Shorter paths claim a shared name first, which
/// is Obsidian's shortest-path rule for ambiguous names.
fn link_map(notes: &[&VaultFile]) -> HashMap<String, String> {
    let mut sorted: Vec<&str> = notes.iter().map(|n| n.path.as_str()).collect();
    sorted.sort_by_key(|p| (p.len(), *p));
    let mut map = HashMap::new();
    for path in sorted {
        let lower = path.to_lowercase();
        let without_ext = lower.strip_suffix(".md").unwrap_or(&lower).to_string();
        let stem = without_ext
            .rsplit('/')
            .next()
            .unwrap_or(&without_ext)
            .to_string();
        for key in [
            &lower,
            without_ext.as_str(),
            stem.as_str(),
            &format!("{stem}.md"),
        ] {
            map.entry(key.to_string())
                .or_insert_with(|| path.to_string());
        }
    }
    map
}

/// The same table for every publishable asset: lower-cased vault path to
/// itself, so an embed's `src` can be checked and kept honest.
fn asset_paths(files: &[VaultFile]) -> HashMap<String, String> {
    files
        .iter()
        .filter(|f| f.kind != FileKind::Markdown)
        .map(|f| {
            (
                f.path.replace('\\', "/").to_lowercase(),
                f.path.replace('\\', "/"),
            )
        })
        .collect()
}

/// The page path a note exports to: same folders, `.md` becomes `.html`.
fn page_path(rel: &str) -> String {
    let without = rel.strip_suffix(".md").unwrap_or(rel);
    format!("{without}.html")
}

/// Resolves a link's text to the vault path it names, or `None`. The subpath
/// (`#heading`, `#^block`) never takes part; the name may be spelled with the
/// extension or without, in any case.
fn resolve<'m>(link: &str, map: &'m HashMap<String, String>) -> Option<&'m str> {
    let name = link.split('#').next().unwrap_or(link).trim();
    if name.is_empty() {
        return None;
    }
    let lower = name.trim_end_matches(".md").to_lowercase();
    map.get(&lower).map(|p| p.as_str())
}

/// The three characters that end a title element or start an entity.
/// `escape_html` in the renderer is crate-private, and a title never
/// needs more than these three.
fn escape_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for ch in title.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// The linker's attribute values arrive HTML-escaped (the renderer escapes
/// the link text), so `A & B.md` reaches here as `A &amp; B.md`. Undoing
/// the five named references is enough for the vault's legal file-name
/// characters; anything else is left as written.
fn unescape_attr(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let entity = [
            ("amp;", "&"),
            ("lt;", "<"),
            ("gt;", ">"),
            ("quot;", "\""),
            ("#39;", "'"),
        ]
        .iter()
        .find(|(name, _)| after.starts_with(name));
        match entity {
            Some((name, ch)) => {
                out.push_str(ch);
                rest = &after[name.len()..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The URL one exported page or asset is reached by from another: the
/// relative walk from `from_depth` (the number of folder components above
/// the page) to the target's path inside the site.
fn rel_url(from_depth: usize, to: &str) -> String {
    let mut url = "../".repeat(from_depth);
    url.push_str(to);
    url
}

/// What rewriting one page did.
struct Rewritten {
    html: String,
    links: usize,
    unresolved: usize,
}

/// Rewrites the internal links and embeds of one rendered page.
///
/// The renderer emits `class="internal-link"` anchors whose `href` is the
/// link text as written and never a working URL, and `class="internal-embed"`
/// spans whose `src` is the resource path as written. The pass finds each
/// such tag, resolves its `data-href` (anchors) or `src` (embeds of a copied
/// asset), and puts the page's relative URL in its place. The renderer's
/// `target="_blank" rel="noopener nofollow"` — right for the app, wrong for
/// a site's own pages — goes with the rewritten `href`. An unresolved link
/// loses the `href` and keeps the text; an embed of a note (which the
/// renderer draws as an empty span anyway) is left as it is.
fn rewrite(
    html: &str,
    map: &HashMap<String, String>,
    assets: &HashMap<String, String>,
    depth: usize,
) -> Rewritten {
    let mut out = String::with_capacity(html.len());
    let mut links = 0usize;
    let mut unresolved = 0usize;
    let mut rest = html;
    while let Some(at) = next_tag(rest) {
        let which = if rest[at..].starts_with("class=\"internal-link\"") {
            "internal-link"
        } else {
            "internal-embed"
        };
        let attr = if which == "internal-link" {
            "data-href"
        } else {
            "src"
        };
        let tag_start = rest[..at].rfind('<').unwrap_or(at);
        let tag_end = at + rest[at..].find('>').map(|i| i + 1).unwrap_or(0);
        let value = rest[tag_start..tag_end]
            .find(&format!("{attr}=\""))
            .map(|i| {
                let at = i + attr.len() + 2;
                let end = rest[tag_start + at..]
                    .find('"')
                    .map(|e| tag_start + at + e)
                    .unwrap_or(tag_end - 1);
                rest[tag_start + at..end].to_string()
            });
        let raw = value.as_deref().unwrap_or_default();
        let unescaped = unescape_attr(raw);
        let value = Some(unescaped);
        let hit = value.as_deref().and_then(|v| match which {
            "internal-link" => resolve(v, map).map(page_path),
            _ => assets.get(&v.to_lowercase()).cloned(),
        });
        let tag = match (value, hit) {
            (Some(_), Some(target)) => {
                links += if which == "internal-link" { 1 } else { 0 };
                let tag = &rest[tag_start..tag_end];
                let tag = replace_attr(
                    tag,
                    if which == "internal-link" {
                        "href"
                    } else {
                        "src"
                    },
                    &rel_url(depth, &target),
                );
                if which == "internal-link" {
                    tag.replace(" target=\"_blank\" rel=\"noopener nofollow\"", "")
                } else {
                    tag
                }
            }
            (Some(_), None) if which == "internal-link" => {
                unresolved += 1;
                remove_attr(&rest[tag_start..tag_end], "href")
                    .replace(" target=\"_blank\" rel=\"noopener nofollow\"", "")
            }
            _ => {
                // * An embed with no resolvable source, or a tag the pass
                //   cannot read, is copied through untouched.
                rest[tag_start..tag_end].to_string()
            }
        };
        out.push_str(&rest[..tag_start]);
        out.push_str(&tag);
        rest = &rest[tag_end..];
    }
    out.push_str(rest);
    Rewritten {
        html: out,
        links,
        unresolved,
    }
}

/// Where the next rewritable tag — an `internal-link` anchor or an
/// `internal-embed` span — starts in `rest`, whichever comes first.
fn next_tag(rest: &str) -> Option<usize> {
    let link = rest.find("class=\"internal-link\"");
    let embed = rest.find("class=\"internal-embed\"");
    match (link, embed) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Replaces one attribute's value inside a tag, keeping the quotes.
///
/// The needle is anchored at a space: a bare `href="` also matches inside
/// `data-href="`, which would rewrite the data attribute and leave the real
/// href as the link text — a dead link on the exported page. The tags this
/// pass rewrites always carry other attributes first (the renderer emits
/// `class` before `href`, `alt` before `src`); the unanchored fallback only
/// runs when the name is the first attribute, where no longer name can
/// precede it.
fn replace_attr(tag: &str, name: &str, value: &str) -> String {
    for needle in [format!(" {name}=\""), format!("{name}=\"")] {
        if let Some(at) = tag.find(&needle) {
            let value_at = at + needle.len();
            let end = tag[value_at..]
                .find('"')
                .map(|i| value_at + i)
                .unwrap_or(value_at);
            return format!("{}{}{}", &tag[..value_at], value, &tag[end..]);
        }
    }
    tag.to_string()
}

/// Removes one attribute and its value from a tag.
fn remove_attr(tag: &str, name: &str) -> String {
    let needle = format!(" {name}=\"");
    match tag.find(&needle) {
        Some(at) => {
            let value_at = at + needle.len();
            let end = tag[value_at..]
                .find('"')
                .map(|i| value_at + i + 1)
                .unwrap_or(value_at);
            format!("{}{}", &tag[..at], &tag[end..])
        }
        None => tag.to_string(),
    }
}

/// Copies every non-note file — the attachments a page may embed — keeping
/// the folder structure. The config folders stay out.
fn copy_assets(root: &Path, files: &[VaultFile], out: &Path) -> Result<(), String> {
    for file in files {
        if file.kind == FileKind::Markdown {
            continue;
        }
        let rel = file.path.replace('\\', "/");
        if SKIPPED_DIRS
            .iter()
            .any(|d| rel.starts_with(&format!("{d}/")))
        {
            continue;
        }
        let from = root.join(&rel);
        let to = out.join(&rel);
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
        }
        std::fs::copy(&from, &to).map_err(|e| format!("Cannot copy {rel}: {e}"))?;
    }
    Ok(())
}

/// The stylesheet every page links. Deliberately small and quiet: a single
/// column, the code and quote shapes, nothing a theme would own.
const STYLE: &str = ":root { color-scheme: light dark; }
body { font: 16px/1.6 -apple-system, 'Segoe UI', Roboto, sans-serif; max-width: 44rem; margin: 2rem auto; padding: 0 1rem; }
h1, h2, h3 { line-height: 1.25; }
a { color: #4a6da7; }
img, video { max-width: 100%; height: auto; }
pre { background: #f4f4f4; padding: 0.75rem 1rem; border-radius: 6px; overflow-x: auto; }
@media (prefers-color-scheme: dark) { pre { background: #1f1f23; } body { background: #141417; color: #e6e6e6; } }
blockquote { border-inline-start: 3px solid #8888; margin-inline: 0; padding-inline-start: 1rem; color: inherit; opacity: 0.85; }
table { border-collapse: collapse; }
th, td { border: 1px solid #8884; padding: 0.3rem 0.6rem; }
a.internal-link[data-href]:not([href]) { color: #a33; }
";

#[cfg(test)]
mod tests {
    use super::*;

    fn note(path: &str) -> VaultFile {
        VaultFile {
            path: path.to_string(),
            kind: FileKind::Markdown,
            size: 0,
            mtime: 0,
            ctime: 0,
        }
    }

    const ANCHOR: &str = "<a data-href=\"{link}\" href=\"{link}\" class=\"internal-link\" target=\"_blank\" rel=\"noopener nofollow\">T</a>";

    fn anchor(link: &str) -> String {
        ANCHOR.replace("{link}", link)
    }

    #[test]
    fn the_link_map_prefers_the_shortest_path_for_a_shared_name() {
        let deep = note("Sub/Deep/Plan.md");
        let top = note("Plan.md");
        let map = link_map(&[&deep, &top]);
        assert_eq!(map["plan"], "Plan.md");
        assert_eq!(map["sub/deep/plan"], "Sub/Deep/Plan.md");
        // The extension spelled out resolves too, any case.
        assert_eq!(map["plan.md"], "Plan.md");
    }

    #[test]
    fn a_resolved_link_points_at_its_page_relatively_and_opens_in_place() {
        let map = link_map(&[&note("Target.md"), &note("Sub/Deep/Note.md")]);
        let empty = HashMap::new();
        // From a root page: the target is beside it.
        let r = rewrite(&anchor("Target"), &map, &empty, 0);
        assert_eq!((r.links, r.unresolved), (1, 0));
        assert!(r.html.contains("href=\"Target.html\""), "{}", r.html);
        assert!(!r.html.contains("_blank"), "{}", r.html);
        // From two folders down, the walk climbs twice.
        let r = rewrite(&anchor("Target"), &map, &empty, 2);
        assert!(r.html.contains("href=\"../../Target.html\""), "{}", r.html);
        // A subpath takes part in resolution but never in the URL.
        let r = rewrite(&anchor("Target#Heading"), &map, &empty, 0);
        assert!(r.html.contains("href=\"Target.html\""), "{}", r.html);
    }

    #[test]
    fn an_unresolved_link_loses_its_href_but_keeps_its_text() {
        let map = link_map(&[&note("Target.md")]);
        let empty = HashMap::new();
        let r = rewrite(&anchor("Nowhere"), &map, &empty, 0);
        assert_eq!((r.links, r.unresolved), (0, 1));
        assert!(r.html.contains("data-href=\"Nowhere\""), "{}", r.html);
        assert!(!r.html.contains(" href=\""), "{}", r.html);
        assert!(!r.html.contains("_blank"), "{}", r.html);
        assert!(r.html.contains(">T</a>"), "{}", r.html);
    }

    #[test]
    fn external_links_and_headings_pass_through() {
        let map = link_map(&[&note("Target.md")]);
        let empty = HashMap::new();
        let src = "<a data-tooltip-position=\"top\" aria-label=\"e.com\" rel=\"noopener nofollow\" class=\"external-link\" href=\"https://e.com\" target=\"_blank\">e</a>\n\
                   <h1 data-heading=\"H\">H</h1>\n".to_string()
            + &anchor("Target");
        let r = rewrite(&src, &map, &empty, 0);
        assert!(r.html.contains("href=\"https://e.com\""), "{}", r.html);
        assert!(r.html.contains("data-heading=\"H\""), "{}", r.html);
        assert_eq!((r.links, r.unresolved), (1, 0));
    }

    #[test]
    fn an_embed_of_a_copied_asset_points_at_it_and_a_note_embed_is_left() {
        let map = link_map(&[&note("Target.md")]);
        let mut assets = HashMap::new();
        assets.insert("pic.png".to_string(), "pic.png".to_string());
        let src = "<span alt=\"pic.png\" src=\"pic.png\" class=\"internal-embed\"></span>\n\
                   <span alt=\"Note\" src=\"Note\" class=\"internal-embed\"></span>";
        let r = rewrite(src, &map, &assets, 1);
        assert!(r.html.contains("src=\"../pic.png\""), "{}", r.html);
        // The note embed resolves to nothing publishable and stays as it was.
        assert!(r.html.contains("src=\"Note\""), "{}", r.html);
    }

    #[test]
    fn a_link_named_with_an_ampersand_resolves_through_the_escaped_attr() {
        let map = link_map(&[&note("A & B.md"), &note("Target.md")]);
        let empty = HashMap::new();
        // The renderer escapes the href, so the tag carries the entity.
        let src = anchor("A &amp; B");
        let r = rewrite(&src, &map, &empty, 0);
        assert_eq!((r.links, r.unresolved), (1, 0), "{}", r.html);
        assert!(r.html.contains("href=\"A & B.html\""), "{}", r.html);
        assert!(!r.html.contains("_blank"), "{}", r.html);
    }

    #[test]
    fn a_rewritten_href_survives_a_data_href_written_before_it() {
        let map = link_map(&[&note("Target.md")]);
        let empty = HashMap::new();
        // The renderer writes aria-label, then data-href, then href — a
        // bare `href="` needle matches inside `data-href="` and rewrites
        // the wrong attribute, leaving the real href as the link text.
        let src = "<a data-tooltip-position=\"top\" aria-label=\"Target\" data-href=\"Target\" href=\"Target\" class=\"internal-link\" target=\"_blank\" rel=\"noopener nofollow\">T</a>";
        let r = rewrite(src, &map, &empty, 0);
        assert_eq!((r.links, r.unresolved), (1, 0), "{}", r.html);
        assert!(r.html.contains(" href=\"Target.html\""), "{}", r.html);
        assert!(r.html.contains("data-href=\"Target\""), "{}", r.html);
        assert!(!r.html.contains("_blank"), "{}", r.html);
    }

    #[test]
    fn the_page_title_escapes_the_three_hostile_characters() {
        assert_eq!(escape_title("A & B"), "A &amp; B");
        assert_eq!(escape_title("<script>"), "&lt;script&gt;");
        assert_eq!(escape_title("plain"), "plain");
        assert_eq!(unescape_attr("A &amp; B"), "A & B");
        assert_eq!(unescape_attr("A & B"), "A & B");
        assert_eq!(unescape_attr("&#39;q&#39;"), "'q'");
    }
}
