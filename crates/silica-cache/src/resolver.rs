//! Link resolution with Obsidian's rules (`getFirstLinkpathDest`).
//!
//! 1. Strip `#subpath` and `|alias`.
//! 2. With a known extension, look for that exact file; without one, try `.md`
//!    then every other known extension.
//! 3. A linkpath starting with `/` resolves from the vault root only; one
//!    starting with `./` or `../` resolves relative to the source file's
//!    folder only (observed: `[A](./A.md)` breaks once the source moves into
//!    a subfolder, `[A](/A.md)` does not); any other linkpath containing `/`
//!    resolves from the vault root, then relative to the source's folder.
//! 4. Otherwise look up the basename: a file at the vault root wins, then
//!    one in the source's own folder, then the shortest path (in characters),
//!    then path order. Observed in Obsidian 1.14.4 from `Long/Way/Down/C.md`:
//!    `[[Q]]` is `Q.md` over `Long/Way/Down/Q.md`, `[[X]]` is
//!    `Long/Way/Down/X.md` over `Mid/X.md`, `[[W]]` is `Long/Way/W.md` over
//!    `Aaaa/Bbbb/W.md`, `[[N]]` is `Aaaa/N.md` over `Zzzz/N.md`.
//! 5. Everything is case-insensitive; `\` is normalized to `/`.

use silica_formats::{link_resolution_extensions, FileKind};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Default, Clone)]
pub struct LinkResolver {
    /// Original-case vault-relative paths.
    files: BTreeSet<String>,
    /// Lowercased path -> every original path that lowercases to it.
    ///
    /// On a case-sensitive filesystem `A.md` and `a.md` are different notes
    /// that share one lowercase key. Storing a single `String` let the second
    /// insert replace the first, and removing either deleted the mapping the
    /// other still needed; a `Vec` keeps both and lets resolution pick.
    by_lower: HashMap<String, Vec<String>>,
    /// Lowercased basename (without extension) -> original paths.
    by_basename: HashMap<String, Vec<String>>,
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

fn basename_no_ext(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    }
}

/// The folder part of a vault path (`""` at the root).
pub fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// Joins `rel` onto `folder`, resolving `.` and `..` segments; `None` when
/// `..` would leave the vault.
fn join_relative(folder: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = folder.split('/').filter(|p| !p.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

impl LinkResolver {
    pub fn new<I, S>(paths: I) -> LinkResolver
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut r = LinkResolver::default();
        for p in paths {
            r.insert(p.as_ref());
        }
        r
    }

    pub fn insert(&mut self, path: &str) {
        let path = normalize(path);
        if !self.files.insert(path.clone()) {
            return;
        }
        let lowered = path.to_lowercase();
        let bucket = self.by_lower.entry(lowered).or_default();
        if !bucket.contains(&path) {
            bucket.push(path.clone());
        }
        self.by_basename
            .entry(basename_no_ext(&path).to_lowercase())
            .or_default()
            .push(path);
    }

    pub fn remove(&mut self, path: &str) {
        let path = normalize(path);
        if !self.files.remove(&path) {
            return;
        }
        if let Some(bucket) = self.by_lower.get_mut(&path.to_lowercase()) {
            bucket.retain(|p| p != &path);
            if bucket.is_empty() {
                self.by_lower.remove(&path.to_lowercase());
            }
        }
        if let Some(v) = self
            .by_basename
            .get_mut(&basename_no_ext(&path).to_lowercase())
        {
            v.retain(|p| p != &path);
        }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn exact(&self, candidate: &str) -> Option<&str> {
        let bucket = self.by_lower.get(&candidate.to_lowercase())?;
        // * A case-sensitive vault can hold `A.md` and `a.md` at once. When
        // * the link names one of them exactly, that is the note meant; only
        // * an inexact spelling falls back to the bucket's first entry, which
        // * is insertion order and therefore stable across runs.
        bucket
            .iter()
            .find(|path| path.as_str() == candidate)
            .or_else(|| bucket.first())
            .map(String::as_str)
    }

    /// Every original-case path that `candidate` could name.
    ///
    /// Resolution takes the first, but a caller that has to report an
    /// ambiguity — or a test asserting both notes survived a removal — needs
    /// them all.
    pub fn all_exact(&self, candidate: &str) -> &[String] {
        self.by_lower
            .get(&candidate.to_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Resolves a wikilink body that names a block — `[[#^id]]` or
    /// `[[path#^id]]` — to the vault path of the note holding it and the
    /// block id. A bare `[[#^id]]` (empty path) names the current note.
    /// Returns `None` when the subpath is not a block reference or the note
    /// path does not resolve in this vault.
    pub fn resolve_block_link<'a>(
        &self,
        linktext: &'a str,
        source_path: &str,
    ) -> Option<(String, &'a str)> {
        let parsed = silica_markdown::parse_linktext(linktext);
        let block_id = match parsed.subpath {
            Some(silica_markdown::linktext::Subpath::Block(id)) => id,
            _ => return None,
        };
        let path = if parsed.path.is_empty() {
            normalize(source_path).to_string()
        } else {
            self.resolve(linktext, source_path)?.to_string()
        };
        Some((path, block_id))
    }

    fn with_extensions(&self, stem: &str) -> Option<&str> {
        if FileKind::from_path(stem) != FileKind::Other {
            if let Some(p) = self.exact(stem) {
                return Some(p);
            }
        }
        link_resolution_extensions().find_map(|ext| self.exact(&format!("{stem}.{ext}")))
    }

    /// Resolves the body of a wikilink (alias and subpath allowed) from `source_path`.
    pub fn resolve(&self, linktext: &str, source_path: &str) -> Option<&str> {
        let target = silica_markdown::parse_linktext(linktext).path;
        if target.is_empty() {
            return self.exact(&normalize(source_path));
        }
        let raw = target.replace('\\', "/");
        if let Some(abs) = raw.strip_prefix('/') {
            return self.with_extensions(&normalize(abs));
        }
        if raw.starts_with("./") || raw.starts_with("../") || raw == "." || raw == ".." {
            let source_norm = normalize(source_path);
            let joined = join_relative(folder_of(&source_norm), &raw)?;
            return self.with_extensions(&joined);
        }
        let target = normalize(target);
        if target.contains('/') {
            if let Some(p) = self.with_extensions(&target) {
                return Some(p);
            }
            let source_norm = normalize(source_path);
            let rel = join_relative(folder_of(&source_norm), &target)?;
            return self.with_extensions(&rel);
        }
        let candidates = self
            .by_basename
            .get(&basename_no_ext(&target).to_lowercase())?;
        let want_ext = target.rsplit_once('.').map(|(_, e)| e.to_lowercase());
        let matching: Vec<&String> = candidates
            .iter()
            .filter(|p| match &want_ext {
                Some(ext) if FileKind::from_extension(ext) != FileKind::Other => {
                    p.to_lowercase().ends_with(&format!(".{ext}"))
                }
                _ => true,
            })
            .collect();
        if matching.is_empty() {
            return None;
        }
        let source_folder = folder_of(&normalize(source_path)).to_lowercase();
        // * Prefer markdown when the link has no extension, then the vault
        // * root, then the source's folder, then the shortest path, then an
        // * exact case match.
        //
        // * The case key is what separates `A.md` from `a.md` on a
        // * case-sensitive vault: the lowercased path ties for both, so
        // * without it the winner was whichever was inserted first — stable,
        // * but a link written `[[a]]` would open `A.md`.
        let want_case = target.clone();
        let score = |p: &&String| {
            let md = if want_ext.is_none() && FileKind::from_path(p).is_markdown() {
                0
            } else {
                1
            };
            let folder = folder_of(p).to_lowercase();
            let root = if folder.is_empty() { 0 } else { 1 };
            let same = if folder == source_folder { 0 } else { 1 };
            let exact_case = if p.as_str() == want_case || basename_no_ext(p) == want_case {
                0
            } else {
                1
            };
            (md, root, same, exact_case, p.len(), p.to_lowercase())
        };
        matching.into_iter().min_by_key(score).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver() -> LinkResolver {
        LinkResolver::new([
            "Note.md",
            "Projects/Note.md",
            "Projects/Plan.md",
            "Deep/Nested/Plan.md",
            "Assets/pic.png",
            "Board.canvas",
            "Projects/Note.canvas",
        ])
    }

    #[test]
    fn shortest_path_wins_then_same_folder() {
        let r = resolver();
        assert_eq!(r.resolve("Note", "Other.md"), Some("Note.md"));
        assert_eq!(
            r.resolve("Note", "Projects/Plan.md"),
            Some("Note.md"),
            "the shorter root path beats the same folder"
        );
        assert_eq!(
            r.resolve("Plan", "Deep/Nested/X.md"),
            Some("Deep/Nested/Plan.md"),
            "the source's folder beats a shorter path elsewhere"
        );
        assert_eq!(r.resolve("Plan", "Other.md"), Some("Projects/Plan.md"));
        // ? Observed in Obsidian 1.14.4 (see the module docs).
        let r = LinkResolver::new([
            "Aa/Dup.md",
            "Bb/Dup.md",
            "Zz/Dup.md",
            "Zed.md",
            "Long/Way/Down/Zed.md",
            "Long/Deep.md",
            "Long/Way/Deep.md",
            "Mid/X.md",
            "Long/Way/Down/X.md",
            "Other/Z.md",
            "Long/Way/Z.md",
            "Aaaa/Bbbb/W.md",
            "Long/Way/W.md",
            "P.md",
            "Long/Way/Down/P.png",
        ]);
        let deep = "Long/Way/Down/C.md";
        assert_eq!(r.resolve("Dup", "Bb/C.md"), Some("Bb/Dup.md"), "own folder");
        assert_eq!(r.resolve("Dup", "Root.md"), Some("Aa/Dup.md"), "path order");
        assert_eq!(
            r.resolve("Zed", deep),
            Some("Zed.md"),
            "root beats own folder"
        );
        assert_eq!(
            r.resolve("Deep", deep),
            Some("Long/Deep.md"),
            "shortest elsewhere"
        );
        assert_eq!(
            r.resolve("Deep", "Long/Way/Deep.md"),
            Some("Long/Way/Deep.md")
        );
        assert_eq!(
            r.resolve("X", deep),
            Some("Long/Way/Down/X.md"),
            "own folder beats shorter"
        );
        assert_eq!(
            r.resolve("Z", deep),
            Some("Other/Z.md"),
            "shorter beats the parent folder"
        );
        assert_eq!(
            r.resolve("W", deep),
            Some("Long/Way/W.md"),
            "length in characters"
        );
        assert_eq!(r.resolve("P", deep), Some("P.md"), "markdown first");
        assert_eq!(r.resolve("P.png", deep), Some("Long/Way/Down/P.png"));
    }

    #[test]
    fn case_insensitive_and_extension_aware() {
        let r = resolver();
        assert_eq!(r.resolve("note#Heading|alias", "x.md"), Some("Note.md"));
        assert_eq!(r.resolve("PIC.PNG", "x.md"), Some("Assets/pic.png"));
        assert_eq!(r.resolve("Board.canvas", "x.md"), Some("Board.canvas"));
        assert_eq!(
            r.resolve("Note.canvas", "x.md"),
            Some("Projects/Note.canvas")
        );
        assert_eq!(r.resolve("Missing", "x.md"), None);
    }

    #[test]
    fn paths_resolve_from_root_then_relative() {
        let r = resolver();
        assert_eq!(r.resolve("projects/note", "x.md"), Some("Projects/Note.md"));
        assert_eq!(
            r.resolve("Nested/Plan", "Deep/Other.md"),
            Some("Deep/Nested/Plan.md")
        );
        assert_eq!(
            r.resolve("#Heading", "Projects/Plan.md"),
            Some("Projects/Plan.md")
        );
    }

    #[test]
    fn dot_segments_are_relative_only_and_a_leading_slash_is_the_root() {
        let r = resolver();
        assert_eq!(r.resolve("./Note", "Other.md"), Some("Note.md"));
        assert_eq!(
            r.resolve("./Note.md", "Projects/Plan.md"),
            Some("Projects/Note.md")
        );
        assert_eq!(r.resolve("../Note", "Projects/Plan.md"), Some("Note.md"));
        assert_eq!(
            r.resolve("../Note", "Other.md"),
            None,
            "cannot leave the vault"
        );
        assert_eq!(r.resolve("/Note.md", "Projects/Plan.md"), Some("Note.md"));
        assert_eq!(
            r.resolve("/Plan", "Projects/Plan.md"),
            None,
            "no basename lookup from `/`"
        );
        assert_eq!(
            r.resolve("../Nested/Plan", "Deep/Other/X.md"),
            Some("Deep/Nested/Plan.md")
        );
    }

    /// On a case-sensitive filesystem `A.md` and `a.md` are two notes with one
    /// lowercase key. Both must resolve, and removing one must leave the other.
    #[test]
    fn case_collisions_keep_both_notes() {
        let mut r = LinkResolver::new(["A.md", "a.md"]);

        assert_eq!(r.all_exact("A.md").len(), 2, "both notes are remembered");
        // * An exactly-cased link resolves to that note, not its neighbour.
        assert_eq!(r.resolve("A", "Note.md"), Some("A.md"));
        assert_eq!(r.resolve("a", "Note.md"), Some("a.md"));

        // * Removing one leaves the other intact.
        r.remove("A.md");
        assert_eq!(r.all_exact("A.md"), &["a.md".to_string()]);
        assert_eq!(r.resolve("a", "Note.md"), Some("a.md"));
        assert_eq!(
            r.resolve("A", "Note.md"),
            Some("a.md"),
            "the survivor still answers"
        );

        r.remove("a.md");
        assert!(r.all_exact("a.md").is_empty());
        assert_eq!(r.resolve("a", "Note.md"), None);
    }

    #[test]
    fn block_links_resolve_to_note_and_block_id() {
        let r = resolver();
        assert_eq!(
            r.resolve_block_link("#^x1", "Note.md"),
            Some(("Note.md".to_string(), "x1"))
        );
        assert_eq!(
            r.resolve_block_link("Note#^x1", "Projects/Plan.md"),
            Some(("Note.md".to_string(), "x1"))
        );
        // A heading subpath is not a block reference.
        assert_eq!(r.resolve_block_link("Note#Heading", "Note.md"), None);
        // An unknown note resolves to nothing.
        assert_eq!(r.resolve_block_link("Missing#^x1", "Note.md"), None);
    }
}
