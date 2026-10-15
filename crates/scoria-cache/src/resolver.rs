//! Link resolution with Obsidian's rules (`getFirstLinkpathDest`).
//!
//! 1. Strip `#subpath` and `|alias`.
//! 2. With a known extension, look for that exact file; without one, try `.md`
//!    then every other known extension.
//! 3. A linkpath containing `/` resolves from the vault root, then relative to
//!    the source file's folder.
//! 4. Otherwise look up the basename: prefer the source's folder, then the
//!    shortest path.
//! 5. Everything is case-insensitive; `\` is normalized to `/`.

use _formats::{link_resolution_extensions, FileKind};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Default, Clone)]
pub struct LinkResolver {
    /// Original-case vault-relative paths.
    files: BTreeSet<String>,
    /// Lowercased path -> original path.
    by_lower: HashMap<String, String>,
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

fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
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
        self.by_lower.insert(path.to_lowercase(), path.clone());
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
        self.by_lower.remove(&path.to_lowercase());
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
        self.by_lower
            .get(&candidate.to_lowercase())
            .map(String::as_str)
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
        let target = _markdown::parse_linktext(linktext).path;
        if target.is_empty() {
            return self.exact(&normalize(source_path));
        }
        let target = normalize(target);
        if target.contains('/') {
            if let Some(p) = self.with_extensions(&target) {
                return Some(p);
            }
            let source_norm = normalize(source_path);
            let folder = folder_of(&source_norm);
            let rel = if folder.is_empty() {
                target.clone()
            } else {
                format!("{folder}/{target}")
            };
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
        // Prefer markdown when the link has no extension, then same folder, then shortest path.
        let score = |p: &&String| {
            let md = if want_ext.is_none() && FileKind::from_path(p).is_markdown() {
                0
            } else {
                1
            };
            let same = if folder_of(p).to_lowercase() == source_folder {
                0
            } else {
                1
            };
            (md, same, p.len(), p.to_lowercase())
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
            Some("Projects/Note.md")
        );
        assert_eq!(
            r.resolve("Plan", "Deep/Nested/X.md"),
            Some("Deep/Nested/Plan.md")
        );
        assert_eq!(r.resolve("Plan", "Other.md"), Some("Projects/Plan.md"));
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
}
