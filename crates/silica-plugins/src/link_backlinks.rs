//! Links and backlinks: the notes that point at the open one, the links
//! that point at nothing, and every tag in the workspace.
//!
//! Obsidian keeps these in three side panes — linked mentions, unlinked
//! mentions and the tag list. `silica:plugin-links-backlinks` answers all
//! three in one command, and it does so over the `silica-cache` engine
//! the rest of the app already runs on: `MetadataCache` (which owns the
//! [`LinkResolver`] and Obsidian's `getFirstLinkpathDest` rules) and the
//! same markdown indexer the editor and the reading view read their links
//! from. Nothing here scans a vault a second time with rules of its own.
//!
//! The answers go out as notices, the way a command that gathers
//! something has to: `run_command` reports only success or failure, so a
//! computed list would otherwise be dropped on the floor.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use silica_cache::{index_markdown, LinkResolver, MetadataCache};

use crate::native::{NativeCommand, Plugin, PluginContext};

/// The command this plugin answers to.
const COMMAND: &str = "plugin-links-backlinks:show";

/// A listing the palette can read is capped in width, with an ellipsis.
const LIST_WIDTH: usize = 200;

/// A workspace is not nested this deep; a symlink loop is.
const MAX_DEPTH: u32 = 64;

/// Backlinks, unresolved links and the tag list for one workspace.
struct LinkReport {
    /// The open note's path, as the index stores it (empty with none open).
    current: String,
    /// Notes whose links, embeds or properties resolve to the open note.
    backlinks: Vec<String>,
    /// Link text that resolves to no file, and the notes carrying it.
    unresolved: BTreeMap<String, Vec<String>>,
    /// Every tag in the workspace, with how many times it occurs.
    tags: Vec<(String, usize)>,
    /// How many notes were read.
    notes: usize,
}

/// Backlinks, unresolved links and the workspace tag list.
pub struct LinksBacklinks;

impl Plugin for LinksBacklinks {
    fn id(&self) -> &'static str {
        "silica:plugin-links-backlinks"
    }

    fn name(&self) -> &'static str {
        "Links & backlinks"
    }

    fn commands(&self) -> Vec<NativeCommand> {
        vec![NativeCommand {
            id: COMMAND.into(),
            name: "Show links, backlinks and tags".into(),
        }]
    }

    fn run_command(
        &self,
        id: &str,
        _value: Option<&Value>,
        ctx: &PluginContext<'_>,
    ) -> Result<(), String> {
        match id {
            COMMAND => {
                let report = survey(ctx)?;
                for line in render(&report) {
                    (ctx.notify)(&line);
                }
                Ok(())
            }
            _ => Err(format!("'{id}' is not a command this plugin registered")),
        }
    }
}

/// Reads the workspace through the context and answers the three questions.
///
/// Every file goes into the resolver — attachments included, so a link to
/// `pic.png` is not reported broken just because only notes are indexed —
/// while only markdown is read and indexed.
fn survey(ctx: &PluginContext<'_>) -> Result<LinkReport, String> {
    let mut files: Vec<String> = Vec::new();
    collect_files(ctx, ".", &mut files, 0)?;
    files.sort();
    files.dedup();

    let resolver = LinkResolver::new(files.iter().map(String::as_str));
    let mut indexed: Vec<(String, silica_cache::CachedMetadata)> = Vec::new();
    for path in files.iter().filter(|p| p.ends_with(".md")) {
        let text = (ctx.read_file)(path)?;
        indexed.push((path.clone(), index_markdown(&text)));
    }
    let notes = indexed.len();
    let cache = MetadataCache::new(resolver, indexed);

    let open = (ctx.current_note)().unwrap_or_default();
    // * Windows is case-insensitive: the note on screen may be spelled
    // * differently than the index stores it, and the backlink edges are
    // * keyed by the stored spelling.
    let current = stored_path(cache.resolver(), &open).to_string();

    Ok(LinkReport {
        backlinks: backlinks_of(&cache, &current),
        current,
        unresolved: unresolved_of(&cache),
        tags: tags_of(&cache),
        notes,
    })
}

/// The notes that link to the open note, the note itself excluded: a note
/// is not a backlink of itself however many `[[self]]` links it carries.
fn backlinks_of(cache: &MetadataCache, current: &str) -> Vec<String> {
    if current.is_empty() {
        return Vec::new();
    }
    let mut sources: Vec<String> = cache
        .sources_linking_to(current)
        .filter(|source| *source != current)
        .map(str::to_string)
        .collect();
    sources.sort();
    sources.dedup();
    sources
}

/// Link text that resolves to no file, and the notes that carry it.
fn unresolved_of(cache: &MetadataCache) -> BTreeMap<String, Vec<String>> {
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, _) in cache.files() {
        let Some(links) = cache.unresolved_links(path) else {
            continue;
        };
        for text in links.keys() {
            found
                .entry(text.clone())
                .or_default()
                .insert(path.to_string());
        }
    }
    found
        .into_iter()
        .map(|(text, sources)| (text, sources.into_iter().collect()))
        .collect()
}

/// Every tag in the workspace with its occurrence count, in name order.
fn tags_of(cache: &MetadataCache) -> Vec<(String, usize)> {
    cache
        .tags()
        .values()
        .map(|entry| (entry.name.clone(), entry.count))
        .collect()
}

/// The path as the resolver stores it, so a differently spelled open note
/// still finds its backlinks.
fn stored_path<'a>(resolver: &'a LinkResolver, path: &'a str) -> &'a str {
    if path.is_empty() {
        return path;
    }
    resolver
        .all_exact(path)
        .iter()
        .find(|stored| stored.as_str() == path)
        .or_else(|| resolver.all_exact(path).first())
        .map(String::as_str)
        .unwrap_or(path)
}

/// The notice lines for one survey: the backlink count, the unresolved
/// targets, and the tag list.
fn render(report: &LinkReport) -> Vec<String> {
    if report.notes == 0 {
        return vec!["No notes in this workspace".to_string()];
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "{} note{} scanned",
        report.notes,
        plural(report.notes)
    ));
    if report.current.is_empty() {
        lines.push("No note is open".to_string());
    } else {
        match report.backlinks.len() {
            0 => lines.push("No backlinks".to_string()),
            1 => lines.push(format!("1 backlink: {}", report.backlinks[0])),
            n => lines.push(format!("{n} backlinks: {}", listing(&report.backlinks))),
        }
    }
    if report.unresolved.is_empty() {
        lines.push("No unresolved links".to_string());
    } else {
        let targets: Vec<String> = report.unresolved.keys().cloned().collect();
        lines.push(format!(
            "{} unresolved links: {}",
            targets.len(),
            listing(&targets)
        ));
    }
    if report.tags.is_empty() {
        lines.push("No tags".to_string());
    } else {
        let named: Vec<String> = report
            .tags
            .iter()
            .map(|(name, count)| {
                if *count > 1 {
                    format!("#{name} ({count})")
                } else {
                    format!("#{name}")
                }
            })
            .collect();
        lines.push(format!("{} tags: {}", named.len(), listing(&named)));
    }
    lines
}

/// A comma-joined listing, capped to a width the palette can show.
fn listing(items: &[String]) -> String {
    let mut out = items.join(", ");
    if out.len() > LIST_WIDTH {
        let mut end = LIST_WIDTH;
        while !out.is_char_boundary(end) {
            end -= 1;
        }
        out.truncate(end);
        out.push('…');
    }
    out
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Every file under `folder`, nested ones included.
///
/// `list_dir` answers for one folder and a file answers with an error, so
/// a folder is recognised by asking for its own listing; a name arriving
/// with a trailing `/` is a folder by its spelling. Dot folders —
/// `.obsidian`, `.trash`, `.git` — hold configuration and trash, not notes.
fn collect_files(
    ctx: &PluginContext<'_>,
    folder: &str,
    out: &mut Vec<String>,
    depth: u32,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("the workspace is nested too deeply to scan".to_string());
    }
    let entries = (ctx.list_dir)(folder).map_err(|e| format!("cannot read {folder}: {e}"))?;
    for entry in entries {
        let name = entry.trim_end_matches('/');
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        let child = join(folder, name);
        if entry.ends_with('/') || (ctx.list_dir)(&child).is_ok() {
            collect_files(ctx, &child, out, depth + 1)?;
        } else {
            out.push(child);
        }
    }
    Ok(())
}

/// A child path under `folder` (`""` and `"."` are the workspace root).
fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() || folder == "." {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// A context over a fixed set of files, recording notices.
    ///
    /// `list_dir` answers for a folder and errors for a file, the shape
    /// the app's own closure has; folder entries carry the trailing `/`.
    struct Harness {
        notices: Rc<RefCell<Vec<String>>>,
    }

    impl Harness {
        fn new(notes: &[(&str, &str)], open: Option<&str>) -> (Self, PluginContext<'static>) {
            let notices = Rc::new(RefCell::new(Vec::new()));
            let shown = Rc::clone(&notices);
            let bodies: HashMap<String, String> = notes
                .iter()
                .map(|(p, b)| (p.to_string(), b.to_string()))
                .collect();
            let paths: Vec<String> = bodies.keys().cloned().collect();
            let mut folders: BTreeSet<String> = BTreeSet::new();
            for path in &paths {
                let mut at = path.as_str();
                while let Some(i) = at.rfind('/') {
                    at = &at[..i];
                    folders.insert(at.to_string());
                }
            }
            let open = open.map(str::to_string);
            let ctx = PluginContext {
                vault_root: std::path::Path::new("."),
                read_file: Box::new(move |p| {
                    bodies
                        .get(p)
                        .cloned()
                        .ok_or_else(|| format!("no such file: {p}"))
                }),
                write_file: Box::new(|_, _| Ok(())),
                append_file: Box::new(|_, _| Ok(())),
                create_file: Box::new(|_, _| Ok(())),
                create_folder: Box::new(|_| Ok(())),
                delete_file: Box::new(|_| Ok(())),
                rename_file: Box::new(|_, _| Ok(())),
                list_dir: Box::new(move |dir: &str| {
                    let dir = dir.trim_matches('/');
                    let dir = if dir == "." { "" } else { dir };
                    // * A file answers with an error, not an empty folder.
                    if !(dir.is_empty() || folders.contains(dir)) {
                        return Err(format!("not a folder: {dir}"));
                    }
                    let prefix = if dir.is_empty() {
                        String::new()
                    } else {
                        format!("{dir}/")
                    };
                    let mut out: Vec<String> = Vec::new();
                    for path in &paths {
                        if let Some(rest) = path.strip_prefix(&prefix) {
                            if !rest.is_empty() && !rest.contains('/') {
                                out.push(rest.to_string());
                            }
                        }
                    }
                    for folder in &folders {
                        if let Some(rest) = folder.strip_prefix(&prefix) {
                            if !rest.is_empty() && !rest.contains('/') {
                                out.push(format!("{rest}/"));
                            }
                        }
                    }
                    out.sort();
                    out.dedup();
                    Ok(out)
                }),
                file_exists: Box::new(|_| Ok(true)),
                metadata_cache: Box::new(|_| Err("unused".to_string())),
                open_note: Box::new(|_| Ok(())),
                current_note: Box::new(move || open.clone()),
                notify: Box::new(move |line: &str| shown.borrow_mut().push(line.to_string())),
            };
            (Self { notices }, ctx)
        }

        fn shown(&self) -> Vec<String> {
            self.notices.borrow().clone()
        }
    }

    /// Backlinks counted through the command: the linking note is named,
    /// and a note's own links do not make it its own backlink.
    #[test]
    fn backlinks_count_the_notes_linking_to_the_open_one() {
        let (harness, ctx) = Harness::new(
            &[
                ("A.md", "points at [[B]] and at itself [[A]]"),
                ("B.md", "nothing here"),
            ],
            Some("B.md"),
        );
        LinksBacklinks.run_command(COMMAND, None, &ctx).unwrap();

        let shown = harness.shown();
        assert_eq!(shown[0], "2 notes scanned", "{shown:?}");
        assert_eq!(shown[1], "1 backlink: A.md", "{shown:?}");

        // * The self-link must not count as a backlink of the open note.
        let (harness, ctx) = Harness::new(
            &[
                ("A.md", "points at [[B]] and at itself [[A]]"),
                ("B.md", "nothing here"),
            ],
            Some("A.md"),
        );
        LinksBacklinks.run_command(COMMAND, None, &ctx).unwrap();
        assert_eq!(harness.shown()[1], "No backlinks");
    }

    /// Unresolved links are listed once per target, with every note that
    /// carries them; an attachment the resolver knows is not broken.
    #[test]
    fn unresolved_links_are_listed_with_their_sources() {
        let (harness, ctx) = Harness::new(
            &[
                ("A.md", "[[Missing]] and ![[pic.png]] and [[C#Heading]]"),
                ("B.md", "[[Missing]] here too"),
                ("C.md", "resolves fine"),
                ("pic.png", "not a note"),
            ],
            Some("C.md"),
        );
        LinksBacklinks.run_command(COMMAND, None, &ctx).unwrap();

        let report = survey(&ctx).unwrap();
        let missing: Vec<&String> = report
            .unresolved
            .get("Missing")
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(missing, vec![&"A.md".to_string(), &"B.md".to_string()]);
        assert_eq!(report.unresolved.len(), 1, "{:?}", report.unresolved);
        assert!(
            harness.shown()[2].starts_with("1 unresolved links: Missing"),
            "{:?}",
            harness.shown()
        );
    }

    /// Tags from note bodies and frontmatter alike, counted across the
    /// workspace, nested folders included.
    #[test]
    fn tags_are_listed_across_the_workspace() {
        let (harness, ctx) = Harness::new(
            &[
                ("A.md", "#alpha twice #alpha\n"),
                ("B.md", "---\ntags: [beta]\n---\n#alpha\n"),
                ("Sub/C.md", "#gamma\n"),
            ],
            None,
        );
        LinksBacklinks.run_command(COMMAND, None, &ctx).unwrap();

        let report = survey(&ctx).unwrap();
        assert_eq!(
            report.tags,
            vec![
                ("alpha".to_string(), 3),
                ("beta".to_string(), 1),
                ("gamma".to_string(), 1),
            ]
        );
        let shown = harness.shown();
        assert!(
            shown[3].starts_with("3 tags: #alpha (3), #beta, #gamma"),
            "{shown:?}"
        );
    }

    /// The note on screen may be spelled differently than the index
    /// stores it; its backlinks are still found.
    #[test]
    fn backlinks_survive_a_case_mismatch_on_the_open_note() {
        let (_harness, ctx) = Harness::new(
            &[("A.md", "points at [[B]]"), ("B.md", "nothing here")],
            Some("b.md"),
        );
        let report = survey(&ctx).unwrap();
        assert_eq!(report.current, "B.md");
        assert_eq!(report.backlinks, vec!["A.md".to_string()]);
    }

    /// A workspace with nothing in it says so instead of listing nothing.
    #[test]
    fn an_empty_workspace_says_so() {
        let (harness, ctx) = Harness::new(&[], None);
        LinksBacklinks.run_command(COMMAND, None, &ctx).unwrap();
        assert_eq!(
            harness.shown(),
            vec!["No notes in this workspace".to_string()]
        );
    }

    /// An id this plugin did not register is an error, not a silent no-op.
    #[test]
    fn an_unknown_command_is_an_error() {
        let (_harness, ctx) = Harness::new(&[("A.md", "")], Some("A.md"));
        let result = LinksBacklinks.run_command("plugin-links-backlinks:nope", None, &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("plugin-links-backlinks:nope"));
    }
}
