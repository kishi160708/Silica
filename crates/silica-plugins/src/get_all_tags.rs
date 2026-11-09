//! Get all tags: every tag in the vault, with counts, as an editable index.
//!
//! The tag pane shows the vault's tags one merge at a time, and nothing
//! writes the whole index down where it can be read, diffed or corrected.
//! This command does: it walks the vault, indexes every note with the same
//! `silica-cache` engine the app uses, and writes one note —
//!
//! - **`All tags.md`** — the tag index as block-style YAML frontmatter,
//!   each tag with its occurrence count, above the same tags as a plain
//!   sorted list for reading.
//!
//! The tag scanner is `silica-cache`'s own: `index_markdown` finds the tags
//! (inline and frontmatter, merged case-insensitively the way the tag pane
//! merges them), and this plugin only reads that index back. The YAML is
//! block style, so the note parses back to the same data through the cache
//! — the index round-trips — and a hand edit of the block stays valid YAML.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use silica_cache::{index_markdown, MetadataCache};

use crate::native::{NativeCommand, Plugin, PluginContext};

/// The note the command writes, at the vault root.
const NOTE: &str = "All tags.md";

/// The command's registered id: the `silica:` prefix marks it as this
/// app's own, so it can never collide with an Obsidian id a vault's
/// `hotkeys.json` means for something else.
const COMMAND_ID: &str = "silica:get-all-tags";

pub struct GetAllTags;

impl Plugin for GetAllTags {
    fn id(&self) -> &'static str {
        "silica:plugin-getalltags"
    }

    fn name(&self) -> &'static str {
        "Get all tags"
    }

    fn commands(&self) -> Vec<NativeCommand> {
        vec![NativeCommand {
            id: COMMAND_ID.into(),
            name: "Get all tags: Write a tag index".into(),
        }]
    }

    fn run_command(
        &self,
        id: &str,
        _value: Option<&Value>,
        ctx: &PluginContext<'_>,
    ) -> Result<(), String> {
        if id != COMMAND_ID {
            return Err(format!("'{id}' is not a command this plugin registered"));
        }
        let index = collect_tags(ctx)?;
        (ctx.write_file)(NOTE, &render_note(&index)?)?;
        (ctx.open_note)(NOTE)?;
        // * The collected tags are what the command is for, so say how many
        //   were found and where they were written.
        (ctx.notify)(&format!(
            "{} tag{} written to {NOTE}",
            index.tags.len(),
            plural(index.tags.len())
        ));
        Ok(())
    }
}

/// One tag as the index carries it: the name as it was first written
/// (without `#`), and how often it occurs across the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCount {
    pub name: String,
    pub count: usize,
}

/// The collected index: how many notes were read and every distinct tag
/// with its count, in tag-pane order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagIndex {
    pub notes: usize,
    pub tags: Vec<TagCount>,
}

/// Reads every note through the context and collects the vault's tags from
/// the cache's tag index.
///
/// The scanner is `silica-cache`'s own: each note is indexed once with
/// `index_markdown`, and `MetadataCache` merges inline and frontmatter tags
/// case-insensitively — the same merge the tag pane shows. This function
/// reads that index; it never scans for tags itself.
fn collect_tags(ctx: &PluginContext<'_>) -> Result<TagIndex, String> {
    let mut notes: Vec<String> = Vec::new();
    collect_markdown(ctx, ".", &mut notes, 0)?;
    notes.sort();
    notes.dedup();
    // * The index note describes the vault's own notes, not itself: once it
    //   exists the walk finds it, and without this the count it reports
    //   would move on every rerun over the same vault.
    notes.retain(|path| path != NOTE);

    let mut cache = MetadataCache::default();
    for path in &notes {
        let text = (ctx.read_file)(path)?;
        cache.insert_file(path.clone(), index_markdown(&text));
    }

    Ok(TagIndex {
        notes: cache.len(),
        // * The cache's index is keyed by lowercased name, so reading it in
        //   order is the tag pane's order.
        tags: cache
            .tags()
            .values()
            .map(|entry| TagCount {
                name: entry.name.clone(),
                count: entry.count,
            })
            .collect(),
    })
}

/// Every `.md` path under `dir`, found by asking `list_dir` one folder at a
/// time.
///
/// `list_dir` answers for a single folder, so a folder is recognised by
/// answering with a listing of its own: an entry that is not a note is
/// asked again, and one that answers with an error is a file this does not
/// index. The depth cap is a runaway guard — a vault is not nested 64 deep,
/// and a symlink loop is.
fn collect_markdown(
    ctx: &PluginContext<'_>,
    dir: &str,
    out: &mut Vec<String>,
    depth: u32,
) -> Result<(), String> {
    if depth > 64 {
        return Err("the vault is nested too deeply to index".to_string());
    }
    let entries = match (ctx.list_dir)(dir) {
        Ok(entries) => entries,
        // * A file is not a folder; that is not an error here.
        Err(_) => return Ok(()),
    };
    for entry in entries {
        // * `.obsidian`, `.trash`, `.git`: configuration and trash, not
        //   notes. The host may or may not put a trailing `/` on a folder,
        //   so trim it before looking at the name.
        let name = entry.trim_end_matches('/');
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        let path = if dir == "." {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        };
        if name.ends_with(".md") {
            out.push(path);
        } else {
            collect_markdown(ctx, &path, out, depth + 1)?;
        }
    }
    Ok(())
}

/// The note the command writes.
///
/// The index goes in the frontmatter as block-style YAML, which the cache
/// parses back to the same data — the index round-trips through the app's
/// own reader — and a hand edit of the block stays valid YAML. Below it,
/// the same tags as a plain list with their counts.
fn render_note(index: &TagIndex) -> Result<String, String> {
    let mut out = String::from(
        "---\n\
         # Written by the Get all tags command; the next run rewrites this\n\
         # note. The YAML block is safe to edit by hand.\n",
    );
    out.push_str(&render_yaml(index)?);
    out.push_str("---\n\n# All tags\n\n");
    let notes = index.notes;
    if index.tags.is_empty() {
        out.push_str(&format!(
            "No tags in this vault ({notes} note{} scanned).\n",
            plural(notes)
        ));
    } else {
        out.push_str(&format!(
            "{} tag{} across {notes} note{}.\n\n",
            index.tags.len(),
            plural(index.tags.len()),
            plural(notes)
        ));
        for tag in &index.tags {
            out.push_str(&format!("- {} ({})\n", tag.name, tag.count));
        }
    }
    Ok(out)
}

/// The index as block-style YAML.
///
/// `serde_yaml` writes block style and quotes a value only when its
/// characters would otherwise change the parse, so the block a reader sees
/// is the data a parser gets back.
fn render_yaml(index: &TagIndex) -> Result<String, String> {
    serde_yaml::to_string(index).map_err(|e| e.to_string())
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// A context over a fixed vault, recording what the command wrote,
    /// opened and said.
    ///
    /// `list_dir` answers from the given map and errors for anything else,
    /// which is how a file answers: a folder answers with its own listing,
    /// so the walk separates the two the same way the app's host does.
    struct Harness {
        written: Rc<RefCell<HashMap<String, String>>>,
        opened: Rc<RefCell<Vec<String>>>,
        notices: Rc<RefCell<Vec<String>>>,
    }

    impl Harness {
        fn new(notes: &[(&str, &str)], dirs: &[(&str, &[&str])]) -> (Self, PluginContext<'static>) {
            let written: Rc<RefCell<HashMap<String, String>>> =
                Rc::new(RefCell::new(HashMap::new()));
            let opened: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let notices: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let written_c = Rc::clone(&written);
            let opened_c = Rc::clone(&opened);
            let notices_c = Rc::clone(&notices);
            let bodies: HashMap<String, String> = notes
                .iter()
                .map(|(path, body)| ((*path).to_string(), (*body).to_string()))
                .collect();
            let listings: HashMap<String, Vec<String>> = dirs
                .iter()
                .map(|(dir, entries)| {
                    (
                        (*dir).to_string(),
                        entries.iter().map(|e| (*e).to_string()).collect(),
                    )
                })
                .collect();
            let ctx = PluginContext {
                vault_root: std::path::Path::new("."),
                read_file: Box::new(move |path| {
                    bodies
                        .get(path)
                        .cloned()
                        .ok_or_else(|| format!("no such note: {path}"))
                }),
                write_file: Box::new(move |path, text| {
                    written_c
                        .borrow_mut()
                        .insert(path.to_string(), text.to_string());
                    Ok(())
                }),
                append_file: Box::new(|_, _| Ok(())),
                create_file: Box::new(|_, _| Ok(())),
                create_folder: Box::new(|_| Ok(())),
                delete_file: Box::new(|_| Ok(())),
                rename_file: Box::new(|_, _| Ok(())),
                list_dir: Box::new(move |dir| {
                    listings
                        .get(dir)
                        .cloned()
                        .ok_or_else(|| format!("not a folder: {dir}"))
                }),
                file_exists: Box::new(|_| Ok(false)),
                metadata_cache: Box::new(|_| Err("no metadata".to_string())),
                open_note: Box::new(move |path| {
                    opened_c.borrow_mut().push(path.to_string());
                    Ok(())
                }),
                current_note: Box::new(|| None),
                notify: Box::new(move |line: &str| {
                    notices_c.borrow_mut().push(line.to_string());
                }),
            };
            (
                Self {
                    written,
                    opened,
                    notices,
                },
                ctx,
            )
        }
    }

    /// The tags are collected from the cache's own index, so a frontmatter
    /// tag and an inline tag of the same name merge, the way the tag pane
    /// merges them, and nested tags come along.
    #[test]
    fn collects_every_tag_with_its_count() {
        let (_, ctx) = Harness::new(
            &[
                (
                    "A.md",
                    "---\ntags: [Project, inbox]\n---\n#project #inbox/weekly body\n",
                ),
                ("sub/B.md", "#Project again, and #Project once more\n"),
            ],
            &[(".", &["A.md", "sub/"]), ("sub", &["B.md"])],
        );

        let index = collect_tags(&ctx).expect("the vault is readable");

        // * Case-insensitive merge, like the tag pane: `Project` is written
        //   in A.md's frontmatter and inline as `#project`, and twice more
        //   inline in B.md.
        let got: Vec<(&str, usize)> = index
            .tags
            .iter()
            .map(|tag| (tag.name.as_str(), tag.count))
            .collect();
        assert_eq!(got, vec![("inbox", 1), ("inbox/weekly", 1), ("Project", 4)]);
        assert_eq!(index.notes, 2);
    }

    /// The walk finds nested notes and leaves configuration, trash and
    /// attachments alone.
    #[test]
    fn the_walk_finds_nested_notes_and_skips_the_rest() {
        let (_, ctx) = Harness::new(
            &[],
            &[
                (".", &["A.md", "sub/", ".obsidian/", "pic.png"]),
                ("sub", &["B.md"]),
            ],
        );

        let mut notes = Vec::new();
        collect_markdown(&ctx, ".", &mut notes, 0).expect("the walk answers");

        assert_eq!(notes, vec!["A.md", "sub/B.md"]);
    }

    /// The YAML block is block style, and it parses back to the index it
    /// was rendered from — the round trip the note is written for.
    #[test]
    fn the_yaml_block_round_trips() {
        let index = TagIndex {
            notes: 5,
            tags: vec![
                TagCount {
                    name: "Project".into(),
                    count: 3,
                },
                TagCount {
                    name: "inbox/weekly".into(),
                    count: 1,
                },
                // * A name YAML reads as a boolean or a number stays a
                //   string: it is quoted, or the parse would come back
                //   changed.
                TagCount {
                    name: "yes".into(),
                    count: 2,
                },
            ],
        };

        let yaml = render_yaml(&index).expect("the index serializes");

        assert!(yaml.starts_with("notes: 5\ntags:\n- name:"), "{yaml}");
        assert!(
            !yaml.contains('[') && !yaml.contains('{'),
            "block style, not flow style: {yaml}"
        );
        let back: TagIndex = serde_yaml::from_str(&yaml).expect("the block parses back");
        assert_eq!(back, index);
    }

    /// The written note round-trips through the cache's own frontmatter
    /// reader: re-indexing the note parses the same index back out.
    #[test]
    fn the_note_round_trips_through_the_cache_index() {
        let index = TagIndex {
            notes: 5,
            tags: vec![
                TagCount {
                    name: "Project".into(),
                    count: 3,
                },
                TagCount {
                    name: "inbox".into(),
                    count: 1,
                },
            ],
        };

        let note = render_note(&index).expect("the note renders");
        let meta = index_markdown(&note);
        let frontmatter = meta.frontmatter.expect("the frontmatter parsed");
        let back: TagIndex =
            serde_yaml::from_value(frontmatter).expect("the frontmatter is the index");

        assert_eq!(back, index);
        // * The list below the block is for reading the same data.
        assert!(note.contains("- Project (3)"), "{note}");
    }

    /// The command writes the index note, opens it, and says how many tags
    /// it found.
    #[test]
    fn the_command_writes_the_index_note_and_says_the_count() {
        let (harness, ctx) = Harness::new(
            &[("A.md", "#alpha #beta"), ("B.md", "#alpha")],
            &[(".", &["A.md", "B.md"])],
        );

        GetAllTags
            .run_command(COMMAND_ID, None, &ctx)
            .expect("the command ran");

        let written = harness.written.borrow();
        let note = written.get(NOTE).expect("the index note was written");
        assert!(note.contains("- name: alpha"), "{note}");
        assert!(note.contains("count: 2"), "alpha occurs twice: {note}");
        assert_eq!(harness.opened.borrow().as_slice(), [NOTE]);
        let notices = harness.notices.borrow();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("2 tags"), "{:?}", notices);
    }

    /// The index note itself contributes no tags: its frontmatter `tags`
    /// is a list of mappings, not tag names, and its body has no inline
    /// tags, so collecting again finds the same index — down to the note
    /// count, because the index note is not one of the vault's own notes.
    #[test]
    fn the_index_note_contributes_no_tags_of_its_own() {
        let (_, first_ctx) = Harness::new(&[("A.md", "#alpha")], &[(".", &["A.md"])]);
        let first = collect_tags(&first_ctx).expect("the first read");
        let note = render_note(&first).expect("the note renders");

        let (_, again_ctx) = Harness::new(
            &[("A.md", "#alpha"), (NOTE, &note)],
            &[(".", &["A.md", NOTE])],
        );
        let second = collect_tags(&again_ctx).expect("the second read");

        assert_eq!(second.tags, first.tags);
        assert_eq!(second.notes, first.notes);
    }

    /// An unknown id is refused, the way every native plugin refuses it.
    #[test]
    fn an_unknown_command_is_refused() {
        let (_, ctx) = Harness::new(&[], &[(".", &[])]);
        let error = GetAllTags
            .run_command("silica:get-all-tags:typo", None, &ctx)
            .expect_err("the plugin does not own that id");
        assert!(
            error.contains("not a command this plugin registered"),
            "{error}"
        );
    }
}
